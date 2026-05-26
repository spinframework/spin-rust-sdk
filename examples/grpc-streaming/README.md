# gRPC Streaming

A Spin HTTP component that serves gRPC endpoints using [tonic](https://github.com/hyperium/tonic),
demonstrating all combinations of unary and streaming requests and responses.

## Prerequisites

- [protoc](https://grpc.io/docs/protoc-installation/) (Protocol Buffers compiler)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`

## Build and Run

```sh
spin up --build --sqlite @db.sql
```

## Test with grpcurl

> **Note:** These commands pass `-import-path` and `-proto` because the server
> does not implement gRPC server reflection.

Unary call:

```sh
grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"latitude":18,"longitude":19}' localhost:3000 routeguide.RouteGuide/GetFeature
```

Server-streaming call:

```sh
 grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"lo":{"latitude":12,"longitude":10},"hi":{"latitude":28,"longitude":25}}' localhost:3000 routeguide.RouteGuide/ListFeatures
```

Client-streaming call:

```sh
grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"latitude":18,"longitude":19}{"latitude":12,"longitude":20}{"latitude":13,"longitude":17}{"latitude":14,"longitude":18}' localhost:3000 routeguide.RouteGuide/RecordRoute
```

Client- and server-streaming:

```sh
grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"location":{"latitude":18,"longitude":19},"message":"hello from fang rock!"}{"location":{"latitude":12,"longitude":20},"message":"i summited mt hobbes!"}' localhost:3000 routeguide.RouteGuide/RouteChat
grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"location":{"latitude":18,"longitude":19},"message":"farewell from fang rock!"}' localhost:3000 routeguide.RouteGuide/RouteChat
grpcurl -plaintext -import-path proto -proto route_guide.proto -d '{"location":{"latitude":12,"longitude":20},"message":"i was impaled on sharp claws at mt hobbes!"}{"location":{"latitude":18,"longitude":19},"message":"i got nibbled by rutans at fang rock!"}' localhost:3000 routeguide.RouteGuide/RouteChat
```

(Multiple commands because the routeguide scenario needs you to build up some history at a location to see interesting responses.)
