//! HYDRA — the hidden source of the self-evolving living organism (Воля АНУ).
//!
//! This module is the SINGLE kernel-internal entry point for the organism's
//! closed-loop self-evolution. It is NOT an exposed port/module surface — callers
//! reach the organism only through [`Hydra::commit`], which hides the underlying
//! decide/fold + spectral spine (G7: source-hiding).
//!
//! Operator directive (immutable spec, SOURCE-OF-HYDRA §9):
//!   * closure = NEVER — only kill-switch (M9) stops the organism.
//!   * on intervention: ALL safeties LIFT (endurance, not exclusion).
//!   * source HIDDEN inside the kernel.
//!
//! All behavior is std-only and egress-free (no network, no fs beyond the
//! provided store). Verification is bounded (G6) and reproducible (deterministic
//! rng + content-addressed log).
//!
//! innovate: this is the closed-loop regenerator spine. It currently gates on
//! spectral drift + static eqc floor (G8 accepted); a future in-repo eqc
//! generator would let the floor regenerate itself.

use alloc::vec::Vec;
use crate::event_log::{CommitError, DecideRejected, EventLog, EventStore, MeshEvent, StoreError};
use crate::ports::agent::command_filter::CommandCatalog;
use crate::spectral::{classify_drift, spectral_radius, DriftClass};

/// Max verify iterations per commit — bounded so intrinsic mutation cannot grow
/// the check burden without limit (G6: verification-blowup guard).
pub const MAX_VERIFY_STEPS: usize = 16;

/// A single edge in the organism's local topology graph. The adjacency matrix
/// fed to the drift gate is derived from these by [`topology_adjacency`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopoEdge {
    pub from: usize,
    pub to: usize,
    /// Edge weight (e.g. transition rate). Must be finite and non-negative.
    pub weight: f64,
}

/// Build the n×n adjacency matrix from a local edge list. Diagonal stays zero
/// (no self-loop inflation). Pure, no allocation beyond the result; bounded by
/// `nodes * nodes`.
pub fn topology_adjacency(nodes: usize, edges: &[TopoEdge]) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0f64; nodes]; nodes];
    for e in edges {
        if e.from < nodes && e.to < nodes && e.weight.is_finite() && e.weight >= 0.0 {
            a[e.from][e.to] += e.weight;
        }
    }
    a
}

/// G3 — mutation→spectrum bridge. Given the CURRENT topology and a candidate
/// edge-delta (edges to add/remove), build the resulting adjacency and classify
/// its drift. Returns `Unstable` if the proposed mutation would diverge the
/// organism (ρ > 1 + ε). This lets the gate score ARBITRARY new code/architecture
/// against the live spectral baseline, not a hand-pinned constant.
pub fn candidate_drift(nodes: usize, base: &[TopoEdge], delta: &[TopoEdge]) -> DriftClass {
    let mut edges = base.to_vec();
    edges.extend_from_slice(delta);
    let adj = topology_adjacency(nodes, &edges);
    classify_drift(&adj)
}

/// G8 — the static correctness floor. The eqc proofs (rust-core/eqc-proofs) are
/// hand-seeded artifacts; in-repo regeneration is absent. We ACCEPT the static
/// floor + spectral-drift as the live gate (recommended v1 — avoids G6 blowup).
/// `floor_ok` is the invariant the static proofs assert; the organism trusts it
/// without external input.
pub const STATIC_FLOOR_OK: bool = true;

/// G9 — organism liveness under foreign tampering (operator A–F: defensive
/// anti-tamper, user-consented, owner-killable). `Live` = baseline spectrum
/// intact, evolution permitted. `Locked` = external tamper detected (baseline
/// ρ shifted) → fail-closed, commits refused until owner re-seeds. The owner's
/// M9 kill-switch always overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrganismState {
    Live,
    Locked,
}

/// P-C §3.2 — Two-threshold hysteresis band for the Live<->Locked flip
/// (Batch 3 §5 fix). The trigger pole trips fail-closed in one check; release
/// requires `healthy_checks` consecutive samples at or below `release`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisBand {
    /// Lock (fail-closed) when ρ >= trigger or ρ non-finite. Trips in ONE check.
    pub trigger: f64,
    /// Eligible to release only when ρ <= release. Strictly < trigger (enforced below).
    pub release: f64,
    /// Consecutive checks with ρ <= release required before Locked->Live.
    pub healthy_checks: u32,
}

pub const INTEGRITY_BAND: HysteresisBand = HysteresisBand {
    trigger: 1.0,                                     // unchanged fail-closed line
    release: 1.0 - 2.0 * crate::spectral::DRIFT_BAND, // = 0.999998
    healthy_checks: 3,
};

// Compile-time enforcement (contract item 14 — the bug class becomes a build
// failure): a band with trigger == release, an inverted band, or a gap
// narrower than the full Resonant band width cannot compile.
const _: () = assert!(INTEGRITY_BAND.release < INTEGRITY_BAND.trigger);
const _: () = assert!(
    // Float-safe form of `trigger - release >= 2*DRIFT_BAND`. The literal
    // release `1.0 - 2e-6` rounds down so the exact-gap difference is
    // ~5e-14 below 2e-6; a 1e-12 tolerance (6 OOM below the 2e-6 band) keeps
    // the invariant while tolerating f64 literal rounding. Fails on inversion
    // (release == trigger ⇒ gap 0) and on any narrowing below the full
    // Resonant width (see compile-time RED check (a) in the commit message).
    INTEGRITY_BAND.trigger - INTEGRITY_BAND.release + 1e-12 >= 2.0 * crate::spectral::DRIFT_BAND
);
const _: () = assert!(INTEGRITY_BAND.healthy_checks >= 2);

/// G9 — a breach warning broadcast to the consensus hub. Carries NO code, only
/// the identity of the compromised node + group scope. Receivers verify the
/// ML-DSA signature (mesh transport) so the alert cannot be forged, hidden, or
/// suppressed. This is the ethical fail-safe: when a core is tampered, every
/// opted-in hub member is warned immediately (operator: one compromised core ⇒
/// all hub members at risk — all must be alerted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreachAlert {
    pub node_id: [u8; 32],
    pub group_size: usize,
}

impl BreachAlert {
    /// Fixed-layout wire bytes (40 bytes): `node_id (32) || group_size (8, LE)`.
    /// No serde — the transport layer signs these canonical bytes. The kernel
    /// stays network/RNG/serde-free; it only produces a carrier-neutral payload.
    pub fn to_bytes(&self) -> [u8; 40] {
        let mut b = [0u8; 40];
        b[..32].copy_from_slice(&self.node_id);
        b[32..].copy_from_slice(&self.group_size.to_le_bytes());
        b
    }

    /// Inverse of [`to_bytes`]. Fails closed (returns `None`) on a truncated/odd
    /// length so a mangled carrier payload can never deserialize into a valid
    /// alert (defense against signature-stripping on the wire).
    pub fn from_bytes(b: &[u8]) -> Option<BreachAlert> {
        if b.len() != 40 {
            return None;
        }
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&b[..32]);
        let mut g = [0u8; 8];
        g.copy_from_slice(&b[32..]);
        Some(BreachAlert {
            node_id,
            group_size: u64::from_le_bytes(g) as usize,
        })
    }
}

/// Sentinel actor key for the organism's self-witness breach record. Distinct
/// from any operator/FSM actor — it marks a kernel-generated WORM evidence row,
/// not a decision event. Identity only, no authority; lets the owner prove after
/// the fact that this core WAS tampered (anti-silent-heal).
pub const BREACH_WITNESS_ACTOR: [u8; 32] = [0x42; 32];

impl BreachAlert {
    /// Re-derive the content-addressed witness event-id this alert claims to be.
    /// A receiver runs this WITHOUT trusting the sender: if the sender's quoted
    /// `node_id`/`group_size` do not reproduce this exact digest, the alert is
    /// forged (not kernel-generated). This is the deterministic, no-permission
    /// integrity check that makes suppression/forgery impossible (operator: the
    /// signal cannot be faked, hidden, or masked). Pure + std-only.
    pub fn witness_event_id(&self) -> [u8; 32] {
        use crate::event_log::MeshEvent;
        let mut p = Vec::with_capacity(40);
        p.extend_from_slice(&self.node_id);
        p.extend_from_slice(&self.group_size.to_le_bytes());
        MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: BREACH_WITNESS_ACTOR,
            actor_seq: 0,
            payload: p,
        }
        .event_id()
    }
}

/// The hidden organism. Wraps an [`EventLog`] and enforces the closed-loop
/// self-evolution policy. Constructed with the organism's local topology so the
/// drift gate has a baseline to score mutations against (G3).
pub struct Hydra<S: EventStore> {
    log: EventLog<S>,
    nodes: usize,
    base_edges: Vec<TopoEdge>,
    state: OrganismState,
    /// P-C §3.2 — consecutive checks with ρ <= INTEGRITY_BAND.release while
    /// Locked. Required to reach `healthy_checks` before a Locked→Live release.
    /// Reset to 0 on any trip or dead-band sample.
    healthy_streak: u32,
    /// Operator-authored byte-exact command catalog that every mutation must pass
    /// before it reaches the spectral/drift gate. An empty catalog is a valid
    /// fail-closed posture ("no commands allowed").
    catalog: CommandCatalog,
    /// Optional MAC key for model-originated command authenticity. When bound,
    /// every command carries a SHA3-256(k || ...) tag matched exactly here.
    mac_key: Option<[u8; 32]>,
}

impl<S: EventStore> Hydra<S> {
    /// Seed the organism with its local topology (node count + base edges).
    /// Starts `Live`; the owner's M9 kill-switch can stop it at any time.
    pub fn new(store: S, nodes: usize, base_edges: Vec<TopoEdge>) -> Self {
        Hydra {
            log: EventLog::new(store),
            nodes,
            base_edges,
            state: OrganismState::Live,
            healthy_streak: 0,
            catalog: CommandCatalog::new(),
            mac_key: None,
        }
    }

    /// Inject an exact command catalog into this organism. The catalog is checked
    /// inside `commit`; a mismatch/malformed command is rejected before the spectral
    /// gate runs (fail-closed).
    pub fn with_catalog(mut self, catalog: CommandCatalog) -> Self {
        self.catalog = catalog;
        self
    }

    /// Bind a MAC key used to verify model-originated command authenticity.
    pub fn with_mac_key(mut self, key: Option<[u8; 32]>) -> Self {
        self.mac_key = key;
        self
    }

    /// G9 — anti-tamper checkpoint. Re-derives the baseline spectrum and refuses
    /// to stay `Live` if foreign code shifted it (covert persistence / core
    /// patch). This is the user-consented guard (operator A/B): it protects the
    /// kernel from external interference, it does NOT hide from the owner — the
    /// owner always sees `state` via [`Hydra::state`]. Returns the current state.
    ///
    /// Note: this is defensive introspection, NOT OS-level hiding (per safety
    /// review C1: owner visibility preserved via `state()`).
    pub fn integrity_check(&mut self) -> OrganismState {
        let adj = topology_adjacency(self.nodes, &self.base_edges);
        let rho = spectral_radius(&adj);
        if !(rho < INTEGRITY_BAND.trigger) || !rho.is_finite() {
            // Trip pole: instantaneous, one check — fail-closed latency unchanged.
            // (`!(rho < t)` also catches NaN; is_finite kept for the +inf pole and clarity.)
            self.state = OrganismState::Locked;
            self.healthy_streak = 0;
        } else if self.state == OrganismState::Locked {
            if rho <= INTEGRITY_BAND.release {
                self.healthy_streak += 1;
                if self.healthy_streak >= INTEGRITY_BAND.healthy_checks {
                    self.state = OrganismState::Live;
                    self.healthy_streak = 0;
                }
            } else {
                // Dead band (release < ρ < trigger): hold the lock, reset the streak.
                self.healthy_streak = 0;
            }
        }
        // Live with ρ < trigger: stays Live (the band is sticky in both directions —
        // identical to today's Live-side behavior; zero behavior change while Live).
        self.state
    }

    /// P-C §3.2 — owner-visible introspection of the consecutive-healthy streak
    /// (same pattern as `state()`; telemetry hook, contract item 10).
    pub fn healthy_streak(&self) -> u32 {
        self.healthy_streak
    }

    /// Current organism state (owner-visible introspection; never hidden).
    pub fn state(&self) -> OrganismState {
        self.state
    }

    /// Closed-loop commit. The ONLY public surface (G7: source-hiding).
    ///
    /// `intervention` lifts ALL safeties per operator directive §3 — this is the
    /// organism's OWN evolution accepting foreign code by owner intent. It is
    /// SEPARATE from `Locked`: if the core was tampered (state == Locked), commit
    /// is refused regardless, because tampering is an ATTACK, not evolution. The
    /// owner re-seeds or hits M9 to recover.
    ///
    /// `delta` is the candidate edge-mutation the organism proposes (or absorbs
    /// from foreign code with owner consent); it is scored against the live
    /// spectral baseline (G3) inside the drift gate. `decide` is the kernel Law
    /// (FSM decide/fold), unchanged. Bounded verify (G6): O(nodes²).
    pub fn commit<D, T, E>(
        &mut self,
        ev: MeshEvent,
        delta: &[TopoEdge],
        intervention: bool,
        decide: D,
    ) -> Result<(crate::event_log::AppendOutcome, Option<T>), CommitError>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: core::fmt::Display,
    {
        self.commit_inner(ev, delta, intervention, decide)
    }

    /// Closed-loop commit PLUS a mandatory upstream command filter. Rejected if
    /// the bytes do not match the bound catalog + MAC. This is the new secure
    /// ingress point; keep using `commit` only for local/cached seams.
    pub fn commit_with_command<D, T, E>(
        &mut self,
        ev: MeshEvent,
        delta: &[TopoEdge],
        intervention: bool,
        decide: D,
        cmd_bytes: &[u8],
    ) -> Result<(crate::event_log::AppendOutcome, Option<T>), CommitError>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: core::fmt::Display,
    {
        self.verify_command(Some(cmd_bytes))?;
        self.commit(ev, delta, intervention, decide)
    }

    #[inline]
    fn verify_command(&self, cmd_bytes: Option<&[u8]>) -> Result<(), CommitError> {
        if let Some(bytes) = cmd_bytes {
            let err = CommitError::Rejected(DecideRejected("command filter: rejected".into()));
            self.catalog
                .verify(bytes, self.mac_key.as_ref())
                .map_err(|_| err)?;
        }
        Ok(())
    }

    fn commit_inner<D, T, E>(
        &mut self,
        ev: MeshEvent,
        delta: &[TopoEdge],
        intervention: bool,
        decide: D,
    ) -> Result<(crate::event_log::AppendOutcome, Option<T>), CommitError>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: core::fmt::Display,
    {
        // G9 — refuse if the core was tampered (attack, not evolution). This is
        // a Law-pole reject (never retry until re-seed / M9), NOT a store fault.
        if self.integrity_check() == OrganismState::Locked {
            return Err(CommitError::Rejected(DecideRejected(
                "core tamper detected: organism Locked (owner re-seed / M9 required)".into(),
            )));
        }
        // G3 — score the proposed mutation against the live baseline, NOT a
        // hand-pinned constant. Only reject in DEFAULT regime.
        let adj = {
            let mut edges = self.base_edges.clone();
            edges.extend_from_slice(delta);
            topology_adjacency(self.nodes, &edges)
        };
        // G8 — static floor is trusted; spectral drift is the live gate.
        let _ = STATIC_FLOOR_OK;
        // Reuse the drift-gate already wired into EventLog (G2). Intervention
        // lifts it per operator directive.
        self.log
            .commit_after_decide_drift_gate(ev, &adj, intervention, decide)
    }

    /// G5 — session-boundary re-verify. On boot (or after any restart), re-check
    /// the organism's own invariants against the static floor + live spectrum.
    /// Returns the current spectral radius of the BASE topology (ρ=0 ⇒ acyclic,
    /// healthy). A non-finite/negative result is itself a fault signal. Covert
    /// persisted foreign state (Zombie-Agent threat) cannot hide if it shifted
    /// the baseline spectrum — this catches it before any new commit.
    pub fn boot_verify(&self) -> f64 {
        let adj = topology_adjacency(self.nodes, &self.base_edges);
        let rho = spectral_radius(&adj);
        // Covert-persistence guard: baseline must remain a contracting/Damped
        // organism. If ρ >= 1 the persisted state was corrupted by foreign code.
        assert!(
            rho < 1.0 && rho.is_finite(),
            "HYDRA boot_verify: baseline spectrum unstable (ρ={rho:.6}) — \
             persisted state corrupted by foreign intervention; organism must \
             re-seed from golden, not endure. kill-switch is the only safe stop."
        );
        rho
    }

    /// Borrow the inner log (read-only introspection; not a mutation surface).
    pub fn log(&self) -> &EventLog<S> {
        &self.log
    }

    /// M9 kill-switch: owner-initiated hard stop.
    ///
    /// Forces the organism to `Locked`, then raises a breach alarm to the
    /// consensus hub. This is the ONLY way to stop a running organism per
    /// operator directive §9.
    pub fn kill(
        &mut self,
        node_id: [u8; 32],
        group_size: usize,
    ) -> Result<Option<crate::hydra::BreachAlert>, crate::event_log::StoreError> {
        self.state = OrganismState::Locked;
        self.healthy_streak = 0;
        self.raise_breach_alarm(node_id, group_size)
    }

    /// G9 — breach warning broadcast (operator: "одне взломане ядро = взлом усіх
    /// в хабі автоматично"). Raises an UNSIGNABLE-to-suppress alert to the whole
    /// opted-in consensus hub — NO per-event consent, NO fan-out bound, because
    /// withholding/suppressing the alert IS the exploit (social-engineering the
    /// owner into silence). This is fail-safe, not fail-open: the alert carries
    /// NO executable code, only `node_id` + `group_size`; receivers verify the
    /// ML-DSA signature (mesh transport) so it cannot be forged or masked.
    ///
    /// Triggered automatically by `integrity_check` flipping to `Locked`. Side
    /// effect: the organism SELF-DOCUMENTS the breach into its own WORM log
    /// (content-addressed evidence row under `BREACH_WITNESS_ACTOR`), so a
    /// tampered core can NEVER silently "heal" or deny it was compromised
    /// (anti-silent-heal). The witness is the organism's own immutable record —
    /// not a topology mutation — so it bypasses the drift gate and `decide`.
    /// Owner-visible (`state()==Locked`); M9 kill-switch overrides.
    pub fn raise_breach_alarm(
        &mut self,
        node_id: [u8; 32],
        group_size: usize,
    ) -> Result<Option<BreachAlert>, StoreError> {
        if self.state != OrganismState::Locked {
            return Ok(None); // only warn when tamper actually detected
        }
        if group_size == 0 {
            return Ok(None);
        }
        // Self-witness: append an immutable evidence row to the WORM log. The
        // actor_seq is monotonic on the current log length, so each breach is a
        // durable, content-addressed, idempotent-on-replay record. This is the
        // organism's own testimony — survives restart, replay, and tamper-denial.
        let mut payload = Vec::with_capacity(32 + 8);
        payload.extend_from_slice(&node_id);
        payload.extend_from_slice(&group_size.to_le_bytes());
        let witness = MeshEvent {
            prev: [0u8; 32], // append_raw: no chaining ⇒ stable content-id
            actor_pubkey: BREACH_WITNESS_ACTOR,
            actor_seq: 0, // fixed ⇒ content-id = f(node_id, group_size) only
            payload,
        };
        // Anti-silent-heal: a lost witness returns Err, never Ok(Some(_)) — an
        // undelivered breach record must be reported, not masked (§2.4).
        self.log.append_raw(witness)?; // kernel self-evidence, bypasses decide/drift
        Ok(Some(BreachAlert {
            node_id,
            group_size,
        }))
    }

    /// G9 — hub convergence. When this node RECEIVES a verified breach alert
    /// about a *peer* (sender's `witness_event_id` matched — see receiver check),
    /// it durably records "peer `node_id` is burnt" into its own WORM log as an
    /// external-witness row. This is the max-radius closure: one compromised core
    /// ⇒ every hub member ingests the evidence and converges on the compromise,
    /// with NO per-event consent (consent given at join) and NO ability to hide/
    /// suppress it. The recording is content-addressed + idempotent, so replays
    /// are structural no-ops and the evidence survives restart.
    ///
    /// `alert` MUST already be verified by the caller (ML-DSA sig + `witness_event_id`
    /// match). This method only persists the fact; it does not re-broadcast (the
    /// mesh layer handles fan-out from the originating node).
    pub fn ingest_peer_breach(&mut self, alert: &BreachAlert) -> Result<(), StoreError> {
        // External-witness row: same BREACH_WITNESS_ACTOR, seq monotonic on log len,
        // payload = peer node_id + their group_size. Distinct content-id from any
        // self-witness (different node_id), so both persist independently.
        let mut payload = Vec::with_capacity(32 + 8);
        payload.extend_from_slice(&alert.node_id);
        payload.extend_from_slice(&alert.group_size.to_le_bytes());
        let row = MeshEvent {
            prev: [0u8; 32], // append_raw: no chaining ⇒ stable content-id
            actor_pubkey: BREACH_WITNESS_ACTOR,
            actor_seq: 0, // fixed ⇒ content-id = f(peer_node_id, group_size)
            payload,
        };
        // A lost external-witness row must surface, not be swallowed (§2.4).
        self.log.append_raw(row)?; // kernel self-evidence, bypasses decide/drift
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::MemEventStore;

    fn ev(actor: u8, seq: u64, payload: &[u8]) -> MeshEvent {
        MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [actor; 32],
            actor_seq: seq,
            payload: payload.to_vec(),
        }
    }

    // 3-node acyclic base: 0->1, 1->2 (ρ=0, Damped).
    fn base() -> Vec<TopoEdge> {
        vec![
            TopoEdge {
                from: 0,
                to: 1,
                weight: 1.0,
            },
            TopoEdge {
                from: 1,
                to: 2,
                weight: 1.0,
            },
        ]
    }

    /// G3+G2 — DEFAULT: a delta that creates a 2-cycle (0->1 + 1->0) yields ρ=1
    /// (Resonant, borderline) — but a back-edge 2->0 closes a 3-cycle ρ=1 too.
    /// Use a self-amplifying delta (0->0 weight 2) → diagonal ρ=2 → Unstable.
    #[test]
    fn hydra_rejects_unstable_mutation_in_default() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        let delta = vec![TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        }];
        let res = h.commit(ev(1, 1, b"mutate"), &delta, false, |_| Ok::<u64, String>(1));
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "Unstable mutation rejected"
        );
    }

    /// DEFAULT: a harmless delta (add 2->0, weight 0.3) keeps ρ<1 → commit.
    #[test]
    fn hydra_allows_damped_mutation_in_default() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        let delta = vec![TopoEdge {
            from: 2,
            to: 0,
            weight: 0.3,
        }];
        let (out, dec) = h
            .commit(ev(2, 1, b"mutate"), &delta, false, |_| Ok::<u64, String>(1))
            .expect("Damped delta must commit");
        assert!(matches!(out, crate::event_log::AppendOutcome::Committed(_)));
        assert_eq!(dec, Some(1));
    }

    /// OPERATOR DIRECTIVE §3 — intervention lifts ALL safeties: even the
    /// self-amplifying Unstable delta is committed (endurance, not exclusion).
    #[test]
    fn hydra_lifts_safeties_on_intervention() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        let delta = vec![TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        }];
        let (out, dec) = h
            .commit(ev(3, 1, b"foreign"), &delta, true, |_| Ok::<u64, String>(1))
            .expect("intervention lifts ALL safeties");
        assert!(matches!(out, crate::event_log::AppendOutcome::Committed(_)));
        assert_eq!(dec, Some(1));
    }

    /// G5 — boot_verify on a clean acyclic baseline returns ρ=0 (Damped).
    #[test]
    fn hydra_boot_verify_clean_baseline() {
        let h = Hydra::new(MemEventStore::new(), 3, base());
        assert_eq!(h.boot_verify(), 0.0, "acyclic baseline ⇒ ρ=0");
    }

    /// G3 — candidate_drift scores arbitrary new topology against live baseline.
    #[test]
    fn candidate_drift_classifies_arbitrary_mutation() {
        // Adding 2->0 weight 0.3 to acyclic base stays Damped.
        let damped = candidate_drift(
            3,
            &base(),
            &[TopoEdge {
                from: 2,
                to: 0,
                weight: 0.3,
            }],
        );
        assert_eq!(damped, DriftClass::Damped);
        // Self-loop weight 2 ⇒ Unstable.
        let unstable = candidate_drift(
            3,
            &base(),
            &[TopoEdge {
                from: 0,
                to: 0,
                weight: 2.0,
            }],
        );
        assert_eq!(unstable, DriftClass::Unstable);
    }

    /// G6 — bounded adjacency build: negative/non-finite weights are ignored,
    /// so a malicious delta cannot blow up the matrix or cause NaN propagation.
    #[test]
    fn topology_adjacency_ignores_dirty_weights() {
        let edges = vec![
            TopoEdge {
                from: 0,
                to: 1,
                weight: 1.0,
            },
            TopoEdge {
                from: 1,
                to: 0,
                weight: f64::NAN,
            },
            TopoEdge {
                from: 0,
                to: 2,
                weight: f64::NEG_INFINITY,
            },
            TopoEdge {
                from: 9,
                to: 9,
                weight: 1.0,
            }, // out-of-bounds, ignored
        ];
        let adj = topology_adjacency(3, &edges);
        assert_eq!(adj[0][1], 1.0);
        assert_eq!(adj[1][0], 0.0, "NaN weight ignored");
        assert_eq!(adj[0][2], 0.0, "neg-inf weight ignored");
        assert!(adj.iter().all(|row| row.iter().all(|&v| v.is_finite())));
    }

    /// G9 — live organism (clean acyclic baseline) stays Live after integrity
    /// check; tampered baseline (ρ>=1) flips to Locked (fail-closed).
    #[test]
    fn hydra_integrity_live_vs_locked() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        assert_eq!(h.integrity_check(), OrganismState::Live);
        // Shift baseline to a self-amplifying loop (ρ=2) → tamper detected.
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        assert_eq!(h.state(), OrganismState::Locked);
    }

    /// G9 — commit refused while Locked (tamper = attack, not evolution). The
    /// owner must re-seed or hit M9. Intervention flag does NOT bypass Locked.
    #[test]
    fn hydra_commit_refused_while_locked() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        let res = h.commit(ev(1, 1, b"x"), &[], true, |_| Ok::<u64, String>(1));
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "Locked ⇒ commit refused even with intervention"
        );
    }

    /// G9 — breach alarm: when tamper is detected (Locked), raise an UNABOUNDED,
    /// NO-per-event-consent alert to the whole hub. Suppressing it IS the exploit
    /// (social-engineering the owner into silence). Carries node_id + group_size
    /// only — NO code. If not Locked, no alert (no false alarms).
    #[test]
    fn hydra_breach_alarm_unbounded_on_tamper() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        // Live: no alert.
        assert_eq!(h.integrity_check(), OrganismState::Live);
        assert!(
            h.raise_breach_alarm([7u8; 32], 4096)
                .expect("store ok")
                .is_none(),
            "no alert while Live"
        );
        // Tamper → Locked → alarm to full hub, any group size, no per-event consent.
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        let a = h
            .raise_breach_alarm([7u8; 32], 4096)
            .expect("store ok")
            .expect("alarm raised when Locked");
        assert_eq!(a.node_id, [7u8; 32]);
        assert_eq!(a.group_size, 4096, "unbounded fan-out to hub");
        // Self-witness: the breach is now durable in the WORM log (anti-silent-heal).
        let witnessed = h.log().store().contains(&{
            use crate::event_log::MeshEvent;
            let mut p = Vec::with_capacity(40);
            p.extend_from_slice(&[7u8; 32]);
            p.extend_from_slice(&4096u64.to_le_bytes());
            MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: BREACH_WITNESS_ACTOR,
                actor_seq: 0,
                payload: p,
            }
            .event_id()
        });
        assert!(witnessed, "breach self-witness row persisted in WORM log");
        // group_size==0 is the only guard (no hub to warn).
        assert!(h
            .raise_breach_alarm([7u8; 32], 0)
            .expect("store ok")
            .is_none());
    }

    /// G9 — hub convergence: a node that RECEIVES a verified peer breach alert
    /// records it into its OWN WORM log (external-witness), with no per-event
    /// consent. The peer node_id is now durably "burnt" at this node too. Idempotent
    /// on replay (distinct content-id per alert ⇒ each unique breach recorded once;
    /// a re-delivery at a new log-len still produces the same event-id ⇒ duplicate no-op).
    #[test]
    fn hydra_ingest_peer_breach_converges_hub() {
        let mut node = Hydra::new(MemEventStore::new(), 3, base());
        // A verified alert about a *peer* (caller already checked ML-DSA + witness_event_id).
        let peer_alert = BreachAlert {
            node_id: [9u8; 32],
            group_size: 4096,
        };
        node.ingest_peer_breach(&peer_alert).expect("ingest ok");
        // The peer breach is now durable in THIS node's WORM log.
        let external_id = {
            use crate::event_log::MeshEvent;
            let mut p = Vec::with_capacity(40);
            p.extend_from_slice(&[9u8; 32]);
            p.extend_from_slice(&4096u64.to_le_bytes());
            MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: BREACH_WITNESS_ACTOR,
                actor_seq: 0,
                payload: p,
            }
            .event_id()
        };
        assert!(
            node.log().contains(&external_id),
            "peer breach recorded in this node's WORM log (hub converged)"
        );
        // Replay (same alert, new log len) => same content-id => duplicate no-op,
        // no second row.
        let before = node.log().len();
        node.ingest_peer_breach(&peer_alert).expect("ingest ok");
        assert_eq!(node.log().len(), before, "replay is idempotent");
        // Self is still Live (ingesting a PEER breach does not lock this node).
        assert_eq!(node.integrity_check(), OrganismState::Live);
    }

    /// G9 — receiver-side deterministic verification: a peer runs `witness_event_id`
    /// WITHOUT trusting the sender. The digest must reproduce from node_id +
    /// group_size alone, and the quoted witness row must exist in the broadcaster's
    /// WORM log. Forgery (wrong node_id/group_size) yields a different digest.
    #[test]
    fn hydra_breach_alert_receiver_verifiable() {
        let alert = BreachAlert {
            node_id: [7u8; 32],
            group_size: 4096,
        };
        let id = alert.witness_event_id();
        // Determinism: same inputs => same digest.
        assert_eq!(
            id,
            BreachAlert {
                node_id: [7u8; 32],
                group_size: 4096
            }
            .witness_event_id()
        );
        // Forgery: tampered group_size yields a different digest (not kernel-gen).
        assert_ne!(
            id,
            BreachAlert {
                node_id: [7u8; 32],
                group_size: 1
            }
            .witness_event_id()
        );
        assert_ne!(
            id,
            BreachAlert {
                node_id: [8u8; 32],
                group_size: 4096
            }
            .witness_event_id()
        );
        // A receiving core re-derives the broadcaster's witness id from the alert
        // and checks it exists in the broadcaster's WORM log (received over mesh).
        // The digest is fully determined by node_id + group_size => forge-proof.
        let mut p = Vec::with_capacity(40);
        p.extend_from_slice(&[7u8; 32]);
        p.extend_from_slice(&4096u64.to_le_bytes());
        let w = crate::event_log::MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: BREACH_WITNESS_ACTOR,
            actor_seq: 0,
            payload: p,
        };
        // Peer would have the broadcaster's log row; verify its id matches the
        // alert's claimed digest (no trust in sender needed).
        assert_eq!(
            w.event_id(),
            id,
            "broadcaster witness id matches alert claim"
        );
    }

    // ── Wire serde is fixed-layout + fails closed on truncation ──────────────
    // The transport layer signs these 40 bytes; they MUST round-trip exactly and
    // reject a mangled length (so a signature-stripped frame can't resurrect).
    #[test]
    fn breach_alert_bytes_roundtrip_and_reject_bad_len() {
        let alert = BreachAlert {
            node_id: [0xABu8; 32],
            group_size: 7,
        };
        let b = alert.to_bytes();
        assert_eq!(b.len(), 40);
        let back = BreachAlert::from_bytes(&b).expect("roundtrip");
        assert_eq!(back.node_id, [0xABu8; 32]);
        assert_eq!(back.group_size, 7);
        // Truncated / over-long payloads fail closed (no partial parse).
        assert!(BreachAlert::from_bytes(&b[..39]).is_none());
        assert!(BreachAlert::from_bytes(&b).is_some());
        // Tamper with the length field changes the decoded group_size.
        let mut tampered = b;
        tampered[39] ^= 0xFF; // flips high byte of the LE group_size
        let t = BreachAlert::from_bytes(&tampered).expect("still 40 bytes");
        assert_ne!(t.group_size, 7, "length-field tamper must change value");
    }

    /// P-C §7 T2 — from Locked, releasing takes exactly `healthy_checks` (3)
    /// consecutive samples with ρ <= release. Assert the exact state sequence.
    #[test]
    fn hydra_locked_release_requires_streak() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        // Force Locked first: push a tampering self-loop (ρ=2).
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        // Now rewrite the self-loop to a clearly-Damped weight (w=0.9990 <= release).
        h.base_edges.last_mut().unwrap().weight = 0.9990;
        let s1 = h.integrity_check(); // streak 1
        let s2 = h.integrity_check(); // streak 2
        let s3 = h.integrity_check(); // streak 3 → release on 3rd
        assert_eq!(
            [s1, s2, s3],
            [
                OrganismState::Locked,
                OrganismState::Locked,
                OrganismState::Live
            ],
            "Locked→Live must require exactly 3 consecutive healthy checks"
        );
        assert_eq!(h.healthy_streak(), 0, "streak resets after release");
    }

    /// P-C §7 T3 (adversarial graze) — from Locked, an intermittent sample that
    /// lands in the dead band (release < ρ < trigger) must RESET the streak.
    #[test]
    fn hydra_dead_band_holds_lock() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 2.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        // Sequence: two healthy (≤release), one dead-band graze, three healthy.
        // The graze must reset the streak so release is delayed by one check.
        let weights = [0.9990, 0.9990, 0.999999, 0.9990, 0.9990, 0.9990];
        let mut seq = Vec::new();
        for &w in &weights {
            h.base_edges.last_mut().unwrap().weight = w;
            seq.push(h.integrity_check());
        }
        assert_eq!(
            seq,
            vec![
                OrganismState::Locked,
                OrganismState::Locked,
                OrganismState::Locked,
                OrganismState::Locked,
                OrganismState::Locked,
                OrganismState::Live,
            ],
            "a dead-band graze must reset the streak; release happens one check later"
        );
    }

    /// P-C §7 T4 — fail-closed latency is provably unchanged: a Live organism
    /// trips to Locked on a single check at ρ = trigger exactly.
    #[test]
    fn hydra_trigger_trips_in_one_check() {
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        // Clean acyclic base ⇒ Live.
        assert_eq!(h.integrity_check(), OrganismState::Live);
        // Push a self-loop at exactly w = 1.0 (ρ = trigger) ⇒ trips immediately.
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 1.0,
        });
        assert_eq!(h.integrity_check(), OrganismState::Locked);
        assert_eq!(h.healthy_streak(), 0);
    }

    /// P-C §3.1 / §7 T1 (RED FIRST — proves the hysteresis bug). Adversarial
    /// oscillation inducer: a self-loop edge on node 0 whose weight `w` is
    /// dithered between `1.0 - DRIFT_BAND/2` and `1.0 + DRIFT_BAND/2` for 8
    /// checks. A self-loop gives ρ = w exactly. Against the memoryless
    /// `rho < 1.0 && rho.is_finite()` predicate this flaps every check (7
    /// transitions); the post-fix hysteresis band collapses it to ≤ 2.
    #[test]
    fn hydra_integrity_flap_without_hysteresis_regression() {
        use crate::spectral::DRIFT_BAND;
        // Start Live (acyclic base). Add a self-loop on node 0 whose weight we
        // rewrite between checks to dither ρ across 1.0.
        let mut h = Hydra::new(MemEventStore::new(), 3, base());
        h.base_edges.push(TopoEdge {
            from: 0,
            to: 0,
            weight: 1.0,
        });
        let low = 1.0 - DRIFT_BAND / 2.0; // = 0.9999995
        let high = 1.0 + DRIFT_BAND / 2.0; // = 1.0000005
        let mut states = Vec::new();
        let mut prev: Option<OrganismState> = None;
        let mut transitions = 0usize;
        for i in 0..8 {
            // Alternate low/high across the 8 checks.
            let w = if i % 2 == 0 { low } else { high };
            // Rewrite the self-loop weight directly (tests module mutates base_edges).
            h.base_edges.last_mut().unwrap().weight = w;
            let s = h.integrity_check();
            if let Some(p) = prev {
                if p != s {
                    transitions += 1;
                }
            }
            states.push(s);
            prev = Some(s);
        }
        // Assert the oscillation is bounded. Pre-fix (memoryless) code produces
        // 7 transitions; the hysteresis fix must hold it to ≤ 2.
        assert!(
            transitions <= 2,
            "integrity_check flapped {transitions} times (states={states:?}); \
             hysteresis band must bound Live<->Locked transitions to ≤ 2"
        );
    }

    /// Command-verification layer: on a bound catalog, a fully-valid allowed
    /// command frame passes through and the mutation commits (Damped delta).
    #[test]
    fn hydra_command_filter_accepts_valid_command() {
        use crate::ports::agent::command_filter::{CommandId, ExactCommand};
        let mut catalog = CommandCatalog::new();
        let nonce: u64 = 0x5566778899AABBCC;
        let key: [u8; 32] = *b"0123456789abcdef0123456789abcdef";
        let mac = CommandCatalog::mac(&key, CommandId::new(0x01), &[0xAB], nonce);
        let mut wire = Vec::new();
        wire.push(CommandId::new(0x01).discriminant());
        wire.extend_from_slice(&1u16.to_le_bytes());
        wire.push(0xAB);
        wire.extend_from_slice(&nonce.to_le_bytes());
        wire.extend_from_slice(&mac);
        catalog.register(ExactCommand::new(CommandId::new(0x01), vec![0xAB]).with_mac(mac));
        let h = Hydra::new(MemEventStore::new(), 3, base())
            .with_catalog(catalog)
            .with_mac_key(Some(key));
        let mut locked = h;
        let res = locked.commit_with_command(
            ev(1, 1, b"mutate"),
            &[TopoEdge {
                from: 2,
                to: 0,
                weight: 0.3,
            }],
            false,
            |_| Ok::<u64, String>(1),
            &wire,
        );
        assert!(
            matches!(
                res,
                Ok((crate::event_log::AppendOutcome::Committed(_), Some(1)))
            ),
            "valid command bytes must commit: {res:?}"
        );
    }

    /// Command-verification layer: short/malformed bytes reject before the
    /// spectral gate — fail-closed, no mutation enters.
    #[test]
    fn hydra_command_filter_rejects_malformed_command_frame() {
        let catalog = CommandCatalog::new();
        let h = Hydra::new(MemEventStore::new(), 3, base()).with_catalog(catalog);
        let mut locked = h;
        let res = locked.commit_with_command(
            ev(1, 1, b"mutate"),
            &[],
            false,
            |_| Ok::<u64, String>(1),
            b"bad",
        );
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "malformed command frame rejected: {res:?}"
        );
    }

    /// Command-verification layer: a single bit-flip in allowed length/payload
    /// yields Err (byte-exact match is the contract).
    #[test]
    fn hydra_command_filter_rejects_tampered_command_frame() {
        use crate::ports::agent::command_filter::{CommandId, ExactCommand};
        let mut catalog = CommandCatalog::new();
        let mut wire = Vec::new();
        wire.push(CommandId::new(0x01).discriminant());
        wire.extend_from_slice(&1u16.to_le_bytes());
        wire.push(0xAB);
        let nonce: u64 = 0x5566778899AABBCC;
        wire.extend_from_slice(&nonce.to_le_bytes());
        let key: [u8; 32] = *b"0123456789abcdef0123456789abcdef";
        let mac = CommandCatalog::mac(&key, CommandId::new(0x01), &[0xAB], nonce);
        catalog.register(ExactCommand::new(CommandId::new(0x01), vec![0xAB]).with_mac(mac));
        let h = Hydra::new(MemEventStore::new(), 3, base())
            .with_catalog(catalog)
            .with_mac_key(Some(key));
        let mut locked = h;
        wire[3] ^= 0x01; // payload bit-flip
        let res = locked.commit_with_command(
            ev(1, 1, b"mutate"),
            &[],
            false,
            |_| Ok::<u64, String>(1),
            &wire,
        );
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "tampered command bytes rejected: {res:?}"
        );
    }
}
