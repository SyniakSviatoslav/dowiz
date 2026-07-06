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

use crate::{DomainError, Lek, OrderStatus, order_status::assert_transition};
use serde::{Deserialize, Serialize};

/// A timestamp in epoch-milliseconds, SUPPLIED BY THE CALLER. The core never reads a clock (Law 1);
/// time enters only as data carried on a [`Command`] and copied verbatim onto the [`Event`] it
/// produces. Two runs with the same commands therefore produce byte-identical event logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ts(pub i64);

/// The canonical hash of the command that CAUSED an event — the D2 dedupe / ordering / causality seam
/// carried on every [`Envelope`]. SUPPLIED BY THE CALLER, exactly like [`Ts`]: the core never computes
/// it (hashing a request would pull request-shaping and a hasher into the core, violating Laws 1–3);
/// the shell's `build_request_hash` (`api::routes::orders::request_hash`) fills it in. Opaque to the
/// kernel — the log only carries and compares it. `#[serde(transparent)]` ⇒ a bare JSON string on the
/// wire. (The plan's `codec/request_hash.rs` placement pre-dated the discovery that the hash's
/// COMPUTATION lives in the shell; the core owns only the type it carries in the log — so it lives
/// here, with the log alphabet, rather than behind a shell it cannot reach.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandHash(pub String);

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

/// The money snapshot an order carries once priced — the four integer totals the live `orders` row
/// persists (`subtotal, delivery_fee, tax_total, total`; `compose_total`'s inputs plus its result).
/// Recorded by the [`Event::Priced`] fact; an unpriced order carries `None`. Integer [`Lek`] by
/// construction (no float ever enters the core — the disallowed-types gate proves it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderTotals {
    pub subtotal: Lek,
    pub delivery_fee: Lek,
    pub tax_total: Lek,
    pub total: Lek,
}

/// A fact that HAS happened — the only thing [`fold`] consumes. Immutable by construction; a log of
/// these IS the order's history, and the current state is their fold.
///
/// The alphabet grew in GRAND-PLAN 0b-2 from the lone `StatusChanged` to the money/binding facts the
/// live lifecycle already produces (matching `policy::TransitionEffects`): a [`Priced`](Event::Priced)
/// snapshot, a [`RefundObligated`](Event::RefundObligated) obligation (policy L-A `refund_due`), and a
/// [`BindingTerminalized`](Event::BindingTerminalized) fact (policy R2-3). Each is part of the alphabet
/// and [`fold`] knows how to apply it; `decide` does NOT yet emit them — composing the corridors that
/// produce them behind the single `decide` door is GRAND-PLAN 0b-3. `at` stays on the frozen
/// `StatusChanged` (its byte shape is unchanged); the newer facts carry no independent time — the
/// [`Envelope`] records WHEN each fact entered the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    StatusChanged { from: OrderStatus, to: OrderStatus, at: Ts },
    /// The order's money snapshot was recorded (the `compute_order_pricing`/`compose_total` result).
    Priced {
        subtotal: Lek,
        delivery_fee: Lek,
        tax_total: Lek,
        total: Lek,
    },
    /// A refund obligation was recorded (policy L-A `refund_due`, `orderStatusService.ts:165`) — fires
    /// on →CANCELLED/→REJECTED for a paid order. INERT on the live path until crypto flips (zero paid
    /// rows today); carried whole so the fold is correct the moment it goes live.
    RefundObligated { amount: Lek },
    /// The active courier binding was terminalized and its shift freed (policy R2-3,
    /// `orderStatusService.ts:139`) — so no order leaves to a terminal/downgrade with a live strand.
    BindingTerminalized,
}

/// The Immutable Log's row: a fact ([`Event`]) plus the metadata that orders and de-duplicates it —
/// its position (`seq`), the time it was recorded (`at`), and the command that caused it (`cause`).
/// Every field enters as DATA; the core invents none of them (Laws 1 & 2). `decide` returns bare
/// [`Event`]s (it knows neither `seq` nor `cause`); the shell wraps each into an `Envelope` at the
/// persistence boundary. [`fold`] consumes only the `event` — `seq`/`at`/`cause` govern ordering,
/// causality, and dedupe, never state accumulation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub seq: u64,
    pub at: Ts,
    pub cause: CommandHash,
    pub event: Event,
}

/// The order aggregate — a value object, never mutated in place. Every [`fold`] yields a NEW one.
/// Beyond `status` it accumulates the money snapshot ([`OrderTotals`]) and the refund/binding facts,
/// so [`replay`] reconstructs the WHOLE order (status + money + binding) from its log alone (0b-2 DoD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderState {
    pub status: OrderStatus,
    /// The money snapshot, set by [`Event::Priced`]; `None` until the order is priced.
    pub totals: Option<OrderTotals>,
    /// The refund obligation recorded by [`Event::RefundObligated`] (L-A `refund_due`); `None` = none.
    pub refund_due: Option<Lek>,
    /// Whether an [`Event::BindingTerminalized`] fact has terminalized the courier binding (R2-3).
    pub binding_terminalized: bool,
}

impl OrderState {
    /// The genesis every brand-new order folds up from.
    pub const fn genesis() -> Self {
        OrderState {
            status: OrderStatus::Pending,
            totals: None,
            refund_due: None,
            binding_terminalized: false,
        }
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
/// it, never a runtime branch here. Each arm carries the rest of the aggregate along unchanged
/// (`..*state`) and mutates only the field its fact owns — the accumulating fold.
///
/// The match is EXHAUSTIVE BY DESIGN — no `_` arm. A new [`Event`] variant added without a fold arm is
/// a **compile error** (rustc E0004 — "forbidden transitions are compile errors", GRAND-PLAN 0b-2 DoD):
/// that is the PRIMARY gate, and the strongest possible (the compiler itself). The belt against the one
/// dodge E0004 permits — silencing it with a catch-all `_ => *state` that swallows the new fact — is the
/// deterministic `fold_stays_exhaustive_no_wildcard_arm` test below, which reads this function's own
/// source and fails on any `_ =>` arm. (clippy's `wildcard_enum_match_arm` was evaluated as the belt
/// and rejected: clippy 1.96 fires it only crate-wide on an owned direct-binding match, never on this
/// `&Event`/deref match — it would have been a false green.)
pub fn fold(state: &OrderState, event: &Event) -> OrderState {
    match *event {
        // The event carries the full new status; the rest of the aggregate rides along unchanged.
        Event::StatusChanged { to, .. } => OrderState { status: to, ..*state },
        Event::Priced {
            subtotal,
            delivery_fee,
            tax_total,
            total,
        } => OrderState {
            totals: Some(OrderTotals {
                subtotal,
                delivery_fee,
                tax_total,
                total,
            }),
            ..*state
        },
        // SET, not sum: at most one obligation per order, and a TOTAL fold cannot do fallible money
        // arithmetic (checked_add returns a Result). Multiple obligations are a 0b-3 corridor concern.
        Event::RefundObligated { amount } => OrderState {
            refund_due: Some(amount),
            ..*state
        },
        Event::BindingTerminalized => OrderState {
            binding_terminalized: true,
            ..*state
        },
    }
}

/// Replay an event log from a starting state. The current state IS this fold — the Immutable Log,
/// not a mutable "current status" column, is the source of truth (Manifesto §2 Storage).
pub fn replay(from: OrderState, events: &[Event]) -> OrderState {
    events.iter().fold(from, |state, event| fold(&state, event))
}

/// Replay an ENVELOPE log — the persisted on-wire form. Folds each envelope's [`Event`] in `seq`
/// order; the envelope metadata (`seq`/`at`/`cause`) governs ordering/causality/dedupe at the
/// persistence boundary, never state accumulation. The reconstructed state is the whole order —
/// status, money, and binding — from the log alone.
pub fn replay_envelopes(from: OrderState, log: &[Envelope]) -> OrderState {
    log.iter().fold(from, |state, env| fold(&state, &env.event))
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
        let ready = OrderState { status: OrderStatus::Ready, ..OrderState::genesis() };
        // READY -> CONFIRMED is not a machine edge.
        assert!(matches!(
            decide(&ready, Command::Confirm { at: T }),
            Err(DomainError::IllegalTransition { .. })
        ));
    }

    #[test]
    fn a_terminal_order_absorbs_every_command() {
        let delivered = OrderState { status: OrderStatus::Delivered, ..OrderState::genesis() };
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

    // ─────────────────────── 0b-2: money/binding facts fold into the aggregate ───────────────────────

    fn lek(n: i64) -> Lek {
        Lek::new(n).unwrap()
    }

    #[test]
    fn priced_fact_records_the_money_snapshot_and_leaves_status() {
        let confirmed = OrderState { status: OrderStatus::Confirmed, ..OrderState::genesis() };
        let after = fold(
            &confirmed,
            &Event::Priced {
                subtotal: lek(1_000),
                delivery_fee: lek(200),
                tax_total: lek(120),
                total: lek(1_320),
            },
        );
        assert_eq!(after.status, OrderStatus::Confirmed); // pricing does not move the machine
        assert_eq!(
            after.totals,
            Some(OrderTotals {
                subtotal: lek(1_000),
                delivery_fee: lek(200),
                tax_total: lek(120),
                total: lek(1_320),
            })
        );
    }

    #[test]
    fn refund_obligated_and_binding_terminalized_fold_independently() {
        let g = OrderState::genesis();
        assert_eq!(fold(&g, &Event::RefundObligated { amount: lek(500) }).refund_due, Some(lek(500)));
        assert!(fold(&g, &Event::BindingTerminalized).binding_terminalized);
        // each fact touches only its own field
        assert_eq!(fold(&g, &Event::BindingTerminalized).refund_due, None);
    }

    #[test]
    fn replay_of_a_mixed_log_reconstructs_status_money_and_binding() {
        // A log carrying every 0b-2 fact — the DoD: `replay` reconstructs the WHOLE order.
        let log = vec![
            Event::StatusChanged { from: OrderStatus::Pending, to: OrderStatus::Confirmed, at: T },
            Event::Priced { subtotal: lek(900), delivery_fee: lek(150), tax_total: lek(0), total: lek(1_050) },
            Event::StatusChanged { from: OrderStatus::Confirmed, to: OrderStatus::Cancelled, at: T },
            Event::BindingTerminalized,
            Event::RefundObligated { amount: lek(1_050) },
        ];
        let state = replay(OrderState::genesis(), &log);
        assert_eq!(state.status, OrderStatus::Cancelled);
        assert_eq!(state.totals.map(|t| t.total), Some(lek(1_050)));
        assert_eq!(state.refund_due, Some(lek(1_050)));
        assert!(state.binding_terminalized);
    }

    #[test]
    fn replay_envelopes_matches_replay_of_the_bare_events() {
        let events = vec![
            Event::StatusChanged { from: OrderStatus::Pending, to: OrderStatus::Confirmed, at: T },
            Event::Priced { subtotal: lek(10), delivery_fee: lek(0), tax_total: lek(0), total: lek(10) },
            Event::BindingTerminalized,
        ];
        let envelopes: Vec<Envelope> = events
            .iter()
            .enumerate()
            .map(|(i, &event)| Envelope {
                seq: i as u64,
                at: T,
                cause: CommandHash(format!("hash-{i}")),
                event,
            })
            .collect();
        assert_eq!(
            replay_envelopes(OrderState::genesis(), &envelopes),
            replay(OrderState::genesis(), &events)
        );
    }

    #[test]
    fn envelope_round_trips_through_canonical_bytes_carrying_its_cause() {
        use crate::{canonical_bytes, from_bytes};
        let env = Envelope {
            seq: 7,
            at: T,
            cause: CommandHash("deadbeef".to_string()),
            event: Event::Priced { subtotal: lek(5), delivery_fee: lek(1), tax_total: lek(0), total: lek(6) },
        };
        let bytes = canonical_bytes(&env).unwrap();
        let decoded: Envelope = from_bytes(&bytes).unwrap();
        assert_eq!(decoded, env);
        // cause survives as a bare JSON string (serde(transparent) on CommandHash)
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\"cause\":\"deadbeef\""), "got {s}");
    }

    /// The BELT to the rustc-E0004 primary gate (GRAND-PLAN 0b-2 DoD): `fold` MUST stay exhaustive —
    /// no catch-all `_` arm may silently swallow a future `Event` variant. E0004 already forces a new
    /// variant to be HANDLED; this deterministic source check forbids the one dodge E0004 permits
    /// (adding `_ => *state` to make it compile). It reads this file's own source and asserts `fold`'s
    /// body — from its signature to the following `replay` fn — contains no `_ =>` arm. RED proof:
    /// insert `_ => *state` into the match and this test fails.
    #[test]
    fn fold_stays_exhaustive_no_wildcard_arm() {
        let src = include_str!("kernel.rs");
        let start = src
            .find("pub fn fold(state: &OrderState, event: &Event)")
            .expect("fold fn signature present");
        let rel_end = src[start..]
            .find("\npub fn replay(")
            .expect("replay fn follows fold");
        let body = &src[start..start + rel_end];
        // Whitespace-insensitive so `_=>`, `_  =>`, and `_\n=>` are all caught, not just `_ =>`
        // (invariant-guardian durability note). No legitimate `_`-then-`=>` exists in the body — the
        // only `..` uses are struct-rest/functional-update, which compact to `..`, never `_=>`.
        let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            !compact.contains("_=>"),
            "fold must stay exhaustive — a catch-all `_ =>` arm would swallow a future Event variant \
             instead of folding it (GRAND-PLAN 0b-2). Handle the new variant explicitly."
        );
    }
}
