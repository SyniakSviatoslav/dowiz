//! Hard Truth suite — kernel layer (Phase-Zero Step 2, extended through 0b-3's composed `decide`).
//!
//! Where `hard_truth.rs` proves the UNBOUNDED money algebra, this proves the KERNEL: that driving an
//! order with an arbitrary stream of commands is deterministic, that the state is always exactly the
//! fold of its event log (replay), that terminal states absorb everything, and — as of 0b-3 — that
//! the machine + actor-gate + CC-1 strand guard + pricing/LC1 corridor compose behind the ONE
//! `decide` door in the live-handler order. These are the Manifesto's core promises ("same intents
//! in → same final state; the UI is a mirror of the fold") stated as falsifiable properties.
//!
//! Layer map (PHASE-ZERO.md §4):
//!   - Layer 1 Determinism — `run(cmds) == run(cmds)`; the event log is byte-identical run to run.
//!   - Layer 2 Totality/Replay — `decide`/`fold` never panic; `state == replay(genesis, log)` at
//!     every prefix.
//!   - Layer 3 Corridor — terminal absorption + the full `states × command-kinds × actor`
//!     enumeration (every pair Ok-with-StatusChanged-first or a typed Err, zero panics), the
//!     actor-gate SYSTEM-only-edge refusal (the 0b-3 RED-proof anchor), and the pricing/LC1
//!     conservation invariants over the REAL `PlaceOrder` composition.

use domain::{
    ALL_STATUSES, Actor, BindingState, Command, CommandHash, Context, DomainError, Envelope,
    ErrorCode, Event, FeeLocation, Lek, OrderState, OrderStatus, OrderTotals, PriceInputs,
    PricingItem, PricingSnapshot, ProductInfo, Ts, canonical_bytes, decide, decode_log, encode_log,
    fold, from_bytes, is_terminal, replay, replay_envelopes,
};
use proptest::prelude::*;

/// The rich observed context the command-stream properties drive against: a `delivered` courier
/// binding (so →DELIVERED/→PICKED_UP is cc1-legal, the deliver-flow completion path) and a paid
/// amount (so a terminal-cancel emits a `RefundObligated`). Fixed ⇒ `run` stays deterministic.
fn rich_ctx() -> Context<'static> {
    Context {
        binding: BindingState {
            has_active_binding: false,
            has_delivered_binding: true,
        },
        refundable_paid: Lek::new(1_000).expect("non-negative"),
        pricing: None,
    }
}

/// Drive a state through a command stream against [`rich_ctx`]. Illegal/refused commands (the
/// machine, the actor-gate, or the cc1 guard) are left as no-ops (the order stays put) — exactly
/// "the kernel refuses, it does not corrupt". Returns the final state and the accepted event log
/// (which as of 0b-3 may carry `BindingTerminalized`/`RefundObligated` alongside `StatusChanged`).
fn run(genesis: OrderState, commands: &[Command]) -> (OrderState, Vec<Event>) {
    let ctx = rich_ctx();
    let mut state = genesis;
    let mut log = Vec::new();
    for cmd in commands {
        if let Ok(events) = decide(&state, cmd.clone(), &ctx) {
            for e in &events {
                state = fold(&state, e);
            }
            log.extend(events);
        }
    }
    (state, log)
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
        (any_lek(), any_lek(), any_lek(), any_lek()).prop_map(
            |(subtotal, delivery_fee, tax_total, total)| {
                Event::Priced { subtotal, delivery_fee, tax_total, total }
            }
        ),
        any_lek().prop_map(|amount| Event::RefundObligated { amount }),
        Just(Event::BindingTerminalized),
    ]
}

/// Any TRANSITION command, with an arbitrary caller-supplied timestamp and actor. (`PlaceOrder` is a
/// create/price command — it carries a cart + needs a price authority, so it is exercised by the
/// dedicated pricing/conservation properties below, not the generic transition streams.)
fn any_command() -> impl Strategy<Value = Command> {
    (0u8..9, any::<i64>(), any::<bool>()).prop_map(|(kind, t, is_owner)| {
        let at = Ts(t);
        let actor = if is_owner { Actor::Owner } else { Actor::System };
        match kind {
            0 => Command::Confirm { at, actor },
            1 => Command::Reject { at, actor },
            2 => Command::StartPreparing { at, actor },
            3 => Command::MarkReady { at, actor },
            4 => Command::Dispatch { at, actor },
            5 => Command::MarkDelivered { at, actor },
            6 => Command::MarkPickedUp { at, actor },
            7 => Command::RevertToReady { at, actor },
            _ => Command::Cancel { at, actor },
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
    ///
    /// 0b-4 DoD — determinism is proven by CANONICAL BYTES, "not `Eq` only": the byte layer is the
    /// one the downstream promises actually stand on. A content hash / PQC signature over the log
    /// (Phase 3) and cross-node mesh replication (every node must derive identical bytes for the same
    /// log — codec.rs module doc) compare BYTES, never Rust `PartialEq`; and a future encoder swap
    /// behind the `codec` seam (serde_json → rkyv/protobuf) could break byte-determinism while `Eq`
    /// stayed green. So we pin BOTH: the value-level `Eq` (cheap, states the intent) AND the encoded
    /// bytes of the event log and the final aggregate.
    #[test]
    fn kernel_run_is_deterministic(cmds in prop::collection::vec(any_command(), 0..40)) {
        let g = OrderState::genesis();
        let (state1, log1) = run(g, &cmds);
        let (state2, log2) = run(g, &cmds);
        // Value-level determinism (the intent, cheaply).
        prop_assert_eq!(state1, state2); // OrderState: Copy
        prop_assert_eq!(&log1, &log2);
        // Byte-level determinism (the load-bearing form — 0b-4 DoD): the event log and the final
        // aggregate each encode to IDENTICAL canonical bytes, run to run.
        prop_assert_eq!(encode_log(&log1).expect("encode log1"), encode_log(&log2).expect("encode log2"));
        prop_assert_eq!(
            canonical_bytes(&state1).expect("encode state1"),
            canonical_bytes(&state2).expect("encode state2")
        );
    }

    // ─────────────────────────── Layer 2 — Replay / totality ───────────────────────────

    /// "The state is only ever the fold of its log." Replaying the accepted log from genesis
    /// reproduces the final WHOLE aggregate (status + money + binding), and the status column
    /// reconstructs correctly at EVERY prefix — the money/binding facts (0b-3-reachable now) do not
    /// move the machine, so a prefix's status is the `to` of its last `StatusChanged`.
    #[test]
    fn state_is_the_fold_of_its_log_at_every_prefix(cmds in prop::collection::vec(any_command(), 0..40)) {
        let g = OrderState::genesis();
        let (final_state, log) = run(g, &cmds);
        prop_assert_eq!(replay(g, &log), final_state);
        let mut expected = OrderStatus::Pending;
        for k in 0..log.len() {
            prop_assert_eq!(replay(g, &log[..k]).status, expected);
            if let Event::StatusChanged { to, .. } = log[k] {
                expected = to;
            }
        }
        prop_assert_eq!(replay(g, &log).status, expected);
    }

    /// Structural law of the composed `decide`: a legal command's FIRST event is always the
    /// `StatusChanged` carrying `from` = current status, `to` = the command's declared target, and
    /// the command's time VERBATIM (time passes through, never invented). Any further events are ONLY
    /// the composed money/binding facts — never a foreign fact. (The exact effect wiring — which
    /// edges terminalize / obligate a refund — is pinned by the concrete unit tests in `kernel.rs`.)
    #[test]
    fn decide_first_event_is_the_timed_status_fact_when_legal(status in any_status(), cmd in any_command()) {
        let state = OrderState { status, ..OrderState::genesis() };
        if let Ok(events) = decide(&state, cmd.clone(), &rich_ctx()) {
            prop_assert!(!events.is_empty());
            let Event::StatusChanged { from, to, at } = events[0] else {
                return Err(TestCaseError::fail("first event must be StatusChanged"));
            };
            prop_assert_eq!(from, status);
            prop_assert_eq!(to, cmd.target());
            prop_assert_eq!(at, cmd.at());
            for e in &events[1..] {
                prop_assert!(
                    matches!(e, Event::BindingTerminalized | Event::RefundObligated { .. }),
                    "unexpected composed event {:?}", e
                );
            }
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
                    decide(&state, cmd.clone(), &rich_ctx()).is_err(),
                    "terminal {:?} must absorb {:?}", status, cmd
                );
            }
        }
    }

    // ─────────────── Layer 3 — Corridor: pricing/LC1 conservation over the REAL composition ───────────────

    /// The conservation invariant over an ARBITRARY cart priced through the composed `decide`
    /// (`Command::PlaceOrder`): `total = subtotal + charged_tax + delivery_fee − discount(0)`, every
    /// component ≥ 0, AND LC1 no-double-tax — on an inclusive venue `charged_tax = 0`, so `total`
    /// EXCLUDES the (still-computed, informational) `tax_total`; on an exclusive venue `total`
    /// includes it. The check is INDEPENDENT of how the numbers were computed (it re-adds the
    /// components in i64, never re-calling `compose_total`), so it falsifies a broken composition.
    #[test]
    fn place_order_priced_fact_satisfies_conservation_and_lc1(
        prices in prop::collection::vec(0i64..100_000, 1..=5),
        lines in prop::collection::vec((0usize..5, 1i64..=10), 1..=8),
        is_pickup in any::<bool>(),
        delivery_flat in 0i64..5_000,
        rate_micro in 0i64..500_000,
        price_includes_tax in any::<bool>(),
    ) {
        use std::collections::HashMap;
        // Products p0..pN keyed by index; cart lines reference them modulo the product count.
        let mut product_map: HashMap<String, ProductInfo> = HashMap::new();
        for (i, &price) in prices.iter().enumerate() {
            product_map.insert(
                format!("p{i}"),
                ProductInfo { name: format!("prod{i}"), price: Lek::new(price).unwrap() },
            );
        }
        let mod_map = HashMap::new();
        let groups_by_product = HashMap::new();
        let cart: Vec<PricingItem> = lines
            .iter()
            .map(|&(idx, qty)| PricingItem {
                product_id: format!("p{}", idx % prices.len()),
                quantity: qty,
                modifier_ids: vec![],
            })
            .collect();
        let snapshot = PricingSnapshot {
            product_map: &product_map,
            mod_map: &mod_map,
            groups_by_product: &groups_by_product,
        };
        let inputs = PriceInputs {
            snapshot,
            is_pickup,
            // A flat delivery fee so a non-pickup order always resolves (no NOT_DELIVERABLE), and no
            // MIN_ORDER / free-threshold gate — the property is about the composition arithmetic.
            location: FeeLocation {
                delivery_fee_flat: Some(delivery_flat),
                free_delivery_threshold: None,
                min_order_value: None,
            },
            distance_m: None,
            tiers: &[],
            rate_micro,
            price_includes_tax,
        };
        let ctx = Context { binding: rich_ctx().binding, refundable_paid: Lek::ZERO, pricing: Some(inputs) };
        let events = decide(
            &OrderState::genesis(),
            Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart },
            &ctx,
        )
        .expect("a well-formed cart with a flat fee always prices");
        prop_assert_eq!(events.len(), 1);
        let Event::Priced { subtotal, delivery_fee, tax_total, total } = events[0] else {
            return Err(TestCaseError::fail("PlaceOrder must emit exactly a Priced fact"));
        };
        // LC1: the tax actually ADDED to `total` is 0 on an inclusive venue, else the whole tax_total.
        let charged_tax = if price_includes_tax { 0 } else { tax_total.minor_units() };
        // Conservation, re-derived independently in i64 (never via compose_total):
        prop_assert_eq!(
            total.minor_units(),
            subtotal.minor_units() + delivery_fee.minor_units() + charged_tax,
            "conservation broke: sub={} del={} chargedTax={} total={}",
            subtotal.minor_units(), delivery_fee.minor_units(), charged_tax, total.minor_units()
        );
        // Non-negativity (Lek guarantees ≥0, restated as the invariant's own clause).
        prop_assert!(subtotal.minor_units() >= 0 && delivery_fee.minor_units() >= 0 && total.minor_units() >= 0);
        // LC1 no-double-tax, stated directly: inclusive ⇒ total carries no tax component.
        if price_includes_tax {
            prop_assert_eq!(total.minor_units(), subtotal.minor_units() + delivery_fee.minor_units());
        }
    }

    // ─────────────────── 0b-2 — the grown alphabet: fold totality + canonical bytes ───────────────────

    /// The whole 0b-2 alphabet folds TOTALLY: an arbitrary log of any events never panics, and
    /// replaying it is deterministic. This exercises the money/binding fold arms directly.
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

// ─────────────── Layer 3 — Corridor: full enumeration + the actor-gate RED-proof anchor ───────────────

/// The full finite `states × transition-command-kinds × actor` enumeration (10 × 9 × 2 = 180 pairs):
/// `decide` NEVER panics (the test completing IS the totality witness), an `Ok` result always leads
/// with a `StatusChanged`, and an `Ok` is possible only out of a NON-terminal state. This is the
/// Layer-3 corridor property stated exhaustively rather than sampled.
#[test]
fn every_state_x_command_kind_x_actor_is_total_and_well_formed() {
    let kinds: [fn(Ts, Actor) -> Command; 9] = [
        |at, actor| Command::Confirm { at, actor },
        |at, actor| Command::Reject { at, actor },
        |at, actor| Command::StartPreparing { at, actor },
        |at, actor| Command::MarkReady { at, actor },
        |at, actor| Command::Dispatch { at, actor },
        |at, actor| Command::MarkDelivered { at, actor },
        |at, actor| Command::MarkPickedUp { at, actor },
        |at, actor| Command::RevertToReady { at, actor },
        |at, actor| Command::Cancel { at, actor },
    ];
    let ctx = rich_ctx();
    for &status in ALL_STATUSES.iter() {
        let state = OrderState { status, ..OrderState::genesis() };
        for make in kinds.iter() {
            for actor in [Actor::Owner, Actor::System] {
                match decide(&state, make(Ts(1), actor), &ctx) {
                    Ok(events) => {
                        assert!(
                            matches!(events[0], Event::StatusChanged { .. }),
                            "{status:?}: an Ok result must lead with a StatusChanged"
                        );
                        assert!(
                            !is_terminal(status),
                            "terminal {status:?} must never yield Ok"
                        );
                    }
                    Err(_) => {} // a typed refusal is fine; the point is zero panics.
                }
            }
        }
    }
}

/// The actor-gate composes behind `decide` — the 0b-3 RED-proof anchor. The deliver-v2 sweep widened
/// the MACHINE to permit CONFIRMED/PREPARING/READY→CANCELLED, but those are SYSTEM-only edges: an
/// OWNER driving one is refused `CANCEL_NOT_PERMITTED` (a `CorridorBreach` carrying the exact wire
/// code), while `System` keeps them. RED proof: commenting out the `assert_owner_target_allowed`
/// call in `decide` flips every owner case here from `Err`→`Ok` and reds this test.
#[test]
fn actor_gate_refuses_owner_on_the_widened_system_only_cancel_edges() {
    let ctx = Context::for_transition(
        BindingState { has_active_binding: false, has_delivered_binding: false },
        Lek::ZERO,
    );
    for from in [OrderStatus::Confirmed, OrderStatus::Preparing, OrderStatus::Ready] {
        let state = OrderState { status: from, ..OrderState::genesis() };
        assert_eq!(
            decide(&state, Command::Cancel { at: Ts(1), actor: Actor::Owner }, &ctx),
            Err(DomainError::CorridorBreach {
                corridor: "actor_gate",
                code: ErrorCode::CancelNotPermitted,
            }),
            "owner must be gated off {from:?}→CANCELLED (SYSTEM-only edge)"
        );
        assert!(
            decide(&state, Command::Cancel { at: Ts(1), actor: Actor::System }, &ctx).is_ok(),
            "system (dispatch-grace) keeps {from:?}→CANCELLED"
        );
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
