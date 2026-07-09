# http-middleware

This example shows how to write an HTTP **middleware** component with the
`spin-sdk`.

A middleware sits in front of another component. It receives the incoming
request, can modify it, forwards it to the next handler in the chain with
`spin_sdk::http::next`, and can then modify the response on the way back out:

```
client ──> middleware ──> service ──> middleware ──> client
             (request phase)   (response phase)
```

## How it works

Two components make up this app:

- [`middleware`](middleware/src/lib.rs) adds an `x-added-by-middleware` header
  to the request, calls `http::next` to forward it, then adds an
  `x-processed-by-middleware` header to the response.
- [`service`](service/src/lib.rs) is the terminal component. It echoes back the
  request headers it received, so you can see the header the middleware added.

The chain is wired up in [`spin.toml`](spin.toml). The middleware is attached to
the `service` trigger via `dependencies.middleware`:

```toml
[[trigger.http]]
route = "/..."
component = "service"
dependencies.middleware = [{ component = "middleware" }]
```

Middleware are applied outermost-first, so you can list several and they will
each wrap the next: `dependencies.middleware = [{ component = "a" }, { component = "b" }]`
sends the request through `a`, then `b`, then the target component.

Forwarding to the next handler requires the `http-middleware` feature of the
SDK, which enables `spin_sdk::http::next`:

```toml
spin-sdk = { path = "...", features = ["http-middleware"] }
```

## Running

> Note: middleware composition relies on the `wasi:http/handler@0.3.0`
> interface. You need a Spin runtime built against the same WASIp3 version as
> this SDK; older runtimes that expect an earlier `wasi:http/handler` release
> will fail to resolve the middleware dependency.

```
spin up --build
```

Then send a request and inspect the headers:

```
curl -v localhost:3000/
```

You will see the response header added by the middleware:

```
< x-processed-by-middleware: spin-rust-sdk
```

and the body, printed by the service, will include the request header the
middleware injected before the request reached it:

```
Hello from the wrapped service!

Request headers seen by this component:
  ...
  x-added-by-middleware: spin-rust-sdk
```
