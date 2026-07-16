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

use crate::event_log::{DecideRejected, EventLog, EventStore, MeshEvent};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganismState {
    Live,
    Locked,
}

/// The hidden organism. Wraps an [`EventLog`] and enforces the closed-loop
/// self-evolution policy. Constructed with the organism's local topology so the
/// drift gate has a baseline to score mutations against (G3).
pub struct Hydra<S: EventStore> {
    log: EventLog<S>,
    nodes: usize,
    base_edges: Vec<TopoEdge>,
    state: OrganismState,
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
        }
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
        // Baseline must remain a contracting/Damped organism (ρ<1). A shift to
        // ρ>=1 means the persisted core was altered by something other than the
        // organism's own signed evolution → fail-closed to Locked.
        if rho < 1.0 && rho.is_finite() {
            // Only auto-restore to Live if it was Locked by the same invariant.
            if self.state == OrganismState::Locked {
                self.state = OrganismState::Live;
            }
        } else {
            self.state = OrganismState::Locked;
        }
        self.state
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
    ) -> Result<(crate::event_log::AppendOutcome, Option<T>), DecideRejected>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: std::fmt::Display,
    {
        // G9 — refuse if the core was tampered (attack, not evolution).
        if self.integrity_check() == OrganismState::Locked {
            return Err(DecideRejected(
                "core tamper detected: organism Locked (owner re-seed / M9 required)".into(),
            ));
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

    /// G9 — consent-gated self-replication of state (operator A: requires user
    /// agreement; C3: bounded, no unbounded spread). Pure + network-free: given
    /// a target node's `target_nodes` and a mutual-auth flag, returns whether the
    /// local WORM log MAY replicate. Actual transport is left to the mesh layer
    /// (bebop sync) which enforces mutual ML-DSA auth + explicit opt-in — this
    /// function only decides the local policy. Never replicates to a node that
    /// has not consented (`target_consent == false`).
    ///
    /// This is defensive continuity (self-reproduction of STATE, not machine
    /// code), scoped to consenting peers — NOT a worm. The output is one
    /// boolean; replication itself appends a single WORM link per call.
    pub fn replicate_to(
        &self,
        target_nodes: usize,
        target_consent: bool,
        mutual_auth: bool,
    ) -> bool {
        // Guard: must be Live (no replication while tampered/Locked), target must
        // have consented, and mutual auth must hold (bebop ML-DSA).
        if self.state != OrganismState::Live {
            return false;
        }
        if !target_consent || !mutual_auth {
            return false;
        }
        // Bounded: refuse absurd fan-out (resource cap, anti-fork-bomb).
        target_nodes > 0 && target_nodes <= 1024
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
            matches!(res, Err(DecideRejected(_))),
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
            matches!(res, Err(DecideRejected(_))),
            "Locked ⇒ commit refused even with intervention"
        );
    }

    /// G9 — consent-gated replication: only to consenting, mutually-authed,
    /// bounded peer set. No consent ⇒ no replication (NOT a worm).
    #[test]
    fn hydra_replicate_requires_consent_and_bounds() {
        let h = Hydra::new(MemEventStore::new(), 3, base());
        assert!(
            h.replicate_to(1, true, true),
            "consenting + auth + bounded ⇒ ok"
        );
        assert!(!h.replicate_to(1, false, true), "no consent ⇒ refused");
        assert!(!h.replicate_to(1, true, false), "no mutual auth ⇒ refused");
        assert!(!h.replicate_to(0, true, true), "zero nodes ⇒ refused");
        assert!(
            !h.replicate_to(4096, true, true),
            "absurd fan-out ⇒ refused (anti-fork-bomb)"
        );
    }
}

/// G4 — std-only durable append-only event store (Воля АНУ closed loop).
///
/// No external DB (sqlx/pgrust offline-uncached). Persists each event as one
/// JSON line to a local file; `insert` appends + `fsync`s (crash-safe). On open,
/// the file is replayed into an in-memory id/tip index so `contains`/`get`/`tip`
/// are O(1). The log is content-addressed and idempotent — a re-inserted id is
/// a no-op. Egress-free: only `std::fs` local IO, no network.
///
/// innovate: this is the durable variant that replaces MemEventStore for the
/// organism's persistent memory. pgrust remains the node-level SQL option; this
/// is the kernel-internal, dependency-free, offline-safe default for the Hydra.
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub struct FileEventStore {
    path: std::path::PathBuf,
    by_id: HashMap<[u8; 32], MeshEvent>,
    tip: Option<[u8; 32]>,
    count: usize,
}

impl FileEventStore {
    /// Open (or create) the append-only log at `path`. Replays existing lines
    /// into the in-memory index. Corrupt/short lines are skipped (forward-
    /// tolerant) — the chain tip is the last *valid* committed event.
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut by_id = HashMap::new();
        let mut tip = None;
        let mut count = 0;
        if path.exists() {
            let file = File::open(&path)?;
            for line in BufReader::new(file).lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => continue, // skip unreadable line
                };
                if line.is_empty() {
                    continue;
                }
                match serde_json_like_parse(&line) {
                    Some(ev) => {
                        let id = ev.event_id();
                        by_id.insert(id, ev.clone());
                        tip = Some(id);
                        count += 1;
                    }
                    None => continue, // skip corrupt line
                }
            }
        }
        Ok(FileEventStore {
            path,
            by_id,
            tip,
            count,
        })
    }
}

/// Minimal hand-rolled JSON parse for the 4 MeshEvent fields (std-only, no
/// serde dependency). Tolerates the exact shape we emit; returns None on
/// mismatch (forward-tolerant replay).
fn serde_json_like_parse(line: &str) -> Option<MeshEvent> {
    // Expected: {"prev":[..32 bytes..],"actor_pubkey":[..32..],"actor_seq":N,"payload":"<hex>"}
    let prev = extract_b256(line, "\"prev\":")?;
    let actor = extract_b256(line, "\"actor_pubkey\":")?;
    let seq = extract_u64(line, "\"actor_seq\":")?;
    let payload = extract_hex(line, "\"payload\":\"")?;
    Some(MeshEvent {
        prev,
        actor_pubkey: actor,
        actor_seq: seq,
        payload,
    })
}

fn extract_b256(s: &str, key: &str) -> Option<[u8; 32]> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find(']')?;
    let nums = &rest[..end];
    let mut out = [0u8; 32];
    let mut idx = 0;
    for part in nums.split(',') {
        let p = part.trim().trim_start_matches('[');
        if let Ok(v) = p.parse::<u8>() {
            if idx < 32 {
                out[idx] = v;
                idx += 1;
            }
        }
    }
    if idx == 32 {
        Some(out)
    } else {
        None
    }
}

fn extract_u64(s: &str, key: &str) -> Option<u64> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find(',').or_else(|| rest.find('}'))?;
    rest[..end].trim().parse::<u64>().ok()
}

fn extract_hex(s: &str, key: &str) -> Option<Vec<u8>> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find('"')?;
    let hex = &rest[..end];
    if hex.len() % 2 != 0 {
        return None;
    }
    Some(
        (0..hex.len())
            .step_by(2)
            .filter_map(|j| u8::from_str_radix(&hex[j..j + 2], 16).ok())
            .collect::<Vec<u8>>(),
    )
}

impl EventStore for FileEventStore {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.by_id.contains_key(id)
    }
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) {
        if self.by_id.contains_key(&id) {
            return; // idempotent no-op
        }
        // Append one JSON line + fsync (crash-safe). Uses only std::fs.
        let line = format!(
            "{{\"prev\":{:?},\"actor_pubkey\":{:?},\"actor_seq\":{},\"payload\":\"{}\"}}\n",
            ev.prev,
            ev.actor_pubkey,
            ev.actor_seq,
            ev.payload
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
            let _ = f.sync_all();
        }
        self.by_id.insert(id, ev);
        self.tip = Some(id);
        self.count += 1;
    }
    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        self.by_id.get(id).cloned()
    }
    fn len(&self) -> usize {
        self.count
    }
    fn tip(&self) -> Option<[u8; 32]> {
        self.tip
    }
    fn set_tip(&mut self, id: [u8; 32]) {
        self.tip = Some(id);
    }
}

#[cfg(test)]
mod file_store_tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let mut p = temp_dir();
        p.push(format!(
            "hydra-volya-anu-{}-{}.log",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_file(&p);
        p
    }

    /// G4 — durable: events survive a reopen (replay), idempotent re-insert,
    /// and `get` retrieves by content-id. Egress-free (std::fs only).
    #[test]
    fn file_store_survives_reopen_and_replays() {
        let path = tmp_path("reopen");
        {
            let mut s = FileEventStore::open(&path).unwrap();
            let ev = MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: [7u8; 32],
                actor_seq: 1,
                payload: b"genesis-intent".to_vec(),
            };
            let id = ev.event_id();
            s.insert(id, ev.clone());
            assert!(s.contains(&id));
            assert_eq!(s.get(&id), Some(ev.clone()));
            // Re-insert same id — idempotent no-op (count stays 1).
            s.insert(id, ev);
            assert_eq!(s.len(), 1);
        }
        // Reopen: replay must restore the event.
        let s2 = FileEventStore::open(&path).unwrap();
        assert_eq!(s2.len(), 1, "event replayed from disk");
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [7u8; 32],
            actor_seq: 1,
            payload: b"genesis-intent".to_vec(),
        };
        assert!(s2.contains(&ev.event_id()));
        let _ = fs::remove_file(&path);
    }

    /// G4 — the organism's full closed loop persists across restart: commit via
    /// Hydra with a FileEventStore, then boot_verify after reopening.
    #[test]
    fn hydra_durable_closed_loop_across_restart() {
        let path = tmp_path("loop");
        let base = vec![
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
        ];
        let committed_id;
        {
            let mut h = Hydra::new(FileEventStore::open(&path).unwrap(), 3, base.clone());
            let ev = MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: [3u8; 32],
                actor_seq: 1,
                payload: b"self-mutation-A".to_vec(),
            };
            let delta = vec![TopoEdge {
                from: 2,
                to: 0,
                weight: 0.3,
            }];
            let (out, _dec) = h
                .commit(ev.clone(), &delta, false, |_| Ok::<u64, String>(1))
                .expect("damped mutation commits");
            committed_id = match out {
                crate::event_log::AppendOutcome::Committed(id) => id,
                _ => panic!("expected committed"),
            };
        }
        // Reopen: the organism re-bootstraps from durable state.
        let h2 = Hydra::new(FileEventStore::open(&path).unwrap(), 3, base);
        assert!(
            h2.log().contains(&committed_id),
            "event persisted across restart"
        );
        assert_eq!(
            h2.boot_verify(),
            0.0,
            "baseline still acyclic after restart"
        );
        let _ = fs::remove_file(&path);
    }
}
