//! attention.rs — scaled dot-product attention as ONE learned-affinity
//! diffusion step (Master-Integration plan, C-tier "attention lens").
//!
//! LENS (the unifying insight, now as code): `softmax(QKᵀ/√d)·V` is a single
//! step of diffusion over a LEARNED affinity matrix `A = softmax(QKᵀ/√d)` —
//! exactly the `f(L)` family the kernel already runs as fixed, multi-step
//! diffusion in `markov` (damped power iteration / PPR) and the retrieval
//! blueprint's heat-kernel. Attention = one step, learned affinity; PPR =
//! many steps, fixed affinity. Same operator family (row-stochastic mixing of
//! value vectors), different affinity source. This module makes that lens a
//! tested organ rather than a prose claim.
//!
//! DETERMINISM: softmax subtracts the row max before exponentiating (numerical
//! stability) and sums in a fixed order — bit-reproducible across native /
//! wasm32. Float is fine here: this is dynamics/affinity, never money.
//!
//! Scope: a reference scalar implementation (no SIMD, no learned weights). The
//! trained-attention path (learning Q/K/V projections) is deliberately NOT
//! here — the kernel stays non-AI (deterministic pure functions); learning
//! lives in `online` / `micrograd` at the edge if ever needed.

/// Numerically-stable, deterministic softmax over a row.
pub fn softmax(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let mut m = xs[0];
    for &x in &xs[1..] {
        if x > m {
            m = x;
        }
    }
    let exps: Vec<f64> = xs.iter().map(|&x| (x - m).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Scaled dot-product attention. `q`, `k`, `v` are row-major matrices:
///   q: [n_q][d], k: [n_k][d], v: [n_k][d_v]  ⇒  out: [n_q][d_v].
/// Returns `None` on any dimension mismatch (fail-closed at the boundary).
pub fn attention(q: &[Vec<f64>], k: &[Vec<f64>], v: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    if k.len() != v.len() || k.is_empty() {
        return None;
    }
    let d = q.first().map(|r| r.len()).unwrap_or(0);
    if d == 0 {
        return None;
    }
    // all key rows share d; all value rows share d_v
    let d_v = v[0].len();
    for kr in k {
        if kr.len() != d {
            return None;
        }
    }
    for vr in v {
        if vr.len() != d_v {
            return None;
        }
    }
    let scale = 1.0 / (d as f64).sqrt();
    let mut out = Vec::with_capacity(q.len());
    for qr in q {
        if qr.len() != d {
            return None;
        }
        // scores = (q · kᵀ) / √d
        let scores: Vec<f64> = k
            .iter()
            .map(|kr| {
                let dot: f64 = qr.iter().zip(kr.iter()).map(|(a, b)| a * b).sum();
                dot * scale
            })
            .collect();
        let w = softmax(&scores);
        // out row = Σ_j w_j · v_j   (fixed summation order → deterministic)
        let mut o = vec![0.0f64; d_v];
        for (j, wj) in w.iter().enumerate() {
            for (oc, vc) in o.iter_mut().zip(v[j].iter()) {
                *oc += wj * vc;
            }
        }
        out.push(o);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// softmax on equal logits → uniform.
    #[test]
    fn softmax_uniform() {
        let w = softmax(&[0.0, 0.0, 0.0]);
        for x in w {
            assert!((x - 1.0 / 3.0).abs() < 1e-12);
        }
    }

    /// HAND ORACLE: q=[ln2], k=[[1],[0]], v=[[1],[0]], d=1.
    /// scores = [ln2, 0]·(1/√1) = [ln2, 0]; softmax = [2/3, 1/3];
    /// out = 2/3·1 + 1/3·0 = 2/3.
    #[test]
    fn attention_hand_oracle() {
        let q = vec![vec![std::f64::consts::LN_2]];
        let k = vec![vec![1.0], vec![0.0]];
        let v = vec![vec![1.0], vec![0.0]];
        let out = attention(&q, &k, &v).unwrap();
        assert_eq!(out.len(), 1);
        assert!((out[0][0] - 2.0 / 3.0).abs() < 1e-12, "got {}", out[0][0]);
    }

    /// Equal keys ⇒ attention is the plain MEAN of the values (uniform
    /// affinity). This is the "one diffusion step over a flat graph" case,
    /// tying the lens back to markov's uniform-teleport limit.
    #[test]
    fn equal_keys_is_mean() {
        let q = vec![vec![0.0, 0.0]];
        let k = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let v = vec![vec![2.0, 4.0], vec![4.0, 8.0]];
        let out = attention(&q, &k, &v).unwrap();
        assert!((out[0][0] - 3.0).abs() < 1e-12);
        assert!((out[0][1] - 6.0).abs() < 1e-12);
    }

    /// Rows of the attention weights sum to 1 (row-stochastic mixing — the
    /// property that makes it the same operator family as PPR).
    #[test]
    fn row_stochastic() {
        let scores = [1.3, -0.7, 2.1, 0.0];
        let w = softmax(&scores);
        let s: f64 = w.iter().sum();
        assert!((s - 1.0).abs() < 1e-12);
    }

    /// Fail-closed: dimension mismatch ⇒ None, never a panic or garbage.
    #[test]
    fn fail_closed_on_mismatch() {
        let q = vec![vec![1.0, 2.0]];
        let k = vec![vec![1.0]]; // wrong d
        let v = vec![vec![1.0]];
        assert!(attention(&q, &k, &v).is_none());
    }

    /// Determinism: two identical calls are bit-identical.
    #[test]
    fn deterministic() {
        let q = vec![vec![0.5, -1.2], vec![2.0, 0.3]];
        let k = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.7, 0.7]];
        let v = vec![vec![1.0], vec![2.0], vec![3.0]];
        let a = attention(&q, &k, &v).unwrap();
        let b = attention(&q, &k, &v).unwrap();
        assert_eq!(a, b);
    }
}
