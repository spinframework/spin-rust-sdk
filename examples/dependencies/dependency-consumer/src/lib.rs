use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

spin_sdk::imports_for!("dependency-consumer");

#[http_component]
fn handle_dependency_consumer(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let sum = calculator_dep::calculator::calc::addition::add(2, 2);
    let message = format!("two and two is - as it always was - {sum}!\n");
    let louder = loudness_services_dep::loudness_services::yelling::yelling::yell(&message);
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(louder)
        .build())
}
