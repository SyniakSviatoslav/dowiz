//! THE FISCAL SEAM (TAX §3.7-3.8, BLIND-SPOTS §2.8): the document an order
//! owes the tax authority, the outbox entry that carries it with its 48 h
//! deadline, the receipt, and the sender that — today — sends nothing.
//!
//! HARD LIMIT (operator): NOTHING is sent to ebills.al or any tax endpoint.
//! Production has exactly one sender, `sender::NotConfigured`, which refuses
//! every send without opening a connection; the drain then leaves the entry
//! untouched and the 48 h clock (law N) is what reports it. The operator turns
//! the real push on later, as an adapter in the ebills lane's scope.
//!
//! - `document`: PURE, order Value -> `Document` or `Refusal` (rules 1-4)
//! - `queue`:    PURE, `outbox::Entry{kind:"fiscal"}`, the non-blocking drain, `health`
//! - `receipt`:  PURE, the art. 29 text ("pa NIVF" until the codes exist)
//! - `sender`:   `FiscalSender`, `NotConfigured`, and the test-only `Mock`
//! - `wire`:     PURE, `fiscal.since_ms`, what a placement queues, `health.fiscal`
//! - `rail`:     the one read `health.fiscal` needs (`hubdo/fiscal.rs` is the one write)

pub mod document;
pub mod queue;
pub mod receipt;
pub mod sender;
pub mod rail;
pub mod wire;
