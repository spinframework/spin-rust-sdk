# gRPC with Tonic

A Spin HTTP component that serves gRPC endpoints using [tonic](https://github.com/hyperium/tonic),
demonstrating both **unary** and **server-streaming** RPCs.

Tonic's generated server types implement `tower::Service<http::Request<B>>`,
so we can forward Spin's incoming `Request` (which is `http::Request<IncomingBody>`)
directly without any body-type conversion.

## Prerequisites

- [protoc](https://grpc.io/docs/protoc-installation/) (Protocol Buffers compiler)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`

## Build and Run

```sh
spin build -u
```

## Test with grpcurl

> **Note:** These commands pass `-import-path` and `-proto` because the server
> does not implement gRPC server reflection.

Unary call:

```sh
grpcurl -plaintext \
  -import-path proto -proto greeter.proto \
  -d '{"name": "Spin"}' \
  localhost:3000 greeter.Greeter/SayHello
```

Server-streaming call:

```sh
grpcurl -plaintext \
  -import-path proto -proto greeter.proto \
  -d '{"name": "Spin"}' \
  localhost:3000 greeter.Greeter/SayHelloStream
```

## How It Works

The key insight is that tonic can run **without its default `transport` feature**.
With only `codegen` and `prost`, tonic generates a tower service that handles
gRPC framing and protobuf serialization—no hyper server or tokio runtime needed.

### Unary RPC

```rust
#[http_service]
async fn handler(req: Request) -> impl IntoResponse {
    spin_sdk::grpc::serve(GreeterServer::new(MyGreeter), req).await
}
```

### Server-Streaming RPC

The streaming RPC returns a `futures::stream::Iter` over a vec of replies.
Tonic encodes each item as a length-prefixed gRPC frame and streams them
on the HTTP response body:

```rust
type SayHelloStreamStream = futures::stream::Iter<...>;

async fn say_hello_stream(
    &self,
    request: tonic::Request<HelloRequest>,
) -> Result<tonic::Response<Self::SayHelloStreamStream>, tonic::Status> {
    let name = request.into_inner().name;
    let greetings = vec![
        Ok(HelloReply { message: format!("Hello, {name}!") }),
        Ok(HelloReply { message: format!("Bonjour, {name}!") }),
        // ...
    ];
    Ok(tonic::Response::new(futures::stream::iter(greetings)))
}
```

The generated `GreeterServer<T>` implements:
```rust
impl<T, B> Service<http::Request<B>> for GreeterServer<T>
where
    T: Greeter,
    B: Body + Send + 'static,
    B::Error: Into<StdError> + Send + 'static,
```

Spin's `IncomingBody` satisfies all of these bounds.
