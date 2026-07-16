//! ppr.rs — Personalized-PageRank (L3) power-iteration engine.
//!
//! Reuses the EXACT deterministic accumulation order of `kernel/src/markov.rs`'s
//! damped-PageRank kernel (its inner `nxt[j] += pii * ((1−α)·W[i][j] …)` loop).
//! We never touch `markov.rs`; we mirror its proven bitwise-reproducible
//! left-product so the same f64 ops land in the same sequence. No
//! eigendecomposition — pure power iteration, the vectorless spectral/relatedness
//! layer for internal-retrieval M3.
//!
//! Determinism proof:
//!   * FIXED iteration count K (no epsilon, no early-out).
//!   * FIXED summation order: i (source) outer, j (target) inner — identical to
//!     `markov.rs`, so floating-point rounding is reproducible.
//!   * Mass is conserved by the restart (Σ nxt = (1−α)·1 + α·1 = 1), so we drop
//!     the per-step `÷ sum` normalization that `markov.rs` needs for a stationary
//!     distribution — removing the only divide and keeping the result bit-exact.

/// Row-stochastic transition matrix W (n·n), stored as dense `Vec<Vec<f64>>`,
/// indexed by integer node id. Built by the caller (see `diffusion`).
pub struct Ppr {
    pub n: usize,
    w: Vec<Vec<f64>>,
}

impl Ppr {
    /// Wrap a pre-built row-stochastic matrix W (each row sums to 1).
    pub fn new(w: Vec<Vec<f64>>) -> Self {
        let n = w.len();
        Self { n, w }
    }

    /// Personalized-PageRank from a one-hot seed.
    ///
    /// Recurrence (mirrors `markov.rs`: fixed K, fixed summation order):
    ///   π₀      = e_seed
    ///   π_{k+1} = (1−α)·(π_k · W)  +  α·e_seed
    ///
    /// The diffusion term uses the SAME inner loop as `markov.rs`
    /// (`nxt[j] += pi[i] * ((1−α)·W[i][j])`, i outer / j inner). The personalized
    /// restart teleports mass back to the seed node (instead of uniformly to all
    /// nodes as `markov.rs` does for its stationary distribution).
    pub fn rank(&self, seed: usize, alpha: f64, k: usize) -> Vec<f64> {
        assert!(seed < self.n, "seed out of range");
        let mut pi = vec![0.0f64; self.n];
        pi[seed] = 1.0;
        for _ in 0..k {
            let mut nxt = vec![0.0f64; self.n];
            // ── SAME accumulation order as markov.rs: i outer, j inner ──
            for i in 0..self.n {
                let pii = pi[i];
                if pii == 0.0 {
                    continue;
                }
                for j in 0..self.n {
                    nxt[j] += pii * ((1.0 - alpha) * self.w[i][j]);
                }
            }
            // personalized restart at the seed (teleport probability α)
            for j in 0..self.n {
                if j == seed {
                    nxt[j] += alpha;
                }
            }
            pi = nxt;
        }
        pi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_graph(n: usize) -> Ppr {
        // 0—1—2—…—(n-1); each node linked to its neighbours.
        let mut w = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            let mut nb = Vec::new();
            if i > 0 {
                nb.push(i - 1);
            }
            if i + 1 < n {
                nb.push(i + 1);
            }
            let inv = 1.0 / nb.len() as f64;
            for &j in &nb {
                w[i][j] = inv;
            }
        }
        Ppr::new(w)
    }

    #[test]
    fn green_mass_conserved_no_normalize() {
        // With a (1−α) diffusion + α restart, total mass stays exactly 1 without
        // any per-step renormalization — the property that buys bitwise determinism.
        let ppr = line_graph(10);
        let scores = ppr.rank(0, 0.15, 20);
        let total: f64 = scores.iter().sum();
        assert!((total - 1.0).abs() < 1e-12, "mass drift = {}", total - 1.0);
    }

    #[test]
    fn green_ppr_byte_identical_across_runs() {
        // Two independent runs with identical (K, α, seed) MUST be byte-identical.
        let ppr = line_graph(20);
        let a = ppr.rank(3, 0.15, 20);
        let b = ppr.rank(3, 0.15, 20);
        assert_eq!(a, b, "PPR is not deterministic across runs");
        // explicit byte-level check via full-precision formatting
        let sa: Vec<String> = a.iter().map(|x| format!("{:.17e}", x)).collect();
        let sb: Vec<String> = b.iter().map(|x| format!("{:.17e}", x)).collect();
        assert_eq!(sa.join(","), sb.join(","), "PPR byte serialization differs");
    }
}
