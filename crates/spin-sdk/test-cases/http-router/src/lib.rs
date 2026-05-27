use bytes::Bytes;
use http_body_util::BodyExt;
use spin_sdk::{
    http::{
        router::{Params, Router},
        FullBody, IntoResponse, Request, StatusCode,
    },
    http_service,
};

fn reply(status: StatusCode, body: impl Into<Bytes>) -> impl IntoResponse {
    (status, FullBody::new(body.into()))
}

#[http_service]
async fn handle(req: Request) -> impl IntoResponse {
    let mut router = Router::new();
    router.get("/hello/:name", hello);
    router.post("/echo", echo);
    router.get("/multiply/:x/:y", multiply);
    router.get("/wild/*", wild);
    router.any("/teapot", |_req: Request, _params: Params| async {
        reply(StatusCode::IM_A_TEAPOT, "short and stout")
    });
    router.put("/widgets/:id", update_widget);
    router.handle(req).await
}

async fn hello(_req: Request, params: Params) -> impl IntoResponse {
    let name = params.get("name").unwrap_or("world").to_owned();
    reply(StatusCode::OK, format!("hello, {name}"))
}

async fn echo(req: Request, _params: Params) -> impl IntoResponse {
    let bytes = req
        .into_body()
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    reply(StatusCode::OK, bytes)
}

async fn multiply(_req: Request, params: Params) -> impl IntoResponse {
    let x: i64 = params.get("x").and_then(|v| v.parse().ok()).unwrap_or(0);
    let y: i64 = params.get("y").and_then(|v| v.parse().ok()).unwrap_or(0);
    reply(StatusCode::OK, format!("{}", x * y))
}

async fn wild(_req: Request, params: Params) -> impl IntoResponse {
    reply(StatusCode::OK, params.wildcard().unwrap_or("").to_owned())
}

async fn update_widget(_req: Request, params: Params) -> impl IntoResponse {
    let id = params.get("id").unwrap_or("?").to_owned();
    reply(StatusCode::OK, format!("updated widget {id}"))
}
