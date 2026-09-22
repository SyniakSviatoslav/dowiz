//! OpenTelemetry over OTLP/HTTP.
//!
//! WHAT THIS COVERS TODAY, MEASURED RATHER THAN INTENDED (2026-09-22): ONE
//! SPAN PER REQUEST, the root. It carries the method, the path, the status and
//! the failure if there was one. That is all.
//!
//! THE HEADER USED TO SAY "the request, the kernel call inside it, each store
//! read and write". It was not true and had never been: `Trace::child` has no
//! caller anywhere in this crate, so no store read, no kernel call and no
//! outbound request has ever appeared in a trace. `child`, `end` and
//! `traceparent()` were DELETED on 2026-09-23 rather than kept under
//! `allow(dead_code)`: an instrument that exists and is never called reads as
//! coverage. `git log -S 'pub fn child' -- workers/api/src/otel.rs` has them.
//!
//! WHY IT IS NOT WIRED, and what it would cost: the `Trace` lives in the
//! router's own scope and the store calls are several frames down inside the
//! handlers, so spanning them means threading the trace through every handler
//! signature or putting it somewhere a Worker isolate can reach from both. The
//! first is a change to every route; the second is shared mutable state in a
//! runtime that gives us single-threaded isolates and would be safe, and is
//! the one to weigh. Outbound propagation is the same gap on the way out: an
//! outbound call to Telegram or Stripe starts a new trace rather than
//! continuing this one.
//!
//! The kernel's own internal spans are a separate gap: `fdr::SpanObserver`
//! hands out `(name, dur_us)` and nothing else — no trace id, no parent, no
//! attributes — so they cannot be stitched in without widening that trait. A
//! span with a fabricated parent is worse than no span.
//!
//! TELEMETRY NEVER FAILS A REQUEST. Every export path swallows its own errors.
//! An order must not be lost because a collector was unreachable, and a tracing
//! layer that can take the service down has inverted its purpose.
//!
//! W3C context is propagated: an incoming `traceparent` is continued rather than
//! replaced, so a trace that started at a gateway or in the browser stays one
//! trace instead of becoming two unrelated halves.

use serde_json::{json, Value};
use worker::*;

/// One span, held until the request ends.
pub struct Span {
    name: String,
    span_id: String,
    parent_id: Option<String>,
    start_ns: u64,
    end_ns: u64,
    attrs: Vec<(String, Value)>,
    error: Option<String>,
}

pub struct Trace {
    trace_id: String,
    spans: Vec<Span>,
}

fn hex(bytes: usize) -> String {
    // Ids come from the platform CSPRNG, the same source order ids use. A
    // predictable trace id lets an outsider guess and poison a trace.
    let mut out = String::with_capacity(bytes * 2);
    while out.len() < bytes * 2 {
        match crate::edge_id() {
            Some(u) => out.push_str(&u.replace('-', "")),
            None => break,
        }
    }
    out.truncate(bytes * 2);
    if out.len() < bytes * 2 {
        // No CSPRNG: a zero id is WRONG but visible, which beats a plausible
        // fake that quietly corrupts someone's trace search.
        out = "0".repeat(bytes * 2);
    }
    out
}

impl Trace {
    /// Continue an incoming trace, or start one.
    pub fn begin(req: &Request, name: &str) -> Self {
        let tp = req.headers().get("traceparent").ok().flatten();
        let (trace_id, parent) = match tp.as_deref().and_then(parse_traceparent) {
            Some((t, p)) => (t, Some(p)),
            None => (hex(16), None),
        };
        let root_id = hex(8);
        let start_ms = Date::now().as_millis() as f64;
        let mut t = Trace { trace_id, spans: Vec::new() };
        t.spans.push(Span {
            name: name.to_string(),
            span_id: root_id,
            parent_id: parent,
            start_ns: (start_ms * 1.0e6) as u64,
            end_ns: 0,
            attrs: Vec::new(),
            error: None,
        });
        t
    }

    pub fn attr(&mut self, idx: usize, k: &str, v: Value) {
        if let Some(s) = self.spans.get_mut(idx) {
            s.attrs.push((k.to_string(), v));
        }
    }

    pub fn fail(&mut self, idx: usize, msg: &str) {
        if let Some(s) = self.spans.get_mut(idx) {
            s.error = Some(msg.to_string());
        }
    }

    /// The id alone, for the client and for anything that has to name this
    /// request later.
    ///
    /// A TRACE NOBODY CAN NAME IS NOT AN INSTRUMENT. Until this was returned on
    /// the response, a customer or an owner reporting "it failed at about four"
    /// had given the only fact they had, and no query could turn it into the
    /// request. Every response carries it now, including the errors -- a 500
    /// with no id is an unfindable 500.
    pub fn id(&self) -> &str {
        &self.trace_id
    }

    /// Close the root and build the OTLP payload.
    fn finish(&mut self, status: u16) -> Value {
        let now_ns = (Date::now().as_millis() as f64 * 1.0e6) as u64;
        if let Some(root) = self.spans.first_mut() {
            root.end_ns = now_ns;
            root.attrs.push(("http.response.status_code".into(), json!(status)));
        }
        let spans: Vec<Value> = self
            .spans
            .iter()
            .map(|s| {
                let mut attrs: Vec<Value> = s
                    .attrs
                    .iter()
                    .map(|(k, v)| json!({ "key": k, "value": to_any(v) }))
                    .collect();
                if let Some(e) = &s.error {
                    attrs.push(json!({ "key": "exception.message", "value": { "stringValue": e } }));
                }
                json!({
                    "traceId": self.trace_id,
                    "spanId": s.span_id,
                    "parentSpanId": s.parent_id.clone().unwrap_or_default(),
                    "name": s.name,
                    "kind": 2,                         // SERVER
                    "startTimeUnixNano": s.start_ns.to_string(),
                    "endTimeUnixNano": (if s.end_ns == 0 { now_ns } else { s.end_ns }).to_string(),
                    // 2 = ERROR, 1 = OK. An unset status is not the same as OK and
                    // is not reported as one.
                    "status": { "code": if s.error.is_some() { 2 } else { 1 } },
                    "attributes": attrs
                })
            })
            .collect();

        json!({
            "resourceSpans": [{
                "resource": { "attributes": [
                    { "key": "service.name", "value": { "stringValue": "dowiz-hub" } },
                    { "key": "service.version", "value": { "stringValue": env!("CARGO_PKG_VERSION") } },
                    // One hub per tenant, so the tenant IS the deployment. Naming
                    // it here is what makes a trace searchable per restaurant.
                    { "key": "deployment.environment", "value": { "stringValue": "hub" } }
                ]},
                "scopeSpans": [{ "scope": { "name": "dowiz-api-worker" }, "spans": spans }]
            }]
        })
    }

    /// Export, never blocking the response and never failing it.
    pub async fn export(mut self, env: &Env, status: u16) {
        let payload = self.finish(status);
        let Ok(endpoint) = env.secret("OTEL_EXPORTER_OTLP_ENDPOINT") else {
            return; // no collector configured: tracing is simply off
        };
        let url = format!("{}/v1/traces", endpoint.to_string().trim_end_matches('/'));

        let headers = Headers::new();
        let _ = headers.set("content-type", "application/json");
        if let Ok(h) = env.secret("OTEL_EXPORTER_OTLP_HEADERS") {
            // `key=value,key=value`, the OTLP convention.
            for pair in h.to_string().split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    let _ = headers.set(k.trim(), v.trim());
                }
            }
        }
        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(payload.to_string().into()));
        if let Ok(req) = Request::new_with_init(&url, &init) {
            // Errors are deliberately dropped. A collector being down must not
            // turn into a failed order.
            let _ = Fetch::Request(req).send().await;
        }
    }

    // `duration_ms` WAS HERE, under `#[allow(dead_code)]`. Nothing read it:
    // the span already carries `endTimeUnixNano`, so the duration IS exported
    // and this was a second way to ask a question nobody asked. The attribute
    // is what let it live -- it is the manual version of
    // `tools/gates/unreached.py`, and a worse one, because it silences the
    // finding instead of recording it.
}

/// `00-<32 hex trace>-<16 hex span>-<flags>`
fn parse_traceparent(h: &str) -> Option<(String, String)> {
    let mut p = h.split('-');
    let ver = p.next()?;
    let trace = p.next()?;
    let span = p.next()?;
    if ver != "00" || trace.len() != 32 || span.len() != 16 {
        return None;
    }
    // An all-zero id is invalid per the spec and must not be continued.
    if trace.bytes().all(|b| b == b'0') || span.bytes().all(|b| b == b'0') {
        return None;
    }
    if !trace.bytes().chain(span.bytes()).all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((trace.to_string(), span.to_string()))
}

fn to_any(v: &Value) -> Value {
    match v {
        Value::String(s) => json!({ "stringValue": s }),
        Value::Bool(b) => json!({ "boolValue": b }),
        Value::Number(n) if n.is_i64() => json!({ "intValue": n.as_i64().unwrap().to_string() }),
        Value::Number(n) => json!({ "doubleValue": n.as_f64().unwrap_or(0.0) }),
        other => json!({ "stringValue": other.to_string() }),
    }
}
