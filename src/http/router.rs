// This router implementation is heavily inspired by the `Endpoint` type in the https://github.com/http-rs/tide project.

use super::conversions::{IntoResponse, TryFromRequest, TryIntoRequest};
use super::{responses, Method, Request, Response};
use async_trait::async_trait;
use routefinder::{Captures, Router as MethodRouter};
use std::future::Future;
use std::{collections::HashMap, fmt::Display};

/// An HTTP request handler.
///  
/// This trait is automatically implemented for `Fn` types, and so is rarely implemented
/// directly by Spin users.
#[async_trait(?Send)]
pub trait Handler {
    /// Invoke the handler.
    async fn handle(&self, req: Request, params: Params) -> Response;
}

#[async_trait(?Send)]
impl Handler for Box<dyn Handler> {
    async fn handle(&self, req: Request, params: Params) -> Response {
        self.as_ref().handle(req, params).await
    }
}

#[async_trait(?Send)]
impl<F, Fut> Handler for F
where
    F: Fn(Request, Params) -> Fut + 'static,
    Fut: Future<Output = Response> + 'static,
{
    async fn handle(&self, req: Request, params: Params) -> Response {
        let fut = (self)(req, params);
        fut.await
    }
}

/// Route parameters extracted from a URI that match a route pattern.
pub type Params = Captures<'static, 'static>;

/// Routes HTTP requests within a Spin component.
///
/// Routes may contain wildcards:
///
/// * `:name` is a single segment wildcard. The handler can retrieve it using
///   [Params::get()].
/// * `*` is a trailing wildcard (matches anything). The handler can retrieve it
///   using [Params::wildcard()].
///
/// If a request matches more than one route, the match is selected according to the follow criteria:
///
/// * An exact route takes priority over any wildcard.
/// * A single segment wildcard takes priority over a trailing wildcard.
///
/// (This is the same logic as overlapping routes in the Spin manifest.)
///
/// # Examples
///
/// Handle GET requests to a path with a wildcard, falling back to "not found":
///
/// ```no_run
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let mut router = Router::new();
///     router.get("/hello/:planet", hello_planet);
///     router.any("/*", not_found);
///     router.handle(req)
/// }
///
/// fn hello_planet(req: Request, params: Params) -> anyhow::Result<Response> {
///     let planet = params.get("planet").unwrap_or("world");
///     Ok(Response::new(200, format!("hello, {planet}")))
/// }
///
/// fn not_found(req: Request, params: Params) -> anyhow::Result<Response> {
///     Ok(Response::new(404, "not found"))
/// }
/// ```
///
/// Handle requests using a mix of synchronous and asynchronous handlers:
///
/// ```no_run
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let mut router = Router::new();
///     router.get("/hello/:planet", hello_planet);
///     router.get_async("/goodbye/:planet", goodbye_planet);
///     router.handle(req)
/// }
///
/// fn hello_planet(req: Request, params: Params) -> anyhow::Result<Response> {
///     todo!()
/// }
///
/// async fn goodbye_planet(req: Request, params: Params) -> anyhow::Result<Response> {
///     todo!()
/// }
/// ```
///
/// Route differently according to HTTP method:
///
/// ```no_run
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let mut router = Router::new();
///     router.get("/user", list_users);
///     router.post("/user", create_user);
///     router.get("/user/:id", get_user);
///     router.put("/user/:id", update_user);
///     router.delete("/user/:id", delete_user);
///     router.any("/user", method_not_allowed);
///     router.any("/user/:id", method_not_allowed);
///     router.handle(req)
/// }
/// # fn list_users(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn create_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn get_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn update_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn delete_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn method_not_allowed(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// ```
///
/// Run the handler asynchronously:
///
/// ```no_run
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// async fn handle_route(req: Request) -> Response {
///     let mut router = Router::new();
///     router.get_async("/user", list_users);
///     router.post_async("/user", create_user);
///     router.handle_async(req).await
/// }
/// # async fn list_users(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # async fn create_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// ```
///
/// Priority when routes overlap:
///
/// ```no_run
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let mut router = Router::new();
///     router.any("/*", handle_any);
///     router.any("/:seg", handle_single_segment);
///     router.any("/fie", handle_exact);
///
///     // '/fie' is routed to `handle_exact`
///     // '/zounds' is routed to `handle_single_segment`
///     // '/zounds/fie' is routed to `handle_any`
///     router.handle(req)
/// }
/// # fn handle_any(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn handle_single_segment(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn handle_exact(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// ```
///
/// Route based on the trailing segment of a Spin wildcard route, instead of on the full path
///
/// ```no_run
/// // spin.toml
/// //
/// // [[trigger.http]]
/// // route = "/shop/..."
///
/// // component
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let mut router = Router::suffix();
///     router.any("/users/*", handle_users);
///     router.any("/products/*", handle_products);
///     router.handle(req)
/// }
///
/// // '/shop/users/1' is routed to `handle_users`
/// // '/shop/products/1' is routed to `handle_products`
/// # fn handle_users(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn handle_products(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// ```
pub struct Router {
    methods_map: HashMap<Method, MethodRouter<Box<dyn Handler>>>,
    any_methods: MethodRouter<Box<dyn Handler>>,
    route_on: RouteOn,
}

/// Describes what part of the path the Router will route on.
enum RouteOn {
    /// The router will route on the full path.
    FullPath,
    /// The router expects the component to be handling a route with a trailing wildcard
    /// (e.g. `route = /shop/...`), and will route on the trailing segment.
    Suffix,
}

impl Default for Router {
    fn default() -> Router {
        Router::new()
    }
}

impl Display for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Registered routes:")?;
        for (method, router) in &self.methods_map {
            for route in router.iter() {
                writeln!(f, "- {}: {}", method, route.0)?;
            }
        }
        Ok(())
    }
}

struct RouteMatch<'a> {
    params: Captures<'static, 'static>,
    handler: &'a dyn Handler,
}

impl Router {
    /// Synchronously dispatches a request to the appropriate handler along with the URI parameters.
    pub fn handle<R>(&self, request: R) -> Response
    where
        R: TryIntoRequest,
        R::Error: IntoResponse,
    {
        crate::http::executor::run(self.handle_async(request))
    }

    /// Asynchronously dispatches a request to the appropriate handler along with the URI parameters.
    pub async fn handle_async<R>(&self, request: R) -> Response
    where
        R: TryIntoRequest,
        R::Error: IntoResponse,
    {
        let request = match R::try_into_request(request) {
            Ok(r) => r,
            Err(e) => return e.into_response(),
        };
        let method = request.method.clone();
        let path = match self.route_on {
            RouteOn::FullPath => request.path(),
            RouteOn::Suffix => match trailing_suffix(&request) {
                Some(path) => path,
                None => {
                    eprintln!("Internal error: Router configured with suffix routing but trigger route has no trailing wildcard");
                    return responses::internal_server_error();
                }
            },
        };
        let RouteMatch { params, handler } = self.find(path, method);
        handler.handle(request, params).await
    }

    fn find(&self, path: &str, method: Method) -> RouteMatch<'_> {
        let best_match = self
            .methods_map
            .get(&method)
            .and_then(|r| r.best_match(path));

        if let Some(m) = best_match {
            let params = m.captures().into_owned();
            let handler = m.handler();
            return RouteMatch { handler, params };
        }

        let best_match = self.any_methods.best_match(path);

        match best_match {
            Some(m) => {
                let params = m.captures().into_owned();
                let handler = m.handler();
                RouteMatch { handler, params }
            }
            None if method == Method::Head => {
                // If it is a HTTP HEAD request then check if there is a callback in the methods map
                // if not then fallback to the behavior of HTTP GET else proceed as usual
                self.find(path, Method::Get)
            }
            None => {
                // Handle the failure case where no match could be resolved.
                self.fail(path, method)
            }
        }
    }

    // Helper function to handle the case where a best match couldn't be resolved.
    fn fail(&self, path: &str, method: Method) -> RouteMatch<'_> {
        // First, filter all routers to determine if the path can match but the provided method is not allowed.
        let is_method_not_allowed = self
            .methods_map
            .iter()
            .filter(|(k, _)| **k != method)
            .any(|(_, r)| r.best_match(path).is_some());

        if is_method_not_allowed {
            // If this `path` can be handled by a callback registered with a different HTTP method
            // should return 405 Method Not Allowed
            RouteMatch {
                handler: &method_not_allowed,
                params: Captures::default(),
            }
        } else {
            // ... Otherwise, nothing matched so 404.
            RouteMatch {
                handler: &not_found,
                params: Captures::default(),
            }
        }
    }

    /// Register a handler at the path for all methods.
    pub fn any<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        let handler = move |req, params| {
            let res = TryFromRequest::try_from_request(req).map(|r| handler(r, params));
            async move {
                match res {
                    Ok(res) => res.into_response(),
                    Err(e) => e.into_response(),
                }
            }
        };

        self.any_async(path, handler)
    }

    /// Register an async handler at the path for all methods.
    pub fn any_async<F, Fut, I, O>(&mut self, path: &str, handler: F)
    where
        F: Fn(I, Params) -> Fut + 'static,
        Fut: Future<Output = O> + 'static,
        I: TryFromRequest + 'static,
        I::Error: IntoResponse + 'static,
        O: IntoResponse + 'static,
    {
        let handler = move |req, params| {
            let res = TryFromRequest::try_from_request(req).map(|r| handler(r, params));
            async move {
                match res {
                    Ok(f) => f.await.into_response(),
                    Err(e) => e.into_response(),
                }
            }
        };

        self.any_methods.add(path, Box::new(handler)).unwrap();
    }

    /// Register a handler at the path for the specified HTTP method.
    pub fn add<F, Req, Resp>(&mut self, path: &str, method: Method, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        let handler = move |req, params| {
            let res = TryFromRequest::try_from_request(req).map(|r| handler(r, params));
            async move {
                match res {
                    Ok(res) => res.into_response(),
                    Err(e) => e.into_response(),
                }
            }
        };

        self.add_async(path, method, handler)
    }

    /// Register an async handler at the path for the specified HTTP method.
    pub fn add_async<F, Fut, I, O>(&mut self, path: &str, method: Method, handler: F)
    where
        F: Fn(I, Params) -> Fut + 'static,
        Fut: Future<Output = O> + 'static,
        I: TryFromRequest + 'static,
        I::Error: IntoResponse + 'static,
        O: IntoResponse + 'static,
    {
        let handler = move |req, params| {
            let res = TryFromRequest::try_from_request(req).map(|r| handler(r, params));
            async move {
                match res {
                    Ok(f) => f.await.into_response(),
                    Err(e) => e.into_response(),
                }
            }
        };

        self.methods_map
            .entry(method)
            .or_default()
            .add(path, Box::new(handler))
            .unwrap();
    }

    /// Register a handler at the path for the HTTP GET method.
    pub fn get<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Get, handler)
    }

    /// Register an async handler at the path for the HTTP GET method.
    pub fn get_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Get, handler)
    }

    /// Register a handler at the path for the HTTP HEAD method.
    pub fn head<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Head, handler)
    }

    /// Register an async handler at the path for the HTTP HEAD method.
    pub fn head_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Head, handler)
    }

    /// Register a handler at the path for the HTTP POST method.
    pub fn post<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Post, handler)
    }

    /// Register an async handler at the path for the HTTP POST method.
    pub fn post_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Post, handler)
    }

    /// Register a handler at the path for the HTTP DELETE method.
    pub fn delete<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Delete, handler)
    }

    /// Register an async handler at the path for the HTTP DELETE method.
    pub fn delete_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Delete, handler)
    }

    /// Register a handler at the path for the HTTP PUT method.
    pub fn put<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Put, handler)
    }

    /// Register an async handler at the path for the HTTP PUT method.
    pub fn put_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Put, handler)
    }

    /// Register a handler at the path for the HTTP PATCH method.
    pub fn patch<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Patch, handler)
    }

    /// Register an async handler at the path for the HTTP PATCH method.
    pub fn patch_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Patch, handler)
    }

    /// Register a handler at the path for the HTTP OPTIONS method.
    pub fn options<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::Options, handler)
    }

    /// Register an async handler at the path for the HTTP OPTIONS method.
    pub fn options_async<F, Fut, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Req: TryFromRequest + 'static,
        Req::Error: IntoResponse + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add_async(path, Method::Options, handler)
    }

    /// Construct a new Router that matches on the full path.
    pub fn new() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
            route_on: RouteOn::FullPath,
        }
    }

    /// Construct a new Router that matches on the trailing wildcard
    /// component of the route.
    pub fn suffix() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
            route_on: RouteOn::Suffix,
        }
    }
}

async fn not_found(_req: Request, _params: Params) -> Response {
    responses::not_found()
}

async fn method_not_allowed(_req: Request, _params: Params) -> Response {
    responses::method_not_allowed()
}

fn trailing_suffix(req: &Request) -> Option<&str> {
    req.header("spin-path-info")
        .and_then(|path_info| path_info.as_str())
}

/// A macro to help with constructing a [`Router`] from method-pattern-handler triples.
///
/// # Examples
///
/// ```no_run
/// # use spin_sdk::http_router;
/// # use spin_sdk::http::{IntoResponse, Params, Request, Response, Router};
/// fn handle_route(req: Request) -> Response {
///     let router = http_router! {
///         GET "/user" => list_users,
///         POST "/user" => create_user,
///         GET "/user/:id" => get_user,
///         PUT "/user/:id" => update_user,
///         DELETE "/user/:id" => delete_user,
///         _ "/user" => method_not_allowed,
///         _ "/user/:id" => method_not_allowed
///     };
///     router.handle(req)
/// }
/// # fn list_users(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn create_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn get_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn update_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn delete_user(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// # fn method_not_allowed(req: Request, params: Params) -> anyhow::Result<Response> { todo!() }
/// ```
#[macro_export]
macro_rules! http_router {
    ($($method:tt $path:literal => $h:expr),*) => {
        {
            let mut router = $crate::http::Router::new();
            $(
                $crate::http_router!(@build router $method $path => $h);
            )*
            router
        }
    };
    (@build $r:ident HEAD $path:literal => $h:expr) => {
        $r.head($path, $h);
    };
    (@build $r:ident GET $path:literal => $h:expr) => {
        $r.get($path, $h);
    };
    (@build $r:ident PUT $path:literal => $h:expr) => {
        $r.put($path, $h);
    };
    (@build $r:ident POST $path:literal => $h:expr) => {
        $r.post($path, $h);
    };
    (@build $r:ident PATCH $path:literal => $h:expr) => {
        $r.patch($path, $h);
    };
    (@build $r:ident DELETE $path:literal => $h:expr) => {
        $r.delete($path, $h);
    };
    (@build $r:ident OPTIONS $path:literal => $h:expr) => {
        $r.options($path, $h);
    };
    (@build $r:ident _ $path:literal => $h:expr) => {
        $r.any($path, $h);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(method: Method, path: &str) -> Request {
        Request::new(method, path)
    }

    fn make_wildcard_request(method: Method, path: &str, trailing: &str) -> Request {
        let mut req = Request::new(method, path);
        req.set_header("spin-path-info", trailing);
        req
    }

    fn echo_param(_req: Request, params: Params) -> Response {
        match params.get("x") {
            Some(path) => Response::new(200, path),
            None => responses::not_found(),
        }
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(Method::Post, "/foobar");
        let res = router.handle(req);
        assert_eq!(res.status, hyperium::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_not_found() {
        fn h1(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(Response::new(200, ()))
        }

        let mut router = Router::default();
        router.get("/h1/:param", h1);

        let req = make_request(Method::Get, "/h1/");
        let res = router.handle(req);
        assert_eq!(res.status, hyperium::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_multi_param() {
        fn multiply(_req: Request, params: Params) -> anyhow::Result<Response> {
            let x: i64 = params.get("x").unwrap().parse()?;
            let y: i64 = params.get("y").unwrap().parse()?;
            Ok(Response::new(200, format!("{result}", result = x * y)))
        }

        let mut router = Router::default();
        router.get("/multiply/:x/:y", multiply);

        let req = make_request(Method::Get, "/multiply/2/4");
        let res = router.handle(req);

        assert_eq!(res.body, "8".to_owned().into_bytes());
    }

    #[test]
    fn test_param() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(Method::Get, "/y");
        let res = router.handle(req);

        assert_eq!(res.body, "y".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard() {
        fn echo_wildcard(_req: Request, params: Params) -> Response {
            match params.wildcard() {
                Some(path) => Response::new(200, path),
                None => responses::not_found(),
            }
        }

        let mut router = Router::default();
        router.get("/*", echo_wildcard);

        let req = make_request(Method::Get, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.status, hyperium::StatusCode::OK);
        assert_eq!(res.body, "foo/bar".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard_last_segment() {
        let mut router = Router::default();
        router.get("/:x/*", echo_param);

        let req = make_request(Method::Get, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.body, "foo".to_owned().into_bytes());
    }

    #[test]
    fn test_spin_trailing_wildcard() {
        let mut router = Router::suffix();
        router.get("/:x/*", echo_param);

        let req = make_wildcard_request(Method::Get, "/base/baser/foo/bar", "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.body, "foo".to_owned().into_bytes());
    }

    #[test]
    fn test_router_display() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let expected = "Registered routes:\n- GET: /:x\n";
        let actual = format!("{}", router);

        assert_eq!(actual.as_str(), expected);
    }

    #[test]
    fn test_ambiguous_wildcard_vs_star() {
        fn h1(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(Response::new(200, "one/two"))
        }

        fn h2(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(Response::new(200, "posts/*"))
        }

        let mut router = Router::default();
        router.get("/:one/:two", h1);
        router.get("/posts/*", h2);

        let req = make_request(Method::Get, "/posts/2");
        let res = router.handle(req);

        assert_eq!(res.body, "posts/*".to_owned().into_bytes());
    }
}
