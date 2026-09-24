//! THE ROOM: rounds placed at a table by a person, the check they add up to,
//! and the money taken against it (docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md).
//!
//! The decisions are pure and live beside the other commands in
//! `crate::command`; what is here is the door (`placer`), the projection a
//! waiter reads (`sitting`), and the handlers that sequence the two.

pub mod floor;
pub mod guest_round;
pub mod handlers;
pub mod pay;
pub mod placer;
pub mod table_link;
pub mod table_qr;
pub mod till;
pub mod transfer;

#[cfg(test)]
mod tests;
