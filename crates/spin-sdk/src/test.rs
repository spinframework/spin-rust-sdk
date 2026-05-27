wasmtime::component::bindgen!({
    path: "wit/deps/spin-redis@3.0.0",
    inline: "package root:component; world redis-trigger { export spin:redis/inbound-redis@3.0.0; }",
    imports: {
        default: async,
    },
    exports: {
        default: async,
    }
});

use {
    anyhow::Result,
    http_body_util::BodyExt,
    std::sync::OnceLock,
    tokio::{fs, process::Command},
    wasmtime::{
        Config, Engine, Store,
        component::{Component, Linker, ResourceTable},
    },
    wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView},
    wasmtime_wasi_http::{WasiHttpCtx, p3::WasiHttpView},
};

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    wasi_http: WasiHttpCtx,
}

impl WasiHttpView for Ctx {
    fn http(&mut self) -> wasmtime_wasi_http::p3::WasiHttpCtxView<'_> {
        wasmtime_wasi_http::p3::WasiHttpCtxView {
            hooks: wasmtime_wasi_http::p3::default_hooks(),
            table: &mut self.table,
            ctx: &mut self.wasi_http,
        }
    }
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

async fn build_component(name: &str) -> Result<Vec<u8>> {
    assert!(
        Command::new("cargo")
            .current_dir(format!("test-cases/{name}"))
            .args(["build", "--workspace", "--target", "wasm32-wasip2"])
            .status()
            .await
            .unwrap()
            .success(),
        "cargo build failed"
    );

    let out_file = format!(
        "test-cases/{name}/target/wasm32-wasip2/debug/{}.wasm",
        name.replace('-', "_")
    );
    Ok(fs::read(&out_file).await?)
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();

    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model_async(true);

        Engine::new(&config).unwrap()
    })
}

fn store_and_linker() -> Result<(Store<Ctx>, Linker<Ctx>)> {
    let mut linker = Linker::new(engine());

    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    Ok((
        Store::new(
            engine(),
            Ctx {
                table: ResourceTable::new(),
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                wasi_http: WasiHttpCtx::new(),
            },
        ),
        linker,
    ))
}

#[tokio::test]
async fn simple_http() -> Result<()> {
    let component = Component::new(engine(), build_component("simple-http").await?)?;

    let (mut store, linker) = store_and_linker()?;

    let (request, fut) = wasmtime_wasi_http::p3::Request::from_http(
        crate::http::Request::post("http://localhost:3000").body(crate::http::FullBody::new(
            bytes::Bytes::copy_from_slice(b"test"),
        ))?,
    );

    let http_service = wasmtime_wasi_http::p3::bindings::Service::instantiate_async(
        &mut store, &component, &linker,
    )
    .await?;

    let (status, resp_bytes) = store
        .run_concurrent(async |accessor| {
            let resp_fut = async {
                let resp = http_service.handle(accessor, request).await??;
                let resp = accessor.with(|access| resp.into_http(access, async { Ok(()) }))?;
                let status = resp.status();
                let body = resp.into_body().collect().await?.to_bytes();
                anyhow::Ok((status, body))
            };
            let (status_body, ()) = tokio::try_join!(resp_fut, async {
                fut.await.map_err(|e| anyhow::anyhow!("{e:?}"))
            })?;
            anyhow::Ok(status_body)
        })
        .await??;

    assert!(status.is_success());
    assert_eq!(resp_bytes.as_ref(), b"Hello, world!");

    Ok(())
}

#[tokio::test]
async fn simple_redis() -> Result<()> {
    let component = Component::new(engine(), build_component("simple-redis").await?)?;

    let (mut store, linker) = store_and_linker()?;

    let trigger = RedisTrigger::instantiate_async(&mut store, &component, &linker).await?;

    let result = store
        .run_concurrent(async |accessor| {
            trigger
                .spin_redis_inbound_redis()
                .call_handle_message(accessor, b"foo".to_vec())
                .await
        })
        .await??;

    result.expect("redis trigger should have returned Ok");

    Ok(())
}

mod http_router {
    use super::*;
    use crate::http::{FullBody, Request as HttpRequest};
    use bytes::Bytes;
    use hyperium::{Method, StatusCode};
    use tokio::sync::OnceCell;

    static COMPONENT_BYTES: OnceCell<Vec<u8>> = OnceCell::const_new();

    async fn component() -> Result<Component> {
        let bytes = COMPONENT_BYTES
            .get_or_try_init(|| async { build_component("http-router").await })
            .await?;
        Ok(Component::new(engine(), bytes)?)
    }

    async fn dispatch(
        component: &Component,
        request: HttpRequest<FullBody<Bytes>>,
    ) -> Result<(StatusCode, Bytes)> {
        let (mut store, linker) = store_and_linker()?;
        let (request, fut) = wasmtime_wasi_http::p3::Request::from_http(request);
        let http_service = wasmtime_wasi_http::p3::bindings::Service::instantiate_async(
            &mut store, component, &linker,
        )
        .await?;

        store
            .run_concurrent(async |accessor| {
                let resp_fut = async {
                    let resp = http_service.handle(accessor, request).await??;
                    let resp = accessor.with(|access| resp.into_http(access, async { Ok(()) }))?;
                    let status = resp.status();
                    let body = resp.into_body().collect().await?.to_bytes();
                    anyhow::Ok((status, body))
                };
                let (status_body, ()) = tokio::try_join!(resp_fut, async {
                    fut.await.map_err(|e| anyhow::anyhow!("{e:?}"))
                })?;
                anyhow::Ok(status_body)
            })
            .await?
    }

    fn empty(method: Method, path: &str) -> HttpRequest<FullBody<Bytes>> {
        HttpRequest::builder()
            .method(method)
            .uri(format!("http://localhost{path}"))
            .body(FullBody::new(Bytes::new()))
            .unwrap()
    }

    #[tokio::test]
    async fn named_capture() -> Result<()> {
        let component = component().await?;
        let (status, body) = dispatch(&component, empty(Method::GET, "/hello/spin")).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), b"hello, spin");
        Ok(())
    }

    #[tokio::test]
    async fn multi_capture() -> Result<()> {
        let component = component().await?;
        let (status, body) = dispatch(&component, empty(Method::GET, "/multiply/6/7")).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), b"42");
        Ok(())
    }

    #[tokio::test]
    async fn wildcard() -> Result<()> {
        let component = component().await?;
        let (status, body) = dispatch(&component, empty(Method::GET, "/wild/a/b/c")).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), b"a/b/c");
        Ok(())
    }

    #[tokio::test]
    async fn echo_post_body() -> Result<()> {
        let component = component().await?;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("http://localhost/echo")
            .body(FullBody::new(Bytes::copy_from_slice(b"ping")))
            .unwrap();
        let (status, body) = dispatch(&component, request).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), b"ping");
        Ok(())
    }

    #[tokio::test]
    async fn put_handler_registered_without_macro() -> Result<()> {
        let component = component().await?;
        let (status, body) = dispatch(&component, empty(Method::PUT, "/widgets/abc")).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), b"updated widget abc");
        Ok(())
    }

    #[tokio::test]
    async fn any_method_via_closure() -> Result<()> {
        let component = component().await?;
        let (status, body) = dispatch(&component, empty(Method::DELETE, "/teapot")).await?;
        assert_eq!(status, StatusCode::IM_A_TEAPOT);
        assert_eq!(body.as_ref(), b"short and stout");
        Ok(())
    }

    #[tokio::test]
    async fn method_not_allowed_when_path_matches_other_method() -> Result<()> {
        let component = component().await?;
        // /multiply/:x/:y is registered for GET only; the catch-all `/*` exists
        // but the router should prefer 405 for a path that matches a known
        // route under a different method.
        let (status, _) = dispatch(&component, empty(Method::POST, "/multiply/2/3")).await?;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        Ok(())
    }

    #[tokio::test]
    async fn catch_all_for_unknown_path() -> Result<()> {
        let component = component().await?;
        let (status, _) = dispatch(&component, empty(Method::GET, "/nope")).await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn head_falls_back_to_get() -> Result<()> {
        let component = component().await?;
        // No HEAD handler is registered; the router should reuse the GET
        // handler for /hello/:name.
        let (status, _) = dispatch(&component, empty(Method::HEAD, "/hello/spin")).await?;
        assert_eq!(status, StatusCode::OK);
        Ok(())
    }
}
