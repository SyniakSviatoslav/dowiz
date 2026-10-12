//! The runtime's ping answer (W-CHEAP A3). `set_websocket_auto_response` is the platform's own
//! and has no Mem twin (`Host::Mem` accepts no sockets), so what is held here is the contract it
//! depends on: the frame the client sends IS the pair's request, byte for byte, and the object's
//! own fallback answers that frame with the pair's response.

use super::{PING, PONG};

#[test]
fn the_pair_is_the_frame_the_client_sends() {
    let live = include_str!("../../../public/lib/live.js");
    assert!(live.contains("ws.send(JSON.stringify({ t: 'ping' }))"), "public/lib/live.js no longer sends the ping the pair names");
    // `JSON.stringify({ t: 'ping' })` is `{"t":"ping"}`: no spaces, one key.
    assert_eq!(serde_json::to_string(&serde_json::json!({"t": "ping"})).unwrap(), PING);
}

#[test]
fn the_objects_fallback_answers_the_ping_with_the_pairs_pong() {
    let h = crate::hubdo::host::mem::MemHost::named("alpha", 0);
    let obj = crate::hubdo::HubImages::in_memory(h);
    assert_eq!(obj.on_message(PING, Vec::new), Some(PONG));
    // And nothing else is a ping: a frame with a space is the object's to judge, not the runtime's.
    assert_ne!(r#"{"t": "ping"}"#, PING);
}
