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
    std::{sync::OnceLock, time::Duration},
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
    wasmtime_wasi::p3::clocks::add_to_linker(&mut linker)?;

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

async fn http_response(component_name: &str) -> Result<(hyperium::StatusCode, bytes::Bytes)> {
    let component = Component::new(engine(), build_component(component_name).await?)?;

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

    let response = store.run_concurrent(async |accessor| {
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
    });
    let result = tokio::time::timeout(Duration::from_secs(5), response)
        .await
        .map_err(|_| anyhow::anyhow!("HTTP component did not finish within 5 seconds"))?;
    result?
}

#[tokio::test]
async fn simple_http() -> Result<()> {
    let (status, resp_bytes) = http_response("simple-http").await?;

    assert!(status.is_success());
    assert_eq!(resp_bytes.as_ref(), b"Hello, world!");

    Ok(())
}

#[tokio::test]
async fn spawned_task_progresses_without_host_wakeup() -> Result<()> {
    let (status, resp_bytes) = http_response("spawn-progress").await?;

    assert!(status.is_success());
    assert_eq!(resp_bytes.as_ref(), b"spawned child completed");

    Ok(())
}

#[test]
#[allow(deprecated)]
fn legacy_spawn_retains_unit_return_and_non_send_input() {
    type LocalTask = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;
    let _: fn(LocalTask) = crate::wasip3::spawn;
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
