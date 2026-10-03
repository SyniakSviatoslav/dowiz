//! WEB PUSH (W-PUSH, operator 2026-10-03: "push сповіщення обов'язкові").
//!
//! The customer's phone hears its order move, the courier's hears an order
//! handed to them, the venue's staff hear a new order -- through the browser's
//! own push service (FCM for Chrome/Android, Mozilla autopush for Firefox,
//! Apple's for Safari and installed iOS apps 16.4+), with no app store and no
//! account at any of them.
//!
//! THE LAYERS, each pure where it can be:
//!   ece      RFC 8291 payload encryption (`aes128gcm`)
//!   vapid    RFC 8292 ES256 token: who is sending
//!   subs     who asked, per venue (image `push`), and the endpoint allowlist
//!   words    the four languages, the order number and nothing personal
//!   plan     what an order turn owes which devices (the producer)
//!   rail     the send, inside the drain (`outbox/rails.rs`), 404/410 = gone
//!   routes   /api/push/{key,subscribe,unsubscribe,state}
//! The producer runs in the venue's object (`hubdo/push_turn.rs`) in the turn
//! that wrote the event, so "the order moved" and "the phone is owed a message"
//! are one write, and the outbox's retries and the alarm do the rest.

pub mod ece;
pub mod vapid;
pub mod plan;
pub mod rail;
pub mod routes;
pub mod subs;
pub mod words;
