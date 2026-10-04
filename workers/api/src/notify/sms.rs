//! ORDER-STATUS SMS TO THE CUSTOMER (W-SMS, operator 2026-10-03: "для sms
//! потрібно знайти і добавити безкоштовний варіант й обов'язково усе перевірити").
//!
//! THE FREE PATH IS THE VENUE'S OWN PHONE. The default gateway is "SMS Gateway
//! for Android" (SMSGate, capcom6/android-sms-gateway, Apache-2.0) in Cloud
//! mode: the Worker posts to `api.sms-gate.app`, the venue's Android phone
//! sends the text from its own SIM. dowiz pays nothing and holds NO platform
//! key; a paid provider (Twilio) or textbee is the owner's own key, exactly as
//! the Telegram bot is the venue's own. Research and sources:
//! `docs/research/2026-10-03-free-sms.md`.
//!
//! ASKED, NEVER ASSUMED: only an order whose customer ticked the UNTICKED SMS
//! box at checkout carries the `sms` stamp (`checkout`), the tick is filed in
//! the venue's consent image (`order_status` on `sms`), and the drain re-asks
//! that fold before every send (`rail`), so a STOP filed after the order wins.
//!
//! THE LAYERS, each pure where it can be:
//!   phone     a typed number to E.164, or nothing
//!   words     the four languages; order number + venue name + STOP, nothing else
//!   plan      what an order turn owes (the producer, `hubdo/sms_turn.rs`)
//!   config    the venue's settings keys, the masked view, the owner's edit
//!   gateway   the three providers' requests and their answers, classified
//!   rail      the send inside the drain: consent, daily budget, health record
//!   checkout  the box at placement: the act, the order's stamp
//!   routes    /api/owner/sms{,/test,/stop}, /api/public/locations/:slug/sms

pub mod checkout;
pub mod config;
pub mod gateway;
pub mod phone;
pub mod plan;
pub mod rail;
pub mod routes;
pub mod words;
