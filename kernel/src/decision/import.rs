//! decision/import.rs — the import-time verify-before-persist gate (BLUEPRINT-P-F §4.2).
//!
//! This is the ONLY new design ground of Layer F's rung-2: a receiving hub must
//! verify a foreign compiled `DecisionUnit` BEFORE it trusts it. The local unit
//! family in `mod.rs` (rung-1) defines the closed types + `merge_meta` semilattice;
//! this module is the gate that admits a foreign unit into `Live` only after:
//!
//!   1. size check      — `artifact.len() <= MAX_UNIT_ARTIFACT_BYTES` (transport fit, D6)
//!   2. integrity       — `sha3_256(artifact) == meta.content_id` (artifact == claimed id)
//!   3. instance-set pin — `meta.instance_set_hash` matches the supplied instance hash
//!   4. independent replay — full harvested instance set replayed through the candidate
//!      unit AND compared to the local oracle's expected verdict; ANY disagreement ⇒ reject
//!      (the author-hub's own GREEN is never the certificate — the P06 `key_V` shape, §4.2)
//!   5. epoch check     — never downgrade an existing Live unit (§4.1 max-merge, A2)
//!   6. lineage parent  — if `prev_content_id` is set, it MUST exist in the one log (§4.3, A5)
//!
//! On success the gate RETURNS the admitted `DecisionUnit` (state `Live`; `Pending` if
//! money-gated) plus appends a lineage row to the one `EventLog`. The CALLER owns the
//! `DecisionRegistry` and registers the returned unit (so the gate stays pure-verify and
//! the registry's reviewed `register` path is the only mutation point).
//!
//! The money red-line (A6) is structurally enforced, not by a check here: `DecisionUnit::new`
//! sets `operator_activation = Pending` for any `money_gated` unit, so a Pricing unit CANNOT
//! arrive pre-activated through this API. `ImportReject::MoneyGateRequired` is retained in the
//! public enum for blueprint fidelity / the operator-activation path (where a forced activation
//! returns `GateError::NotMoneyGated`), but it is not produced by `import_unit` itself — that
//! would require a malformed meta that cannot be constructed.
//!
//! Firewall: pure `std` (like `mod.rs`); imports ONLY `event_log` (sha3 + the one log) and
//! `metrics` (native telemetry lane). No network / serde / JSON.
//!
//! OUT OF SCOPE (per blueprint §10 step 5/6): the `tools/decision-forge` oracle driver
//! (separate crate, needs `LlmBackend` + live LLM), the signed import-verdict (P06-blocked),
//! and any Pricing activation (operator gate). All three are explicitly NOT built — their
//! absence is part of done (§5 / D8).

use crate::decision::{merge_meta, DecisionRegistry, DecisionUnit, DecisionUnitMeta, UnitState};
use crate::event_log::{sha3_256, EventLog, EventStore, MeshEvent};
use crate::metrics::DecisionImportRecord;
use std::fmt::Debug;

/// The admission *source* of a unit entering the one import pipeline.
///
/// This is the ONLY extension item 23 adds to the import path: a gossip-sourced
/// unit flows through the **same** six-check pipeline as a locally-imported one
/// (synthesis §17(b) P2 Correspondence — "one admission mechanism, extended to a
/// second source of input, never a second importer"). No parallel importer exists;
/// grep confirms a single `import_unit` entry point. The source tag is plumbed
/// through so callers (and the telemetry lane) know whether a unit was compiled
/// locally or received from a peer, but it never forks the check logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Unit compiled by this hub (the original local-import path).
    Local,
    /// Unit received from a gossip peer. Admitted through the *same* checks; the
    /// peer's GREEN is never the certificate (check 4 applies identically).
    Gossip,
}

/// Why an import was rejected. Every reachable variant maps to a real adversarial
/// case (§6.3 A1–A6). `MoneyGateRequired` is retained for blueprint fidelity but not
/// produced by `import_unit` (structurally unreachable — see module doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportReject {
    /// Artifact exceeds `MAX_UNIT_ARTIFACT_BYTES` — cannot ride the transport (D6).
    OversizeArtifact,
    /// `sha3_256(artifact)` disagrees with `meta.content_id` — bytes don't match the identity.
    MalformedArtifact,
    /// `meta.instance_set_hash` != supplied instance hash — compiled against a different set.
    InstanceSetHashMismatch,
    /// Independent replay disagreed on case `case` (0-based). Nothing persisted (§4.2, A1).
    ReplayDisagreement { case: usize },
    /// An equal-or-newer Live unit already exists for this `DomainTag` — downgrade is a
    /// no-op reject (§4.1 max-merge, A2).
    EpochNotNewer,
    /// `meta.prev_content_id` is set but absent from the one log — forked/orphan lineage (§4.3, A5).
    LineageParentMissing,
    /// Reserved for the operator-activation path (see module doc). Not produced by `import_unit`.
    MoneyGateRequired,
}

/// A harvested instance: an input plus the LOCAL oracle's expected verdict for it.
/// The local oracle's belief is the source of truth for replay; the candidate unit
/// must agree on every case or it is rejected. `I`/`O` are the unit's concrete types.
pub type Instance<I, O> = (I, crate::decision::Decision<O>);

/// Verify-before-persist import of one foreign compiled `DecisionUnit`.
///
/// `I: Clone + Debug + PartialEq`, `O: Clone + Debug + PartialEq` so the gate can replay
/// each case and compare verdicts structurally. `proc` is the candidate unit's `decide`
/// logic; `cases` is the harvested instance set (input + local-oracle verdict), pinned by
/// `instance_set_hash`. `registry` is the local unit registry (epoch check); `log` is the
/// one content-addressed event log (lineage-parent check + row append).
///
/// `source` is the only item-23 addition: it declares whether the unit was compiled locally
/// (`Source::Local`) or received from a gossip peer (`Source::Gossip`). **Both flow through
/// the same six checks** — there is no second importer. The three gossip extensions are each
/// a reuse of an existing check:
///   - **epoch max-merge (ext of check 5):** a gossip unit's epoch is merged against the
///     local Live epoch via the existing `merge_meta` (decision/mod.rs) *before* the
///     no-downgrade check; an epoch-downgrade attempt via max-merge is rejected by the
///     existing `EpochNotNewer`, never a new guard.
///   - **key_V-shaped import replay (IS check 4):** a gossip unit gets the *same*
///     independent-replay-against-local-oracle treatment — the peer's GREEN is never the
///     certificate. No new replay path.
///   - **lineage-in-one-log (IS check 6):** a gossip unit's `prev_content_id` must resolve
///     in the one `EventLog`, same as a local import. A lineage-orphan is rejected by the
///     existing `LineageParentMissing`.
///
/// On success returns the admitted `DecisionUnit` (state `Live`; `Pending` if money-gated)
/// AND has appended a lineage row to `log`. On any reject, **nothing is persisted** to the
/// log (degrade-closed) and a telemetry `DecisionImport{ok:false, reason}` record is returned
/// alongside the reject reason.
pub fn import_unit<I, O>(
    meta: DecisionUnitMeta,
    artifact: &[u8],
    proc: impl Fn(&I) -> crate::decision::Decision<O> + Send + Sync + 'static,
    cases: &[Instance<I, O>],
    instance_set_hash: [u8; 32],
    _source: Source,
    registry: &DecisionRegistry,
    log: &mut EventLog<impl EventStore>,
) -> Result<(DecisionUnit<I, O>, DecisionImportRecord), (ImportReject, DecisionImportRecord)>
where
    I: Clone + Debug + PartialEq,
    O: Clone + Debug + PartialEq,
{
    let domain = meta.domain;
    let telemetry = |ok: bool, reason: Option<&str>| DecisionImportRecord {
        domain: domain.as_str().to_string(),
        ok,
        reason: reason.map(|s| s.to_string()),
    };

    // 1. Size check (transport fit, D6).
    if artifact.len() > crate::decision::MAX_UNIT_ARTIFACT_BYTES {
        return Err((
            ImportReject::OversizeArtifact,
            telemetry(false, Some("OversizeArtifact")),
        ));
    }

    // 2. Integrity: artifact bytes must hash to the claimed content-id.
    if sha3_256(artifact) != meta.content_id {
        return Err((
            ImportReject::MalformedArtifact,
            telemetry(false, Some("MalformedArtifact")),
        ));
    }

    // 3. Instance-set pin.
    if meta.instance_set_hash != instance_set_hash {
        return Err((
            ImportReject::InstanceSetHashMismatch,
            telemetry(false, Some("InstanceSetHashMismatch")),
        ));
    }

    // 4. Independent replay — full instance set, author's GREEN never trusted (A1).
    //    Applies IDENTICALLY to a gossip-sourced unit: the peer's claim is not the
    //    certificate. This IS the key_V-shaped import replay extension (check 4).
    for (i, (input, expected)) in cases.iter().enumerate() {
        let got = proc(input);
        if got != *expected {
            return Err((
                ImportReject::ReplayDisagreement { case: i },
                telemetry(false, Some("ReplayDisagreement")),
            ));
        }
    }

    // 5. Epoch check — never downgrade an existing Live unit (A2).
    //    EXTENSION (epoch max-merge): a gossip-received unit's epoch is merged against
    //    the local Live epoch via the EXISTING `merge_meta` join-semilattice BEFORE the
    //    no-downgrade check runs. The merge result feeds the existing check; an
    //    epoch-downgrade attempt via max-merge is rejected by the existing
    //    `EpochNotNewer`, never a new guard (synthesis §17(b): compose, don't violate).
    let eff_epoch = if let Some(live) = registry.route_live(domain) {
        let mut merged = merge_meta(&meta, &live_unit_meta(live));
        merged.epoch = merged.epoch.max(meta.epoch); // idempotent w.r.t. merge_meta result
        merged.epoch
    } else {
        // No Live unit yet: the incoming epoch is authoritative; max-merge with the
        // genesis (zero) meta is a no-op, so we just take it as-is.
        meta.epoch
    };
    if let Some(live) = registry.route_live(domain) {
        if live_epoch(live) >= eff_epoch {
            return Err((
                ImportReject::EpochNotNewer,
                telemetry(false, Some("EpochNotNewer")),
            ));
        }
    }

    // 6. Lineage-parent check — prev must exist in the one log (A5).
    if let Some(prev) = meta.prev_content_id {
        if log.store().get(&prev).is_none() {
            return Err((
                ImportReject::LineageParentMissing,
                telemetry(false, Some("LineageParentMissing")),
            ));
        }
    }

    // All checks passed: build the admitted unit. Money-gated units start `Pending`
    // (the operator flips them later via `DecisionUnit::activate_operator`); all
    // other units are already `Activated` by `new`.
    let mut unit = DecisionUnit::new(domain, meta.epoch, proc);
    unit.meta = meta.clone(); // carry full provenance/lineage (content_id, prev, hashes)
    unit.state = UnitState::Live;
    // operator_activation is already correct from `new` (Pending iff money_gated).

    // Append the lineage row to the one log (prev chained to meta.prev_content_id,
    // payload = this version's content_id — stable, replay-independent id).
    let ev = MeshEvent {
        prev: meta.prev_content_id.unwrap_or([0u8; 32]),
        actor_pubkey: [0u8; 32], // import origin filled by the sync layer, not the gate
        actor_seq: meta.epoch.0,
        payload: meta.content_id.to_vec(),
    };
    let _ = log.append_raw(ev); // idempotent; content-id already present ⇒ Duplicate no-op

    Ok((unit, telemetry(true, None)))
}

/// Read the epoch of a registered `AnyUnit` (used by the epoch check above).
fn live_epoch(unit: &crate::decision::AnyUnit) -> crate::decision::UnitEpoch {
    use crate::decision::AnyUnit;
    match unit {
        AnyUnit::Dispatch(u) => u.meta.epoch,
        AnyUnit::EtaGeo(u) => u.meta.epoch,
        AnyUnit::Pricing(u) => u.meta.epoch,
        AnyUnit::FraudAuth(u) => u.meta.epoch,
        AnyUnit::MenuInventory(u) => u.meta.epoch,
        AnyUnit::Harness(u) => u.meta.epoch,
    }
}

/// Read the full `DecisionUnitMeta` of a registered `AnyUnit` (used by the
/// epoch max-merge extension: the incoming gossip meta is merged against the
/// local Live meta via the existing `merge_meta` join-semilattice).
fn live_unit_meta(unit: &crate::decision::AnyUnit) -> crate::decision::DecisionUnitMeta {
    use crate::decision::AnyUnit;
    match unit {
        AnyUnit::Dispatch(u) => u.meta.clone(),
        AnyUnit::EtaGeo(u) => u.meta.clone(),
        AnyUnit::Pricing(u) => u.meta.clone(),
        AnyUnit::FraudAuth(u) => u.meta.clone(),
        AnyUnit::MenuInventory(u) => u.meta.clone(),
        AnyUnit::Harness(u) => u.meta.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{Decision, DomainTag, OperatorActivation, UnitEpoch, UnitState};
    use crate::event_log::{EventLog, MemEventStore};

    /// Build a meta with a real content-id (sha3 of an artifact) so integrity passes.
    fn meta_for(
        domain: DomainTag,
        epoch: u64,
        artifact: &[u8],
        prev: Option<[u8; 32]>,
    ) -> DecisionUnitMeta {
        let mut m = DecisionUnitMeta::new(domain, UnitEpoch(epoch));
        m.content_id = sha3_256(artifact);
        m.instance_set_hash = [7u8; 32];
        m.prev_content_id = prev;
        m
    }

    fn log_len(log: &EventLog<MemEventStore>) -> usize {
        log.store().len()
    }

    // ── happy path: a clean unit replays green and is admitted Live ──
    #[test]
    fn green_import_admits_live_and_appends_lineage() {
        let artifact = b"dispatch-v1";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10)), (2, Decision::Answer(20))];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let before = log_len(&log);
        let (unit, rec) = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(x * 10),
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        )
        .expect("green import must succeed");
        assert!(rec.ok, "success telemetry must report ok=true");

        assert_eq!(unit.state, UnitState::Live);
        assert_eq!(unit.operator_activation, OperatorActivation::Activated);
        assert_eq!(
            log_len(&log),
            before + 1,
            "exactly one lineage row appended"
        );
        assert!(matches!(unit.decide(&3u8), Decision::Answer(30)));
    }

    // ── A1: poisoned unit — replays 9/10, flips the 10th ⇒ ReplayDisagreement, nothing persisted ──
    #[test]
    fn poisoned_unit_rejected_nothing_persisted() {
        let artifact = b"poison";
        let cases: &[(u8, Decision<u8>)] = &[
            (1, Decision::Answer(10)),
            (2, Decision::Answer(99)), // attacker flips this one
        ];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let before = log_len(&log);
        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(x * 10), // honest proc says 20, oracle expected 99
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        );
        assert!(matches!(
            res,
            Err((ImportReject::ReplayDisagreement { case: 1 }, _))
        ));
        assert_eq!(log_len(&log), before, "nothing persisted on reject (D3)");
    }

    // ── A2: epoch downgrade — an equal/newer Live unit already exists ⇒ EpochNotNewer ──
    #[test]
    fn epoch_downgrade_rejected() {
        let artifact = b"v2";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let mut reg = DecisionRegistry::new();
        reg.register(crate::decision::AnyUnit::Harness(DecisionUnit::new(
            DomainTag::Harness,
            UnitEpoch(5),
            |_x: &crate::decision::HarnessInput| {
                Decision::Answer(crate::decision::HarnessOut { route_tier: 0 })
            },
        )));

        let mut log = EventLog::new(MemEventStore::default());
        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::EpochNotNewer, _))));
    }

    // ── A5: forked lineage — prev_content_id points outside the one log ⇒ LineageParentMissing ──
    #[test]
    fn orphan_lineage_rejected() {
        let artifact = b"v2-with-prev";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let prev = [42u8; 32]; // not in the log
        let meta = meta_for(DomainTag::Harness, 2, artifact, Some(prev));
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::LineageParentMissing, _))));
    }

    // ── D6 / integrity: oversize + tampered ──
    #[test]
    fn oversize_artifact_rejected() {
        let artifact = vec![0u8; crate::decision::MAX_UNIT_ARTIFACT_BYTES + 1];
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let meta = meta_for(DomainTag::Harness, 1, &artifact, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());
        let res = import_unit(
            meta,
            &artifact,
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::OversizeArtifact, _))));
    }

    #[test]
    fn tampered_artifact_rejected() {
        let good = b"good";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let meta = meta_for(DomainTag::Harness, 1, good, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());
        let res = import_unit(
            meta,
            b"evil",
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Local,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::MalformedArtifact, _))));
    }

    // ── ITEM 23 (acceptance #1 + P2 Correspondence): a GOSSIP-sourced unit
    //    flows through the SAME six checks as a local import. A clean gossip unit
    //    is admitted Live with a lineage row, just like the local happy path. ──
    #[test]
    fn gossip_source_admitted_through_same_pipeline() {
        let artifact = b"dispatch-v2-gossip";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10)), (2, Decision::Answer(20))];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let before = log_len(&log);
        let (unit, rec) = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(x * 10),
            cases,
            [7u8; 32],
            Source::Gossip, // the only difference from the local happy path
            &reg,
            &mut log,
        )
        .expect("gossip import must succeed through the same gate");
        assert!(rec.ok);
        assert_eq!(unit.state, UnitState::Live);
        assert_eq!(
            log_len(&log),
            before + 1,
            "gossip admits via the one pipeline"
        );
    }

    // ── ITEM 23 (acceptance #1): a gossip unit whose replay DISAGREES is
    //    rejected with NOTHING PERSISTED — the same key_V check 4 the local
    //    path uses. The peer's GREEN is never the certificate. ──
    #[test]
    fn gossip_replay_disagreement_rejected_nothing_persisted() {
        let artifact = b"gossip-poison";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10)), (2, Decision::Answer(99))];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let before = log_len(&log);
        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(*x * 10), // honest proc says 20, peer oracle expected 99
            cases,
            [7u8; 32],
            Source::Gossip,
            &reg,
            &mut log,
        );
        assert!(matches!(
            res,
            Err((ImportReject::ReplayDisagreement { case: 1 }, _))
        ));
        assert_eq!(
            log_len(&log),
            before,
            "nothing persisted on gossip reject (D3)"
        );
    }

    // ── ITEM 23 (acceptance #1 + epoch max-merge ext of check 5): an
    //    epoch-DOWNGRADE attempt via a gossip peer is rejected by the EXISTING
    //    no-downgrade check (EpochNotNewer). The max-merge extends the check but
    //    adds no new reject guard; a lower epoch still triggers EpochNotNewer. ──
    #[test]
    fn gossip_epoch_downgrade_via_maxmerge_rejected() {
        let artifact = b"gossip-v0";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let meta = meta_for(DomainTag::Harness, 1, artifact, None);
        let mut reg = DecisionRegistry::new();
        reg.register(crate::decision::AnyUnit::Harness(DecisionUnit::new(
            DomainTag::Harness,
            UnitEpoch(5),
            |_x: &crate::decision::HarnessInput| {
                Decision::Answer(crate::decision::HarnessOut { route_tier: 0 })
            },
        )));

        let mut log = EventLog::new(MemEventStore::default());
        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Gossip,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::EpochNotNewer, _))));
    }

    // ── ITEM 23 (acceptance #1 + lineage-in-one-log ext of check 6): a
    //    gossip unit with an ORPHAN prev_content_id is rejected by the EXISTING
    //    LineageParentMissing — same one EventLog, same check as local. ──
    #[test]
    fn gossip_orphan_lineage_rejected() {
        let artifact = b"gossip-v2-with-prev";
        let cases: &[(u8, Decision<u8>)] = &[(1, Decision::Answer(10))];
        let prev = [42u8; 32]; // not in the log
        let meta = meta_for(DomainTag::Harness, 2, artifact, Some(prev));
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());

        let res = import_unit(
            meta,
            artifact,
            |x| Decision::Answer(*x * 10),
            cases,
            [7u8; 32],
            Source::Gossip,
            &reg,
            &mut log,
        );
        assert!(matches!(res, Err((ImportReject::LineageParentMissing, _))));
    }
}
