//! THE FISCAL SEAM (TAX §3.7-3.8, BLIND-SPOTS §2.8): the document an order
//! owes the tax authority, the outbox entry that carries it with its 48 h
//! deadline, the receipt, and the one sender that reaches the tax platform.
//!
//! THE SEND IS ARMED BY THE OWNER, PER VENUE (card L70, operator-authorised
//! 2026-09-24). Until `ebills_arm` says every condition holds, nothing is
//! claimed and nothing is sent; the entry waits and the 48 h clock (law N)
//! reports it. Tests never touch the network: every one runs a mock till.
//!
//! - `document`: PURE, order Value -> `Document` or `Refusal` (rules 1-4)
//! - `queue`:    PURE, `outbox::Entry{kind:"fiscal"}`, the non-blocking drain, `health`
//! - `receipt`:  PURE, the art. 29 text ("pa NIVF" until the codes exist)
//! - `sender`:   `FiscalSender`, `NotConfigured`, and the test-only `Mock`
//! - `wire`:     PURE, `fiscal.since_ms`, what a placement queues, `health.fiscal`
//! - `rail`:     the health read, and the minute cron's eBills firing
//! - `ebills_arm`:    PURE, when a venue may send; the refund's cancel entry
//! - `ebills_body`:   PURE, a Document as the `POST /api/sales` body (golden: sale 7783)
//! - `ebills_sender`: PURE, intents, the plan, the settle, `EbillsSender`, `Noted{fiscal}`
//! - `ebills_fire`:   one firing over a `Transport` (the allow-listed client, or a mock)
//! - `routes`:        the owner's fiscal pane and the receipt

pub mod document;
pub mod ebills_arm;
pub mod ebills_body;
pub mod ebills_cmd;
pub mod ebills_fire;
pub mod ebills_sender;
pub mod queue;
pub mod receipt;
pub mod sender;
pub mod rail;
pub mod routes;
pub mod wire;

/// THE PLATFORM SWITCH for dowiz -> ebills (operator, 2026-09-24): OFF.
/// ebills -> dowiz (the till import) stays on; nothing is ever SENT to the
/// tax authority while this is false -- the cron sweep returns at once and the
/// arm route refuses. The sender, its tests and the console pane stay, so
/// turning it on is this one line plus a deploy.
pub const SEND_ENABLED: bool = false;
