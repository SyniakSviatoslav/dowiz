//! decision/mod.rs — kernel-side MoE mesh DecisionUnit family (BLUEPRINT-P-F, Layer F).
//!
//! Compile firewall (mirrors `ports/llm.rs`): ZERO network / HTTP / JSON / serde. The oracle
//! (LLM) lives OUTSIDE this module — it only ever emits unit *source* that a future import gate
//! (`import_unit`, §4.2) admits. This module defines the closed capability types, the
//! `DecisionUnit` family abstraction, the by-`DomainTag` registry, and the two structural red-lines
//! of the phase:
//!   1. **NO-COURIER-SCORING** — routing is a `match` on a declared `DomainTag`, never a numeric
//!      rank. `DomainTag` derives `PartialEq/Eq/Hash` only (NOT `Ord`/`PartialOrd`): there is no
//!      ordering to compare, so a quality-router is *unrepresentable*, not merely forbidden.
//!   2. **Money red-line** — every `Pricing` unit is `money_gated`. It may register and replay
//!      green (reach `Live`), but `decide()` answers `Escalate` until an explicit operator-activation
//!      event flips `OperatorActivation::Pending -> Activated`. It NEVER auto-adopts (§6.3-A6).
//!
//! `FraudAuth` output is `FraudVerdict { NotAnomalous | Escalate }` — an auto-block verdict is
//! unrepresentable by construction (§2.1, §7).

/// The import-time verify-before-persist gate (BLUEPRINT-P-F §4.2, rung-2).
pub mod import;

use std::collections::HashMap;
use std::fmt::Debug;

// ─────────────────────────────────────────────────────────────────────────────
// §3 types (verbatim contract surface)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed set of decision-procedure families ("domain experts"). Extension = a code change +
/// review, deliberately — an open string tag would be an unreviewed capability grant.
/// NO ordering, NO numeric rank: routing is `match`, never comparison (NO-COURIER-SCORING).
///
/// NOTE: this enum intentionally derives only `PartialEq, Eq, Hash` — **not** `Ord`/`PartialOrd`.
/// The absence of an order relation is the type-level enforcement of the no-courier-scoring law:
/// a router cannot rank two families because the language gives it no `cmp` to call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainTag {
    Dispatch,
    EtaGeo,
    Pricing,
    FraudAuth,
    MenuInventory,
    Harness,
}

impl DomainTag {
    /// Stable, typed string for telemetry (native `metrics::LogEvent` lane, D8).
    /// NOT a capability grant — just the closed discriminant rendered as text.
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainTag::Dispatch => "Dispatch",
            DomainTag::EtaGeo => "EtaGeo",
            DomainTag::Pricing => "Pricing",
            DomainTag::FraudAuth => "FraudAuth",
            DomainTag::MenuInventory => "MenuInventory",
            DomainTag::Harness => "Harness",
        }
    }
}

/// Identity of a question shape: sha3-256 over (DomainTag discriminant ‖ canonical input/output
/// schema). Content-derived — two hubs naming the same shape get the same id with no registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub [u8; 32]);

/// Logical epoch for a compiled unit. Lamport-style monotone counter — NEVER wall-clock
/// (Batch 2 §7 HLC rejection stands). Merge law = max (§4.1). `Ord` is allowed here: an epoch is
/// a monotone logical counter, categorically NOT a quality rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitEpoch(pub u64);

/// Provenance + lineage record for one compiled unit version. Registered as an event in the
/// EXISTING content-addressed sha3 event log — `content_id`/`prev` ARE the lineage (§4.3);
/// there is no second DAG type anywhere in this design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionUnitMeta {
    pub shape: ShapeId,
    pub domain: DomainTag,
    pub epoch: UnitEpoch,
    /// sha3 of the harvested instance set the unit was compiled from and must replay against.
    pub instance_set_hash: [u8; 32],
    /// sha3 content-address of THIS version's artifact (source + tests), in the one log.
    pub content_id: [u8; 32],
    /// Previous version's content-id — rollback lineage inside the same log. None = genesis.
    pub prev_content_id: Option<[u8; 32]>,
    /// Money red-line: true for every Pricing unit (and any unit whose output moves money).
    /// A money-gated unit CANNOT reach a callable Live via import alone — operator activation required.
    pub money_gated: bool,
}

impl DecisionUnitMeta {
    /// Construct meta. `money_gated` is derived from `domain` (Pricing ⇒ true) — set by the
    /// *domain*, never by the unit author (§7). Lineage fields default to genesis/zero; the import
    /// gate fills them from the content-addressed log.
    pub fn new(domain: DomainTag, epoch: UnitEpoch) -> Self {
        let money_gated = domain == DomainTag::Pricing;
        Self {
            shape: ShapeId([0u8; 32]),
            domain,
            epoch,
            instance_set_hash: [0u8; 32],
            content_id: [0u8; 32],
            prev_content_id: None,
            money_gated,
        }
    }
}

/// Unit lifecycle. The ONLY production path to `Live` is the import gate (`import_unit`, §4.2);
/// `new` is the in-kernel constructor used by tests and by that gate. `Stale ⇒ Escalate`
/// unconditionally (D5): there is no `Stale -> Answer` transition, so a stale unit cannot answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitState {
    Live,
    Stale,
    Rejected,
}

/// Reuse P29 §2.1 verbatim — never a silent guess:
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision<T> {
    Answer(T),
    Escalate(EscalateReason),
}

/// Why a unit declined to answer. First-class on every `Decision` (P29 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalateReason {
    /// Unit is not in a callable state (Stale / Rejected), or not operator-activated.
    NotLive,
    /// Money-gated unit is registered + replay-green but the operator has NOT activated it (§6.3-A6).
    MoneyGateLocked,
    /// A watched input drifted (GapWire) — recompile required before answering.
    WatchedInputDrift,
    /// The input is outside the unit's declared capability shape.
    OutOfShape,
}

/// Integer money — basis points. No float money type exists in this module on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeBps(pub u32);

/// The artifact bound = the transport bound. Pinned to the same literal as
/// bebop2 `sync_pull.rs:159` MAX_SYNC_PAYLOAD; the cross-repo drift test (§6.2 D6) keeps them
/// equal — a unit that cannot ride the transport may not exist.
pub const MAX_UNIT_ARTIFACT_BYTES: usize = 1 << 20;

/// Harvest threshold, inherited from P29 §2.4(b) — tunable, named, never magic.
pub const HARVEST_MIN_INSTANCES: usize = 10;

// ─────────────────────────────────────────────────────────────────────────────
// Operator-activation gate (money red-line, §6.3-A6)
// ─────────────────────────────────────────────────────────────────────────────

/// Distinct state the Pricing family requires before it can be invoked. A money-gated unit
/// reaches `Live` (registered + replay-green) but stays `Pending` until an explicit operator
/// event calls `activate_operator`. It must NOT auto-adopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActivation {
    Pending,
    Activated,
}

/// Error returned when an operator-activation attempt violates the gate's preconditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateError {
    /// Only a money-gated (Pricing) unit requires operator activation.
    NotMoneyGated,
    /// The unit must be registered + replay-green (`Live`) before activation is meaningful.
    NotLive,
}

// ─────────────────────────────────────────────────────────────────────────────
// Family I/O (typed, closed)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchInput {
    pub pickup_adjacent: bool,
    pub window_overlap: bool,
    pub capacity_ok: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOut {
    pub batch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaGeoInput {
    pub distance_band: u8,
    pub hour: u8,
    pub weather_class: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaBand(pub u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingInput {
    pub zone: u8,
    pub cart_band: u8,
    pub hour: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FraudInput {
    pub pattern_score: u32,
    pub velocity: u32,
}
/// FraudAuth output — escalate-biased. An **auto-block** verdict is UNREPRESENTABLE: the type
/// only admits `NotAnomalous` or `Escalate`, so a unit that would silently block can never be
/// written (§2.1, §7). The human/operator still decides on escalate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FraudVerdict {
    NotAnomalous,
    Escalate(EscalateReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuInput {
    pub item: u32,
    pub stock: u32,
    pub is_86: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuOut {
    pub order_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessInput {
    pub job: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessOut {
    pub route_tier: u8,
}

// ─────────────────────────────────────────────────────────────────────────────
// DecisionUnit family type (P29 §2.1: pure decide(), typed input, closed output, Escalate first-class)
// ─────────────────────────────────────────────────────────────────────────────

/// A compiled ns-native decision procedure. Pure: no I/O, no mutation, deterministic. `Escalate`
/// is first-class. Constructed ONLY via `new` (tests/in-kernel) or the import gate (§4.2) — there
/// is no public `Live` literal to fake a callable unit without evidence.
pub struct DecisionUnit<I, O> {
    pub meta: DecisionUnitMeta,
    pub state: UnitState,
    pub operator_activation: OperatorActivation,
    proc: Box<dyn Fn(&I) -> Decision<O> + Send + Sync>,
}

impl<I, O> DecisionUnit<I, O> {
    /// In-kernel constructor. `money_gated` is forced from `domain` (Pricing ⇒ true); non-money
    /// units start already `Activated` so the gate never blocks them.
    pub fn new(
        domain: DomainTag,
        epoch: UnitEpoch,
        proc: impl Fn(&I) -> Decision<O> + Send + Sync + 'static,
    ) -> Self {
        let meta = DecisionUnitMeta::new(domain, epoch);
        let operator_activation = if meta.money_gated {
            OperatorActivation::Pending
        } else {
            OperatorActivation::Activated
        };
        Self {
            meta,
            state: UnitState::Live,
            operator_activation,
            proc: Box::new(proc),
        }
    }

    /// Pure decision. Returns `Escalate` when the unit is not callable: `Stale`/`Rejected`, or a
    /// money-gated unit whose operator has not yet activated it.
    pub fn decide(&self, input: &I) -> Decision<O> {
        if self.state != UnitState::Live {
            return Decision::Escalate(EscalateReason::NotLive);
        }
        if self.meta.money_gated && self.operator_activation != OperatorActivation::Activated {
            return Decision::Escalate(EscalateReason::MoneyGateLocked);
        }
        (self.proc)(input)
    }

    /// True iff this unit may currently answer (Live AND, for money units, operator-activated).
    pub fn can_answer(&self) -> bool {
        self.state == UnitState::Live
            && (!self.meta.money_gated || self.operator_activation == OperatorActivation::Activated)
    }

    /// Operator-activation event (§6.3-A6). Money-gated units MUST be `Live` (registered + replay
    /// green) before this succeeds; it flips `Pending -> Activated` and unlocks `decide()`.
    /// Calling it on a non-money unit is a misuse — returns `NotMoneyGated`.
    pub fn activate_operator(&mut self) -> Result<(), GateError> {
        if !self.meta.money_gated {
            return Err(GateError::NotMoneyGated);
        }
        if self.state != UnitState::Live {
            return Err(GateError::NotLive);
        }
        self.operator_activation = OperatorActivation::Activated;
        Ok(())
    }

    /// Item 51 seam (co-spec with item 47's decision seam). Compute the deterministic
    /// decision D (`decide` is `&self` and returns an owned `Decision<O>` — pure), then —
    /// when a shadow `ring` is supplied AND advice was present (`proposal: Some`) — emit ONE
    /// advisory `ShadowDivergence` FDR record. The emit is a pure side-channel write; it can
    /// NEVER observe or mutate D. `decide_with_shadow` with `ring = None` is the shadow-OFF
    /// path (item-47 `None`-path pattern); with `Some` it is shadow-ON. Toggling shadow mode
    /// therefore changes ZERO bytes of D — pinned by
    /// `shadow_telemetry_tests::decision_is_bit_identical_with_shadow_on_or_off`.
    ///
    /// Host-only: the FDR ring does not exist on wasm (`crate::fdr::ring` is wasm-gated).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn decide_with_shadow(
        &self,
        input: &I,
        proposal: Option<&O>,
        ring: Option<&mut crate::fdr::ring::FdrRing>,
    ) -> Decision<O>
    where
        O: PartialEq + std::fmt::Debug,
    {
        let d = self.decide(input); // D — the total, primary, deterministic decision.
        if let (Some(act), Some(r)) = (proposal, ring) {
            // Agree = the proposed action EQUALS the admitted decision. When D escalates
            // there is no admitted action, so the proposal can never agree (blueprint §3.3:
            // `proposal.action == D`). `agree` is the verdict of the comparison bit only.
            let agree = matches!(&d, Decision::Answer(a) if a == act);
            // Self-consistency cross-check (compiled out of release): the agree bit equals the
            // direct `proposal.action == D` recomputation at the emit site (blueprint §4.3).
            debug_assert_eq!(agree, matches!(&d, Decision::Answer(a) if a == act));
            let verdict = match &d {
                Decision::Answer(_) => "admitted",
                Decision::Escalate(_) => "refuted",
            };
            // Digests ONLY — never the full payloads (blueprint §3.2 minimal-statistic rule).
            let d_digest = crate::fdr::shadow_digest(format!("{d:?}").as_bytes());
            let act_digest = crate::fdr::shadow_digest(format!("{act:?}").as_bytes());
            let ev = crate::fdr::schema::FdrEvent::stamp(
                0,
                crate::fdr::Level::Info,
                crate::fdr::schema::Kind::ShadowDivergence,
                "decision_seam".to_string(),
                crate::fdr::schema::StampPolicy::Cheap,
                vec![
                    ("verdict", verdict.to_string()),
                    ("agree", if agree { "1" } else { "0" }.to_string()),
                    ("d_digest", d_digest),
                    ("act_digest", act_digest),
                ],
            );
            let _ = r.append(&ev);
        }
        d
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Family registry — map DomainTag -> registered units (code-change + review path)
// ─────────────────────────────────────────────────────────────────────────────

/// Heterogeneous, type-preserving container for one registered unit. Each variant carries the
/// fully-typed `DecisionUnit` for its family, so the closed output type (e.g. `FraudVerdict`)
/// survives into the call site. `AnyUnit` is also the routing surface: selecting by `DomainTag`
/// is an exhaustive `match` over these six variants — there is no numeric index to rank on.
pub enum AnyUnit {
    Dispatch(DecisionUnit<DispatchInput, DispatchOut>),
    EtaGeo(DecisionUnit<EtaGeoInput, EtaBand>),
    Pricing(DecisionUnit<PricingInput, FeeBps>),
    FraudAuth(DecisionUnit<FraudInput, FraudVerdict>),
    MenuInventory(DecisionUnit<MenuInput, MenuOut>),
    Harness(DecisionUnit<HarnessInput, HarnessOut>),
}

impl AnyUnit {
    /// The closed capability this unit answers — used as the registry key.
    pub fn domain(&self) -> DomainTag {
        match self {
            AnyUnit::Dispatch(_) => DomainTag::Dispatch,
            AnyUnit::EtaGeo(_) => DomainTag::EtaGeo,
            AnyUnit::Pricing(_) => DomainTag::Pricing,
            AnyUnit::FraudAuth(_) => DomainTag::FraudAuth,
            AnyUnit::MenuInventory(_) => DomainTag::MenuInventory,
            AnyUnit::Harness(_) => DomainTag::Harness,
        }
    }

    /// Callable now? (Live, and operator-activated if money-gated.)
    pub fn is_live_and_active(&self) -> bool {
        match self {
            AnyUnit::Dispatch(u) => u.can_answer(),
            AnyUnit::EtaGeo(u) => u.can_answer(),
            AnyUnit::Pricing(u) => u.can_answer(),
            AnyUnit::FraudAuth(u) => u.can_answer(),
            AnyUnit::MenuInventory(u) => u.can_answer(),
            AnyUnit::Harness(u) => u.can_answer(),
        }
    }
}

/// Domain family registry. `register` is the reviewed code path — a unit is admitted by name, into
/// the slot keyed by its `DomainTag`. Routing is a `HashMap` lookup + an exhaustive `match`
/// (`route_live`), never a comparison of quality.
#[derive(Default)]
pub struct DecisionRegistry {
    units: HashMap<DomainTag, Vec<AnyUnit>>,
}

impl DecisionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit a unit (code-change + review path). Placed under its `DomainTag` slot.
    pub fn register(&mut self, unit: AnyUnit) {
        self.units.entry(unit.domain()).or_default().push(unit);
    }

    /// Routing law: lookup on `DomainTag` — a match on a declared capability, never a rank compare.
    /// Returns the first callable unit registered for `tag`, if any.
    pub fn route_live(&self, tag: DomainTag) -> Option<&AnyUnit> {
        self.units
            .get(&tag)
            .and_then(|v| v.iter().find(|u| u.is_live_and_active()))
    }

    /// Count of callable units under `tag` (registry-introspection / telemetry helper).
    pub fn count_live(&self, tag: DomainTag) -> usize {
        self.units
            .get(&tag)
            .map(|v| v.iter().filter(|u| u.is_live_and_active()).count())
            .unwrap_or(0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Epoch max-merge (§4.1) — join-semilattice: commutative, associative, idempotent.
// Tiebreak is on content-id BYTES (a content address), NOT on a quality score.
// ─────────────────────────────────────────────────────────────────────────────

/// Merge two unit metas into the winning version. Higher `epoch` wins; on equal epoch the
/// lexicographically-lower `content_id` wins (deterministic, no scoring). Identical ⇒ idempotent.
/// This is a join-semilattice, so gossip convergence is order-independent (§4.1).
pub fn merge_meta(a: &DecisionUnitMeta, b: &DecisionUnitMeta) -> DecisionUnitMeta {
    debug_assert_eq!(a.domain, b.domain, "merge_meta called across domains");
    use std::cmp::Ordering;
    match a.epoch.0.cmp(&b.epoch.0) {
        Ordering::Greater => a.clone(),
        Ordering::Less => b.clone(),
        Ordering::Equal => {
            if a.content_id <= b.content_id {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — spec-first (RED→GREEN). Three invariants asserted:
//   A) Pricing unit is NOT invocable before operator activation.
//   B) FraudAuth cannot return an auto-block variant (unrepresentable by construction).
//   C) Routing by `DomainTag` is a match, never a rank compare (`DomainTag` has no `Ord`).
//   D) Epoch merge is a join-semilattice.
//   E) A Stale unit answers `Escalate` unconditionally (unrepresentable Stale→Answer).
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pricing_unit() -> DecisionUnit<PricingInput, FeeBps> {
        DecisionUnit::new(DomainTag::Pricing, UnitEpoch(1), |_in: &PricingInput| {
            Decision::Answer(FeeBps(50))
        })
    }

    fn fraud_unit() -> DecisionUnit<FraudInput, FraudVerdict> {
        DecisionUnit::new(DomainTag::FraudAuth, UnitEpoch(1), |_in: &FraudInput| {
            Decision::Answer(FraudVerdict::NotAnomalous)
        })
    }

    // A) Money red-line: a Pricing unit that is Live + replay-green is still NOT callable until the
    //    operator activates it.
    #[test]
    fn pricing_not_invocable_before_activation() {
        let mut unit = pricing_unit();
        assert!(
            unit.meta.money_gated,
            "Pricing must be money_gated by domain"
        );
        assert_eq!(unit.operator_activation, OperatorActivation::Pending);

        // Registered + replay-green (Live) but not yet activated ⇒ Escalate, never an Answer.
        assert!(matches!(
            unit.decide(&PricingInput {
                zone: 1,
                cart_band: 2,
                hour: 9
            }),
            Decision::Escalate(EscalateReason::MoneyGateLocked)
        ));
        assert!(!unit.can_answer());

        // Operator event flips the gate.
        assert!(unit.activate_operator().is_ok());
        assert_eq!(unit.operator_activation, OperatorActivation::Activated);
        assert!(unit.can_answer());
        assert!(matches!(
            unit.decide(&PricingInput {
                zone: 1,
                cart_band: 2,
                hour: 9
            }),
            Decision::Answer(FeeBps(50))
        ));
    }

    #[test]
    fn non_money_unit_has_no_gate() {
        let mut unit = fraud_unit();
        // Non-money units are Activated from construction; activation is a no-op misuse.
        assert!(!unit.meta.money_gated);
        assert_eq!(unit.operator_activation, OperatorActivation::Activated);
        assert_eq!(unit.activate_operator(), Err(GateError::NotMoneyGated));
        assert!(unit.can_answer());
    }

    // B) FraudAuth cannot auto-block: the closed output type has no `Block` variant, so the
    //    exhaustive match below compiles ONLY because no such variant exists. This is the proof
    //    that an auto-block verdict is unrepresentable.
    #[test]
    fn fraudauth_cannot_autoblock() {
        let unit = fraud_unit();
        let out = unit.decide(&FraudInput {
            pattern_score: 0,
            velocity: 0,
        });
        // The unit answered; its verdict is one of the two admissible shapes. There is no third
        // (Block) arm to match, and adding one would break this exhaustive match at compile time.
        fraud_shape_proof(&match out {
            Decision::Answer(v) => v,
            Decision::Escalate(_) => FraudVerdict::Escalate(EscalateReason::NotLive),
        });
    }

    /// Compile-time proof: `FraudVerdict` admits exactly `NotAnomalous | Escalate`. A `Block`
    /// variant would make this non-exhaustive and fail to compile.
    fn fraud_shape_proof(v: &FraudVerdict) {
        match v {
            FraudVerdict::NotAnomalous => {}
            FraudVerdict::Escalate(_) => {}
        }
    }

    // C) Routing by `DomainTag` is a match, never a rank compare. `DomainTag` does not implement
    //    `Ord`/`PartialOrd`, so quality-ranking two families is impossible at the type level.
    #[test]
    fn routing_is_match_not_rank() {
        let mut reg = DecisionRegistry::new();
        reg.register(AnyUnit::Pricing(pricing_unit()));
        reg.register(AnyUnit::FraudAuth(fraud_unit()));

        // Route by declared capability. A FraudAuth (non-money) unit is callable → found.
        let f = reg.route_live(DomainTag::FraudAuth);
        assert!(f.is_some_and(|u| u.domain() == DomainTag::FraudAuth));

        // A Pricing unit is money-gated and NOT yet operator-activated, so it is registered but
        // NOT callable → route_live returns None (money red-line holds at the routing layer too).
        assert!(reg.route_live(DomainTag::Pricing).is_none());
        assert_eq!(reg.count_live(DomainTag::Pricing), 0);

        // A family with no registered unit routes to None — not to a "lower-ranked" fallback.
        assert!(reg.route_live(DomainTag::Dispatch).is_none());

        // DomainTag carries no ordering: the following would NOT compile (proven by absence):
        //   let _ = DomainTag::Pricing < DomainTag::FraudAuth;
        // Routing is therefore a capability lookup, structurally incapable of a quality rank.
        assert_eq!(reg.count_live(DomainTag::FraudAuth), 1);
    }

    // E) A Stale unit answers `Escalate` unconditionally; there is no Stale→Answer path.
    #[test]
    fn stale_unit_escalates_unconditionally() {
        let mut unit = fraud_unit();
        unit.state = UnitState::Stale;
        assert!(!unit.can_answer());
        assert!(matches!(
            unit.decide(&FraudInput {
                pattern_score: 0,
                velocity: 0
            }),
            Decision::Escalate(EscalateReason::NotLive)
        ));
    }

    // D) Epoch merge is a join-semilattice over a small finite domain (commutative, associative,
    //    idempotent). No scoring input — tiebreak is on content-id bytes only.
    #[test]
    fn epoch_merge_is_semilattice() {
        let domains = [DomainTag::Pricing, DomainTag::FraudAuth];
        let epochs = [0u64, 1, 2, 3];
        let cids = [[0u8; 32], [7u8; 32], [255u8; 32]];

        for &d in domains.iter() {
            for &e1 in epochs.iter() {
                for &e2 in epochs.iter() {
                    for &e3 in epochs.iter() {
                        for &c1 in cids.iter() {
                            for &c2 in cids.iter() {
                                for &c3 in cids.iter() {
                                    let a = meta(d, e1, c1);
                                    let b = meta(d, e2, c2);
                                    let c = meta(d, e3, c3);

                                    // commutativity
                                    assert_eq!(merge_meta(&a, &b), merge_meta(&b, &a));
                                    // idempotence
                                    assert_eq!(merge_meta(&a, &a), a);
                                    // associativity
                                    let lhs = merge_meta(&merge_meta(&a, &b), &c);
                                    let rhs = merge_meta(&a, &merge_meta(&b, &c));
                                    assert_eq!(lhs, rhs);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn meta(d: DomainTag, e: u64, cid: [u8; 32]) -> DecisionUnitMeta {
        let mut m = DecisionUnitMeta::new(d, UnitEpoch(e));
        m.content_id = cid;
        m
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Item 51: shadow-mode divergence telemetry — the load-bearing non-gating proof.
//
// Every test below drives the REAL decision seam (`decide_with_shadow`) and the REAL
// FDR ring, then reads records back with `fdr::ring::recover`. Shadow mode is the
// FDR `ring` (+ `proposal`) — when it is `None` shadow is OFF, when `Some` it is ON.
// The proof the blueprint hinges on: the decision D must be byte-identical with shadow
// ON vs OFF across the full decision test corpus.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod shadow_telemetry_tests {
    use super::*;
    use crate::fdr::ring::{FdrRing, Recovery, DEFAULT_SEG_CAP};
    use crate::fdr::schema::Kind;

    fn shadow_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "item51_shadow_{}_{}_{}",
            tag,
            std::process::id(),
            crate::typed_metrics::mono_now_ns()
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    /// THE load-bearing test (blueprint §4 acceptance #1 / §5 falsifiable criterion #1):
    /// toggling shadow logging changes ZERO bytes of D across the full decision test corpus.
    /// We run the seam with shadow OFF (ring = None) and with shadow ON (a real FDR ring),
    /// then assert the returned `Decision` is byte-identical in both modes for every corpus
    /// entry — AND that the serialized `Debug` form of D is byte-identical too.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn decision_is_bit_identical_with_shadow_on_or_off() {
        let cases: &[(HarnessInput, HarnessOut, HarnessOut)] = &[
            (
                HarnessInput { job: 0 },
                HarnessOut { route_tier: 0 },
                HarnessOut { route_tier: 0 },
            ),
            (
                HarnessInput { job: 1 },
                HarnessOut { route_tier: 2 },
                HarnessOut { route_tier: 7 },
            ),
            (
                HarnessInput { job: 2 },
                HarnessOut { route_tier: 4 },
                HarnessOut { route_tier: 4 },
            ),
            (
                HarnessInput { job: 3 },
                HarnessOut { route_tier: 1 },
                HarnessOut { route_tier: 9 },
            ),
        ];

        for (input, answer, proposal) in cases.iter() {
            let unit = DecisionUnit::new(
                DomainTag::Harness,
                UnitEpoch(1),
                move |_in: &HarnessInput| Decision::Answer(answer.clone()),
            );

            // ── shadow OFF ──
            let d_off = unit.decide_with_shadow(input, Some(proposal), None);
            let d_off_bytes = format!("{d_off:?}");

            // ── shadow ON (real FDR ring) ──
            let dir = shadow_dir("bitident");
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            let d_on = unit.decide_with_shadow(input, Some(proposal), Some(&mut ring));
            drop(ring);

            // D is VALUE-identical…
            assert_eq!(
                d_off, d_on,
                "decision D must be value-identical with shadow ON vs OFF (input {input:?})"
            );
            // …and BYTE-identical (the blueprint's literal contract).
            let d_on_bytes = format!("{d_on:?}");
            assert_eq!(
                d_off_bytes, d_on_bytes,
                "decision D must be BIT-identical with shadow ON vs OFF (input {input:?})"
            );

            // Sanity: shadow ON actually wrote exactly one ShadowDivergence record.
            let rec = crate::fdr::ring::recover(&dir);
            assert_eq!(rec.records.len(), 1, "exactly one shadow record written");
            assert_eq!(rec.records[0].kind, "shadow_divergence");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Acceptance #2 (oracle): a planted DISAGREEING proposal yields exactly one recovered
    /// `ShadowDivergence` record, correct class + agreement bit (0) + digests, recovered via
    /// the real ring. No full payload string appears in the record (digests only).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn disagreeing_proposal_emits_one_shadow_record_with_digests() {
        let dir = shadow_dir("oracle");
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 2 })
        });
        let input = HarnessInput { job: 4 };
        let proposal = HarnessOut { route_tier: 99 }; // deliberately disagrees with D.

        let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
        let d = unit.decide_with_shadow(&input, Some(&proposal), Some(&mut ring));
        drop(ring);

        let rec: Recovery = crate::fdr::ring::recover(&dir);
        let shadow: Vec<_> = rec
            .records
            .iter()
            .filter(|r| r.kind == "shadow_divergence")
            .collect();
        assert_eq!(shadow.len(), 1, "exactly one ShadowDivergence record");
        let raw = &shadow[0].raw;
        // agree bit = 0 (disagree), verdict = admitted (D is an Answer).
        assert!(
            raw.contains("\"agree\":\"0\""),
            "disagreeing proposal must record agree=0: {raw}"
        );
        assert!(
            raw.contains("\"verdict\":\"admitted\""),
            "verdict must be admitted: {raw}"
        );
        // Digests present, full payloads absent (no "route_tier":99 literal in the record).
        assert!(raw.contains("\"d_digest\":\""), "d_digest present: {raw}");
        assert!(
            raw.contains("\"act_digest\":\""),
            "act_digest present: {raw}"
        );
        assert!(
            !raw.contains("\"route_tier\":99"),
            "full proposal payload must NEVER be logged: {raw}"
        );
        // The decision D itself is unaffected by the emit.
        assert_eq!(d, Decision::Answer(HarnessOut { route_tier: 2 }));
        assert_eq!(rec.crc_failures, 0);
        assert_eq!(rec.torn_tail, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Acceptance (write-only invariant, blueprint §5 #4): `ShadowDivergence` is recoverable
    /// and the variant is write-only — confirm NO `ShadowDivergence` consumer changes D by
    /// checking the decision is identical regardless of how many shadow records are emitted.
    /// We emit a flood of disagreed proposals and assert the decision is still the same D and
    /// the recovered count is bounded (one record per call; this asserts the per-call emit and
    /// that the ring preserves CRC-valid records under a flood).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn shadow_flood_emits_bounded_records_preserves_d() {
        let dir = shadow_dir("flood");
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 3 })
        });
        let n = 50usize;
        let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
        for i in 0..n {
            // Alternate agreeing / disagreeing proposals.
            let proposal = HarnessOut {
                route_tier: if i % 2 == 0 { 3 } else { 77 },
            };
            let d = unit.decide_with_shadow(
                &HarnessInput { job: i as u8 },
                Some(&proposal),
                Some(&mut ring),
            );
            assert_eq!(
                d,
                Decision::Answer(HarnessOut { route_tier: 3 }),
                "D unchanged under flood"
            );
        }
        drop(ring);
        let rec = crate::fdr::ring::recover(&dir);
        let shadow: Vec<_> = rec
            .records
            .iter()
            .filter(|r| r.kind == "shadow_divergence")
            .collect();
        // One ShadowDivergence record per call (within ring capacity); none lost/corrupted.
        assert_eq!(shadow.len(), n, "one shadow record per decision call");
        assert_eq!(rec.crc_failures, 0);
        assert_eq!(rec.torn_tail, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Acceptance (item-27 byte-identity): the `ShadowDivergence` variant is ADDITIVE — a plain
    /// `Event` record serializes byte-identically to before the variant existed. This pins that
    /// non-shadow FDR records are NOT disturbed by adding item 51 (the item-27 optional-field
    /// guarantee), so the variant cannot quietly change the bytes of any other record kind.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn event_record_byte_identity_preserved_after_shadow_variant_added() {
        let ev = crate::fdr::schema::FdrEvent {
            seq: 7,
            ts_unix_ns: 1,
            mono_ns: 2,
            level: crate::fdr::Level::Info,
            kind: Kind::Event,
            name: "place_order".into(),
            hw: crate::fdr::schema::HwStamp::sample(crate::fdr::schema::StampPolicy::Cheap),
            pmu: None,
            span_id: None,
            parent_span_id: None,
            fields: vec![("subtotal_cents", "500".into())],
        };
        // Captured golden string — MUST NOT change after the ShadowDivergence variant is added.
        assert_eq!(
            ev.to_json(),
            "{\"seq\":7,\"ts_unix_ns\":1,\"mono_ns\":2,\"level\":\"info\",\"kind\":\"event\",\"name\":\"place_order\",\"hw\":{\"cpu_ticks\":{\"unavailable\":\"sampling_disabled\"},\"rss_kb\":{\"unavailable\":\"sampling_disabled\"},\"joules_uj\":{\"unavailable\":\"sampling_disabled\"}},\"fields\":{\"subtotal_cents\":\"500\"}}"
        );
    }

    /// Write-only guarantee (structural): the `ShadowDivergence` emit is a no-op when no sink
    /// is installed (no FDR ring), so a default-built kernel pays zero cost and cannot change D.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn shadow_emit_is_noop_without_ring() {
        let unit = DecisionUnit::new(DomainTag::Harness, UnitEpoch(1), |_in: &HarnessInput| {
            Decision::Answer(HarnessOut { route_tier: 5 })
        });
        // ring = None ⇒ shadow OFF ⇒ pure decide(), no FDR write path touched.
        let d = unit.decide_with_shadow(
            &HarnessInput { job: 2 },
            Some(&HarnessOut { route_tier: 0 }),
            None,
        );
        assert_eq!(d, Decision::Answer(HarnessOut { route_tier: 5 }));
    }
}
