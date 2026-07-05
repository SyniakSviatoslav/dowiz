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
    ALL_STATUSES, Command, Event, OrderState, OrderStatus, Ts, decide, fold, is_terminal, replay,
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

/// The `to` a `StatusChanged` event carries (the only event variant today).
fn event_to(e: &Event) -> OrderStatus {
    let Event::StatusChanged { to, .. } = e;
    *to
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
                // each prefix ends on exactly the status the k-th accepted event moved to
                prop_assert_eq!(partial.status, event_to(&log[k - 1]));
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
        let state = OrderState { status };
        if let Ok(events) = decide(&state, cmd) {
            prop_assert_eq!(events.len(), 1);
            let Event::StatusChanged { from, to, at } = events[0];
            prop_assert_eq!(from, status);
            prop_assert_eq!(to, cmd.target());
            prop_assert_eq!(at, cmd.at());
        }
    }

    // ─────────────────────────── Layer 3 — Corridor: terminal absorption ───────────────────────────

    /// No command escapes a terminal order. For every terminal status and every command, `decide`
    /// returns `Err` — the physical-law guarantee that a delivered/cancelled/rejected/picked-up
    /// order can never be moved again.
    #[test]
    fn terminal_states_absorb_all_commands(cmd in any_command()) {
        for &status in ALL_STATUSES.iter() {
            if is_terminal(status) {
                let state = OrderState { status };
                prop_assert!(
                    decide(&state, cmd).is_err(),
                    "terminal {status:?} must absorb {cmd:?}"
                );
            }
        }
    }
}
