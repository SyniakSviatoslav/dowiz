//! The edge table against the code it describes (R-GRAPH D4): the route table
//! is READ OUT OF THE SOURCE, not kept as a second hand-written list.

use super::{memos, row, Step, EDGES, MENU, ORDERS};
use crate::hubdo::host::mem::Harness;
use crate::hubdo::menu::tests::catalog;

/// The object's route table and the two read dispatchers, as compiled.
const HUBDO: &str = include_str!("../../hubdo.rs");
const READS: &str = include_str!("../reads.rs");
const FACTS: &str = include_str!("../facts.rs");

/// A line without its comment. `//` starts one only at the start or after
/// whitespace, so `https://hub/...` inside a string is kept.
fn code(line: &str) -> &str {
    let b = line.as_bytes();
    for i in 0..b.len().saturating_sub(1) {
        if b[i] == b'/' && b[i + 1] == b'/' && (i == 0 || b[i - 1].is_ascii_whitespace()) {
            return &line[..i];
        }
    }
    line
}

/// The quoted names of `"a" | "b"`, or None when `s` is anything else.
fn names(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for part in s.split('|') {
        let p = part.trim();
        let inner = p.strip_prefix('"')?.strip_suffix('"')?;
        if inner.is_empty() || inner.contains('"') {
            return None;
        }
        out.push(inner.to_string());
    }
    Some(out)
}

/// Every `/fold/*` route a `match (method, segment)` arm names:
/// `(Method::Get, "a" | "b")` and `(_, "c")`, comments stripped.
fn routes(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines().map(code) {
        let mut rest = line;
        while let Some(at) = rest.find("(Method::").or_else(|| rest.find("(_, \"")) {
            let tail = &rest[at + 1..];
            let Some(close) = tail.find(')') else { break };
            if let Some((_, segs)) = tail[..close].split_once(',') {
                out.extend(names(segs).unwrap_or_default());
            }
            rest = &tail[close..];
        }
    }
    out
}

/// The names a read dispatcher (`fn <name>(&self, what: &str, ..)`) matches on.
fn dispatched(src: &str, fun: &str) -> Vec<String> {
    let start = src.find(&format!("fn {fun}(")).unwrap_or_else(|| panic!("no fn {fun}"));
    let body = &src[start..];
    let end = body.find("_ => ").unwrap_or_else(|| panic!("fn {fun} has no fallback arm"));
    body[..end]
        .lines()
        .map(code)
        .filter_map(|l| l.trim().split_once(" => ").and_then(|(head, _)| names(head)))
        .flatten()
        .collect()
}

/// What the table and the routes disagree on, each line naming the route.
fn disagreements(routed: &[String], served: &[String]) -> Vec<String> {
    let mut bad = Vec::new();
    for r in routed {
        match EDGES.iter().filter(|e| e.route == r).count() {
            1 => {}
            0 => bad.push(format!("/fold/{r} is routed and names no row of hubdo/edges.rs")),
            n => bad.push(format!("/fold/{r} has {n} rows")),
        }
        if routed.iter().filter(|x| *x == r).count() > 1 {
            bad.push(format!("/fold/{r} is routed twice"));
        }
    }
    for e in EDGES {
        if !routed.iter().any(|r| r == e.route) {
            bad.push(format!("row {} names no /fold/ route", e.route));
        }
    }
    for s in served {
        if !routed.iter().any(|r| r == s) {
            bad.push(format!("/fold/{s} is served by a dispatcher and no route reaches it"));
        }
    }
    bad.sort();
    bad.dedup();
    bad
}

fn served() -> Vec<String> {
    let mut s = dispatched(READS, "fold_read");
    s.extend(dispatched(FACTS, "fold_facts"));
    s
}

/// THE REFUSAL: a route with no row, a row with no route, a dispatched read
/// nothing routes to. Rows == routes, MEASURED off `hubdo.rs`.
#[test]
fn every_fold_route_names_one_row_and_every_row_a_route() {
    let routed = routes(HUBDO);
    assert!(routed.len() > 30, "the parser lost the route table: {routed:?}");
    assert_eq!(disagreements(&routed, &served()), Vec::<String>::new());
    assert_eq!(EDGES.len(), routed.len(), "rows == /fold/* routes");
}

/// The parser's twin: a scratch route the table does not know IS seen and
/// IS refused; a dispatched name no arm lists is refused too.
#[test]
fn a_route_without_a_row_is_refused_by_name() {
    let scratch = format!("{}\n(Method::Get, \"x\") => Response::error(\"scratch\", 404), // not a row\n", HUBDO);
    let bad = disagreements(&routes(&scratch), &served());
    assert_eq!(bad, vec!["/fold/x is routed and names no row of hubdo/edges.rs".to_string()]);
    let mut more = served();
    more.push("y".into());
    assert_eq!(disagreements(&routes(HUBDO), &more), vec!["/fold/y is served by a dispatcher and no route reaches it".to_string()]);
    // Comments and URLs are not routes.
    assert!(routes("// (Method::Get, \"z\")\nlet u = \"https://hub/fold/z\";").is_empty());
}

#[test]
fn the_memo_rows_are_the_two_the_object_keeps() {
    assert_eq!(memos(), vec![ORDERS, MENU]);
    assert_eq!(row("menu").map(|r| r.out_key), Some(super::OutKey::K64));
    assert_eq!(row("orders").map(|r| r.step), Some(Step::Memo));
    let menu_inputs: Vec<&str> = crate::hubdo::menu::MENU_INPUTS.to_vec();
    assert_eq!(row("menu").map(|r| r.inputs.to_vec()), Some(menu_inputs), "the menu row names what the memo is keyed by");
    assert!(EDGES.iter().all(|e| !e.inputs.is_empty() || matches!(e.step, Step::Memory | Step::Platform | Step::Command)));
}

/// LAW 8 WALKS THE TABLE: a rebuild names every memo row it checked.
#[test]
fn rebuild_checks_every_memo_row() {
    let h = Harness::new();
    assert_eq!(h.put("catalog", 0, &catalog(900).to_bytes().unwrap()).status_code(), 200);
    let _ = h.get("/fold/menu?slug=dubin&now=0"); // the memo is served
    let r = h.get("/fold/rebuild").body_value();
    assert_eq!(r["checked"], serde_json::json!(["orders", "menu"]));
    assert_eq!(r["stale_memos"], serde_json::json!([]));
}

/// The twin: a menu memo standing at the current generations over OTHER bytes
/// is a served lie, and the rebuild names it.
#[test]
fn rebuild_names_a_menu_memo_that_lies() {
    let h = Harness::new();
    assert_eq!(h.put("catalog", 0, &catalog(900).to_bytes().unwrap()).status_code(), 200);
    let gens = h.obj.menu_gens_in_memory();
    let wrong = catalog(1).to_bytes().unwrap();
    let lie = crate::fold::menu::Memo::from_images(gens, Some(&wrong), None, None, Default::default()).unwrap();
    *h.obj.menu.borrow_mut() = Some(lie);
    let r = h.get("/fold/rebuild").body_value();
    assert_eq!(r["stale_memos"], serde_json::json!(["menu"]));
    let report: crate::rebuild::Report = serde_json::from_value(r).unwrap();
    assert!(!report.intact(), "a stale memo fails the gate's one question");
}

/// What the table found the day it was written: the owner's HACCP export
/// (`services/operations/stock/haccp.rs` asks `/fold/haccp`) was dispatched by
/// `fold_read` and reached by no route -- every export answered 404.
#[test]
fn the_haccp_export_reaches_its_fold() {
    let h = Harness::new();
    let r = h.get("/fold/haccp?kind=frozen&from=2023-11-01&to=2023-11-30");
    assert_ne!(String::from_utf8_lossy(r.body()), "no such projection", "status {}", r.status_code());
    let none = h.get("/fold/no_such_thing");
    assert_eq!((none.status_code(), String::from_utf8_lossy(none.body()).to_string()), (404, "no such projection".to_string()), "the twin: an unrouted name still 404s");
}
