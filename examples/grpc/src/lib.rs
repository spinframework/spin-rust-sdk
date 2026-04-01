use spin_sdk::http::{IntoResponse, Request};
use spin_sdk::http_service;

mod proto {
    tonic::include_proto!("greeter");
}

use proto::greeter_server::{Greeter, GreeterServer};
use proto::{HelloReply, HelloRequest};

struct MyGreeter;

type ResponseStream = futures::stream::Iter<std::vec::IntoIter<Result<HelloReply, tonic::Status>>>;

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

        let greetings = vec![
            Ok(HelloReply {
                message: format!("Hello, {name}!"),
            }),
            Ok(HelloReply {
                message: format!("Hola, {name}!"),
            }),
            Ok(HelloReply {
                message: format!("Bonjour, {name}!"),
            }),
            Ok(HelloReply {
                message: format!("Ciao, {name}!"),
            }),
            Ok(HelloReply {
                message: format!("こんにちは, {name}!"),
            }),
        ];

        let stream = futures::stream::iter(greetings);
        Ok(tonic::Response::new(stream))
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
