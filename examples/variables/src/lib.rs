use spin_sdk::{
    http::{IntoResponse, Request},
    http_service, variables,
};

/// This endpoint returns the config value specified by key.
#[http_service]
async fn get(req: Request) -> anyhow::Result<impl IntoResponse> {
    if req.uri().path().contains("dotenv") {
        let val = variables::get("dotenv".to_string())
            .await
            .expect("Failed to acquire dotenv from spin.toml");
        return Ok(http::Response::builder().status(200).body(val).unwrap());
    }
    let val = format!("message: {}", variables::get("message".to_string()).await?);
    Ok(http::Response::builder().status(200).body(val).unwrap())
}
