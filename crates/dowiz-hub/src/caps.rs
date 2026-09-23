//! What a member of staff may do: a CLOSED set of signed capabilities.
//!
//! THE SIGNER COMES BEFORE ANYTHING SIGNED. Every record the room is about to
//! grow — an amendment, a void, a payment, a till count — has to name who did
//! it, and a capability added after the fact is a capability nobody holds on
//! the records already written. So this lands first, empty-handed, before the
//! things it will authorise exist.
//!
//! A CAPABILITY, NEVER A SCORE (`DECISIONS.md` OD-8). Nothing here ranks,
//! rates or tiers a person. A capability is a name that is either in the set or
//! is not; there is no ordering on `Cap` and no arithmetic anywhere in this
//! file, so "who is the better waiter" is not a question this type can answer.
//!
//! DENY BY DEFAULT, the shape `crates/dowiz-core/src/ports/agent/scope.rs`
//! states as `RedLinePolicy::DenyByDefault`. `Caps::none()` is the starting
//! state. An unknown capability NAME refuses the whole set rather than being
//! dropped — a dropped name is a silent downgrade, and a silent downgrade of an
//! authority check is the defect this repo has paid for more than once.
//!
//! FOUR STAFF WORDS. `DECISIONS.md:297-298` rules three — **Owner / Kitchen / Counter-Manager** —
//! and the operator named the fourth (2026-09-23): **Waiter** / Офіціант / Kamarier.
//! All four are `Preset` below. The day the operator rules a fifth, it is one variant plus one
//! row in the `caps()` table (BLUEPRINT-POS-THE-ROOM §2.8).
//!
//! WHERE THIS LIVES AND WHY. In `dowiz-hub`, not in the Worker, because there
//! are TWO token implementations that must mean the same thing by the same
//! word: this crate's own `token.rs` (the self-hosted hub, read by
//! `tools/native-spa-server`) and `workers/api/src/auth.rs` (the Worker's
//! HS256 JWT). `workers/api` depends on this crate and this crate cannot depend
//! on it, so the Worker is the only one of the two that could import the other.
//! A second copy of the vocabulary is the `kit-had-a-fourth-money-copy` shape:
//! one spelling drifts, and the drift is an authority check.

use core::fmt;

/// One capability. A closed set: the admission decision is exhaustively
/// checkable, and a byte or a word outside it fails closed.
///
/// DELIBERATELY NOT `Ord`/`PartialOrd`. Ordering capabilities is the first
/// step towards ranking the people who hold them, and `kernel/src/decision`
/// omits those derives for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cap {
    /// Move a round through the kitchen's edges (`order_action`'s intents).
    Advance,
    /// Place a round, and amend one the kitchen has not taken yet.
    TakeOrders,
    /// Take a payment against an open bill.
    TakePayment,
    /// Void or comp a line after the kitchen has it.
    Void,
    /// Open, count and close the drawer; pay in and pay out.
    OpenTill,
}

impl Cap {
    /// Every capability, in canonical order. The order is the SPELLING of a
    /// set, so it is fixed here once: two orderings would be two strings for
    /// one set of rights, and a token is compared as bytes.
    pub const ALL: [Cap; 5] =
        [Cap::Advance, Cap::TakeOrders, Cap::TakePayment, Cap::Void, Cap::OpenTill];

    /// The wire name. Pinned, like `scope.rs`'s discriminants: a rename is a
    /// change to every token already minted, so it is a decision, not a tidy-up.
    pub fn as_str(self) -> &'static str {
        match self {
            Cap::Advance => "advance",
            Cap::TakeOrders => "take_orders",
            Cap::TakePayment => "take_payment",
            Cap::Void => "void",
            Cap::OpenTill => "open_till",
        }
    }

    /// `None` for anything else, which is what makes the parse fail closed.
    pub fn from_str(s: &str) -> Option<Cap> {
        Cap::ALL.into_iter().find(|c| c.as_str() == s)
    }

    /// The bit this capability occupies. Pinned rather than derived from the
    /// variant's position, so reordering the enum cannot silently re-mean an
    /// existing set.
    fn bit(self) -> u8 {
        match self {
            Cap::Advance => 0x01,
            Cap::TakeOrders => 0x02,
            Cap::TakePayment => 0x04,
            Cap::Void => 0x08,
            Cap::OpenTill => 0x10,
        }
    }
}

/// A set of capabilities. Five bits; no allocation, and `==` compares SETS
/// rather than spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Caps(u8);

impl Caps {
    /// The state a capability set starts in, and the answer to every question
    /// it has not been told otherwise about.
    pub fn none() -> Caps {
        Caps(0)
    }

    pub fn of(caps: &[Cap]) -> Caps {
        Caps(caps.iter().fold(0, |acc, c| acc | c.bit()))
    }

    pub fn allows(&self, c: Cap) -> bool {
        self.0 & c.bit() != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Read a comma-joined list of names.
    ///
    /// `None` on ANY name the set does not hold — the whole token is refused,
    /// not the one word. Honouring the rest would mean a typo, or a capability
    /// minted by a newer build, quietly becomes a smaller authority that
    /// nothing reports.
    pub fn parse(s: &str) -> Option<Caps> {
        let mut out = Caps::none();
        for name in s.split(',').filter(|p| !p.is_empty()) {
            out.0 |= Cap::from_str(name)?.bit();
        }
        Some(out)
    }

    /// This set, narrowed to what `live` still grants.
    ///
    /// The name is the rule: a token can only ever LOSE rights here. It is
    /// `auth.rs`'s first law — "authority is re-derived, never trusted from the
    /// token" — said about capabilities instead of about owner membership.
    pub fn narrowed_to(&self, live: &Caps) -> Caps {
        Caps(self.0 & live.0)
    }
}

impl fmt::Display for Caps {
    /// The canonical spelling: names in `Cap::ALL` order, comma-joined, and the
    /// empty set is the empty string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for c in Cap::ALL {
            if self.allows(c) {
                if !first {
                    f.write_str(",")?;
                }
                f.write_str(c.as_str())?;
                first = false;
            }
        }
        Ok(())
    }
}

/// A staff word the operator has ruled, and the capabilities behind it.
///
/// These are `DECISIONS.md:297-298`'s three plus the operator's fourth word (2026-09-23).
/// The capabilities table is in BLUEPRINT-POS-THE-ROOM §2.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Moves rounds through the kitchen. Takes no money.
    Kitchen,
    /// Takes orders and payments before the kitchen, voids before kitchen, no till access.
    Waiter,
    /// The counter: orders, payments, voids after the kitchen, and the drawer.
    CounterManager,
    /// Everything. An owner who also works the floor signs with the same set.
    Owner,
}

impl Preset {
    pub fn as_str(self) -> &'static str {
        match self {
            Preset::Kitchen => "kitchen",
            Preset::Waiter => "waiter",
            Preset::CounterManager => "counter-manager",
            Preset::Owner => "owner",
        }
    }

    /// The roster row's `role` word, as `bootstrap.rs:386` already writes one
    /// (`"role": "courier"`). Unknown — including `"courier"` — is `None`, and
    /// `admit` refuses on `None`.
    pub fn from_str(s: &str) -> Option<Preset> {
        [Preset::Kitchen, Preset::Waiter, Preset::CounterManager, Preset::Owner]
            .into_iter()
            .find(|p| p.as_str() == s)
    }

    pub fn caps(self) -> Caps {
        match self {
            Preset::Kitchen => Caps::of(&[Cap::Advance]),
            Preset::Waiter => Caps::of(&[Cap::TakeOrders, Cap::TakePayment]),
            Preset::CounterManager => {
                Caps::of(&[Cap::TakeOrders, Cap::TakePayment, Cap::Void, Cap::OpenTill])
            }
            Preset::Owner => Caps::of(&Cap::ALL),
        }
    }
}

/// What a staff token actually confers — the whole law, as pure logic, so the
/// refusal can be proved without a Durable Object or a signing key.
///
/// `minted` is the capability list the token was signed with; `roster_role` is
/// the word on the venue's LIVE roster row for this person, read at request
/// time. The answer is the intersection, and `None` means there is no principal
/// here at all.
///
/// DENY BY DEFAULT, three ways, all answering the same:
///   * a roster word the operator has not ruled grants nothing;
///   * a token naming a capability outside the closed set is refused entire;
///   * an empty intersection is not a signer — a token that can do nothing must
///     not authenticate, because a principal that exists is a principal some
///     later route will find a use for.
pub fn admit(minted: &str, roster_role: &str) -> Option<Caps> {
    let granted = Caps::parse(minted)?;
    let live = Preset::from_str(roster_role)?.caps();
    let caps = granted.narrowed_to(&live);
    if caps.is_empty() {
        return None;
    }
    Some(caps)
}

#[cfg(test)]
mod tests;
