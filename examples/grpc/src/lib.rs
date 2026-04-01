use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;

mod proto {
    tonic::include_proto!("greeter");
}

use proto::greeter_server::{Greeter, GreeterServer};
use proto::{HelloReply, HelloRequest};

use std::pin::Pin;

struct MyGreeter;

type ResponseStream =
    Pin<Box<dyn futures::Stream<Item = Result<HelloReply, tonic::Status>> + Send>>;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    /// Unary RPC — single request, single response.
    async fn say_hello(
        &self,
        request: tonic::Request<HelloRequest>,
    ) -> Result<tonic::Response<HelloReply>, tonic::Status> {
        let name = request.into_inner().name;
        let reply = HelloReply {
            message: format!("Hello, {name}!"),
        };
        Ok(tonic::Response::new(reply))
    }

    type SayHelloStreamStream = ResponseStream;

    /// Server-streaming RPC — single request, stream of responses.
    ///
    /// Returns multiple greetings in different languages for the
    /// requested name. Each `HelloReply` is sent as a separate
    /// gRPC frame on the response stream.
    async fn say_hello_stream(
        &self,
        request: tonic::Request<HelloRequest>,
    ) -> Result<tonic::Response<Self::SayHelloStreamStream>, tonic::Status> {
        let name = request.into_inner().name;

        let stream = async_stream::stream! {
            for greeting in ["Hello", "Hola", "Bonjour", "Ciao", "こんにちは"] {
                yield Ok(HelloReply {
                    message: format!("{greeting}, {name}!"),
                });
            }
        };

        Ok(tonic::Response::new(Box::pin(stream)))
    }
}

/// A Spin HTTP component that serves gRPC endpoints via tonic.
///
/// Tonic's generated `GreeterServer` implements `tower::Service<http::Request<B>>`
/// for any body type satisfying `http_body::Body + Send + 'static`. Spin's
/// incoming `Request` (an `http::Request<IncomingBody>`) meets these bounds,
/// so we can forward it directly.
///
/// This example demonstrates both unary and server-streaming RPCs.
#[http_service]
async fn handler(req: Request) -> impl IntoResponse {
    spin_sdk::grpc::serve(GreeterServer::new(MyGreeter), req).await
}
