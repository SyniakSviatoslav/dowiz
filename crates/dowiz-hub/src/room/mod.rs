//! THE ROOM'S DECIDERS, as pure functions over this crate's images (D7 phase 1,
//! BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.11, BLUEPRINT-BEBOP-IN-WASM §9).
//!
//! WHY THEY LIVE HERE AND NOT IN THE WORKER. `amend` and `pay` were already
//! pure -- images and an input in, a delta and a refusal out, no clock, no I/O
//! -- but they sat inside the Worker crate, which links `worker::` and cannot
//! be built for anything but a Cloudflare isolate. A tablet that keeps selling
//! with the network down needs the SAME decision, not a second one written in
//! JavaScript, and "the same" is only provable if it is the same code. So the
//! code moved: the Worker re-exports it (`command::amend`, `command::pay`,
//! `command::room_rules`, `fold`), and `crates/bebop-wasm --features decide`
//! exports it to any wasm32 host. `bebop-wasm/tests/decide_agrees.rs` and the
//! Worker's `command/amend/agrees.rs` hold both builds to one committed delta.
//!
//! WHAT DID NOT MOVE: anything that needs the Worker or the kernel. The wallet
//! leg (`command::pay::wallet`, `legs`) replays `dowiz_kernel`'s ledger, and the
//! tender fold (`pay::tender`) reads the sitting projection; both stay behind,
//! and a wallet payment's DEBIT is therefore decided by the server only.
//!
//! JSON HERE IS `serde_json`, not `minijson`. `minijson` says it is never
//! pointed at untrusted documents, and an order envelope holds a customer's own
//! words; the real parser is the only one that may fold it.

pub mod amend;
pub mod delta;
pub mod pay;
pub mod rules;
pub mod view;

/// Why a command was refused, and with what status.
///
/// THE STATUS IS PART OF THE REFUSAL because these are not the same
/// conversation. A short ingredient is a 409 that names the ingredient, so the
/// customer can change one line; a refused promo code is a 400 about what they
/// typed; an order this venue does not have is a 404 and must not leak that it
/// exists elsewhere. Collapsing them into one code would tell a customer their
/// basket was the problem when their coupon was.
///
/// Defined here, re-exported as `command::Refused` by the Worker, so every
/// command there and every decider here speaks one type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// An ingredient is short. Carries the ledger's own words, which name it.
    Stock(String),
    /// The code exists and cannot be redeemed: expired, spent, under the floor.
    Promo(String),
    /// The log would not take the event.
    Append(String),
    /// No such order in THIS venue's log. Deliberately indistinguishable from
    /// an order that belongs to another venue — see `assign::decide`.
    NotFound,
    /// The order is real and this is not something that can be done to it now:
    /// an illegal FSM edge, a second courier, a status that cannot be handed
    /// out. The kernel's own words where it has them.
    Conflict(String),
    /// The request itself is not one this command can take: a room event with
    /// no signer, a reason outside the closed set, a quantity that is not one.
    /// A 400, because retrying the same bytes will never succeed.
    Invalid(String),
    /// G2 (TAX): this venue has a tax rate and this order cannot carry its
    /// `tax` block. Never placed untaxed; the reason is named. 500: the
    /// platform's fault, not the customer's basket.
    Untaxed(String),
}

impl Refused {
    pub fn status(&self) -> u16 {
        match self {
            Refused::Promo(_) | Refused::Invalid(_) => 400,
            Refused::NotFound => 404,
            Refused::Stock(_) | Refused::Conflict(_) => 409,
            Refused::Append(_) | Refused::Untaxed(_) => 500,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Refused::Stock(m)
            | Refused::Promo(m)
            | Refused::Append(m)
            | Refused::Conflict(m)
            | Refused::Invalid(m)
            | Refused::Untaxed(m) => m,
            Refused::NotFound => "order not found",
        }
    }
}

#[cfg(test)]
mod tests;
