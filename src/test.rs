wasmtime::component::bindgen!({
    path: "wit",
    world: "fermyon:spin/redis-trigger",
    imports: {
        default: async,
    },
    exports: {
        default: async,
    }
});

use {
    anyhow::{anyhow, bail, Context, Result},
    http_body_util::{combinators::BoxBody, BodyExt, Empty},
    hyper::Request,
    std::{ops::Deref, sync::OnceLock},
    tokio::{
        fs,
        process::Command,
        sync::{oneshot, OnceCell},
        task,
    },
    wasmtime::{
        component::{Component, Linker, ResourceTable},
        Config, Engine, Store,
    },
    wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView},
    wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView},
    wit_component::ComponentEncoder,
};

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    wasi_http: WasiHttpCtx,
}

impl WasiHttpView for Ctx {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.wasi_http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
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
    static BUILD: OnceCell<()> = OnceCell::const_new();

    BUILD
        .get_or_init(|| async {
            assert!(
                Command::new("cargo")
                    .current_dir("test-cases")
                    .args(["build", "--workspace", "--target", "wasm32-wasip1"])
                    .status()
                    .await
                    .unwrap()
                    .success(),
                "cargo build failed"
            );
        })
        .await;

    const ADAPTER_PATH: &str = "adapters/ab5a4484/wasi_snapshot_preview1.reactor.wasm";

    ComponentEncoder::default()
        .validate(true)
        .module(&fs::read(format!("target/wasm32-wasip1/debug/{name}.wasm")).await?)?
        .adapter("wasi_snapshot_preview1", &fs::read(ADAPTER_PATH).await?)?
        .encode()
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();

    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.async_support(true);

        Engine::new(&config).unwrap()
    })
}

fn store_and_linker() -> Result<(Store<Ctx>, Linker<Ctx>)> {
    let mut linker = Linker::new(engine());

    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;
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
    use wasmtime_wasi_http::bindings::http::types::Scheme;

    let component = Component::new(engine(), build_component("simple_http").await?)?;

    let (mut store, linker) = store_and_linker()?;

    let request = Request::builder()
        .method(hyper::Method::GET)
        .uri("http://test.com:8080/")
        .body(BoxBody::new(Empty::new().map_err(|_| unreachable!())))?;

    let request = store
        .data_mut()
        .new_incoming_request(Scheme::Http, request)?;

    let (response_tx, response_rx) = oneshot::channel();
    let response = store.data_mut().new_response_outparam(response_tx)?;

    let proxy =
        wasmtime_wasi_http::bindings::Proxy::instantiate_async(&mut store, &component, &linker)
            .await?;

    let handle = task::spawn(async move {
        proxy
            .wasi_http_incoming_handler()
            .call_handle(&mut store, request, response)
            .await
    });

    let response = match response_rx.await {
        Ok(response) => response.context("guest failed to produce a response")?,

        Err(_) => {
            handle
                .await
                .context("guest invocation panicked")?
                .context("guest invocation failed")?;

            bail!("guest failed to produce a response prior to returning")
        }
    };

    assert!(response.status().is_success());
    assert_eq!(
        response.into_body().collect().await?.to_bytes().deref(),
        b"Hello, world!"
    );

    handle
        .await
        .context("guest invocation panicked")?
        .context("guest invocation failed")?;

    Ok(())
}

#[tokio::test]
async fn simple_redis() -> Result<()> {
    let component = Component::new(engine(), build_component("simple_redis").await?)?;

    let (mut store, linker) = store_and_linker()?;

    let trigger = RedisTrigger::instantiate_async(&mut store, &component, &linker).await?;

    trigger
        .fermyon_spin_inbound_redis()
        .call_handle_message(&mut store, &b"foo".to_vec())
        .await?
        .map_err(|e| anyhow!("{e}"))?;

    Ok(())
}
