//! The kernel — the Manifesto's single "Law".
//!
//! Two functions and nothing else is the truth:
//!   - [`decide`]: `(&OrderState, Command) -> Result<Vec<Event>, DomainError>` — the ONE door every
//!     business action passes through. Pure, total, side-effect-free.
//!   - [`fold`]: `(&OrderState, &Event) -> OrderState` — TOTAL. Applying a fact cannot fail (a fact
//!     already happened); it produces a NEW state value, never mutating in place.
//!
//! `decide` returns EVENTS, not the next state — the current state is `events.fold(genesis, fold)`.
//! That is deliberately stronger than the Manifesto's `transition(State, Command) -> State`: it makes
//! the immutable event log the primary output by construction, so the log can never rot into an
//! afterthought. The next state is always recoverable by replaying the log ([`replay`]).
//!
//! ## Scope of this kernel (Phase-Zero, pre-extraction)
//! Today `decide` enforces exactly the MACHINE — `order_status::assert_transition`, the byte-frozen
//! 10-state relation. The richer corridors (the owner actor-gate, the CC-1 courier-binding guard,
//! the money composition) still live in the `api` shell (`routes/orders/{state,pricing}.rs`) and are
//! folded into this door in Phase-Zero Step 3, once the S5 money batch merges and those pure modules
//! can be relocated into the core without collision. Until then this is the honest minimum: the
//! status state machine expressed as the single `decide`/`fold` law, with time and identity entering
//! only as data on a Command (never read from a clock or an RNG — Laws 1 & 2).

// Sovereign-core Phase-Zero Step 3 — the pure lifecycle decisions that used to live in the `api`
// shell (`routes/orders/state.rs`), now relocated into the core alongside the `decide`/`fold` law they
// belong with. `policy` = actor-gate + fold-effects + CC-1 strand guards + honest-dispatch gate;
// `idempotency` = the create-idempotency branch decision. Both are float/clock/entropy/IO-free (the
// wasm sovereignty gate proves it). They are not yet composed INTO `decide` (that is the final wire
// step); today they are the honest relocation of the decisions the shell already consumes.
pub mod idempotency;
pub mod policy;
// Sovereign-core money composition (GRAND-PLAN 0b-1) — the pure order-total arithmetic relocated
// from the `api` shell (`routes/orders/pricing.rs`). Integer-only (no f64 — the disallowed-types
// gate proves it); the shell keeps a thin f64 adapter over this module.
pub mod pricing;

use crate::{DomainError, OrderStatus, order_status::assert_transition};
use serde::{Deserialize, Serialize};

/// A timestamp in epoch-milliseconds, SUPPLIED BY THE CALLER. The core never reads a clock (Law 1);
/// time enters only as data carried on a [`Command`] and copied verbatim onto the [`Event`] it
/// produces. Two runs with the same commands therefore produce byte-identical event logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ts(pub i64);

/// The intent alphabet — the actions an actor can *attempt* against an order. Humans and agents are
/// indistinguishable to the kernel: both are `Command` sources (Manifesto Phase Three, by
/// construction). Each command maps to exactly one target [`OrderStatus`]; the machine — not this
/// mapping — is the sole authority on whether the attempt is *legal*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Command {
    Confirm { at: Ts },
    Reject { at: Ts },
    StartPreparing { at: Ts },
    MarkReady { at: Ts },
    /// Drive toward IN_DELIVERY (legal from CONFIRMED or READY).
    Dispatch { at: Ts },
    MarkDelivered { at: Ts },
    MarkPickedUp { at: Ts },
    /// The IN_DELIVERY → READY revert (courier cancel/abort/owner-reassign never strands an order).
    RevertToReady { at: Ts },
    Cancel { at: Ts },
}

impl Command {
    /// The status this command drives the order toward. A pure total mapping — legality is decided
    /// by [`decide`] via the machine, never here.
    pub fn target(self) -> OrderStatus {
        match self {
            Command::Confirm { .. } => OrderStatus::Confirmed,
            Command::Reject { .. } => OrderStatus::Rejected,
            Command::StartPreparing { .. } => OrderStatus::Preparing,
            Command::MarkReady { .. } => OrderStatus::Ready,
            Command::Dispatch { .. } => OrderStatus::InDelivery,
            Command::MarkDelivered { .. } => OrderStatus::Delivered,
            Command::MarkPickedUp { .. } => OrderStatus::PickedUp,
            Command::RevertToReady { .. } => OrderStatus::Ready,
            Command::Cancel { .. } => OrderStatus::Cancelled,
        }
    }

    /// The caller-supplied event time carried by this command.
    pub fn at(self) -> Ts {
        match self {
            Command::Confirm { at }
            | Command::Reject { at }
            | Command::StartPreparing { at }
            | Command::MarkReady { at }
            | Command::Dispatch { at }
            | Command::MarkDelivered { at }
            | Command::MarkPickedUp { at }
            | Command::RevertToReady { at }
            | Command::Cancel { at } => at,
        }
    }
}

/// A fact that HAS happened — the only thing [`fold`] consumes. Immutable by construction; a log of
/// these IS the order's history, and the current state is their fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    StatusChanged { from: OrderStatus, to: OrderStatus, at: Ts },
}

/// The order aggregate — a value object, never mutated in place. Every [`fold`] yields a NEW one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderState {
    pub status: OrderStatus,
}

impl OrderState {
    /// The genesis every brand-new order folds up from.
    pub const fn genesis() -> Self {
        OrderState { status: OrderStatus::Pending }
    }
}

/// THE LAW. Given the current state and an attempted command, return the events it produces — or the
/// reason it is refused. Pure, total, side-effect-free: no clock, no entropy, no IO. The machine
/// (`assert_transition`) is the sole legality authority, so a terminal order absorbs every command
/// (its transition table is empty → `Err`), and no event can ever escape a terminal state.
pub fn decide(state: &OrderState, command: Command) -> Result<Vec<Event>, DomainError> {
    let to = command.target();
    assert_transition(state.status, to)?;
    Ok(vec![Event::StatusChanged {
        from: state.status,
        to,
        at: command.at(),
    }])
}

/// TOTAL. Applying a fact cannot fail — it already happened. Produces a new state (no `&mut self`);
/// if this could ever fail, `decide` was wrong, and the replay property (Hard Truth Layer 2) catches
/// it, never a runtime branch here.
pub fn fold(_state: &OrderState, event: &Event) -> OrderState {
    // `_state` is unused for the single `StatusChanged` variant (the event carries the full new
    // status). The parameter stays in the signature because it IS the fold contract — future event
    // variants that accumulate (e.g. money) will read it, at which point it loses its underscore.
    match event {
        Event::StatusChanged { to, .. } => OrderState { status: *to },
    }
}

/// Replay an event log from a starting state. The current state IS this fold — the Immutable Log,
/// not a mutable "current status" column, is the source of truth (Manifesto §2 Storage).
pub fn replay(from: OrderState, events: &[Event]) -> OrderState {
    events.iter().fold(from, |state, event| fold(&state, event))
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Ts = Ts(1_700_000_000_000);

    #[test]
    fn decide_emits_one_status_changed_event_carrying_the_command_time() {
        let state = OrderState::genesis(); // PENDING
        let events = decide(&state, Command::Confirm { at: T }).unwrap();
        assert_eq!(
            events,
            vec![Event::StatusChanged {
                from: OrderStatus::Pending,
                to: OrderStatus::Confirmed,
                at: T,
            }]
        );
    }

    #[test]
    fn fold_produces_a_new_state_and_leaves_the_input_untouched() {
        let before = OrderState::genesis();
        let event = Event::StatusChanged {
            from: OrderStatus::Pending,
            to: OrderStatus::Confirmed,
            at: T,
        };
        let after = fold(&before, &event);
        assert_eq!(before.status, OrderStatus::Pending); // input value unchanged
        assert_eq!(after.status, OrderStatus::Confirmed);
    }

    #[test]
    fn illegal_command_is_refused_by_the_machine() {
        let ready = OrderState { status: OrderStatus::Ready };
        // READY -> CONFIRMED is not a machine edge.
        assert!(matches!(
            decide(&ready, Command::Confirm { at: T }),
            Err(DomainError::IllegalTransition { .. })
        ));
    }

    #[test]
    fn a_terminal_order_absorbs_every_command() {
        let delivered = OrderState { status: OrderStatus::Delivered };
        for cmd in [
            Command::Cancel { at: T },
            Command::Confirm { at: T },
            Command::RevertToReady { at: T },
        ] {
            assert!(decide(&delivered, cmd).is_err(), "delivered must absorb {cmd:?}");
        }
    }

    #[test]
    fn a_full_lifecycle_folds_up_from_the_event_log() {
        let genesis = OrderState::genesis();
        let commands = [
            Command::Confirm { at: T },
            Command::StartPreparing { at: T },
            Command::MarkReady { at: T },
            Command::Dispatch { at: T },
            Command::MarkDelivered { at: T },
        ];
        let mut state = genesis;
        let mut log = Vec::new();
        for &c in &commands {
            let events = decide(&state, c).expect("each step is a legal edge");
            for e in &events {
                state = fold(&state, e);
            }
            log.extend(events);
        }
        assert_eq!(state.status, OrderStatus::Delivered);
        // The state is fully reconstructible from the log alone.
        assert_eq!(replay(genesis, &log), state);
    }
}
