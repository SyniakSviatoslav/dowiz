//! ports/agent/admission.rs — the B1 admission path (§2.2) + the SH-1 pre-crypto
//! guards + the SH-3 Poly-Network invariant, all fail-closed.
//!
//! Compile firewall (mirrors `ports/llm.rs:3-7`): ZERO network / HTTP / JSON / serde.
//!
//! `admit(...)` runs, in this fixed order:
//!   0a. SH-1 Guard A — pre-crypto admission-attempt limiter (reused `TokenBucket`).
//!   0b. SH-1 Guard B — hard `MAX_VERIFY_CHAIN_LINKS` length cap (zero crypto).
//!   1.  strict `AgentManifest` TLV decode (free-form dies here, before any gate).
//!   2.  `AdmissionGate::check` — the exact bebop2 `HybridGate::check` sequence.
//!   3.  identity binding: recomputed `NodeId` must equal the claimed one.
//!   4.  sandbox tier via `register_adapter` (no-KVM ⇒ no native, never a downgrade).
//!   5.  budget envelope: mint a dedicated per-agent `TokenBucket` (F2 rate-limit).
//!   6.  depth grant (F10): `min(depth_request, DEFAULT_MAX_AGENT_DEPTH)`, 0 if !delegate.
//!   7.  record: append an `AgentAdmitted` `MeshEvent` via `commit_after_decide`
//!       (NOT the drift-gate variant — admission is a trust transition, not a flow one).
//!
//! Roster/revocation are taken as SHARED (`&`) references — the SH-3 layer-3 structural
//! guard: no capability-bearing input can obtain the `&mut` the anchor/revocation
//! mutators require.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::event_log::{AppendOutcome, CommitError, EventLog, EventStore, MeshEvent};
use crate::isolation::microvm::{register_adapter, SandboxTier};
use crate::token_bucket::TokenBucket;

use super::cap::{
    pq_key_id, revocation_hash, verify_chain, AnchorRoster, ChainError, Delegation, HybridPolicy,
    NodeId, RevocationSet, SignatureVerifier, SignedFrame,
};
use super::manifest::{AgentCaps, AgentManifest, ManifestParseError};
use super::scope::{Action, RedLinePolicy, Resource, Scope};

/// F10 global sub-agent depth cap (§2.2 step 6 / §2.4). A per-manifest grant may only
/// narrow this.
///
/// ⚠ CANON-DIFF (flagged, NOT resolved here): P02-O8 proposed a default of **8** for a
/// *different* anchor. B1's own stated default is **3** (used here). Reconciling the two
/// is a lead-agent canon ruling, not this unit's call.
pub const DEFAULT_MAX_AGENT_DEPTH: u8 = 3;

/// SH-1 Guard B: the hard cap on delegation-chain links checked BEFORE the first
/// signature verification. Must be ≥ `DEFAULT_MAX_AGENT_DEPTH` (invoke-depth ⊆
/// chain-length). Operator-config overridable via [`Admitter::with_max_chain_links`].
pub const MAX_VERIFY_CHAIN_LINKS: usize = 16;

/// Wasmtime fuel per one token-bucket unit (§2.2). **PLACEHOLDER — pending B4.**
///
/// This constant converts billing units → CPU-instruction fuel. Its value is NOT yet
/// ledger-grounded: B4's criterion bench (which pins it) has not landed. `100_000` is
/// the blueprint's initial placeholder; treat it as provisional and re-pin (kernel↔
/// adapter, mirror-pinned) once the bench exists. See `agent-adapters/src/fuel.rs`.
pub const FUEL_PER_UNIT: u64 = 100_000;

/// Default prepaid tranche size (units) for the fuel loop (§2.2).
pub const TRANCHE_UNITS: u64 = 8;

/// Nonce-replay ledger bound (mirrors bebop2 `MAX_SEEN_NONCES`).
const MAX_SEEN_NONCES: usize = 1 << 20;

/// The exhaustive enumeration contract of the `(Resource, Action)` scopes B1 introduces.
/// The SH-3 Poly-Network test iterates this array — adding a B1 scope without listing it
/// here makes the test fail to compile (the array IS the enumeration contract).
pub const B1_NEW_SCOPES: [(Resource, Action); 2] = [
    (Resource::AgentBridge, Action::AdmitAgent),
    (Resource::AgentBridge, Action::InvokeAgent),
];

/// Typed admission failure. Every reject is fail-closed: no capability, bucket, sandbox,
/// or event exists afterward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// SH-1 Guard A: the pre-crypto attempt limiter dropped this frame (never reached
    /// parse or the gate).
    AdmissionThrottled,
    /// SH-1 Guard B: the delegation chain exceeds `MAX_VERIFY_CHAIN_LINKS` (rejected
    /// before any `verify_signature`).
    ChainTooLong,
    /// Manifest TLV decode failed (free-form/unknown/short/weak-policy) — before the gate.
    ManifestParseError(ManifestParseError),
    /// The admission frame's capability scope is not `(AgentBridge, AdmitAgent)`.
    WrongAdmissionScope,
    /// Capability expired (`now >= expiry`).
    Expired,
    /// No anchor-rooted delegation chain (self-issue).
    UnknownIssuer,
    /// A signature (chain link, classical, or PQ) did not verify.
    BadSignature,
    /// Red-line scope denied by the armed deny-by-default policy.
    RedLineViolation,
    /// Subject key / cap hash / PQ key id is revoked.
    Revoked,
    /// PQ leg missing or invalid under `RequireBoth`.
    PqVerifyFailed,
    /// Nonce replay.
    NonceRejected,
    /// Chain attenuation/tail-binding/effect-subset violated.
    ScopeViolation,
    /// Item 54 Sentinel — a critical live authority struct (`AnchorRoster` /
    /// `RevocationSet`) failed its read-time CRC integrity check. The admission is
    /// REFUSED (fail-closed; deny-closed) and exactly one fsynced FDR `Alarm` has been
    /// emitted naming the corrupted struct. A flipped trust-root key or revocation bit is
    /// hardware-fault evidence; certify no agent.
    SentinelTripped,
    /// The nonce ledger lock was poisoned.
    LockPoisoned,
    /// Recomputed `NodeId` (or frame↔manifest key mismatch) ≠ claimed identity.
    IdentityMismatch,
    /// Sandbox tier refused (e.g. native-process without KVM) — never downgraded.
    AdapterRejected,
    /// Durable append failed (accepted-but-not-durable pole).
    StoreFault(String),
}

impl From<ManifestParseError> for AdmissionError {
    fn from(e: ManifestParseError) -> Self {
        AdmissionError::ManifestParseError(e)
    }
}

fn map_chain_err(e: ChainError) -> AdmissionError {
    match e {
        ChainError::UnknownIssuer => AdmissionError::UnknownIssuer,
        ChainError::BadSignature => AdmissionError::BadSignature,
        ChainError::Expired => AdmissionError::Expired,
        ChainError::ScopeViolation => AdmissionError::ScopeViolation,
        // Item 54 Sentinel: a live-struct integrity fault refuses the admission
        // (fail-closed; deny-closed). The FDR Alarm was already emitted inside
        // `verify_chain` before this error propagated.
        ChainError::IntegrityFault => AdmissionError::SentinelTripped,
    }
}

/// The admission gate seam. Production injects the real bebop2 `HybridGate::check`;
/// [`ReferenceHybridGate`] is the in-tree faithful reproduction of its exact sequence.
pub trait AdmissionGate {
    /// Run the verification sequence over the frame. `Ok(())` iff every leg passes.
    fn check(
        &self,
        frame: &SignedFrame,
        roster: &AnchorRoster,
        chain: &[Delegation],
        revocations: &RevocationSet,
        now: u64,
    ) -> Result<(), AdmissionError>;
}

/// The in-tree reproduction of bebop2 `HybridGate::check` (`hybrid_gate.rs:124-209`),
/// verbatim ordering: freshness → `verify_chain` → armed red-line deny-by-default →
/// revocation (classical key, cap hash, pq_key_id) → classical verify → PQ verify under
/// `RequireBoth` → verify-then-record nonce (bounded).
#[derive(Debug)]
pub struct ReferenceHybridGate<V: SignatureVerifier> {
    policy: HybridPolicy,
    redline: Option<RedLinePolicy>,
    verifier: V,
    seen: Mutex<std::collections::HashSet<[u8; 8]>>,
    check_count: AtomicUsize,
}

impl<V: SignatureVerifier> ReferenceHybridGate<V> {
    /// Build a gate with the red-line policy ARMED (production posture).
    pub fn new_redlined(policy: HybridPolicy, redline: RedLinePolicy, verifier: V) -> Self {
        ReferenceHybridGate {
            policy,
            redline: Some(redline),
            verifier,
            seen: Mutex::new(std::collections::HashSet::new()),
            check_count: AtomicUsize::new(0),
        }
    }

    /// Build a gate with the red-line gate UNARMED (test/isolation).
    pub fn new(policy: HybridPolicy, verifier: V) -> Self {
        ReferenceHybridGate {
            policy,
            redline: None,
            verifier,
            seen: Mutex::new(std::collections::HashSet::new()),
            check_count: AtomicUsize::new(0),
        }
    }

    /// How many times `check` was entered (crit 2 / crit 11: "gate call-count").
    pub fn check_count(&self) -> usize {
        self.check_count.load(Ordering::SeqCst)
    }
}

impl<V: SignatureVerifier> AdmissionGate for ReferenceHybridGate<V> {
    fn check(
        &self,
        frame: &SignedFrame,
        roster: &AnchorRoster,
        chain: &[Delegation],
        revocations: &RevocationSet,
        now: u64,
    ) -> Result<(), AdmissionError> {
        self.check_count.fetch_add(1, Ordering::SeqCst);

        // 1. freshness (cheap, fail-closed) — read the nonce but do NOT record it yet.
        if !frame.capability.is_fresh(now) {
            return Err(AdmissionError::Expired);
        }
        let nonce = frame.capability.nonce;

        // Item 54 Sentinel — read-time integrity check over the live `RevocationSet` at this
        // authority-use transition point. `verify_chain` (step 2) already sentinels the
        // `AnchorRoster`; here we cover the revocation set, whose corruption silently
        // UN-revokes a revoked key. On mismatch: exactly one fsynced FDR `Alarm` then deny.
        if let Err(c) = crate::ports::agent::sentinel::verify_candidate(None, Some(revocations)) {
            crate::ports::agent::sentinel::safe_state_on_corruption(&c);
            return Err(AdmissionError::SentinelTripped);
        }

        // 2. authorization root-of-trust: anchor-rooted UCAN-subset chain.
        verify_chain(&self.verifier, roster, chain, &frame.capability, now)
            .map_err(map_chain_err)?;

        // 3. red-line (armed) — after chain (never burn a nonce on an unauthenticated
        //    frame), before revocation. Deny-by-default.
        if let Some(rl) = &self.redline {
            if rl.check(&frame.capability.scope).is_err() {
                return Err(AdmissionError::RedLineViolation);
            }
        }

        // 4. revocation (classical key, cap hash, pq key id).
        if revocations.is_revoked_key(&frame.capability.subject_key)
            || revocations.is_revoked_capability(&revocation_hash(&frame.capability))
        {
            return Err(AdmissionError::Revoked);
        }
        if let Some(pq) = &frame.capability.subject_key_pq {
            if revocations.is_revoked_key(&pq_key_id(pq)) {
                return Err(AdmissionError::Revoked);
            }
        }

        // 5. classical leg — always real, never relaxed.
        if !frame.verify_classical(&self.verifier) {
            return Err(AdmissionError::BadSignature);
        }

        // 6. PQ leg under RequireBoth — a real ML-DSA-65 verification.
        match self.policy {
            HybridPolicy::RequireBoth => {
                if !frame.verify_pq(&self.verifier) {
                    return Err(AdmissionError::PqVerifyFailed);
                }
            }
        }

        // 7. verify-then-record: only now commit the nonce (H2 ordering). Bounded.
        {
            let mut seen = self.seen.lock().map_err(|_| AdmissionError::LockPoisoned)?;
            if !seen.insert(nonce) {
                return Err(AdmissionError::NonceRejected);
            }
            if seen.len() > MAX_SEEN_NONCES {
                let keep: std::collections::HashSet<[u8; 8]> =
                    seen.iter().take(MAX_SEEN_NONCES / 2).copied().collect();
                *seen = keep;
            }
        }
        Ok(())
    }
}

/// SH-1 Guard A: the pre-cryptographic admission-attempt limiter, built ENTIRELY from
/// the existing `TokenBucket` (no new primitive). A mandatory global bucket bounds total
/// pre-crypto verify work node-wide regardless of source cardinality/spoofing; an
/// optional fixed-size sharded array adds bounded-memory per-source fairness (never a
/// per-source map an attacker could grow).
pub struct AdmissionLimiter {
    global: TokenBucket,
    shards: Vec<TokenBucket>,
}

impl AdmissionLimiter {
    /// `global_*` sizes the mandatory node-wide ceiling. `shard_count == 0` disables the
    /// fairness refinement (global-only). Sizes are operator config (M5), never attacker.
    pub fn new(
        global_capacity: u64,
        global_refill: f64,
        shard_count: usize,
        shard_capacity: u64,
        shard_refill: f64,
    ) -> Self {
        let shards = (0..shard_count)
            .map(|_| TokenBucket::new(shard_capacity as f64, shard_refill))
            .collect();
        AdmissionLimiter {
            global: TokenBucket::new(global_capacity as f64, global_refill),
            shards,
        }
    }

    /// One O(1) integer decrement (+ optional shard decrement). `false` ⇒ drop the frame
    /// pre-crypto. Global is the ceiling; a shard denial is per-source fairness only.
    pub fn try_admit(&self, conn_id: u64) -> bool {
        if !self.global.try_acquire(1.0) {
            return false;
        }
        if self.shards.is_empty() {
            return true;
        }
        let idx = (conn_id as usize) % self.shards.len();
        self.shards[idx].try_acquire(1.0)
    }
}

/// The durable outcome of a successful admission (§2.2 step 7). `bucket` is the minted
/// per-agent budget envelope (F2 rate-limit); `tier` is the assigned sandbox.
#[derive(Clone)]
pub struct AdmissionRecord {
    /// `SHA3-256(canonical manifest bytes)` — the content-addressed identity.
    pub content_id: [u8; 32],
    /// The WORM event id of the `AgentAdmitted` record.
    pub event_id: [u8; 32],
    /// The admitted agent's `NodeId`.
    pub node_id: NodeId,
    /// Granted bucket capacity (min of request and per-peer cap).
    pub granted_capacity: u64,
    /// Granted refill (milli-units/sec) — the integer wire form.
    pub granted_refill_milli: u64,
    /// Assigned sandbox tier.
    pub tier: SandboxTier,
    /// Granted sub-agent depth (F10).
    pub granted_depth: u8,
    /// The admitted capability bitmap.
    pub caps: AgentCaps,
    /// Expiry tick (from the manifest).
    pub expiry: u64,
    /// The minted per-agent budget envelope.
    pub bucket: Arc<TokenBucket>,
}

impl AdmissionRecord {
    /// Whether the granted ENVELOPE (not the bucket state) matches another record — used
    /// for the semantic re-admission short-circuit.
    fn same_envelope(
        &self,
        capacity: u64,
        refill_milli: u64,
        tier: SandboxTier,
        depth: u8,
    ) -> bool {
        self.granted_capacity == capacity
            && self.granted_refill_milli == refill_milli
            && self.tier == tier
            && self.granted_depth == depth
    }
}

/// The stateful admitter: holds the gate, the SH-1 limiter, operator caps, the admitting
/// node's identity + monotonic seq, and the live admitted-set (content-id → record) that
/// gives semantic-re-admission idempotency ABOVE `commit_after_decide`'s byte-`Duplicate`.
pub struct Admitter<G: AdmissionGate> {
    gate: G,
    limiter: AdmissionLimiter,
    per_peer_budget_cap: u64,
    max_chain_links: usize,
    actor_pubkey: [u8; 32],
    actor_seq: u64,
    admitted: HashMap<[u8; 32], AdmissionRecord>,
}

impl<G: AdmissionGate> Admitter<G> {
    /// Build an admitter. `actor_pubkey` is the ADMITTING NODE's identity (the local
    /// operator making the decision — the agent is the subject, the node is the actor).
    pub fn new(
        gate: G,
        limiter: AdmissionLimiter,
        per_peer_budget_cap: u64,
        actor_pubkey: [u8; 32],
    ) -> Self {
        Admitter {
            gate,
            limiter,
            per_peer_budget_cap,
            max_chain_links: MAX_VERIFY_CHAIN_LINKS,
            actor_pubkey,
            actor_seq: 0,
            admitted: HashMap::new(),
        }
    }

    /// Override the SH-1 Guard B chain-length cap (operator config, M5).
    pub fn with_max_chain_links(mut self, n: usize) -> Self {
        self.max_chain_links = n;
        self
    }

    /// Borrow the gate (tests inspect its `check_count`).
    pub fn gate(&self) -> &G {
        &self.gate
    }

    /// The live admitted record for a manifest content-id, if any.
    pub fn admitted(&self, content_id: &[u8; 32]) -> Option<&AdmissionRecord> {
        self.admitted.get(content_id)
    }

    /// Admit an agent manifest frame. See the module doc for the fixed step order.
    ///
    /// `roster` / `revocations` are SHARED references — the SH-3 layer-3 structural
    /// guard. `conn_id` is the MESH-09 coarse peer key for SH-1 Guard A.
    #[allow(clippy::too_many_arguments)]
    pub fn admit<S: EventStore>(
        &mut self,
        frame: &SignedFrame,
        roster: &AnchorRoster,
        chain: &[Delegation],
        revocations: &RevocationSet,
        event_log: &mut EventLog<S>,
        conn_id: u64,
        now: u64,
    ) -> Result<AdmissionRecord, AdmissionError> {
        // 0a. SH-1 Guard A — pre-crypto attempt limiter. FIRST thing; O(1), no crypto.
        if !self.limiter.try_admit(conn_id) {
            return Err(AdmissionError::AdmissionThrottled);
        }

        // 0b. SH-1 Guard B — hard chain-length cap BEFORE any verify_signature.
        if chain.len() > self.max_chain_links {
            return Err(AdmissionError::ChainTooLong);
        }

        // 1. strict manifest decode (free-form dies here, before the gate).
        let manifest = AgentManifest::decode(&frame.payload)?;

        // The admission frame's capability must authorize admission itself.
        let admit_scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        if frame.capability.scope != admit_scope {
            return Err(AdmissionError::WrongAdmissionScope);
        }

        // 2. the exact HybridGate::check sequence.
        self.gate.check(frame, roster, chain, revocations, now)?;

        // 3. identity binding: recomputed NodeId == claimed, and frame keys == manifest keys.
        let recomputed = NodeId::from_keys(&manifest.subject_key_pq, &manifest.subject_key);
        if recomputed.0 != manifest.agent_node_id
            || frame.capability.subject_key != manifest.subject_key
            || frame.capability.subject_key_pq.as_deref()
                != Some(manifest.subject_key_pq.as_slice())
        {
            return Err(AdmissionError::IdentityMismatch);
        }

        // 4. sandbox tier via register_adapter (no unsandboxed fallback).
        let exec = manifest.execution_model;
        register_adapter(exec.as_adapter_str()).map_err(|_| AdmissionError::AdapterRejected)?;
        let tier = match exec {
            super::manifest::ExecutionModel::WasmComponent => SandboxTier::WasmComponent,
            super::manifest::ExecutionModel::NativeProcess => SandboxTier::NativeProcessRequiresKvm,
        };

        // 5. budget envelope (F2 rate-limit): min(request, per-peer cap).
        let granted_capacity = manifest
            .budget_request
            .capacity
            .min(self.per_peer_budget_cap);
        let granted_refill_milli = manifest.budget_request.refill_milli_units_per_sec;
        let refill = granted_refill_milli as f64 / 1000.0;

        // 6. depth grant (F10): 0 when !delegate, else min(request, DEFAULT cap).
        let granted_depth = if manifest.agent_caps.delegate {
            manifest.depth_request.min(DEFAULT_MAX_AGENT_DEPTH)
        } else {
            0
        };

        // 6.5. semantic re-admission idempotency (ABOVE the byte-Duplicate layer): if the
        // content-id already maps to a LIVE record with an identical granted envelope,
        // yield it — no new event (blueprint Event-Driven §Idempotency layer ii).
        let content_id = manifest.content_id();
        if let Some(existing) = self.admitted.get(&content_id) {
            let live = existing.expiry > now;
            if live
                && existing.same_envelope(
                    granted_capacity,
                    granted_refill_milli,
                    tier,
                    granted_depth,
                )
            {
                return Ok(existing.clone());
            }
        }

        // 7. record the admission decision as a WORM MeshEvent via commit_after_decide
        // (NOT the drift-gate variant — admission is a trust transition, not flow).
        let payload = encode_admission_event(
            &content_id,
            granted_capacity,
            granted_refill_milli,
            tier,
            granted_depth,
        );
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: self.actor_pubkey,
            actor_seq: self.actor_seq,
            payload,
        };
        let (outcome, _) = event_log
            .commit_after_decide(ev, |_| Ok::<(), std::convert::Infallible>(()))
            .map_err(|e| match e {
                CommitError::Store(s) => AdmissionError::StoreFault(format!("{s:?}")),
                CommitError::Rejected(r) => AdmissionError::StoreFault(r.0),
            })?;
        let event_id = match outcome {
            AppendOutcome::Committed(id) => {
                self.actor_seq += 1;
                id
            }
            AppendOutcome::Duplicate(id) => id,
        };

        let record = AdmissionRecord {
            content_id,
            event_id,
            node_id: NodeId(manifest.agent_node_id),
            granted_capacity,
            granted_refill_milli,
            tier,
            granted_depth,
            caps: manifest.agent_caps,
            expiry: manifest.expiry,
            bucket: Arc::new(TokenBucket::new(granted_capacity as f64, refill)),
        };
        self.admitted.insert(content_id, record.clone());
        Ok(record)
    }
}

/// Canonical TLV of the `AgentAdmitted` event payload (§Event-Driven): `kind(0x01) ||
/// content_id(32) || capacity(8 LE) || refill_milli(8 LE) || tier(1) || depth(1)`.
fn encode_admission_event(
    content_id: &[u8; 32],
    capacity: u64,
    refill_milli: u64,
    tier: SandboxTier,
    depth: u8,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 32 + 8 + 8 + 1 + 1);
    out.push(0x01); // kind = AgentAdmitted
    out.extend_from_slice(content_id);
    out.extend_from_slice(&capacity.to_le_bytes());
    out.extend_from_slice(&refill_milli.to_le_bytes());
    out.push(match tier {
        SandboxTier::WasmComponent => 0x01,
        SandboxTier::NativeProcessRequiresKvm => 0x02,
    });
    out.push(depth);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::MemEventStore;
    use crate::ports::agent::cap::{Capability, Delegation, RefSigner, ML_DSA_65_PK_LEN};
    use crate::ports::agent::manifest::{
        BudgetRequest, CostDenomination, ExecutionModel, QuirksProfile, ResourceNeed,
        ValidationPolicy,
    };
    use std::sync::atomic::AtomicUsize;

    // ── test fixtures ──────────────────────────────────────────────────────────

    /// A verifier that counts `verify_classical` + `verify_pq` calls (shared counter),
    /// wrapping the real `RefSigner`. Proves crit 12 spends zero crypto.
    struct CountingVerifier {
        inner: RefSigner,
        verifies: Arc<AtomicUsize>,
    }
    impl SignatureVerifier for CountingVerifier {
        fn classical_public(&self, s: &[u8; 32]) -> [u8; 32] {
            self.inner.classical_public(s)
        }
        fn sign_classical(&self, s: &[u8; 32], m: &[u8]) -> Vec<u8> {
            self.inner.sign_classical(s, m)
        }
        fn verify_classical(&self, p: &[u8; 32], m: &[u8], sig: &[u8]) -> bool {
            self.verifies.fetch_add(1, Ordering::SeqCst);
            self.inner.verify_classical(p, m, sig)
        }
        fn pq_public(&self, s: &[u8; 32]) -> Vec<u8> {
            self.inner.pq_public(s)
        }
        fn sign_pq(&self, s: &[u8; 32], m: &[u8]) -> Vec<u8> {
            self.inner.sign_pq(s, m)
        }
        fn verify_pq(&self, p: &[u8], m: &[u8], sig: &[u8]) -> bool {
            self.verifies.fetch_add(1, Ordering::SeqCst);
            self.inner.verify_pq(p, m, sig)
        }
    }

    /// A fully-formed, admittable manifest for the given execution model + caps.
    fn build_manifest(
        v: &RefSigner,
        cls_secret: &[u8; 32],
        pq_secret: &[u8; 32],
        exec: ExecutionModel,
        delegate: bool,
        depth_request: u8,
    ) -> AgentManifest {
        let cls = v.classical_public(cls_secret);
        let pq = v.pq_public(pq_secret);
        let node_id = NodeId::from_keys(&pq, &cls);
        AgentManifest {
            agent_node_id: node_id.0,
            subject_key: cls,
            subject_key_pq: pq,
            agent_caps: AgentCaps {
                invoke_tool: true,
                read_resource: true,
                delegate,
                ..Default::default()
            },
            action_scopes: Scope::single(Resource::Menu, Action::Read),
            resource_needs: vec![ResourceNeed::WallClock],
            cost_denomination: CostDenomination::TokenBucketUnits,
            budget_request: BudgetRequest {
                capacity: 4096,
                refill_milli_units_per_sec: 8000,
            },
            validation_policy: ValidationPolicy::RequireBoth,
            execution_model: exec,
            config_axes: vec![(0x01, 0)],
            depth_request,
            quirks_profile: QuirksProfile::McpServer,
            nonce: [5u8; 8],
            expiry: 9999,
        }
    }

    /// Build a fully valid admission frame + its anchor-rooted chain + roster.
    /// `scope` is the capability scope (normally `(AgentBridge, AdmitAgent)`).
    fn valid_frame(
        v: &RefSigner,
        cls_secret: &[u8; 32],
        pq_secret: &[u8; 32],
        anchor_secret: &[u8; 32],
        manifest: &AgentManifest,
        scope: Scope,
        nonce: [u8; 8],
    ) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
        let cls = v.classical_public(cls_secret);
        let pq = v.pq_public(pq_secret);
        let anchor = v.classical_public(anchor_secret);
        let cap = Capability::new_hybrid(cls, pq, scope.clone(), nonce, 9999);
        let mut frame = SignedFrame::new(cap, manifest.canonical_bytes());
        frame.sign_classical(v, cls_secret);
        frame.sign_pq(v, pq_secret);
        let link = Delegation::sign(
            v,
            anchor,
            cls,
            scope.clone(),
            scope,
            9999,
            nonce,
            anchor_secret,
        );
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor);
        (frame, roster, vec![link])
    }

    fn open_gate() -> ReferenceHybridGate<RefSigner> {
        ReferenceHybridGate::new_redlined(
            HybridPolicy::RequireBoth,
            RedLinePolicy::DenyByDefault,
            RefSigner,
        )
    }

    /// A generous limiter that never throttles (for the non-SH-1 tests).
    fn open_limiter() -> AdmissionLimiter {
        AdmissionLimiter::new(1_000_000, 0.0, 0, 0, 0.0)
    }

    fn admitter() -> Admitter<ReferenceHybridGate<RefSigner>> {
        Admitter::new(open_gate(), open_limiter(), 1_000_000, [0u8; 32])
    }

    // ── §4 criterion 1 — unsigned = nothing (store byte-identical) ──────────────
    #[test]
    fn crit1_corrupted_signature_admits_nothing() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (mut frame, roster, chain) =
            valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);
        // Corrupt one bit of the classical signature.
        frame.classical_sig.as_mut().unwrap()[0] ^= 0x01;

        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let len_before = log.len();
        let res = adm.admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        assert!(matches!(res, Err(AdmissionError::BadSignature)));
        // No event, no admitted record — store byte-identical.
        assert_eq!(
            log.len(),
            len_before,
            "no event on a corrupted-signature frame"
        );
        assert!(adm.admitted(&manifest.content_id()).is_none());
    }

    // ── §4 criterion 2 — free-form dies at parse, gate call-count 0 ─────────────
    #[test]
    fn crit2_free_form_dies_before_gate() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let mut manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        manifest.config_axes = vec![(0x7F, 0)]; // unknown axis id
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);

        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let res = adm.admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        assert!(matches!(res, Err(AdmissionError::ManifestParseError(_))));
        assert_eq!(
            adm.gate().check_count(),
            0,
            "gate MUST NOT be reached on a parse failure"
        );
        assert_eq!(log.len(), 0);
    }

    // ── §4 criterion 6 — no KVM, no native (never downgraded) ───────────────────
    #[test]
    fn crit6_native_without_kvm_is_rejected() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::NativeProcess, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);

        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let res = adm.admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        // This host has no /dev/kvm (see microvm.rs R1) ⇒ AdapterRejected, no downgrade.
        assert!(matches!(res, Err(AdmissionError::AdapterRejected)));
        assert_eq!(log.len(), 0);
    }

    // ── §4 criterion 7 — identity binds ─────────────────────────────────────────
    #[test]
    fn crit7_node_id_mismatch_rejected_even_with_valid_sigs() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let mut manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        // Break the identity: claim a node_id that does NOT hash from the carried keys.
        manifest.agent_node_id = [0xAB; 32];
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        // Re-sign the frame over the tampered manifest so the signatures are VALID.
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);

        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let res = adm.admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        assert!(matches!(res, Err(AdmissionError::IdentityMismatch)));
        assert_eq!(log.len(), 0);
    }

    // ── happy path: a valid WasmComponent manifest is admitted, event recorded ──
    #[test]
    fn happy_path_admits_and_records_event() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, true, 2);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);

        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let rec = adm
            .admit(
                &frame,
                &roster,
                &chain,
                &RevocationSet::new(),
                &mut log,
                0,
                0,
            )
            .expect("valid manifest admits");
        assert_eq!(rec.tier, SandboxTier::WasmComponent);
        assert_eq!(rec.granted_depth, 2, "delegate=true, min(2,3)=2");
        assert_eq!(rec.granted_capacity, 4096);
        assert_eq!(log.len(), 1, "exactly one AgentAdmitted event");
        assert_eq!(rec.node_id.0, manifest.agent_node_id);
        // The minted bucket enforces the envelope.
        assert!(rec.bucket.try_acquire(4096.0));
        assert!(!rec.bucket.try_acquire(1.0), "budget envelope is real");
    }

    // ── depth grant: delegate=false ⇒ granted_depth 0 ───────────────────────────
    #[test]
    fn depth_zero_when_delegate_false() {
        let v = RefSigner;
        let (cls, pq, anch) = ([4u8; 32], [5u8; 32], [6u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 3);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [8u8; 8]);
        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let rec = adm
            .admit(
                &frame,
                &roster,
                &chain,
                &RevocationSet::new(),
                &mut log,
                0,
                0,
            )
            .unwrap();
        assert_eq!(rec.granted_depth, 0, "delegate=false ⇒ depth 0 (F10)");
    }

    // ── semantic re-admission idempotency: same manifest ⇒ no second event ──────
    #[test]
    fn semantic_readmission_is_a_no_op() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) =
            valid_frame(&v, &cls, &pq, &anch, &manifest, scope.clone(), [7u8; 8]);
        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        let r1 = adm
            .admit(
                &frame,
                &roster,
                &chain,
                &RevocationSet::new(),
                &mut log,
                0,
                0,
            )
            .unwrap();
        assert_eq!(log.len(), 1);
        // Re-present the SAME manifest bytes (a NEW frame w/ a fresh nonce so the gate
        // does not replay-reject) — envelope identical ⇒ short-circuit, no new event.
        let (frame2, _, chain2) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [9u8; 8]);
        let r2 = adm
            .admit(
                &frame2,
                &roster,
                &chain2,
                &RevocationSet::new(),
                &mut log,
                0,
                0,
            )
            .unwrap();
        assert_eq!(log.len(), 1, "semantic re-admission appends NO new event");
        assert_eq!(r1.content_id, r2.content_id);
        assert_eq!(r1.event_id, r2.event_id);
    }

    // ── SH-1 Guard A (crit 11) — pre-crypto flood throttled before the gate ─────
    #[test]
    fn crit11_flood_throttled_before_crypto() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        // Global ceiling = 3, no refill; shards disabled ⇒ only 3 frames reach the gate.
        let limiter = AdmissionLimiter::new(3, 0.0, 0, 0, 0.0);
        let mut adm = Admitter::new(open_gate(), limiter, 1_000_000, [0u8; 32]);
        let mut log = EventLog::new(MemEventStore::new());
        let mut throttled = 0usize;
        for i in 0..20u8 {
            // Distinct nonces so absent throttling every frame would reach the gate.
            let (frame, _, chain) =
                valid_frame(&v, &cls, &pq, &anch, &manifest, scope.clone(), [i; 8]);
            if let Err(AdmissionError::AdmissionThrottled) = adm.admit(
                &frame,
                &roster_for(&v, &anch),
                &chain,
                &RevocationSet::new(),
                &mut log,
                0,
                0,
            ) {
                throttled += 1;
            }
        }
        assert_eq!(
            adm.gate().check_count(),
            3,
            "only the 3 within the ceiling reach the gate"
        );
        assert_eq!(
            throttled, 17,
            "the flood beyond the ceiling is dropped pre-crypto"
        );
    }

    fn roster_for(v: &RefSigner, anchor_secret: &[u8; 32]) -> AnchorRoster {
        let mut r = AnchorRoster::new();
        r.enroll(&v.classical_public(anchor_secret));
        r
    }

    // ── SH-1 Guard B (crit 12) — over-length chain dies at decode, zero crypto ──
    #[test]
    fn crit12_over_length_chain_zero_crypto() {
        let verifies = Arc::new(AtomicUsize::new(0));
        let gate = ReferenceHybridGate::new_redlined(
            HybridPolicy::RequireBoth,
            RedLinePolicy::DenyByDefault,
            CountingVerifier {
                inner: RefSigner,
                verifies: verifies.clone(),
            },
        );
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, _chain) =
            valid_frame(&v, &cls, &pq, &anch, &manifest, scope.clone(), [7u8; 8]);
        // A chain longer than MAX_VERIFY_CHAIN_LINKS (16) — 17 links.
        let anchor = v.classical_public(&anch);
        let over: Vec<Delegation> = (0..17)
            .map(|i| {
                Delegation::sign(
                    &v,
                    anchor,
                    anchor,
                    scope.clone(),
                    scope.clone(),
                    9999,
                    [i as u8; 8],
                    &anch,
                )
            })
            .collect();
        let mut adm = Admitter::new(gate, open_limiter(), 1_000_000, [0u8; 32]);
        let mut log = EventLog::new(MemEventStore::new());
        let res = adm.admit(
            &frame,
            &roster,
            &over,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        assert!(matches!(res, Err(AdmissionError::ChainTooLong)));
        assert_eq!(adm.gate().check_count(), 0, "gate never reached");
        assert_eq!(
            verifies.load(Ordering::SeqCst),
            0,
            "ZERO signature verifications"
        );
        // Distinct from the DEFAULT_MAX_AGENT_DEPTH=3 dispatch cap.
        assert_ne!(MAX_VERIFY_CHAIN_LINKS, DEFAULT_MAX_AGENT_DEPTH as usize);
    }

    // ── red-line: an admission frame touching a red line is denied ──────────────
    #[test]
    fn red_line_admission_scope_denied() {
        // A frame whose capability scope is a red line (Ledger/SettlementRecorded) is
        // rejected by the armed gate even with valid signatures + chain.
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::Ledger, Action::SettlementRecorded);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);
        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());
        // Wrong admission scope is checked first (before the gate) — so this proves the
        // admission-scope gate; the red-line gate itself is unit-tested in scope.rs.
        let res = adm.admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        );
        assert!(matches!(res, Err(AdmissionError::WrongAdmissionScope)));
    }

    // ════════════════════════════════════════════════════════════════════════════
    // §4 criterion 10 (BLOCKING, SH-3) — the Poly-Network invariant, RED-first, 3 layers
    // ════════════════════════════════════════════════════════════════════════════

    /// LAYER 1 (behavioural): for EVERY B1 scope, a fully-valid, admittable frame drives
    /// the real verification path with a MUTABLE roster + revocation threaded in, and
    /// BOTH are byte-identical before/after — no B1 scope reached `enroll`/`remove`/
    /// `drop_anchor`/`load_genesis`.
    #[test]
    fn crit10_poly_network_layer1_no_mutation_for_any_b1_scope() {
        let v = RefSigner;
        for (r, a) in B1_NEW_SCOPES {
            let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
            let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
            let scope = Scope::single(r, a);
            let (frame, mut roster, chain) =
                valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);
            let mut revocations = RevocationSet::new();

            let roster_before = roster.snapshot_sorted();
            let revs_before = revocations.snapshot_sorted();

            // Drive the REAL paths. Roster/revocations are passed by SHARED ref (admit
            // and gate.check both take `&`), so the mutable bindings are simply
            // re-borrowed immutably — the capability-authorized surface never gets `&mut`.
            let gate = open_gate();
            let _ = gate.check(&frame, &roster, &chain, &revocations, 0);
            if (r, a) == (Resource::AgentBridge, Action::AdmitAgent) {
                let mut adm = admitter();
                let mut log = EventLog::new(MemEventStore::new());
                let _ = adm.admit(&frame, &roster, &chain, &revocations, &mut log, 0, 0);
            }

            assert_eq!(
                roster.snapshot_sorted(),
                roster_before,
                "roster unchanged for scope {r:?}/{a:?}"
            );
            assert_eq!(
                revocations.snapshot_sorted(),
                revs_before,
                "revocations unchanged for {r:?}/{a:?}"
            );

            // The bindings ARE genuinely `&mut`-capable — but mutation is reachable ONLY
            // via the out-of-band operator path, NEVER from the capability surface above.
            // (This net-out-of-band write proves the mutable API exists yet admit/gate
            // never took it; it runs AFTER the before/after assertions.)
            roster.enroll(&[0xEEu8; 32]);
            revocations.revoke_key([0xEEu8; 32]);
            assert!(roster.contains(&[0xEEu8; 32]));
        }
    }

    /// LAYER 2 (negative control): prove the layer-1 equality assertion has TEETH — a
    /// poison handler that DOES call `roster.remove(anchor)` makes the before/after
    /// check FAIL. Without this, layer 1 could be vacuously true.
    #[test]
    fn crit10_poly_network_layer2_negative_control_detects_mutation() {
        let v = RefSigner;
        let anchor = v.classical_public(&[3u8; 32]);
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor);
        let before = roster.snapshot_sorted();

        // The poison: a handler that mutates the roster (the very thing the invariant
        // forbids from a capability path). Here we call it DIRECTLY to prove detection.
        fn poison_remove(roster: &mut AnchorRoster, anchor: &[u8; 32]) {
            RevocationSet::drop_anchor(roster, anchor); // == roster.remove(anchor)
        }
        poison_remove(&mut roster, &anchor);

        let after = roster.snapshot_sorted();
        assert_ne!(
            after, before,
            "the before/after check MUST detect roster mutation (not vacuous)"
        );
        assert!(
            !roster.contains(&anchor),
            "poison actually removed the anchor"
        );
    }

    /// LAYER 3 (structural): `admit` borrows roster/revocations SHARED (`&`), never
    /// `&mut`. Proven at compile time — a second shared borrow is held live ACROSS the
    /// `admit` call and used AFTER it. If `admit` took `&mut AnchorRoster`, the `&roster`
    /// it needs could not coexist with `shared_hold`, and this test would fail to
    /// compile. The mutators (`enroll`/`remove`/`drop_anchor`) require `&mut` and are
    /// only reachable from the out-of-band operator/genesis path — never from a
    /// `Capability`/`SignedFrame`.
    #[test]
    fn crit10_poly_network_layer3_admit_uses_shared_borrow_only() {
        let v = RefSigner;
        let (cls, pq, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let manifest = build_manifest(&v, &cls, &pq, ExecutionModel::WasmComponent, false, 0);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let (frame, roster, chain) = valid_frame(&v, &cls, &pq, &anch, &manifest, scope, [7u8; 8]);
        let revocations = RevocationSet::new();
        let mut adm = admitter();
        let mut log = EventLog::new(MemEventStore::new());

        let anchor = v.classical_public(&anch);
        // Shared borrow #1 — held live across `admit`.
        let shared_hold: &AnchorRoster = &roster;
        // Shared borrow #2 — `admit` takes `&roster`. Coexists ONLY because it is `&`.
        let _ = adm.admit(&frame, &roster, &chain, &revocations, &mut log, 0, 0);
        // Use #1 AFTER `admit` — compiles only because `admit` borrowed shared, not `&mut`.
        assert!(
            shared_hold.contains(&anchor),
            "roster still readable — admit took a shared borrow"
        );
    }
}
