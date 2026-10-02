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
