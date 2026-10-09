//! THE ROUTE TABLE, BUILT ONCE PER ISOLATE (W-LOOPA row 2).
//!
//! `route` used to build the whole table -- 257 `matchit` inserts into a fresh
//! `worker::Router` -- on EVERY request, to answer one lookup: 200.7 us release
//! (R-LOOPS B1), 2 % of the Free plan's 10 ms before any handler ran. workers-rs
//! 0.8.5's `Router` cannot be kept: `run` consumes it, its fields are private,
//! and a `RouteContext` (the only way a handler is given its params) can only be
//! made by `Router::run`. So the table is not the `Router`; it is the same chain
//! of registrations read three ways through the [`Routes`] trait:
//!
//! * [`Recorder`] -- once per isolate: every `(method, pattern)` in chain order,
//!   inserted into a `matchit` table of route INDICES ([`Table`]), with the same
//!   `matchit` version and the same panic on a conflicting pair as workers-rs.
//! * [`Picker`] -- per request: walks the chain and registers ONLY route `k`,
//!   the one the table found, into the per-request `worker::Router`. That router
//!   holds one route, matches the path again (one route, ~1 us) and hands the
//!   handler its `RouteContext` exactly as before.
//! * `worker::Router` itself -- the old full build, kept for the conflict test.
//!
//! 404 and 405 are answered from the table with the bodies workers-rs writes:
//! `Router::run`'s own rule (`worker-0.8.5/src/router.rs`), copied, and held to
//! a model of the old full router over every pattern in `routes_table/tests.rs`.
//! No handler changed shape: the chain in `lib.rs` is the same text it was.

use std::collections::HashMap;
use std::future::Future;
use worker::{Env, Method, Request, Response, Result, RouteContext, Router};

use crate::Req;

/// A synchronous handler, as `worker::Router::get` takes it.
pub type SyncFn = fn(Request, RouteContext<Req>) -> Result<Response>;

/// The registrations the chain in `crate::router` makes. Each call is ONE route under ONE
/// method -- the chain uses no `on`/`or_else_any_method` (the table would have to learn them).
pub trait Routes: Sized {
    fn add<F, T>(self, method: Method, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static;
    fn add_sync(self, method: Method, pattern: &str, f: SyncFn) -> Self;

    fn get(self, pattern: &str, f: SyncFn) -> Self {
        self.add_sync(Method::Get, pattern, f)
    }
    fn get_async<F, T>(self, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        self.add(Method::Get, pattern, f)
    }
    fn post_async<F, T>(self, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        self.add(Method::Post, pattern, f)
    }
    fn put_async<F, T>(self, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        self.add(Method::Put, pattern, f)
    }
    fn delete_async<F, T>(self, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        self.add(Method::Delete, pattern, f)
    }
}

/// The real router: each registration is workers-rs's own.
impl Routes for Router<'static, Req> {
    fn add<F, T>(self, method: Method, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        match method {
            Method::Get => Router::get_async(self, pattern, f),
            Method::Post => Router::post_async(self, pattern, f),
            Method::Put => Router::put_async(self, pattern, f),
            Method::Delete => Router::delete_async(self, pattern, f),
            other => panic!("the route table registers no {other:?} route ({pattern})"),
        }
    }
    fn add_sync(self, method: Method, pattern: &str, f: SyncFn) -> Self {
        match method {
            Method::Get => Router::get(self, pattern, f),
            other => panic!("the route table registers no sync {other:?} route ({pattern})"),
        }
    }
}

/// Every `(method, pattern)` the chain registers, in chain order: route `k` is the `k`th call.
#[derive(Default)]
pub struct Recorder {
    pub routes: Vec<(Method, String)>,
}

impl Routes for Recorder {
    fn add<F, T>(mut self, method: Method, pattern: &str, _: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        self.routes.push((method, pattern.to_string()));
        self
    }
    fn add_sync(mut self, method: Method, pattern: &str, _: SyncFn) -> Self {
        self.routes.push((method, pattern.to_string()));
        self
    }
}

/// Registers only the `want`th route of the chain into `inner`; drops every other closure.
pub struct Picker<R> {
    want: usize,
    at: usize,
    pub inner: R,
}

impl<R: Routes> Picker<R> {
    pub fn new(want: usize, inner: R) -> Self {
        Picker { want, at: 0, inner }
    }
}

impl<R: Routes> Routes for Picker<R> {
    fn add<F, T>(mut self, method: Method, pattern: &str, f: F) -> Self
    where
        F: Fn(Request, RouteContext<Req>) -> T + 'static,
        T: Future<Output = Result<Response>> + 'static,
    {
        if self.at == self.want {
            self.inner = self.inner.add(method, pattern, f);
        }
        self.at += 1;
        self
    }
    fn add_sync(mut self, method: Method, pattern: &str, f: SyncFn) -> Self {
        if self.at == self.want {
            self.inner = self.inner.add_sync(method, pattern, f);
        }
        self.at += 1;
        self
    }
}

/// What the table says about one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Route `k` of the chain, with the params `matchit` read out of the path.
    Route(usize, Vec<(String, String)>),
    MethodNotAllowed,
    NotFound,
}

/// The chain's routes as `matchit` tables of indices, one per method -- the shape
/// `worker::Router` keeps (`HashMap<Method, matchit::Router<Handler>>`) with an index
/// where it keeps a handler.
pub struct Table {
    /// Route `k`'s `(method, pattern)`; the tests walk it, the request path needs only the index.
    #[cfg_attr(not(test), allow(dead_code))]
    pub routes: Vec<(Method, String)>,
    by_method: HashMap<Method, matchit::Router<usize>>,
}

impl Table {
    /// Same inserts, same order, same panic text as `worker::Router::add_handler`.
    pub fn build() -> Table {
        let routes = crate::router(Recorder::default()).routes;
        let mut by_method: HashMap<Method, matchit::Router<usize>> = HashMap::new();
        for (k, (method, pattern)) in routes.iter().enumerate() {
            by_method
                .entry(method.clone())
                .or_default()
                .insert(pattern.as_str(), k)
                .unwrap_or_else(|e| panic!("failed to register {method:?} route for {pattern} pattern: {e}"));
        }
        Table { routes, by_method }
    }

    /// `worker::Router::run`'s decision, without the handler: the method's table, else 405
    /// when another method (not HEAD, OPTIONS, TRACE) has the path, else 404.
    pub fn find(&self, method: &Method, path: &str) -> Found {
        if let Some(Ok(m)) = self.by_method.get(method).map(|t| t.at(path)) {
            return Found::Route(*m.value, m.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        }
        for other in Method::all() {
            if other == Method::Head || other == Method::Options || other == Method::Trace {
                continue;
            }
            if self.by_method.get(&other).is_some_and(|t| t.at(path).is_ok()) {
                return Found::MethodNotAllowed;
            }
        }
        Found::NotFound
    }
}

thread_local! {
    /// Once per isolate. A conflicting pair panics here, on the first request, exactly where
    /// the per-request build used to panic on every one.
    static TABLE: Table = Table::build();
}

/// The isolate's table, asked once.
pub fn find(method: &Method, path: &str) -> Found {
    TABLE.with(|t| t.find(method, path))
}

/// THE REQUEST PATH: one table lookup, then a router holding only the route it found.
/// `vessel` is the per-request `Router::with_data(Req { now_ms })` -- `route` still makes it.
pub async fn dispatch(vessel: Router<'static, Req>, req: Request, env: Env) -> Result<Response> {
    match find(&req.method(), &req.path()) {
        Found::Route(k, _) => crate::router(Picker::new(k, vessel)).inner.run(req, env).await,
        Found::MethodNotAllowed => Response::error("Method Not Allowed", 405),
        Found::NotFound => Response::error("Not Found", 404),
    }
}
