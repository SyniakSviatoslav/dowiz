//! evals.rs — E1 benchmark-generation + scoring primitives (VERIFIABLE-COGNITION §3).
//!
//! Metamorphic generation with programmatic oracles: each generator mints fresh
//! (instance, oracle) pairs whose PASS/FAIL is checkable WITHOUT a fixed answer
//! (the metamorphic relation *is* the oracle). A deterministic mint-log + exact
//! duplicate rejection is the structural leakage gate — the cosine-0.9 *semantic*
//! gate (§3.3) requires the embedding bridge and is deferred to an adapter (§7).
//!
//! All generators are seeded (mulberry32) so items are reproducible but varied,
//! and the oracle is a kernel primitive — no LLM judge, no external dependency.
//!
//! ZERO new dependencies (matches the kernel's zero-dep invariant).

use crate::csr::{recall_at_k, Csr};
use crate::kalman::KalmanFilter;
use crate::noether::invariant_drift;
use crate::spectral::spectral_radius;

// ── Deterministic RNG (mulberry32) ─────────────────────────────────────────
// The kernel is zero-dep; this is a 4-line predictable generator so benchmark
// items are reproducible across runs / machines.
struct Lcg {
    s: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg { s: seed | 1 }
    }
    fn next_u32(&mut self) -> u32 {
        self.s = self.s.wrapping_add(0x6D2B79F5);
        let mut z = self.s;
        z = z ^ (z >> 15);
        z = z.wrapping_mul(z | 1);
        z ^= z.wrapping_add(z ^ (z >> 7));
        z = z.wrapping_mul(z | 61);
        (z ^ (z >> 14)) as u32
    }
    fn f64(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64)
    }
    fn range(&mut self, n: usize) -> usize {
        (self.f64() * n as f64) as usize
    }
    /// Fisher–Yates permutation.
    fn perm(&mut self, n: usize) -> Vec<usize> {
        let mut v: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = self.range(i + 1);
            v.swap(i, j);
        }
        v
    }
}

// ── Mint log (structural leakage gate) ─────────────────────────────────────
// §3.3: a held-out item is only scored if minted after the artifact's freeze,
// and a duplicate (semantic or exact) is rejected. The exact-duplicate half is
// implementable with zero deps: FNV-1a over (kind ‖ instance bytes). The
// cosine-0.9 semantic gate is the embedding-bridge analogue (deferred §7).

fn fnv1a_seed(seed: u64, bytes: &[u8]) -> u64 {
    let mut h: u64 = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Deterministic mint ledger. `mint()` returns `None` for a payload it has
/// already seen (leakage rejection); otherwise a monotonic id.
#[derive(Debug, Clone, Default)]
pub struct MintLog {
    counter: u64,
    seen: std::collections::HashSet<(u64, u64)>,
}

impl MintLog {
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns a fresh id, or `None` if this exact (kind, payload) was minted
    /// before. The kind namespace keeps two different MRs from colliding. A
    /// 128-bit (two independent FNV-1a streams) key makes cross-kind accidental
    /// collisions astronomically unlikely without storing the payloads.
    pub fn mint(&mut self, kind: &str, payload: &[u8]) -> Option<u64> {
        let h1 = fnv1a_seed(0xcbf29ce484222325, kind.as_bytes());
        let h2 = fnv1a_seed(0x84222325cbf29ce4, kind.as_bytes());
        // Fold the payload into both streams via a single pass (different
        // per-byte shifts ⇒ two decorrelated hashes).
        let mut a = h1;
        let mut b = h2;
        for &x in payload {
            a ^= x as u64;
            a = a.wrapping_mul(0x100000001b3);
            b = b.wrapping_mul(0x100000001b3);
            b ^= (x as u64).rotate_left(3);
        }
        let key = (a, b);
        if self.seen.contains(&key) {
            return None;
        }
        self.seen.insert(key);
        self.counter += 1;
        Some(self.counter)
    }
    pub fn count(&self) -> u64 {
        self.counter
    }
}

// ── Metamorphic item + generator ───────────────────────────────────────────

/// A single metamorphic benchmark item: a kind tag, a mint id (0 = rejected
/// duplicate), and whether the oracle (the MR) held.
#[derive(Debug, Clone, PartialEq)]
pub struct MrItem {
    pub id: u64,
    pub kind: &'static str,
    /// True iff the metamorphic relation held for this instance.
    pub passed: bool,
}

/// Mints fresh metamorphic instances whose oracle is a kernel primitive.
pub struct MetamorphicGenerator {
    rng: Lcg,
    mint: MintLog,
}

impl MetamorphicGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Lcg::new(seed),
            mint: MintLog::new(),
        }
    }

    pub fn mint_log(&self) -> &MintLog {
        &self.mint
    }

    // MR-1: spectral similarity invariance. A' = P·A·Pᵀ (permute rows+cols by
    // the same permutation) is a similarity transform ⇒ same eigenvalues ⇒
    // same spectral radius. Oracle: |ρ(A) − ρ(A')| ≤ 1e-9. Also cross-checks
    // the eigensolver parity (blueprint §3.1 parity gate).
    pub fn spectral_similarity(&mut self, n: usize) -> Option<MrItem> {
        let mut a: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                a[i][j] = self.rng.f64() * 2.0 - 1.0;
            }
        }
        let perm = self.rng.perm(n);
        let mut ap = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                ap[i][j] = a[perm[i]][perm[j]];
            }
        }
        let rho_a = spectral_radius(&a);
        let rho_ap = spectral_radius(&ap);
        let passed = (rho_a - rho_ap).abs() <= 1e-9;
        let mut bytes = Vec::new();
        for row in &a {
            for v in row {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let id = self.mint.mint("spectral_similarity", &bytes)?;
        Some(MrItem {
            id,
            kind: "spectral_similarity",
            passed,
        })
    }

    // MR-2: Kalman Q/R uniform-scaling law. Scaling BOTH process noise Q and
    // measurement noise R by λ leaves the steady gain unchanged and scales the
    // steady posterior covariance P* by exactly λ (1-D Riccati homogeneity).
    // Oracle: |P*_λ − λ·P*_1| ≤ 1e-4 after warm-up.
    pub fn kalman_q_scaling(&mut self, lambda: f64) -> Option<MrItem> {
        let q = 0.01_f64;
        let r = 1.0_f64;
        let steps = 600;
        let warm = |q: f64, r: f64| -> f64 {
            let mut kf = KalmanFilter::scalar(0.0, 1.0, 1.0, 1.0, q, r);
            for _ in 0..steps {
                kf.predict();
                let _ = kf.update(&[1.0]);
            }
            kf.p.get(0, 0)
        };
        let p_base = warm(q, r);
        let p_scaled = warm(lambda * q, lambda * r);
        let passed = (p_scaled - lambda * p_base).abs() <= 1e-4;
        let mut bytes = lambda.to_le_bytes().to_vec();
        bytes.extend_from_slice(&q.to_le_bytes());
        let id = self.mint.mint("kalman_q_scaling", &bytes)?;
        Some(MrItem {
            id,
            kind: "kalman_q_scaling",
            passed,
        })
    }

    // MR-3a: Noether conservation. A structure-preserving exchange update
    // x_i ← x_i + ε(x_{π(i)} − x_i) conserves Σx exactly. Oracle: invariant
    // drift over `steps` ≤ 1e-6.
    pub fn noether_conserving(&mut self, dim: usize) -> Option<MrItem> {
        let perm = self.rng.perm(dim);
        let eps = 0.2_f64;
        let update = {
            let perm = perm.clone();
            move |x: &[f64]| -> Vec<f64> {
                let mut y = x.to_vec();
                for i in 0..x.len() {
                    y[i] = x[i] + eps * (x[perm[i]] - x[i]);
                }
                y
            }
        };
        let sum = |x: &[f64]| x.iter().sum::<f64>();
        let x0: Vec<f64> = (0..dim).map(|i| (i as f64) * 0.13 + 1.0).collect();
        let drift = invariant_drift(&x0, update, sum, 50);
        let passed = drift <= 1e-6;
        let mut bytes = (dim as u64).to_le_bytes().to_vec();
        for &p in &perm {
            bytes.extend_from_slice(&p.to_le_bytes());
        }
        let id = self.mint.mint("noether_conserving", &bytes)?;
        Some(MrItem {
            id,
            kind: "noether_conserving",
            passed,
        })
    }

    // MR-3b: Noether NEGATION. An update that injects energy (x_i ← x_i + ε)
    // does NOT conserve Σx ⇒ drift must exceed tol. This proves the oracle is
    // not vacuous (blueprint §3 discipline).
    pub fn noether_nonconserving(&mut self, dim: usize) -> Option<MrItem> {
        let eps = 0.2_f64;
        let update = move |x: &[f64]| -> Vec<f64> {
            x.iter().map(|v| v + eps).collect()
        };
        let sum = |x: &[f64]| x.iter().sum::<f64>();
        let x0: Vec<f64> = (0..dim).map(|i| (i as f64) * 0.13 + 1.0).collect();
        let drift = invariant_drift(&x0, update, sum, 50);
        let passed = drift > 1e-6; // oracle: non-conserving ⇒ drift exceeds tol
        let mut bytes = (dim as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0xAB, 0xCD]); // mark as the negation variant
        let id = self.mint.mint("noether_nonconserving", &bytes)?;
        Some(MrItem {
            id,
            kind: "noether_nonconserving",
            passed,
        })
    }

    // MR-4: recall with constructed relevance. Build a graph where node 0 is the
    // seed and a tight cluster {0,1..=m} is strongly linked (weight 10), plus a
    // weak distractor (weight 0.001). By construction PPR from node 0 must rank
    // the whole cluster in the top-(m+1) ⇒ recall_at_k(rel, m+1) == 1.0.
    pub fn recall_constructed(&mut self, m: usize) -> Option<MrItem> {
        let n = m + 2;
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();
        // strong bidirectional cluster
        for j in 1..=m {
            edges.push((0, j, 10.0));
            edges.push((j, 0, 10.0));
            edges.push((j, j, 1.0)); // mild self-loop for stability
        }
        edges.push((0, 0, 1.0));
        // weak distractor
        let weak = m + 1;
        edges.push((0, weak, 0.001));
        edges.push((weak, 0, 0.001));

        let a = Csr::from_edges(n, &edges).row_normalize();
        let mut seed = vec![0.0; n];
        seed[0] = 1.0;
        let pi = a.personalized_pagerank(&seed, 0.15, 120);

        let relevant: Vec<usize> = (0..=m).collect();
        let recall = recall_at_k(&pi, &relevant, m + 1);
        let passed = (recall - 1.0).abs() <= 1e-6;
        let bytes = (m as u64).to_le_bytes().to_vec();
        let id = self.mint.mint("recall_constructed", &bytes)?;
        Some(MrItem {
            id,
            kind: "recall_constructed",
            passed,
        })
    }
}

// ── Calibration (§3.4) — deterministic ECE / Brier / AURC over kernel outputs ──

/// Brier score: mean squared error of probabilities vs binary outcomes.
pub fn brier(prob: &[f64], outcome: &[u8]) -> f64 {
    assert_eq!(prob.len(), outcome.len());
    if prob.is_empty() {
        return 0.0;
    }
    prob.iter()
        .zip(outcome.iter())
        .map(|(p, &o)| {
            let o = o as f64;
            (p - o) * (p - o)
        })
        .sum::<f64>()
        / prob.len() as f64
}

/// Expected Calibration Error: weighted mean |accuracy − confidence| over
/// `bins` equal-width confidence bins.
pub fn ece(prob: &[f64], outcome: &[u8], bins: usize) -> f64 {
    assert_eq!(prob.len(), outcome.len());
    if prob.is_empty() || bins == 0 {
        return 0.0;
    }
    let bins = bins.max(1);
    let mut acc_sum = vec![0.0f64; bins];
    let mut conf_sum = vec![0.0f64; bins];
    let mut count = vec![0usize; bins];
    for (&p, &o) in prob.iter().zip(outcome.iter()) {
        let mut b = (p * bins as f64) as usize;
        if b >= bins {
            b = bins - 1;
        }
        acc_sum[b] += o as f64;
        conf_sum[b] += p;
        count[b] += 1;
    }
    let mut ece = 0.0;
    let n = prob.len() as f64;
    for i in 0..bins {
        if count[i] == 0 {
            continue;
        }
        let acc = acc_sum[i] / count[i] as f64;
        let conf = conf_sum[i] / count[i] as f64;
        ece += (count[i] as f64 / n) * (acc - conf).abs();
    }
    ece
}

/// Area Under the Risk-Coverage curve. Sort by confidence descending; at each
/// coverage (fraction of most-confident samples) risk = error rate among them.
/// AURC = average risk. Perfect predictor ⇒ 0.
pub fn aurc(prob: &[f64], outcome: &[u8]) -> f64 {
    assert_eq!(prob.len(), outcome.len());
    let n = prob.len();
    if n == 0 {
        return 0.0;
    }
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| prob[b].partial_cmp(&prob[a]).unwrap_or(std::cmp::Ordering::Equal));
    let mut cum_err = 0usize;
    let mut aurc = 0.0;
    for (k, &i) in idx.iter().enumerate() {
        if (prob[i].round() as u8) != outcome[i] {
            cum_err += 1;
        }
        aurc += cum_err as f64 / (k + 1) as f64;
    }
    aurc / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    // E1 proof obligation 1: distinct (kind, params) mints yield fresh, passing items.
    #[test]
    fn emits_fresh_passing_items() {
        let mut g = MetamorphicGenerator::new(0x1234_5678);
        let mut ok = 0;
        // Every parameter is monotonic in `i` so each of the 100 calls is a
        // globally unique (kind, params) item — self-research sweeps the grid.
        for i in 0..20 {
            let s = g.spectral_similarity(4 + i).expect("distinct n for spectral");
            let k = g.kalman_q_scaling(1.0 + 0.1 * (i as f64)).expect("distinct q for kalman");
            let nc = g.noether_conserving(4 + i).expect("distinct dim for conserving");
            let nn = g.noether_nonconserving(4 + i).expect("distinct dim for nonconserving");
            let r = g.recall_constructed(3 + i).expect("distinct m for recall");
            assert!(s.passed, "spectral similarity must hold");
            assert!(k.passed, "kalman Q-scaling must hold");
            assert!(nc.passed, "noether conserving must hold");
            assert!(nn.passed, "noether negation must hold (drift > tol)");
            assert!(r.passed, "constructed recall must be 1.0");
            ok += 5;
        }
        assert_eq!(ok, 100);
        // 20 * 5 = 100 distinct mints, none rejected.
        assert_eq!(g.mint_log().count(), 100, "every item got a unique mint id");
    }

    // E1 proof obligation 1b: identical (kind, params) re-minted → leakage gate
    // rejects (this is the data-leakage guard firing, not a bug).
    #[test]
    fn identical_params_rejected_as_duplicate() {
        let mut g = MetamorphicGenerator::new(0xCAFE);
        let first = g.noether_nonconserving(6).expect("first mint");
        assert!(first.passed);
        // Same kind + same params ⇒ identical payload ⇒ must be rejected.
        for _ in 0..4 {
            assert!(
                g.noether_nonconserving(6).is_none(),
                "re-minting identical params must be blocked by the leakage gate"
            );
        }
        assert_eq!(g.mint_log().count(), 1);
    }

    // E1 proof obligation 2: the leakage gate rejects an EXACT duplicate (same
    // kind + same payload bytes via MintLog directly — the generator is
    // stateful so re-calling would advance the RNG, not reproduce).
    #[test]
    fn leakage_gate_rejects_duplicate() {
        let mut log = MintLog::new();
        let payload = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let a = log.mint("spectral_similarity", &payload).expect("first mint");
        // Identical kind + bytes ⇒ rejected.
        let dup = log.mint("spectral_similarity", &payload);
        assert!(dup.is_none(), "identical payload must be rejected by the mint log");
        assert_eq!(a, 1);
        // Same payload, different kind ⇒ distinct (no cross-kind alias).
        let b = log.mint("kalman_q_scaling", &payload).expect("different kind");
        assert_eq!(b, 2);
        // Different payload, same kind ⇒ distinct.
        let c = log.mint("spectral_similarity", &[9u8, 9, 9]).expect("different payload");
        assert_eq!(c, 3);
    }

    // Calibration: a perfect predictor scores 0 on all three metrics.
    #[test]
    fn calibration_perfect_is_zero() {
        let prob = [0.0, 1.0, 0.0, 1.0, 1.0];
        let outcome = [0, 1, 0, 1, 1];
        assert!((brier(&prob, &outcome) - 0.0).abs() < 1e-12);
        assert!((ece(&prob, &outcome, 10) - 0.0).abs() < 1e-12);
        assert!((aurc(&prob, &outcome) - 0.0).abs() < 1e-12);
    }

    // Brier hand-oracle: p=[0.5,0.5], o=[1,0] ⇒ (0.25 + 0.25)/2 = 0.25.
    #[test]
    fn brier_hand_oracle() {
        let prob = [0.5, 0.5];
        let outcome = [1, 0];
        assert!((brier(&prob, &outcome) - 0.25).abs() < 1e-12);
    }

    // AURC ordering property: a perfect predictor's AURC ≤ a random one's.
    #[test]
    fn aurc_perfect_beats_random() {
        let perfect_p = [0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let perfect_o = [0, 1, 0, 1, 1, 0];
        // Random-looking (anti-correlated) confidences vs outcomes.
        let rand_p = [0.9, 0.9, 0.9, 0.1, 0.1, 0.1];
        let rand_o = [0, 0, 0, 1, 1, 1];
        let a_perfect = aurc(&perfect_p, &perfect_o);
        let a_rand = aurc(&rand_p, &rand_o);
        assert!(a_perfect < a_rand, "perfect AURC {a_perfect} must beat random {a_rand}");
    }
}
