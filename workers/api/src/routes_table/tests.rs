//! THE ROUTE TABLE BUILDS (W-COV C2). The router refuses two routes that conflict when it is
//! built -- `matchit` rejects, for one method, a pattern that collides with one already there,
//! and `workers-rs` turns that into a panic -- and on the Worker it is built on every request,
//! so a conflicting pair would take down every route, not just the two. Built here, natively,
//! the panic is a red test instead.

#[test]
fn every_route_in_the_table_registers_without_a_conflict() {
    let built = std::panic::catch_unwind(|| {
        let _ = crate::router(worker::Router::with_data(crate::Req { now_ms: 0 }));
    });
    assert!(built.is_ok(), "the route table panicked while it was built: two routes conflict");
}

#[test]
#[should_panic(expected = "failed to register")]
fn a_conflicting_pair_is_what_this_would_catch() {
    // The control: the same pattern twice under one method is refused, so the test above is
    // able to fail.
    let _ = crate::router(worker::Router::with_data(crate::Req { now_ms: 0 })).get("/healthz", |_, _| worker::Response::ok("again"));
}

// ── W-LOOPA row 2: the table built once answers what the per-request router answered ──────────

use crate::routes_dispatch::{self as dispatch, Found, Picker, Recorder, Routes, SyncFn, Table};
use std::collections::HashMap;
use worker::{Method, Request, Response, Result, RouteContext};

/// THE OLD ROUTER, modelled: what `worker::Router` 0.8.5 did with the chain on every request.
/// `add_handler` (insert as each registration arrives, per method) and `run`'s decision,
/// copied from `worker-0.8.5/src/router.rs`, with the route's index where it kept a handler.
#[derive(Default)]
struct OldRouter {
    handlers: HashMap<Method, matchit::Router<usize>>,
    next: usize,
}

impl OldRouter {
    fn add_handler(&mut self, pattern: &str, methods: Vec<Method>) {
        for method in methods {
            self.handlers
                .entry(method.clone())
                .or_default()
                .insert(pattern, self.next)
                .unwrap_or_else(|e| panic!("failed to register {method:?} route for {pattern} pattern: {e}"));
        }
        self.next += 1;
    }

    fn run(&self, method: &Method, path: &str) -> Found {
        if let Some(handlers) = self.handlers.get(method) {
            if let Ok(matchit::Match { value, params }) = handlers.at(path) {
                return Found::Route(*value, params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
            }
        }
        for method in Method::all() {
            if method == Method::Head || method == Method::Options || method == Method::Trace {
                continue;
            }
            if let Some(handlers) = self.handlers.get(&method) {
                if let Ok(matchit::Match { .. }) = handlers.at(path) {
                    return Found::MethodNotAllowed;
                }
            }
        }
        Found::NotFound
    }
}

impl Routes for OldRouter {
    fn add<F, T>(mut self, method: Method, pattern: &str, _: F) -> Self
    where
        F: Fn(Request, RouteContext<crate::Req>) -> T + 'static,
        T: std::future::Future<Output = Result<Response>> + 'static,
    {
        self.add_handler(pattern, vec![method]);
        self
    }
    fn add_sync(mut self, method: Method, pattern: &str, _: SyncFn) -> Self {
        self.add_handler(pattern, vec![method]);
        self
    }
}

/// THE NEW PATH, natively: the isolate's table finds `k`, the `Picker` registers ONE route (it
/// must be route `k`), and that one-route router re-matches the path for the handler's params.
fn new_answer(table: &Table, method: &Method, path: &str) -> Found {
    match dispatch::find(method, path) {
        Found::Route(k, _) => {
            let picked = crate::router(Picker::new(k, Recorder::default())).inner.routes;
            assert_eq!(picked, vec![table.routes[k].clone()], "the picker registers route {k} and nothing else");
            let mut one: matchit::Router<usize> = matchit::Router::new();
            one.insert(picked[0].1.as_str(), k).unwrap();
            let m = one.at(path).unwrap_or_else(|e| panic!("route {k} alone does not match {path}: {e}"));
            Found::Route(*m.value, m.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
        }
        other => other,
    }
}

/// A concrete path for a pattern: `:name` -> `v-name-7`, `*name` -> `a/b.png`.
fn sample(pattern: &str) -> String {
    pattern
        .split('/')
        .map(|seg| match seg.chars().next() {
            Some(':') => format!("v-{}-7", &seg[1..]),
            Some('*') => "a/b.png".to_string(),
            _ => seg.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// EVERY PATTERN IN THE TABLE (from the table itself, not a hand list) x every method x the
/// path, its trailing-slash twin, its parent, and a few strangers: the same route index, the
/// same params, the same 405 and 404 as the old per-request router.
#[test]
fn the_once_built_table_answers_every_route_as_the_per_request_router_did() {
    let table = Table::build();
    let old = crate::router(OldRouter::default());
    assert_eq!(old.next, table.routes.len());
    assert!(table.routes.len() >= 250, "{} routes: the chain was not walked", table.routes.len());
    let mut paths: Vec<String> = vec![
        "".into(), "/".into(), "/nope".into(), "/api".into(), "/api/".into(), "/api/owner".into(),
        "/HEALTHZ".into(), "//healthz".into(), "/healthz/".into(), "/api/learn/media/".into(),
    ];
    for (_, p) in &table.routes {
        let s = sample(p);
        paths.push(format!("{s}/"));
        paths.push(s.rsplit_once('/').map(|(head, _)| head.to_string()).unwrap_or_default());
        paths.push(s);
    }
    paths.sort();
    paths.dedup();
    let (mut routes, mut not_allowed, mut not_found) = (0, 0, 0);
    let mut own = std::collections::HashSet::new();
    for path in &paths {
        for method in Method::all() {
            let want = old.run(&method, path);
            assert_eq!(new_answer(&table, &method, path), want, "{method:?} {path}");
            match want {
                Found::Route(k, _) => {
                    routes += 1;
                    if table.routes[k].0 == method && sample(&table.routes[k].1) == *path {
                        own.insert(k);
                    }
                }
                Found::MethodNotAllowed => not_allowed += 1,
                Found::NotFound => not_found += 1,
            }
        }
    }
    // not vacuous: every route was reached by its own sample, and both refusals happened
    assert_eq!(own.len(), table.routes.len(), "a route its own sample path did not reach");
    assert!(routes > 0 && not_allowed > 100 && not_found > 100, "{routes} {not_allowed} {not_found}");
}

/// The picker over the REAL `worker::Router`: every route registers alone without a panic.
#[test]
fn every_route_registers_alone_into_the_real_router() {
    for k in 0..Table::build().routes.len() {
        let built = std::panic::catch_unwind(|| {
            let _ = crate::router(Picker::new(k, worker::Router::with_data(crate::Req { now_ms: 0 })));
        });
        assert!(built.is_ok(), "route {k} panicked registering alone");
    }
}

#[test]
#[ignore]
fn measure_loopa_row2() {
    let median_us = |f: &dyn Fn()| {
        let mut t: Vec<u128> = (0..9)
            .map(|_| {
                let s = std::time::Instant::now();
                for _ in 0..20 {
                    f();
                }
                s.elapsed().as_nanos() / 20
            })
            .collect();
        t.sort_unstable();
        t[4] as f64 / 1e3
    };
    let _ = dispatch::find(&Method::Get, "/healthz"); // the isolate's one build, outside the timing
    for path in ["/api/owner/dashboard", "/api/public/locations/v-slug-7/menu", "/nope"] {
        let before = median_us(&|| {
            std::hint::black_box(crate::router(worker::Router::with_data(crate::Req { now_ms: 0 })));
        });
        let after = median_us(&|| {
            if let Found::Route(k, _) = dispatch::find(&Method::Get, path) {
                std::hint::black_box(crate::router(Picker::new(k, worker::Router::with_data(crate::Req { now_ms: 0 }))));
            }
        });
        println!("ROW2 router GET {path}: BEFORE {before:.1} us | AFTER {after:.2} us | {:.0}x | cap share {:.2}% -> {:.3}%",
            before / after.max(0.001), before / 100.0, after / 100.0);
    }
}
