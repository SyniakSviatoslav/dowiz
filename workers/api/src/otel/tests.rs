//! Tracing (W-COV C2): a valid incoming `traceparent` is continued and an invalid one is not,
//! the OTLP payload says what happened (status, attributes, the error), and the export goes to
//! the configured collector with its headers -- or nowhere when none is configured.

use super::*;
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::Site;

const TP: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

#[test]
fn a_valid_traceparent_is_continued_and_an_invalid_one_starts_a_new_trace() {
    let t = Trace::begin(Some(TP), "GET /x");
    assert_eq!(t.id(), "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(t.spans[0].parent_id.as_deref(), Some("00f067aa0ba902b7"));
    for bad in [
        "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01", // version
        "00-00000000000000000000000000000000-00f067aa0ba902b7-01", // all-zero trace
        "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01", // all-zero span
        "00-4bf92f3577b34da6a3ce929d0e0e47zz-00f067aa0ba902b7-01", // not hex
        "00-4bf92f35-00f067aa0ba902b7-01",                         // short
        "garbage",
    ] {
        let t = Trace::begin(Some(bad), "GET /x");
        assert_ne!(t.id(), "4bf92f3577b34da6a3ce929d0e0e4736", "{bad}");
        assert_eq!(t.id().len(), 32, "{bad}");
        assert!(t.spans[0].parent_id.is_none(), "{bad}");
    }
    let a = Trace::begin(None, "GET /x");
    let b = Trace::begin(None, "GET /x");
    assert_ne!(a.id(), b.id(), "fresh ids are random");
    assert!(a.id().bytes().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn the_payload_carries_the_status_the_attributes_and_the_error() {
    let mut t = Trace::begin(Some(TP), "POST /api/orders");
    t.attr(0, "http.request.method", json!("POST"));
    t.attr(0, "n", json!(3));
    t.attr(0, "ratio", json!(0.5));
    t.attr(0, "ok", json!(true));
    t.attr(0, "list", json!([1]));
    t.attr(9, "ignored", json!("no span 9"));
    let ok = t.finish(200);
    let span = &ok["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert_eq!(span["traceId"], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(span["parentSpanId"], "00f067aa0ba902b7");
    assert_eq!(span["status"]["code"], 1);
    let attrs = span["attributes"].to_string();
    for want in [r#""stringValue":"POST""#, r#""intValue":"3""#, r#""doubleValue":0.5"#, r#""boolValue":true"#, r#""stringValue":"[1]""#, r#""intValue":"200""#] {
        assert!(attrs.contains(want), "{want} in {attrs}");
    }
    assert!(!attrs.contains("ignored"));
    let mut t = Trace::begin(None, "GET /boom");
    t.fail(0, "the store refused");
    t.fail(5, "no such span");
    let bad = t.finish(500);
    let span = &bad["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert_eq!(span["status"]["code"], 2);
    assert!(span["attributes"].to_string().contains("the store refused"));
    assert_eq!(span["parentSpanId"], "", "a new trace has no parent");
}

#[test]
fn the_export_goes_to_the_configured_collector_with_its_headers_or_nowhere() {
    // No collector: nothing leaves.
    let site = Site::new();
    block_on(Trace::begin(None, "GET /a").export(&site.env(), 200));
    assert!(sent().is_empty());
    let site = Site::with_secrets(&[
        ("OTEL_EXPORTER_OTLP_ENDPOINT", "https://otel.example/"),
        ("OTEL_EXPORTER_OTLP_HEADERS", "x-api-key = k1, x-team=t2,malformed"),
    ]);
    answer_outbound(|_| Err(worker::Error::RustError("collector down".into())));
    // A collector that is down costs the request nothing: export returns.
    block_on(Trace::begin(Some(TP), "GET /a").export(&site.env(), 204));
    let calls = sent();
    assert_eq!(calls.len(), 1);
    let c = &calls[0];
    assert_eq!(c.url().unwrap().as_str(), "https://otel.example/v1/traces");
    assert_eq!(c.method(), worker::Method::Post);
    assert_eq!(c.headers().get("x-api-key").unwrap().as_deref(), Some("k1"));
    assert_eq!(c.headers().get("x-team").unwrap().as_deref(), Some("t2"));
    assert_eq!(c.headers().get("content-type").unwrap().as_deref(), Some("application/json"));
    let body: Value = serde_json::from_slice(&c.body_bytes()).unwrap();
    assert_eq!(body["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["traceId"], "4bf92f3577b34da6a3ce929d0e0e4736");
}
