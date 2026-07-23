//! Courier quality gates — grep-based scope enforcement.
//!
//! These gates verify that certain concepts NEVER appear in courier module
//! source code. They live in a separate file so the test assertions do NOT
//! self-reference (the forbidden tokens are here, not in the modules under test).

/// R4 grep gate — assert NO routing / Kalman / map MATH lives in courier modules.
pub fn no_routing_code(src: &str) -> bool {
    let forbidden = [
        "kalman",
        "router",
        "route_compute",
        "eta_calc",
        "shortest_path",
        "dijkstra",
        "a_star",
        "reroute",
    ];
    !forbidden.iter().any(|f| src.contains(f))
}

/// P38 grep gate — assert no visible DOM widget is authored in courier.
pub fn no_visible_dom_widget(src: &str) -> bool {
    !src.contains("document.")
        && !src.contains("window.")
        && !src.contains("getelementbyid")
        && !src.contains("<div")
        && !src.contains("createElement")
}
