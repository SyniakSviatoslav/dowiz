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
// The Validation Layer (VALIDATION-LAYER-SPEC) — the invariant gate the orchestrator/shell runs
// BEFORE `decide`. Pure/total/IO-free like the rest of the kernel (the wasm sovereignty gate
// proves it); it lifts `decide`'s preconditions to the seam and returns every violation as data.
// It does NOT modify `decide` (the gate sits AROUND it) — wiring/cutover (0b-5) stays human-gated.
pub mod validate;
// Sovereign-core money composition (GRAND-PLAN 0b-1) — the pure order-total arithmetic relocated
// from the `api` shell (`routes/orders/pricing.rs`). Integer-only (no f64 — the disallowed-types
// gate proves it); the shell keeps a thin f64 adapter over this module.
pub mod pricing;

use crate::{DomainError, ErrorCode, Lek, OrderStatus, order_status::assert_transition};
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

/// WHO is attempting the command — the D2 identity seam carried on every [`Command`]. The MACHINE
/// ([`assert_transition`]) says an edge is *possible*; the ACTOR-GATE
/// ([`policy::assert_owner_target_allowed`]) says who is *allowed* to drive it (REV-S5-9 Q2). The
/// deliver-v2 offer-sweep widened the machine to permit CONFIRMED/PREPARING/READY→CANCELLED, but
/// those are SYSTEM-only edges (the dispatch-grace path): an [`Owner`](Actor::Owner) driving one is
/// refused, while [`System`](Actor::System) (courier sweeps, schedulers, automation) keeps them.
/// Humans and agents are indistinguishable to the kernel — both are just command sources with an
/// `Actor` (Manifesto Phase Three, by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Actor {
    /// The venue owner/staff acting through the dashboard — actor-gated on the widened cancel edges.
    Owner,
    /// Platform automation (dispatch grace, courier sweeps, schedulers) — keeps the SYSTEM-only edges.
    System,
}

/// The intent alphabet — the actions an actor can *attempt* against an order. Every command carries
/// its [`Actor`] (WHO attempts) and its [`Ts`] (WHEN, caller-supplied). The status commands map to
/// exactly one target [`OrderStatus`]; [`PlaceOrder`](Command::PlaceOrder) is the CREATE+PRICE door
/// (it does not transition an existing edge — the order is born PENDING — it prices a cart). The
/// machine and the composed corridors — not this alphabet — decide whether an attempt is *legal*.
///
/// NOT `Copy`: `PlaceOrder` carries a `Vec<PricingItem>` cart. Clone where a value is needed twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Command {
    Confirm { at: Ts, actor: Actor },
    Reject { at: Ts, actor: Actor },
    StartPreparing { at: Ts, actor: Actor },
    MarkReady { at: Ts, actor: Actor },
    /// Drive toward IN_DELIVERY (legal from CONFIRMED or READY).
    Dispatch { at: Ts, actor: Actor },
    MarkDelivered { at: Ts, actor: Actor },
    MarkPickedUp { at: Ts, actor: Actor },
    /// The IN_DELIVERY → READY revert (courier cancel/abort/owner-reassign never strands an order).
    RevertToReady { at: Ts, actor: Actor },
    Cancel { at: Ts, actor: Actor },
    /// CREATE+PRICE — the customer/owner places an order. Carries the cart (pure INTENT); the price
    /// authority (product/modifier snapshot, delivery/tax inputs) is OBSERVED context supplied on
    /// [`Context`], not intent. Produces an [`Event::Priced`] money snapshot; the order is born
    /// PENDING (genesis), so it drives no `StatusChanged` and is NOT machine/actor/cc1-gated.
    PlaceOrder {
        at: Ts,
        actor: Actor,
        cart: Vec<crate::kernel::pricing::PricingItem>,
    },
}

impl Command {
    /// The status a *transition* command drives the order toward. A pure total mapping — legality is
    /// decided by [`decide`] via the machine, never here. [`PlaceOrder`](Command::PlaceOrder) creates
    /// at genesis (PENDING) rather than transitioning, so its "target" is `Pending` (never used by
    /// `decide`, which routes `PlaceOrder` around the machine entirely).
    pub fn target(&self) -> OrderStatus {
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
            Command::PlaceOrder { .. } => OrderStatus::Pending,
        }
    }

    /// The caller-supplied event time carried by this command.
    pub fn at(&self) -> Ts {
        match self {
            Command::Confirm { at, .. }
            | Command::Reject { at, .. }
            | Command::StartPreparing { at, .. }
            | Command::MarkReady { at, .. }
            | Command::Dispatch { at, .. }
            | Command::MarkDelivered { at, .. }
            | Command::MarkPickedUp { at, .. }
            | Command::RevertToReady { at, .. }
            | Command::Cancel { at, .. }
            | Command::PlaceOrder { at, .. } => *at,
        }
    }

    /// WHO is attempting the command — the input to the actor-gate corridor.
    pub fn actor(&self) -> Actor {
        match self {
            Command::Confirm { actor, .. }
            | Command::Reject { actor, .. }
            | Command::StartPreparing { actor, .. }
            | Command::MarkReady { actor, .. }
            | Command::Dispatch { actor, .. }
            | Command::MarkDelivered { actor, .. }
            | Command::MarkPickedUp { actor, .. }
            | Command::RevertToReady { actor, .. }
            | Command::Cancel { actor, .. }
            | Command::PlaceOrder { actor, .. } => *actor,
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

/// Observed CONTEXT the corridors read — the facts the shell OBSERVES about OTHER aggregates and the
/// price authority, as opposed to the actor's INTENT (which rides on the [`Command`]). Supplied to
/// [`decide`] like [`Ts`]/[`CommandHash`]: the core carries and composes it, never reading a
/// clock/DB/RNG to obtain it (Laws 1–3). The 0b-3 split (operator decision): binding facts, the paid
/// amount, and the price snapshot are OBSERVED ⇒ `Context`; the actor is intent-adjacent ⇒ `Command`.
pub struct Context<'a> {
    /// CC-1 strand-guard input — the order's courier-binding facts (read from `courier_assignments`
    /// inside the tx). Default `{ false, false }` for a never-dispatched order.
    pub binding: policy::BindingState,
    /// The sum of PAID payments to refund on a terminal-cancel (policy L-A `refund_due`). The SHELL
    /// computes this money aggregate over `payments` — exactly as it owns the Haversine sum over
    /// coordinates (the 0b-1 f64/aggregate boundary) — and the core carries it. `ZERO` today (zero
    /// `paid` rows) ⇒ no [`Event::RefundObligated`] fires; the obligation stays correct the moment
    /// crypto flips and real paid rows appear.
    pub refundable_paid: Lek,
    /// The price authority for a [`Command::PlaceOrder`]: the in-tx product/modifier/group snapshot
    /// plus the already-integerized delivery/tax inputs (the f64→i64 marshalling stays in the shell,
    /// 0b-1). `None` for transition commands (they do not price); its absence on a `PlaceOrder` is a
    /// caller-contract [`DomainError::CorridorBreach`].
    pub pricing: Option<PriceInputs<'a>>,
}

/// The already-integerized price inputs a [`Command::PlaceOrder`] prices against — the OBSERVED half
/// of pricing (the cart is the intent half, on the command). Mirrors the shell create-handler's in-tx
/// reads (`pg.rs:282-337`): the product/modifier/group snapshot, whether the order is pickup, the
/// location fee config, the whole-meter delivery distance, the delivery tiers, and the micro-scaled
/// tax rate + inclusive flag. Every float is already an integer here (the 0b-1 boundary).
pub struct PriceInputs<'a> {
    pub snapshot: pricing::PricingSnapshot<'a>,
    pub is_pickup: bool,
    pub location: pricing::FeeLocation,
    pub distance_m: Option<i64>,
    pub tiers: &'a [pricing::DeliveryTier],
    pub rate_micro: i64,
    pub price_includes_tax: bool,
}

impl Context<'_> {
    /// The context for a status-transition command: the courier-binding facts + the refundable paid
    /// sum, with no pricing authority (transitions do not price).
    pub fn for_transition(binding: policy::BindingState, refundable_paid: Lek) -> Self {
        Context {
            binding,
            refundable_paid,
            pricing: None,
        }
    }
}

/// THE LAW. Given the current state, an attempted command, and the observed [`Context`], return the
/// events it produces — or the reason it is refused. Pure, total, side-effect-free: no clock, no
/// entropy, no IO. This is the ONE door — the machine, the actor-gate, the CC-1 strand guard, and
/// the pricing/LC1 conservation corridor all compose HERE, in the LIVE-HANDLER ORDER (GRAND-PLAN
/// 0b-3), so one command yields the full event set under one [`Envelope`]. The machine
/// (`assert_transition`) is the sole legality authority, so a terminal order absorbs every command
/// (its transition table is empty → `Err`), and no event can ever escape a terminal state. The
/// scattered `policy`/`pricing` fns are no longer a public mutation API — they are this door's
/// internals ([`policy::transition_effects`] became an internal emission detail).
pub fn decide(
    state: &OrderState,
    command: Command,
    ctx: &Context,
) -> Result<Vec<Event>, DomainError> {
    // PlaceOrder is the CREATE+PRICE door: a placed order is born PENDING (= genesis), so it does
    // not transition an existing edge and never touches the machine/actor/cc1 corridors (which gate
    // TRANSITIONS). It prices the cart against the observed authority and records the money snapshot.
    if let Command::PlaceOrder { cart, .. } = &command {
        let inputs = ctx.pricing.as_ref().ok_or(DomainError::CorridorBreach {
            corridor: "pricing",
            // A PlaceOrder with no price authority is a caller-contract violation (the shell must
            // read the snapshot before calling decide, pg.rs order), not a business outcome.
            code: ErrorCode::Internal,
        })?;
        return Ok(vec![price_cart(cart, inputs)?]);
    }

    let from = state.status;
    let to = command.target();

    // 1. The MACHINE — the byte-frozen 10-state relation is the SOLE legality authority.
    assert_transition(from, to)?;

    // 2. The ACTOR-GATE — an OWNER may not drive the SYSTEM-only widened cancel edges
    //    (CONFIRMED/PREPARING/READY→CANCELLED, orderAuthz.ts). System keeps them, so the gate applies
    //    only to an owner-driven command. Runs AFTER the machine vetted the edge, BEFORE the effects.
    if command.actor() == Actor::Owner {
        policy::assert_owner_target_allowed(from, to)
            .map_err(|code| DomainError::CorridorBreach {
                corridor: "actor_gate",
                code,
            })?;
    }

    // 3. The CC-1 STRAND GUARD — a →DELIVERED/PICKED_UP with a live (or IN_DELIVERY-but-undelivered)
    //    courier binding is refused, so no order is marked done while a courier is still strand-bound
    //    (orders.ts:929-955, money-audit H1). A no-op for every other target.
    policy::cc1_strand_guard(to, from, ctx.binding).map_err(|code| DomainError::CorridorBreach {
        corridor: "cc1_strand",
        code,
    })?;

    // 4. Emit the status fact + the fold effects the transition implies. `transition_effects` is now
    //    an INTERNAL emission detail (0b-3 DoD) — the shell no longer reads it directly.
    let effects = policy::transition_effects(from, to);
    let mut events = Vec::with_capacity(3);
    events.push(Event::StatusChanged {
        from,
        to,
        at: command.at(),
    });
    // R2-3: terminalize the active courier binding on any →CANCELLED/→REJECTED and the
    // IN_DELIVERY→READY revert (orderStatusService.ts:139) — no order leaves with a live strand.
    if effects.terminalize_assignment {
        events.push(Event::BindingTerminalized);
    }
    // L-A: record a refund obligation on a terminal-cancel — but only when there IS a paid amount to
    // refund (orderStatusService.ts:165, per-paid-payment). Zero paid rows today ⇒ inert. The amount
    // is the SHELL-observed paid sum, never core-derived (no money number is invented in the core).
    if effects.record_refund_due && ctx.refundable_paid > Lek::ZERO {
        events.push(Event::RefundObligated {
            amount: ctx.refundable_paid,
        });
    }
    Ok(events)
}

/// The pricing/LC1 conservation corridor — ports the shell create-handler's section-6→9 assembly
/// VERBATIM (`api::routes::orders::pg.rs:287-343`), the ONE order that must never diverge (a
/// divergence here IS the mirror-oracle failure mode): `compute_order_pricing` →
/// `delivery_fee_for_order` → `apply_tax` → `charged_tax` (LC1) → `compose_total`. Every pricing
/// refusal maps to a [`DomainError::CorridorBreach`] carrying the exact pricing [`ErrorCode`]; a
/// money-math error (unreachable for real inputs — see [`pricing`]) surfaces as an `Internal` breach,
/// never a silent wrong charge and never a panic (this stays TOTAL).
fn price_cart(cart: &[pricing::PricingItem], p: &PriceInputs) -> Result<Event, DomainError> {
    let breach = |code| DomainError::CorridorBreach {
        corridor: "pricing",
        code,
    };
    let (subtotal, _rows) =
        pricing::compute_order_pricing(cart, &p.snapshot).map_err(|e| breach(e.code))?;
    let delivery_fee =
        pricing::delivery_fee_for_order(subtotal, p.is_pickup, p.location, p.distance_m, p.tiers)
            .map_err(|e| breach(e.code))?;
    let tax_i64 = pricing::apply_tax(subtotal.minor_units(), p.rate_micro, p.price_includes_tax)
        .map_err(money_math_breach)?;
    let tax_total = Lek::new(tax_i64).map_err(money_math_breach)?;
    let charged = pricing::charged_tax(tax_total, p.price_includes_tax);
    let total = pricing::compose_total(subtotal, delivery_fee, charged, Lek::ZERO)
        .map_err(money_math_breach)?;
    Ok(Event::Priced {
        subtotal,
        delivery_fee,
        tax_total,
        total,
    })
}

/// A money-math failure (`apply_tax`/`Lek::new`/`compose_total` overflow or a negative composition)
/// is UNREACHABLE for real inputs (see [`pricing`]'s REV-S5-4 headroom analysis); it surfaces as an
/// `Internal` corridor breach rather than a silent wrong charge or a panic. A named fn (not a
/// discarding `|_|` closure) so the core lib stays clean under `-D warnings` (`clippy::map_err_ignore`).
fn money_math_breach(_e: crate::MoneyError) -> DomainError {
    DomainError::CorridorBreach {
        corridor: "pricing",
        code: ErrorCode::Internal,
    }
}

// ─────────────────────── ReAct agentic loop (Reason→Act→Observe→Reflect) ───────────────────────
//
// The promo-demo failure mode: a HIDDEN retry loop shows the user a single "perfect" iteration while
// the real system silently retried/rewrote behind the scenes. `react_decide` makes that loop VISIBLE
// and AUDITABLE: every attempt (draft → collision → rewrite) is recorded in a [`ReactTrace`] the shell
// can persist as audit metadata. It composes AROUND `decide` (the red-line door) — `decide` itself is
// NEVER modified. On a collision/denial it runs `react_reflect`, which may rewrite the command into an
// equivalent LEGAL one (the only honest rewrite: owner→system on a SYSTEM-only edge) and retry up to
// `iterations` (default 3). A genuinely illegal command has NO valid rewrite → the loop STOPS and
// returns the original error (never loops forever, never invents a state).

/// One VISIBLE ReAct step. Serializable so the whole trace can be persisted as audit metadata and
/// replayed later — nothing here is hidden from the operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactStep {
    pub iter: u32,
    /// The combined Reason→Act→Observe→Reflect phase label for this visible iteration.
    pub phase: String,
    /// WHY this command / what the rewrite was (the visible self-correction note).
    pub thought: String,
    /// Ok(N events) or Err(code) — the environment's response this iteration.
    pub observation: String,
    /// 0..100 real-time quality score for THIS iteration (the eval gate). 0 = denied, 100 = clean.
    pub eval_score: u8,
    /// Did this iteration make progress (not denied)? The thing promo demos hide shows up here.
    pub ok: bool,
    /// Did a rewrite get GENERATED from this denial (an honest self-correction)? Distinct from `ok`:
    /// a denial WITHOUT a rewrite is an honest "no valid action" (had_rewrite stays false); a denial
    /// WITH a rewrite is the visible retry promo demos used to hide (had_rewrite becomes true).
    pub rewrote: bool,
}

/// The full, visible Reason→Act→Observe→Reflect trace for one `react_decide` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReactTrace {
    pub steps: Vec<ReactStep>,
}

impl ReactTrace {
    fn push(&mut self, step: ReactStep) {
        self.steps.push(step);
    }
    /// Was at least one iteration a DENIAL-WITH-REWRITE (the visible self-correction promo demos hide)?
    /// Distinct from a bare denial: an honest "no valid action" denial has `rewrote:false` and does NOT
    /// make `had_rewrite` true.
    pub fn had_rewrite(&self) -> bool {
        self.steps.iter().any(|s| s.rewrote)
    }
}

/// The result of a ReAct decide: the produced events (if any) plus the ALWAYS-returned visible trace.
pub struct ReactResult {
    pub events: Option<Vec<Event>>,
    pub error: Option<DomainError>,
    pub trace: ReactTrace,
}

/// The default visible ReAct iteration count. MUST be 3 (user requirement); overridable per call.
pub const DEFAULT_REACT_ITERS: u32 = 3;

/// REAL-TIME EVAL GATE for one ReAct iteration. Mirrors the bebop `evalStep`: `decide` (the guard)
/// decides legality; this decides QUALITY of the iteration. Falsifiable: a denial scores 0, a clean
/// success scores 100, a neutral step 50.
fn react_eval(denied: bool, events: usize) -> u8 {
    if denied {
        0
    } else if events > 0 {
        100
    } else {
        50
    }
}

/// The ONLY honest self-correction the kernel may make: a SYSTEM-only cancel edge
/// (CONFIRMED/PREPARING/READY→CANCELLED) attempted by an OWNER is rewritten to a SYSTEM-actor command
/// (the dispatch-grace path keeps the edge). Any other error (a genuinely illegal transition, a
/// pricing breach, a cc1 strand) has NO valid rewrite → `None`, and the loop must stop. No business
/// logic is invented here — this is a documented corridor, not a new rule.
fn react_reflect(command: &Command, err: &DomainError) -> Option<Command> {
    if let DomainError::CorridorBreach { corridor, .. } = err {
        if *corridor == "actor_gate" {
            if let Command::Cancel { at, actor: Actor::Owner } = command {
                return Some(Command::Cancel {
                    at: *at,
                    actor: Actor::System,
                });
            }
        }
    }
    None
}

/// ReAct wrapper around [`decide`]. Visible, auditable, default 3 iterations. NEVER modifies `decide`.
///
/// Semantics (the promo-demo fix): each attempt is recorded in `trace`. On `Err`, `react_reflect` may
/// rewrite the command into an equivalent LEGAL one and retry (up to `iterations`). If no valid
/// rewrite exists, the loop stops immediately and returns the original error — it never loops forever
/// and never fabricates a state. The trace is ALWAYS returned so the hidden retry becomes visible.
pub fn react_decide(
    state: &OrderState,
    mut command: Command,
    ctx: &Context,
    iterations: u32,
) -> ReactResult {
    let iters = if iterations >= 1 { iterations } else { DEFAULT_REACT_ITERS };
    let mut trace = ReactTrace::default();
    for iter in 1..=iters {
        let thought = format!("react attempt {iter}");
        // ACT through the red-line door.
        let res = decide(state, command.clone(), ctx);
        match &res {
            Ok(events) => {
                trace.push(ReactStep {
                    iter,
                    phase: "reason→act→observe→reflect".into(),
                    thought,
                    observation: format!("Ok({} events)", events.len()),
                    eval_score: react_eval(false, events.len()),
                    ok: true,
                    rewrote: false,
                });
                return ReactResult {
                    events: Some(events.clone()),
                    error: None,
                    trace,
                };
            }
            Err(err) => {
                let observation = format!("Err({:?})", err.code());
                match react_reflect(&command, err) {
                    Some(rewritten) => {
                        trace.push(ReactStep {
                            iter,
                            phase: "reason→act→observe→reflect".into(),
                            thought: format!("{thought}: rewrote command for next iteration"),
                            observation,
                            eval_score: react_eval(true, 0),
                            ok: false,
                            rewrote: true,
                        });
                        command = rewritten;
                        continue;
                    }
                    None => {
                        trace.push(ReactStep {
                            iter,
                            phase: "reason→act→observe→reflect".into(),
                            thought,
                            observation,
                            eval_score: react_eval(true, 0),
                            ok: false,
                            rewrote: false,
                        });
                        // No valid rewrite → STOP (do not loop forever, do not invent a state).
                        return ReactResult {
                            events: None,
                            error: Some(*err),
                            trace,
                        };
                    }
                }
            }
        }
    }
    // Exhausted iterations without success (only reachable if a rewrite kept colliding). Honest stop.
    ReactResult {
        events: None,
        error: Some(DomainError::CorridorBreach {
            corridor: "react_exhausted",
            code: ErrorCode::Internal,
        }),
        trace,
    }
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

    /// A no-binding, nothing-paid context — what a transition command sees on a never-dispatched,
    /// unpaid order (cc1 is a no-op, no refund fires).
    const NO_BINDING: policy::BindingState = policy::BindingState {
        has_active_binding: false,
        has_delivered_binding: false,
    };
    fn plain_ctx() -> Context<'static> {
        Context {
            binding: NO_BINDING,
            refundable_paid: Lek::ZERO,
            pricing: None,
        }
    }
    /// The completeDelivery context — a `delivered` assignment exists, none active — so cc1 permits a
    /// →DELIVERED/→PICKED_UP (the deliver-flow completion path).
    fn delivered_ctx() -> Context<'static> {
        Context {
            binding: policy::BindingState {
                has_active_binding: false,
                has_delivered_binding: true,
            },
            refundable_paid: Lek::ZERO,
            pricing: None,
        }
    }

    #[test]
    fn decide_emits_one_status_changed_event_carrying_the_command_time() {
        let state = OrderState::genesis(); // PENDING
        let events = decide(
            &state,
            Command::Confirm {
                at: T,
                actor: Actor::Owner,
            },
            &plain_ctx(),
        )
        .unwrap();
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
            decide(
                &ready,
                Command::Confirm {
                    at: T,
                    actor: Actor::Owner
                },
                &plain_ctx()
            ),
            Err(DomainError::IllegalTransition { .. })
        ));
    }

    #[test]
    fn a_terminal_order_absorbs_every_command() {
        let delivered = OrderState { status: OrderStatus::Delivered, ..OrderState::genesis() };
        for cmd in [
            Command::Cancel {
                at: T,
                actor: Actor::System,
            },
            Command::Confirm {
                at: T,
                actor: Actor::Owner,
            },
            Command::RevertToReady {
                at: T,
                actor: Actor::System,
            },
        ] {
            assert!(
                decide(&delivered, cmd.clone(), &plain_ctx()).is_err(),
                "delivered must absorb {cmd:?}"
            );
        }
    }

    #[test]
    fn a_full_lifecycle_folds_up_from_the_event_log() {
        let genesis = OrderState::genesis();
        let commands = [
            Command::Confirm {
                at: T,
                actor: Actor::Owner,
            },
            Command::StartPreparing {
                at: T,
                actor: Actor::Owner,
            },
            Command::MarkReady {
                at: T,
                actor: Actor::Owner,
            },
            Command::Dispatch {
                at: T,
                actor: Actor::Owner,
            },
            // The deliver-flow completion — cc1 now composes into `decide`, so →DELIVERED needs the
            // delivered-binding context (a `delivered` assignment exists), else it is USE_DELIVER_FLOW.
            Command::MarkDelivered {
                at: T,
                actor: Actor::System,
            },
        ];
        let mut state = genesis;
        let mut log = Vec::new();
        for c in &commands {
            // →DELIVERED/→PICKED_UP need the completeDelivery context; every other edge is cc1-inert.
            let ctx = if matches!(c.target(), OrderStatus::Delivered | OrderStatus::PickedUp) {
                delivered_ctx()
            } else {
                plain_ctx()
            };
            let events = decide(&state, c.clone(), &ctx).expect("each step is a legal edge");
            for e in &events {
                state = fold(&state, e);
            }
            log.extend(events);
        }
        assert_eq!(state.status, OrderStatus::Delivered);
        // The state is fully reconstructible from the log alone.
        assert_eq!(replay(genesis, &log), state);
    }

    // ─────────────────────── 0b-3: corridors composed behind the single `decide` door ───────────────────────

    #[test]
    fn owner_is_actor_gated_off_the_system_only_cancel_edges_but_system_keeps_them() {
        // CONFIRMED→CANCELLED is machine-legal (deliver-v2 sweep) but a SYSTEM-only edge (orderAuthz).
        let confirmed = OrderState { status: OrderStatus::Confirmed, ..OrderState::genesis() };
        // Owner driving it → CorridorBreach carrying the EXACT wire code the shell returns.
        assert_eq!(
            decide(
                &confirmed,
                Command::Cancel { at: T, actor: Actor::Owner },
                &plain_ctx()
            ),
            Err(DomainError::CorridorBreach {
                corridor: "actor_gate",
                code: ErrorCode::CancelNotPermitted,
            })
        );
        // System (dispatch-grace) keeps the edge → StatusChanged + BindingTerminalized (R2-3).
        let events = decide(
            &confirmed,
            Command::Cancel { at: T, actor: Actor::System },
            &plain_ctx(),
        )
        .unwrap();
        assert_eq!(
            events[0],
            Event::StatusChanged {
                from: OrderStatus::Confirmed,
                to: OrderStatus::Cancelled,
                at: T,
            }
        );
        assert!(events.contains(&Event::BindingTerminalized));
    }

    #[test]
    fn cc1_strand_guard_composes_an_active_binding_blocks_delivered() {
        let in_delivery = OrderState { status: OrderStatus::InDelivery, ..OrderState::genesis() };
        let ctx = Context {
            binding: policy::BindingState {
                has_active_binding: true,
                has_delivered_binding: false,
            },
            refundable_paid: Lek::ZERO,
            pricing: None,
        };
        assert_eq!(
            decide(
                &in_delivery,
                Command::MarkDelivered { at: T, actor: Actor::System },
                &ctx
            ),
            Err(DomainError::CorridorBreach {
                corridor: "cc1_strand",
                code: ErrorCode::AssignmentActive,
            })
        );
    }

    #[test]
    fn cancel_of_a_paid_order_emits_status_binding_and_refund_facts() {
        // The PROGRESS 0b-3 example verbatim: Cancel of a paid+priced order →
        // [StatusChanged, BindingTerminalized, RefundObligated]. IN_DELIVERY→CANCELLED is owner-legal
        // (no-show); the refund fires because a paid amount is OBSERVED on the context.
        let priced = OrderState {
            status: OrderStatus::InDelivery,
            totals: Some(OrderTotals {
                subtotal: lek(1_000),
                delivery_fee: lek(200),
                tax_total: lek(120),
                total: lek(1_320),
            }),
            ..OrderState::genesis()
        };
        let ctx = Context {
            binding: NO_BINDING,
            refundable_paid: lek(1_320),
            pricing: None,
        };
        let events = decide(
            &priced,
            Command::Cancel { at: T, actor: Actor::Owner },
            &ctx,
        )
        .unwrap();
        assert_eq!(
            events,
            vec![
                Event::StatusChanged {
                    from: OrderStatus::InDelivery,
                    to: OrderStatus::Cancelled,
                    at: T,
                },
                Event::BindingTerminalized,
                Event::RefundObligated { amount: lek(1_320) },
            ]
        );
    }

    #[test]
    fn cancel_with_no_paid_amount_emits_no_refund_obligation() {
        // Zero paid rows (today's reality) ⇒ terminalize fires, refund does NOT (nothing was charged).
        let unpaid = OrderState { status: OrderStatus::InDelivery, ..OrderState::genesis() };
        let events = decide(
            &unpaid,
            Command::Cancel { at: T, actor: Actor::Owner },
            &plain_ctx(),
        )
        .unwrap();
        assert_eq!(
            events,
            vec![
                Event::StatusChanged {
                    from: OrderStatus::InDelivery,
                    to: OrderStatus::Cancelled,
                    at: T,
                },
                Event::BindingTerminalized,
            ]
        );
    }

    #[test]
    fn place_order_prices_the_cart_into_a_priced_fact() {
        use std::collections::HashMap;
        // one product @ 1000, qty 2, no modifiers; pickup (no delivery fee); 20% exclusive tax.
        let mut product_map = HashMap::new();
        product_map.insert(
            "p1".to_string(),
            pricing::ProductInfo { name: "Pizza".to_string(), price: lek(1_000) },
        );
        let mod_map = HashMap::new();
        let groups_by_product = HashMap::new();
        let snapshot = pricing::PricingSnapshot {
            product_map: &product_map,
            mod_map: &mod_map,
            groups_by_product: &groups_by_product,
        };
        let inputs = PriceInputs {
            snapshot,
            is_pickup: true,
            location: pricing::FeeLocation {
                delivery_fee_flat: None,
                free_delivery_threshold: None,
                min_order_value: None,
            },
            distance_m: None,
            tiers: &[],
            rate_micro: 200_000, // 0.2 exclusive
            price_includes_tax: false,
        };
        let ctx = Context { binding: NO_BINDING, refundable_paid: Lek::ZERO, pricing: Some(inputs) };
        let cart = vec![pricing::PricingItem {
            product_id: "p1".to_string(),
            quantity: 2,
            modifier_ids: vec![],
        }];
        let events = decide(
            &OrderState::genesis(),
            Command::PlaceOrder { at: T, actor: Actor::Owner, cart },
            &ctx,
        )
        .unwrap();
        // subtotal = 2000, delivery = 0 (pickup), tax = 400 (2000·0.2), total = 2400. LC1 (exclusive)
        // adds the whole tax; conservation total = subtotal + delivery_fee + charged_tax − 0.
        assert_eq!(
            events,
            vec![Event::Priced {
                subtotal: lek(2_000),
                delivery_fee: lek(0),
                tax_total: lek(400),
                total: lek(2_400),
            }]
        );
    }

    #[test]
    fn place_order_without_pricing_context_is_a_caller_contract_breach() {
        // PlaceOrder REQUIRES a price authority on the context; its absence is Internal (a caller
        // contract violation), never a panic — decide stays TOTAL.
        let cart = vec![pricing::PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec![],
        }];
        assert_eq!(
            decide(
                &OrderState::genesis(),
                Command::PlaceOrder { at: T, actor: Actor::Owner, cart },
                &plain_ctx()
            ),
            Err(DomainError::CorridorBreach {
                corridor: "pricing",
                code: ErrorCode::Internal,
            })
        );
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

    // ─────────────────────── ReAct agentic loop (Reason→Act→Observe→Reflect) ───────────────────────

    fn confirmed_state() -> OrderState {
        OrderState { status: OrderStatus::Confirmed, ..OrderState::genesis() }
    }

    #[test]
    fn react_default_iteration_count_is_3() {
        // The user requirement: default ReAct iterations == 3.
        assert_eq!(DEFAULT_REACT_ITERS, 3);
    }

    #[test]
    fn react_decide_succeeds_on_first_try_and_emits_one_visible_step() {
        // A legal command → first iteration succeeds, trace has exactly 1 step, ok=true.
        let res = react_decide(
            &OrderState::genesis(),
            Command::Confirm { at: T, actor: Actor::Owner },
            &plain_ctx(),
            DEFAULT_REACT_ITERS,
        );
        assert!(res.error.is_none());
        assert!(res.events.is_some());
        assert_eq!(res.trace.steps.len(), 1);
        assert!(res.trace.steps[0].ok);
        // eval gate: clean success scores 100
        assert_eq!(res.trace.steps[0].eval_score, 100);
        assert!(!res.trace.had_rewrite());
    }

    #[test]
    fn react_decide_rewrites_owner_cancel_to_system_and_makes_the_retry_visible() {
        // CONFIRMED→CANCELLED is machine-legal but a SYSTEM-only edge: an OWNER hitting it is denied,
        // react_reflect rewrites it to a SYSTEM-actor command, and the NEXT iteration succeeds. The
        // hidden retry is now VISIBLE: trace shows 2 steps and had_rewrite()==true.
        let res = react_decide(
            &confirmed_state(),
            Command::Cancel { at: T, actor: Actor::Owner },
            &plain_ctx(),
            DEFAULT_REACT_ITERS,
        );
        assert!(res.error.is_none(), "rewritten System cancel must succeed");
        assert!(res.events.is_some());
        // iter 1 = denied+rewrite (ok=false), iter 2 = success (ok=true) → 2 visible steps
        assert_eq!(res.trace.steps.len(), 2, "both iterations must be recorded");
        assert!(!res.trace.steps[0].ok, "first attempt (owner) is the visible denial");
        assert!(res.trace.steps[1].ok, "second attempt (system) succeeds");
        assert!(res.trace.had_rewrite(), "the rewrite must be visible in the trace");
        // eval gate: denial scored 0, success scored 100
        assert_eq!(res.trace.steps[0].eval_score, 0);
        assert_eq!(res.trace.steps[1].eval_score, 100);
    }

    #[test]
    fn react_iteration_count_is_configurable_and_honored() {
        // With iterations=1 a needed-rewrite command CANNOT retry → it fails (exhausted). With 2 it
        // rewrites on iter1 and succeeds on iter2. This proves the count is real, not cosmetic.
        let one = react_decide(
            &confirmed_state(),
            Command::Cancel { at: T, actor: Actor::Owner },
            &plain_ctx(),
            1,
        );
        assert!(one.error.is_some(), "iterations=1 must not allow the rewrite to land");
        let two = react_decide(
            &confirmed_state(),
            Command::Cancel { at: T, actor: Actor::Owner },
            &plain_ctx(),
            2,
        );
        assert!(two.error.is_none(), "iterations=2 allows the rewrite to succeed");
        assert_eq!(two.trace.steps.len(), 2);
    }

    #[test]
    fn react_decide_stops_on_a_genuinely_illegal_command_and_does_not_loop_forever() {
        // A terminal DELIVERED order absorbing CANCEL is a hard IllegalTransition — there is NO valid
        // rewrite. The loop must STOP at iteration 1 (not spin 3 times) and return the original error.
        let delivered = OrderState { status: OrderStatus::Delivered, ..OrderState::genesis() };
        let res = react_decide(
            &delivered,
            Command::Cancel { at: T, actor: Actor::System },
            &plain_ctx(),
            DEFAULT_REACT_ITERS,
        );
        assert!(res.error.is_some());
        assert!(res.events.is_none());
        assert_eq!(res.trace.steps.len(), 1, "must not loop 3x on an unrewritable denial");
        assert!(!res.trace.steps[0].ok);
        assert_eq!(res.trace.steps[0].eval_score, 0);
        assert!(!res.trace.had_rewrite(), "no rewrite happened — the denial is honest");
    }

    #[test]
    fn react_trace_is_serializable_as_audit_metadata() {
        // The trace must round-trip through the crate's canonical bytes so the shell can persist it
        // alongside the Envelope as audit metadata (never hidden from the operator).
        let res = react_decide(
            &confirmed_state(),
            Command::Cancel { at: T, actor: Actor::Owner },
            &plain_ctx(),
            DEFAULT_REACT_ITERS,
        );
        let bytes = crate::canonical_bytes(&res.trace).expect("trace serializes");
        let decoded: ReactTrace = crate::from_bytes(&bytes).expect("trace deserializes");
        assert_eq!(decoded, res.trace);
        assert!(decoded.had_rewrite());
    }
}
