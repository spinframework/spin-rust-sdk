// This router implementation is heavily inspired by the `Endpoint` type in the https://github.com/http-rs/tide project.

use super::{HttpResult, IntoResponse, Request};
use async_trait::async_trait;
use hyperium::{Method, StatusCode};
use routefinder::{Captures, Router as MethodRouter};
use std::future::Future;
use std::{collections::HashMap, fmt::Display};
use wasip3::http::types;
use wasip3::http_compat::IncomingRequestBody;

/// The output of a router-dispatched handler: a low-level WASI HTTP response
/// (or error code), suitable for returning directly from an `#[http_service]`
/// handler.
type HandlerOutput = HttpResult<types::Response>;

/// An HTTP request handler.
///
/// This trait is automatically implemented for `Fn` types, and so is rarely
/// implemented directly by Spin users.
#[async_trait(?Send)]
pub trait Handler<B = IncomingRequestBody> {
    /// Invoke the handler.
    async fn handle(&self, req: Request<B>, params: Params) -> HandlerOutput;
}

#[async_trait(?Send)]
impl<B: 'static> Handler<B> for Box<dyn Handler<B>> {
    async fn handle(&self, req: Request<B>, params: Params) -> HandlerOutput {
        self.as_ref().handle(req, params).await
    }
}

#[async_trait(?Send)]
impl<B, F, Fut> Handler<B> for F
where
    B: 'static,
    F: Fn(Request<B>, Params) -> Fut + 'static,
    Fut: Future<Output = HandlerOutput> + 'static,
{
    async fn handle(&self, req: Request<B>, params: Params) -> HandlerOutput {
        (self)(req, params).await
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
/// If a request matches more than one route, the match is selected according to
/// the following criteria:
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
/// ```ignore
/// use spin_sdk::http::{IntoResponse, Request, StatusCode};
/// use spin_sdk::http::router::{Params, Router};
/// use spin_sdk::http_service;
///
/// #[http_service]
/// async fn handle_route(req: Request) -> impl IntoResponse {
///     let mut router = Router::new();
///     router.get("/hello/:planet", hello_planet);
///     router.any("/*", not_found);
///     router.handle(req).await
/// }
///
/// async fn hello_planet(_req: Request, params: Params) -> impl IntoResponse {
///     let planet = params.get("planet").unwrap_or("world").to_owned();
///     (StatusCode::OK, format!("hello, {planet}"))
/// }
///
/// async fn not_found(_req: Request, _params: Params) -> impl IntoResponse {
///     (StatusCode::NOT_FOUND, "not found")
/// }
/// ```
pub struct Router<B = IncomingRequestBody> {
    methods_map: HashMap<Method, MethodRouter<Box<dyn Handler<B>>>>,
    any_methods: MethodRouter<Box<dyn Handler<B>>>,
    route_on: RouteOn,
}

/// Describes what part of the path the Router will route on.
enum RouteOn {
    /// The router will route on the full path.
    FullPath,
    /// The router expects the component to be handling a route with a trailing
    /// wildcard (e.g. `route = /shop/...`), and will route on the trailing
    /// segment.
    Suffix,
}

impl<B: 'static> Default for Router<B> {
    fn default() -> Router<B> {
        Router::new()
    }
}

impl<B> Display for Router<B> {
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

struct RouteMatch<'a, B> {
    params: Captures<'static, 'static>,
    handler: &'a dyn Handler<B>,
}

impl<B: 'static> Router<B> {
    /// Asynchronously dispatches a request to the appropriate handler along
    /// with the URI parameters.
    pub async fn handle(&self, request: Request<B>) -> HandlerOutput {
        let method = request.method().clone();
        let path: String = match self.route_on {
            RouteOn::FullPath => request.uri().path().to_owned(),
            RouteOn::Suffix => match trailing_suffix(&request) {
                Some(path) => path.to_owned(),
                None => {
                    eprintln!(
                        "Internal error: Router configured with suffix routing but trigger route has no trailing wildcard"
                    );
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            },
        };
        let RouteMatch { params, handler } = self.find(&path, method);
        handler.handle(request, params).await
    }

    fn find(&self, path: &str, method: Method) -> RouteMatch<'_, B> {
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
            None if method == Method::HEAD => {
                // If it is a HTTP HEAD request then check if there is a
                // callback in the methods map; if not then fall back to the
                // behavior of HTTP GET; else proceed as usual.
                self.find(path, Method::GET)
            }
            None => {
                // Handle the failure case where no match could be resolved.
                self.fail(path, method)
            }
        }
    }

    // Helper function to handle the case where a best match couldn't be
    // resolved.
    fn fail(&self, path: &str, method: Method) -> RouteMatch<'_, B> {
        // First, filter all routers to determine if the path can match but the
        // provided method is not allowed.
        let is_method_not_allowed = self
            .methods_map
            .iter()
            .filter(|(k, _)| **k != method)
            .any(|(_, r)| r.best_match(path).is_some());

        if is_method_not_allowed {
            // If this `path` can be handled by a callback registered with a
            // different HTTP method, return 405 Method Not Allowed.
            RouteMatch {
                handler: &method_not_allowed::<B>,
                params: Captures::default(),
            }
        } else {
            // ... Otherwise, nothing matched so 404.
            RouteMatch {
                handler: &not_found::<B>,
                params: Captures::default(),
            }
        }
    }

    /// Register an async handler at the path for all methods.
    pub fn any<F, Fut, O>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = O> + 'static,
        O: IntoResponse + 'static,
    {
        let wrapped = move |req, params| {
            let fut = handler(req, params);
            async move { fut.await.into_response() }
        };

        self.any_methods.add(path, Box::new(wrapped)).unwrap();
    }

    /// Register an async handler at the path for the specified HTTP method.
    pub fn add<F, Fut, O>(&mut self, path: &str, method: Method, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = O> + 'static,
        O: IntoResponse + 'static,
    {
        let wrapped = move |req, params| {
            let fut = handler(req, params);
            async move { fut.await.into_response() }
        };

        self.methods_map
            .entry(method)
            .or_default()
            .add(path, Box::new(wrapped))
            .unwrap();
    }

    /// Register an async handler at the path for the HTTP GET method.
    pub fn get<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::GET, handler)
    }

    /// Register an async handler at the path for the HTTP HEAD method.
    pub fn head<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::HEAD, handler)
    }

    /// Register an async handler at the path for the HTTP POST method.
    pub fn post<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::POST, handler)
    }

    /// Register an async handler at the path for the HTTP DELETE method.
    pub fn delete<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::DELETE, handler)
    }

    /// Register an async handler at the path for the HTTP PUT method.
    pub fn put<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::PUT, handler)
    }

    /// Register an async handler at the path for the HTTP PATCH method.
    pub fn patch<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::PATCH, handler)
    }

    /// Register an async handler at the path for the HTTP OPTIONS method.
    pub fn options<F, Fut, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request<B>, Params) -> Fut + 'static,
        Fut: Future<Output = Resp> + 'static,
        Resp: IntoResponse + 'static,
    {
        self.add(path, Method::OPTIONS, handler)
    }

    /// Construct a new Router that matches on the full path.
    pub fn new() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
            route_on: RouteOn::FullPath,
        }
    }

    /// Construct a new Router that matches on the trailing wildcard component
    /// of the route.
    pub fn suffix() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
            route_on: RouteOn::Suffix,
        }
    }
}

async fn not_found<B>(_req: Request<B>, _params: Params) -> HandlerOutput {
    StatusCode::NOT_FOUND.into_response()
}

async fn method_not_allowed<B>(_req: Request<B>, _params: Params) -> HandlerOutput {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

fn trailing_suffix<B>(req: &Request<B>) -> Option<&str> {
    req.headers()
        .get("spin-path-info")
        .and_then(|v| v.to_str().ok())
}
