//! The WS wire contract — ported from `apps/api/src/websocket.ts`'s inline `JSON.parse`/
//! `JSON.stringify` shapes (there was never a shared schema on the Node side; this module is the
//! first time the shapes are named).
//!
//! ## REV-S6-3 — opaque passthrough during the cutover overlap (🔴)
//! The room-fanout envelope (`{room, data}`, `websocket.ts:218`) is NOT typed-decoded here. `data`
//! stays [`serde_json::value::RawValue`] end to end: `PgListener` receives whatever JSON a Node
//! producer (S5/S7/S8, still on Node during overlap) published, and this module splices it into
//! the envelope UNPARSED. A typed `Event` enum that decoded and re-encoded `data` would drop
//! unmodeled producer fields, reformat numbers (`52` → `52.0`) — AND (a drift this build's own
//! golden-frame test caught live) reorder object keys, since `serde_json::Value`'s map is a sorted
//! `BTreeMap` by default: parsing into `Value` and re-serializing is ALREADY a re-encode, even
//! though every field content survives it. `RawValue` is the one serde_json type that carries the
//! original bytes through untouched — breaking a client that reconnects mid-session from Node onto
//! Rust (or back) is exactly what a byte-level (not just field-level) passthrough prevents. Typed
//! `Event` variants are DEFERRED to a post-cutover, FE-lockstep release (resolution.md REV-S6-3,
//! proposal §7).
//!
//! Only the CONTROL frames (`auth_success`/`subscribed`/`error`/`client_location`/
//! `client_location_stop`/`resync`) are fully typed here, because the Rust WS server ITSELF
//! originates them (they are not passed through from a Node producer) — carrying their exact wire
//! shape is a byte-for-byte port, not a re-encode of someone else's payload.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

// ─────────────────────────── inbound (5 ClientMsg kinds) ───────────────────────────

/// `{lat, lng}` — `websocket.ts:459-461`'s inline range validation (`-90..=90` / `-180..=180`) is
/// enforced by the caller (`ws::mod`), not by this deserializer, so an out-of-range value still
/// parses (matching Node, which parses first and range-checks second, silently dropping an
/// out-of-range payload rather than protocol-erroring on it).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ClientLocationInput {
    pub lat: f64,
    pub lng: f64,
}

/// The 5 inbound message kinds (`websocket.ts:364-490`). Deliberately NOT `deny_unknown_fields`:
/// the live Node server never rejected an extra field on a recognized `type` — only a
/// wholly-unrecognized `type` (or invalid JSON) gets any special handling (see `ws::mod`'s dispatch
/// doc). Carrying that leniency avoids a behavior change nobody asked for during the overlap.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    #[serde(rename = "auth")]
    Auth { token: String },
    #[serde(rename = "subscribe")]
    Subscribe { room: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { room: String },
    #[serde(rename = "client_location")]
    ClientLocation { payload: ClientLocationInput },
    #[serde(rename = "client_location_stop")]
    ClientLocationStop {},
}

// ─────────────────────────── outbound control frames ───────────────────────────

/// The frames the Rust server itself originates (never passed through from a bus NOTIFY). Every
/// shape here is pinned VERBATIM against the live Node strings by the golden-frame tests below —
/// this is the wire contract a client reconnecting mid-session from Node must still parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlFrame {
    /// `{type:'auth_success', role}` — `websocket.ts:346,379`.
    #[serde(rename = "auth_success")]
    AuthSuccess { role: &'static str },
    /// `{type:'subscribed', room}` — `websocket.ts:436`.
    #[serde(rename = "subscribed")]
    Subscribed { room: String },
    /// The one frame shape covering EVERY error/soft-deny/eviction notice today
    /// (`{type:'error', error}` and, only on the WS-only retryable soft-error,
    /// `{type:'error', error, retryable:true}` — `websocket.ts:395,400,405,412,421,425,430`). This
    /// is ALSO the Q-WS-EVICT-FRAME carry: `binding_revoked`/`membership_revoked` eviction notices
    /// use this exact shape during the overlap (a typed `Event::Evicted{reason}` is post-cutover).
    #[serde(rename = "error")]
    Error {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        retryable: Option<bool>,
    },
    /// Customer GPS relay to bound couriers — `{type:'client_location', payload:{lat,lng,timestamp}}`
    /// (`websocket.ts:465-468`).
    #[serde(rename = "client_location")]
    ClientLocation { payload: ClientLocationRelay },
    /// `{type:'client_location_stop'}` — `websocket.ts:482`, no payload field.
    #[serde(rename = "client_location_stop")]
    ClientLocationStop {},
    /// REV-S6-6 — a first-class signal the FE refetches on: fired (a) on a claim-check truncation
    /// (Q-WS-CLAIMCHECK, replacing the old accidental-refetch `_truncated` heuristic with an
    /// explicit contract) and (b) on `PgListener` degraded→healthy recovery (REV-S6-1's heartbeat
    /// is the trigger — a NOTIFY lost mid-outage leaves the client silently stale otherwise,
    /// resolution.md REV-S6-6). This is a NEW frame kind (no Node equivalent to carry), so it is
    /// typed rather than opaque from day one.
    #[serde(rename = "resync")]
    Resync { entity: String, id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClientLocationRelay {
    pub lat: f64,
    pub lng: f64,
    pub timestamp: i64,
}

/// The generic bus-fanout envelope — `{room, data}` (`websocket.ts:218`). `data` is the OPAQUE
/// passthrough (see module doc, REV-S6-3): the producer's original JSON bytes, unparsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEnvelope {
    pub room: String,
    pub data: Box<RawValue>,
}

/// Everything the per-connection writer task can be asked to push. A thin sum of the two outbound
/// shapes above so `ws::rooms`'s registry can be generic over "a frame", without needing to know
/// whether it originated as a control frame or a bus fan-out.
#[derive(Debug, Clone)]
pub enum WireMessage {
    Control(ControlFrame),
    Room(RoomEnvelope),
}

impl WireMessage {
    /// Renders the exact wire text. Encoding one of these well-formed internal types can only fail
    /// on a non-finite float or a non-string map key, neither of which this module ever constructs
    /// (`ClientLocationRelay`'s f64s come from a validated `-90..=90`/`-180..=180` range check
    /// upstream) — the fallback below exists purely so this stays `unwrap`-free, not because the
    /// error arm is expected to be reachable.
    pub fn into_text(self) -> String {
        let encoded = match self {
            WireMessage::Control(frame) => serde_json::to_string(&frame),
            WireMessage::Room(envelope) => serde_json::to_string(&envelope),
        };
        encoded
            .unwrap_or_else(|_| r#"{"type":"error","error":"internal encode error"}"#.to_string())
    }
}

impl From<ControlFrame> for WireMessage {
    fn from(frame: ControlFrame) -> Self {
        WireMessage::Control(frame)
    }
}

impl From<RoomEnvelope> for WireMessage {
    fn from(envelope: RoomEnvelope) -> Self {
        WireMessage::Room(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── REV-S6-3 golden-frame parity: fixed event → exact Node JSON string, both directions ──

    #[test]
    fn golden_auth_success_matches_node_shape() {
        let frame = ControlFrame::AuthSuccess { role: "owner" };
        assert_eq!(
            serde_json::to_string(&frame).unwrap(),
            r#"{"type":"auth_success","role":"owner"}"#
        );
    }

    #[test]
    fn golden_subscribed_matches_node_shape() {
        let frame = ControlFrame::Subscribed {
            room: "order:11111111-1111-1111-1111-111111111111".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&frame).unwrap(),
            r#"{"type":"subscribed","room":"order:11111111-1111-1111-1111-111111111111"}"#
        );
    }

    #[test]
    fn golden_bare_error_omits_retryable_field_entirely() {
        // websocket.ts:395 `{type:'error', error:'Forbidden room'}` — no `retryable` key at all,
        // not `retryable: null`. A typed re-encode that emitted `"retryable":null` would already
        // be a shape drift a strict Node-side consumer could choke on.
        let frame = ControlFrame::Error {
            error: "Forbidden room".to_string(),
            retryable: None,
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert_eq!(json, r#"{"type":"error","error":"Forbidden room"}"#);
        assert!(!json.contains("retryable"));
    }

    #[test]
    fn golden_retryable_error_matches_node_shape() {
        // websocket.ts:421 `{type:'error', error:'Service temporarily unavailable', retryable:true}`.
        let frame = ControlFrame::Error {
            error: "Service temporarily unavailable".to_string(),
            retryable: Some(true),
        };
        assert_eq!(
            serde_json::to_string(&frame).unwrap(),
            r#"{"type":"error","error":"Service temporarily unavailable","retryable":true}"#
        );
    }

    #[test]
    fn golden_client_location_stop_has_no_payload_field() {
        let frame = ControlFrame::ClientLocationStop {};
        assert_eq!(
            serde_json::to_string(&frame).unwrap(),
            r#"{"type":"client_location_stop"}"#
        );
    }

    #[test]
    fn golden_client_location_relay_matches_node_shape() {
        let frame = ControlFrame::ClientLocation {
            payload: ClientLocationRelay {
                lat: 41.3275,
                lng: 19.8187,
                timestamp: 1_700_000_000_000,
            },
        };
        assert_eq!(
            serde_json::to_string(&frame).unwrap(),
            r#"{"type":"client_location","payload":{"lat":41.3275,"lng":19.8187,"timestamp":1700000000000}}"#
        );
    }

    /// REV-S6-3's actual load-bearing property: a producer payload with fields this Rust binary
    /// has NEVER modeled (an unmodeled key, nested objects, an integer that must stay bare, not
    /// `.0`-suffixed) AND its exact KEY ORDER round-trip byte-identically through the envelope,
    /// because `data` is `RawValue` (unparsed bytes), never a typed struct or a re-serialized
    /// `serde_json::Value` (whose map is a sorted `BTreeMap` by default — parsing into `Value` and
    /// re-emitting it is ALREADY a re-encode that reorders keys, a drift this exact test caught
    /// live during the build before the fix landed).
    #[test]
    fn opaque_passthrough_carries_unmodeled_fields_and_integer_shape_verbatim() {
        let producer_json = r#"{"type":"order.status","data":{"status":"CONFIRMED","total":52,"nested":{"a":1,"b":[1,2,3]},"futureField":"unmodeled-by-rust"}}"#;
        let data = RawValue::from_string(producer_json.to_string()).unwrap();
        let envelope = RoomEnvelope {
            room: "order:22222222-2222-2222-2222-222222222222".to_string(),
            data,
        };
        let wire = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            wire,
            format!(
                r#"{{"room":"order:22222222-2222-2222-2222-222222222222","data":{producer_json}}}"#
            ),
            "the opaque data blob must reappear byte-for-byte (same key ORDER too) inside the {{room,data}} envelope"
        );
        // The number `52` must stay a bare integer (not become `52.0`) — the exact drift class
        // REV-S6-3 names as unrepresentable-by-design under a typed re-encode.
        assert!(
            wire.contains(r#""total":52,"#),
            "an integer must not reformat to 52.0"
        );
    }

    #[test]
    fn client_msg_parses_the_five_inbound_kinds() {
        let auth: ClientMsg = serde_json::from_str(r#"{"type":"auth","token":"t"}"#).unwrap();
        assert!(matches!(auth, ClientMsg::Auth { token } if token == "t"));

        let sub: ClientMsg =
            serde_json::from_str(r#"{"type":"subscribe","room":"order:x"}"#).unwrap();
        assert!(matches!(sub, ClientMsg::Subscribe { room } if room == "order:x"));

        let unsub: ClientMsg =
            serde_json::from_str(r#"{"type":"unsubscribe","room":"order:x"}"#).unwrap();
        assert!(matches!(unsub, ClientMsg::Unsubscribe { .. }));

        let loc: ClientMsg =
            serde_json::from_str(r#"{"type":"client_location","payload":{"lat":1.0,"lng":2.0}}"#)
                .unwrap();
        assert!(
            matches!(loc, ClientMsg::ClientLocation { payload } if payload.lat == 1.0 && payload.lng == 2.0)
        );

        let stop: ClientMsg = serde_json::from_str(r#"{"type":"client_location_stop"}"#).unwrap();
        assert!(matches!(stop, ClientMsg::ClientLocationStop {}));
    }

    #[test]
    fn client_msg_rejects_an_unrecognized_type() {
        // The caller (ws::mod) treats this Err as "unknown message type" (console.warn parity,
        // socket stays open) — see that module's dispatch doc.
        let result: Result<ClientMsg, _> =
            serde_json::from_str(r#"{"type":"teleport","room":"x"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn client_msg_tolerates_an_extra_unknown_field_on_a_known_type() {
        // NOT deny_unknown_fields (module doc) — an extra field on a recognized type must still parse.
        let result: Result<ClientMsg, _> =
            serde_json::from_str(r#"{"type":"subscribe","room":"order:x","extra":true}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn wire_message_into_text_dispatches_to_the_right_encoder() {
        let control: WireMessage = ControlFrame::Subscribed {
            room: "order:x".to_string(),
        }
        .into();
        assert_eq!(
            control.into_text(),
            r#"{"type":"subscribed","room":"order:x"}"#
        );

        let room_msg: WireMessage = RoomEnvelope {
            room: "order:x".to_string(),
            data: RawValue::from_string("{\"a\":1}".to_string()).unwrap(),
        }
        .into();
        assert_eq!(room_msg.into_text(), r#"{"room":"order:x","data":{"a":1}}"#);
    }
}
