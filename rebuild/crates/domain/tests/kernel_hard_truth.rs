//! Hard Truth suite — kernel layer (Phase-Zero Step 2, extended to the `decide`/`fold` Law).
//!
//! Where `hard_truth.rs` proves the UNBOUNDED money algebra, this proves the KERNEL: that driving
//! an order with an arbitrary stream of commands is deterministic, that the state is always exactly
//! the fold of its event log (replay), and that terminal states absorb everything. These are the
//! Manifesto's core promises ("same intents in → same final state; the UI is a mirror of the fold")
//! stated as falsifiable properties.
//!
//! Layer map (PHASE-ZERO.md §4):
//!   - Layer 1 Determinism — `run(cmds) == run(cmds)`; the event log is byte-identical run to run.
//!   - Layer 2 Totality/Replay — `decide`/`fold` never panic; `state == replay(genesis, log)` at
//!     every prefix.
//!   - Layer 3 Corridor — terminal absorption: no command produces an event out of a terminal state.

use domain::{
    ALL_STATUSES, Command, CommandHash, Envelope, Event, Lek, OrderState, OrderStatus, OrderTotals,
    Ts, canonical_bytes, decide, decode_log, encode_log, fold, from_bytes, is_terminal, replay,
    replay_envelopes,
};
use proptest::prelude::*;

/// Drive a state through a command stream. Illegal commands are refused by the machine and left as
/// no-ops (the order stays put) — exactly "the kernel refuses, it does not corrupt". Returns the
/// final state and the accepted event log.
fn run(genesis: OrderState, commands: &[Command]) -> (OrderState, Vec<Event>) {
    let mut state = genesis;
    let mut log = Vec::new();
    for &cmd in commands {
        if let Ok(events) = decide(&state, cmd) {
            for e in &events {
                state = fold(&state, e);
            }
            log.extend(events);
        }
    }
    (state, log)
}

/// The `to` a `StatusChanged` event carries (the status-moving variant). The 0b-2 money/binding
/// facts do not move the machine, so only `StatusChanged` reports a target here.
fn status_change_to(e: &Event) -> Option<OrderStatus> {
    match e {
        Event::StatusChanged { to, .. } => Some(*to),
        Event::Priced { .. } | Event::RefundObligated { .. } | Event::BindingTerminalized => None,
    }
}

/// A non-negative money amount (the only kind `Lek` represents).
fn any_lek() -> impl Strategy<Value = Lek> {
    (0i64..1_000_000_000).prop_map(|v| Lek::new(v).expect("non-negative"))
}

/// Any event variant — the full 0b-2 alphabet, with arbitrary caller-supplied data.
fn any_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (any_status(), any_status(), any::<i64>())
            .prop_map(|(from, to, t)| Event::StatusChanged { from, to, at: Ts(t) }),
        (any_lek(), any_lek(), any_lek(), any_lek()).prop_map(|(subtotal, delivery_fee, tax_total, total)| {
            Event::Priced { subtotal, delivery_fee, tax_total, total }
        }),
        any_lek().prop_map(|amount| Event::RefundObligated { amount }),
        Just(Event::BindingTerminalized),
    ]
}

/// Any command variant, with an arbitrary caller-supplied timestamp.
fn any_command() -> impl Strategy<Value = Command> {
    (0u8..9, any::<i64>()).prop_map(|(kind, t)| {
        let at = Ts(t);
        match kind {
            0 => Command::Confirm { at },
            1 => Command::Reject { at },
            2 => Command::StartPreparing { at },
            3 => Command::MarkReady { at },
            4 => Command::Dispatch { at },
            5 => Command::MarkDelivered { at },
            6 => Command::MarkPickedUp { at },
            7 => Command::RevertToReady { at },
            _ => Command::Cancel { at },
        }
    })
}

fn any_status() -> impl Strategy<Value = OrderStatus> {
    prop::sample::select(ALL_STATUSES.to_vec())
}

proptest! {
    // ─────────────────────────── Layer 1 — Determinism ───────────────────────────

    /// The Manifesto's headline promise: the same intents over the same genesis yield the identical
    /// final state AND the identical event log — every time. No clock/entropy/ordering can leak in
    /// (there is none to leak). Because `run` also completes for EVERY generated stream, this is the
    /// Layer-2 totality witness too.
    #[test]
    fn kernel_run_is_deterministic(cmds in prop::collection::vec(any_command(), 0..40)) {
        let g = OrderState::genesis();
        prop_assert_eq!(run(g, &cmds), run(g, &cmds));
    }

    // ─────────────────────────── Layer 2 — Replay / totality ───────────────────────────

    /// "The state is only ever the fold of its log." Replaying the accepted log from genesis
    /// reproduces the final state exactly — at EVERY prefix, not just the end. This is the property
    /// that makes the event log (not a mutable status column) the source of truth.
    #[test]
    fn state_is_the_fold_of_its_log_at_every_prefix(cmds in prop::collection::vec(any_command(), 0..40)) {
        let g = OrderState::genesis();
        let (final_state, log) = run(g, &cmds);
        prop_assert_eq!(replay(g, &log), final_state);
        for k in 0..=log.len() {
            let partial = replay(g, &log[..k]);
            if k > 0 {
                // each prefix ends on exactly the status the k-th accepted event moved to (every
                // event `run` accepts is a StatusChanged — `decide` emits nothing else pre-0b-3)
                let moved_to = status_change_to(&log[k - 1]).expect("run's log is StatusChanged-only");
                prop_assert_eq!(partial.status, moved_to);
            } else {
                prop_assert_eq!(partial.status, OrderStatus::Pending);
            }
        }
    }

    /// Structural law of `decide`: a legal command yields exactly one `StatusChanged` whose `from`
    /// is the current status, whose `to` is the command's declared target, and whose time is the
    /// command's time verbatim — time PASSES THROUGH, it is never invented by the core.
    #[test]
    fn decide_event_is_consistent_when_legal(status in any_status(), cmd in any_command()) {
        let state = OrderState { status, ..OrderState::genesis() };
        if let Ok(events) = decide(&state, cmd) {
            prop_assert_eq!(events.len(), 1);
            // `decide` emits ONLY `StatusChanged` (the money/binding facts are 0b-3-reachable, not here).
            let Event::StatusChanged { from, to, at } = events[0] else {
                return Err(TestCaseError::fail("decide must emit a StatusChanged"));
            };
            prop_assert_eq!(from, status);
            prop_assert_eq!(to, cmd.target());
            prop_assert_eq!(at, cmd.at());
        }
    }

    /// Codec closure: any log `decide`/`fold` produces survives a canonical-bytes round-trip
    /// unchanged, and encoding is deterministic — the Immutable Log is faithfully persistable and a
    /// replayed decode reconstructs the exact same state (the property PQC signing + mesh replication
    /// both stand on).
    #[test]
    fn log_survives_canonical_bytes_round_trip(cmds in prop::collection::vec(any_command(), 0..40)) {
        let g = OrderState::genesis();
        let (final_state, log) = run(g, &cmds);
        let bytes = encode_log(&log).expect("encode");
        prop_assert_eq!(encode_log(&log).expect("encode2"), bytes.clone()); // deterministic
        let decoded = decode_log(&bytes).expect("decode");
        prop_assert_eq!(&decoded, &log);
        prop_assert_eq!(replay(g, &decoded), final_state); // state reconstructs from decoded log
    }

    // ─────────────────────────── Layer 3 — Corridor: terminal absorption ───────────────────────────

    /// No command escapes a terminal order. For every terminal status and every command, `decide`
    /// returns `Err` — the physical-law guarantee that a delivered/cancelled/rejected/picked-up
    /// order can never be moved again.
    #[test]
    fn terminal_states_absorb_all_commands(cmd in any_command()) {
        for &status in ALL_STATUSES.iter() {
            if is_terminal(status) {
                let state = OrderState { status, ..OrderState::genesis() };
                prop_assert!(
                    decide(&state, cmd).is_err(),
                    "terminal {status:?} must absorb {cmd:?}"
                );
            }
        }
    }

    // ─────────────────── 0b-2 — the grown alphabet: fold totality + canonical bytes ───────────────────

    /// The whole 0b-2 alphabet (not just the `decide`-reachable `StatusChanged`) folds TOTALLY: an
    /// arbitrary log of any events never panics, and replaying it is deterministic. This exercises the
    /// money/binding fold arms directly (they are not yet `decide`-reachable — that is 0b-3).
    #[test]
    fn fold_over_any_event_log_is_total_and_deterministic(events in prop::collection::vec(any_event(), 0..40)) {
        let g = OrderState::genesis();
        prop_assert_eq!(replay(g, &events), replay(g, &events));
    }

    /// Every event in the grown alphabet is faithfully persistable: an arbitrary log survives a
    /// canonical-bytes round-trip unchanged, encoding is deterministic, and the state reconstructs
    /// from the decoded log. Extends the pre-0b-2 codec-closure property to the money/binding facts.
    #[test]
    fn any_event_log_survives_canonical_bytes_round_trip(events in prop::collection::vec(any_event(), 0..40)) {
        let g = OrderState::genesis();
        let final_state = replay(g, &events);
        let bytes = encode_log(&events).expect("encode");
        prop_assert_eq!(encode_log(&events).expect("encode2"), bytes.clone()); // deterministic
        let decoded = decode_log(&bytes).expect("decode");
        prop_assert_eq!(&decoded, &events);
        prop_assert_eq!(replay(g, &decoded), final_state);
    }

    /// An ENVELOPE log — the persisted row form carrying `seq`/`at`/`cause` — survives a
    /// canonical-bytes round-trip unchanged (the `CommandHash` cause included), and replaying the
    /// envelopes reconstructs exactly what folding the bare events does. This is the property mesh
    /// replication + PQC signing stand on: every node derives identical bytes for the same log.
    #[test]
    fn envelope_log_round_trips_and_replays_to_the_same_state(events in prop::collection::vec(any_event(), 0..40)) {
        let g = OrderState::genesis();
        let envelopes: Vec<Envelope> = events
            .iter()
            .enumerate()
            .map(|(i, &event)| Envelope {
                seq: i as u64,
                at: Ts(i as i64),
                cause: CommandHash(format!("cause-{i}")),
                event,
            })
            .collect();
        let bytes = canonical_bytes(&envelopes).expect("encode envelopes");
        prop_assert_eq!(canonical_bytes(&envelopes).expect("encode2"), bytes.clone()); // deterministic
        let decoded: Vec<Envelope> = from_bytes(&bytes).expect("decode envelopes");
        prop_assert_eq!(&decoded, &envelopes);
        prop_assert_eq!(replay_envelopes(g, &decoded), replay(g, &events));
    }
}

// A compile-time anchor for the 0b-2 DoD: `OrderTotals` is the money-snapshot shape the `Priced`
// fact records, four integer `Lek` fields. Referencing it here keeps the import honest and documents
// the exact shape a reader should expect from `state.totals`.
#[test]
fn order_totals_is_four_integer_lek_fields() {
    let t = OrderTotals {
        subtotal: Lek::new(1_000).unwrap(),
        delivery_fee: Lek::new(200).unwrap(),
        tax_total: Lek::new(120).unwrap(),
        total: Lek::new(1_320).unwrap(),
    };
    // total == subtotal + delivery_fee + tax_total for this constructed snapshot (a reader's sanity
    // check on the field meaning — the fold stores whatever the `Priced` fact carried, verbatim).
    assert_eq!(
        t.total,
        t.subtotal
            .checked_add(t.delivery_fee)
            .unwrap()
            .checked_add(t.tax_total)
            .unwrap()
    );
}
