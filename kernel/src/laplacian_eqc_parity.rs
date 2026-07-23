//! ITEM 32 — Part A acceptance #2: the eqc-emitted Laplacian as a THIRD
//! parity leg against the kernel's two existing representations.
//!
//! Ground truth (read before editing): item 36 shipped the shared `IndexSum`
//! IR in `tools/eqc-rs/src/lib.rs` as INTEGER-EXACT ONLY — `&[i8]` arrays, an
//! `i32` accumulator, and `emit_f64_rust`/`Expr::eval` REFUSE indexed nodes
//! (lib.rs:469-471, 588-591, 336-338). The one IR expresses BOTH the Laplacian
//! neighbor-sum AND the quantized-dot inner law (lib.rs:1255-1268). The
//! Laplacian neighbor-sum shape `(Lu)_i = Σ_{j∈N(i)} w_ij (u_i − u_j)` is
//! exactly the per-node integer dot `Σ_k w[k]·(u_i − u_j[k])` over the i-th
//! row's edges — so the eqc `IndexSum` IS the Laplacian, authored as one
//! equation.
//!
//! Because the eqc IR is integer-exact, the parity graph is quantized to `i8`
//! (weights ∈ {0..=3}, field ∈ small integers). For integer data the three
//! representations agree EXACTLY (i8→f64 is exact; every sum is a small exact
//! integer f64), so the assertions below hold at float epsilon (1e-9) — in
//! fact at 0.0. This pins the eqc equation to the SAME combinatorial Laplacian
//! the dense `laplacian()` and `laplacian_spmv` already parity-pinned (item 18):
//! one Laplacian, now THREE representations, all equal.
//!
//! Naming note on the blueprint's "emit_f64_rust": the IR as merged (item 36,
//! 15f3b8df8) is integer-exact, not f64 — `emit_f64_rust` refuses it. We
//! therefore author + EMIT via `emit_int_checked_rust` and EVALUATE via the
//! independent integer-exact interpreter `Expr::eval_int_indexed` (the same
//! referee path item 36's own tests use). All three legs are then compared.

#[cfg(test)]
mod tests {
    use crate::csr::{Csr, LaplacianKind};
    use crate::spectral::laplacian as dense_laplacian;
    use eqc_rs::{Equation, Expr};
    use std::collections::HashMap;

    /// Float tolerance. Integer data ⇒ agreement is exact (0.0); 1e-9 is a
    /// generous float-epsilon guard that also satisfies the blueprint's
    /// "green to float epsilon" criterion.
    const TOL: f64 = 1e-9;

    /// Build the eqc `IndexSum` for node `i`'s neighbor-sum AND emit it (proving
    /// the equation is authorable + emittable by eqc), then return the
    /// independent integer-exact evaluation `Σ_k w[k]·(u_i − u_j[k])`.
    ///
    /// `edges` = `[(w_ij, u_j)]` for the (non-isolated) neighbors of node i, in
    /// ascending column order. `u_i` is the field at node i.
    fn eqc_laplacian_node(edges: &[(i8, i8)], u_i: i8) -> i32 {
        let ne = edges.len();
        if ne == 0 {
            return 0i32;
        }
        let expr = Expr::index_sum(
            "k",
            ne,
            Expr::index("w", Expr::sym("k"))
                * (Expr::sym("u_i") - Expr::index("u_j", Expr::sym("k"))),
        );
        // Emit — exercises the eqc codegen path for this graph's Laplacian.
        let eq = Equation::new(&format!("lap_node"), &["u_i", "u_j"], expr.clone());
        eq.emit_int_checked_rust()
            .expect("eqc must EMIT the integer-exact Laplacian IndexSum");
        // Evaluate via the independent integer-exact interpreter (a SEPARATE
        // code path from the string emitter — the real differential oracle).
        let w: Vec<i8> = edges.iter().map(|&(w, _)| w).collect();
        let uj: Vec<i8> = edges.iter().map(|&(_, uj)| uj).collect();
        let mut arrays = HashMap::new();
        arrays.insert("w".to_string(), w);
        arrays.insert("u_j".to_string(), uj);
        let mut scalars = HashMap::new();
        scalars.insert("u_i".to_string(), u_i as i128);
        expr.eval_int_indexed(&scalars, &arrays, &HashMap::new())
            .expect("eqc integer-exact eval must succeed") as i32
    }

    /// (Lu)_i via the DENSE Laplacian L = D − A, materialized, then mat-vec.
    fn laplacian_dense(adj: &[Vec<f64>], u: &[f64]) -> Vec<f64> {
        let l = dense_laplacian(adj);
        let n = adj.len();
        (0..n)
            .map(|i| (0..n).map(|j| l[i][j] * u[j]).sum())
            .collect()
    }

    /// (Lu)_i via the matrix-free `laplacian_spmv` (CSR, Unnormalized = D − A,
    /// row-sum degree — the SAME operator the dense oracle implements; the only
    /// kind that has a dense counterpart, per csr.rs:1212-1240).
    fn laplacian_spmv(adj: &[Vec<f64>], u: &[f64]) -> Vec<f64> {
        let csr = Csr::from_dense(adj);
        let mut out = vec![0.0; adj.len()];
        csr.laplacian_spmv(u, &mut out, LaplacianKind::Unnormalized);
        out
    }

    /// The eqc third leg, over all nodes of an i8-quantized graph.
    fn laplacian_eqc(adj_i8: &[Vec<i8>], u: &[i8]) -> Vec<i32> {
        let n = adj_i8.len();
        (0..n)
            .map(|i| {
                let edges: Vec<(i8, i8)> = (0..n)
                    .filter(|&j| adj_i8[i][j] != 0)
                    .map(|j| (adj_i8[i][j], u[j]))
                    .collect();
                eqc_laplacian_node(&edges, u[i])
            })
            .collect()
    }

    /// Assert all three representations agree for one (graph, field) pair.
    fn assert_three_way_parity(adj_i8: &[Vec<i8>], u: &[i8]) {
        let n = adj_i8.len();
        // Fed to the kernel as the SAME integer-valued adjacency (0.0/1.0/2.0/3.0
        // are exact f64), so dense + spmv are exact integer math too.
        let adj_f: Vec<Vec<f64>> = adj_i8
            .iter()
            .map(|row| row.iter().map(|&v| v as f64).collect())
            .collect();
        let u_f: Vec<f64> = u.iter().map(|&v| v as f64).collect();

        let dense = laplacian_dense(&adj_f, &u_f);
        let spmv = laplacian_spmv(&adj_f, &u_f);
        let eqc = laplacian_eqc(adj_i8, u);

        for i in 0..n {
            // dense ↔ spmv (item 18's own pin, re-checked under the third leg).
            assert!(
                (dense[i] - spmv[i]).abs() <= TOL,
                "dense vs spmv diverged at node {i}: {} vs {} (graph={:?})",
                dense[i],
                spmv[i],
                adj_i8
            );
            // eqc (integer-exact) ↔ dense (must be integer-valued f64 here).
            let dense_i = dense[i];
            assert!(
                (dense_i - eqc[i] as f64).abs() <= TOL,
                "eqc IndexSum Laplacian != dense laplacian() at node {i}: eqc={} dense={} (graph={:?}, field={:?})",
                eqc[i],
                dense_i,
                adj_i8,
                u
            );
        }
    }

    // ── Acceptance #1 (item 36 IR already merged) + the authorable/emits unit ──
    /// The Laplacian neighbor-sum is authorable as an eqc `Expr` tree, EMITS via
    /// the integer-exact emitter WITHOUT error, and the emitted/evaluated form
    /// matches a hand-computed value.
    #[test]
    fn laplacian_eqc_indexsum_authorable_and_emits() {
        // Triangle node 0, unit weights, field u = [1, 2, 3].
        //   (Lu)_0 = 1·(1−2) + 1·(1−3) = −1 + −2 = −3.
        let edges = vec![(1i8, 2i8), (1i8, 3i8)]; // (w_01, u_1), (w_02, u_2)
        let got = eqc_laplacian_node(&edges, 1i8);
        assert_eq!(
            got, -3i32,
            "eqc Laplacian node-0 must equal hand-computed −3"
        );

        // Also prove the emitted Rust source carries the canonical i32-accumulator
        // IndexSum loop (trip count baked at build time, every step checked).
        let expr = Expr::index_sum(
            "k",
            edges.len(),
            Expr::index("w", Expr::sym("k"))
                * (Expr::sym("u_i") - Expr::index("u_j", Expr::sym("k"))),
        );
        let src = Equation::new("lap_node", &["u_i", "u_j"], expr)
            .emit_int_checked_rust()
            .expect("must emit");
        assert!(
            src.contains("Result<i32, &'static str>"),
            "emitted fn must be the integer-exact i32 form:\n{src}"
        );
        assert!(
            src.contains("for k in 0..2"),
            "trip count must be the build-time edge count (cyclomatic-1 loop):\n{src}"
        );
        assert!(src.contains("checked_add"), "accumulator must be checked");

        // The f64 path MUST honestly refuse the indexed IR (the merged item-36
        // boundary) — acceptance #3's discipline, re-checked from the consumer.
        let eq_f64 = Equation::new(
            "lap_node_f64",
            &["u_i", "u_j"],
            Expr::index_sum(
                "k",
                2,
                Expr::index("w", Expr::sym("k"))
                    * (Expr::sym("u_i") - Expr::index("u_j", Expr::sym("k"))),
            ),
        );
        assert!(
            eq_f64.emit_f64_rust().is_err(),
            "f64 emission MUST refuse the integer-exact IndexSum IR"
        );
    }

    // ── Acceptance #2 (a): EXHAUSTIVE small graphs (N = 1..=5, all 0/1
    //    undirected simple graphs), parity over three representations. ────────
    #[test]
    fn laplacian_eqc_dense_spmv_parity_exhaustive_small() {
        let mut graphs = 0usize;
        for n in 1..=5usize {
            let pairs: Vec<(usize, usize)> = (0..n)
                .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
                .collect();
            let m = pairs.len();
            // Non-constant integer field (avoids trivial all-equal collapse).
            let u: Vec<i8> = (0..n)
                .map(|i| (((i as i64 * 7 + 3) % 11) - 5) as i8)
                .collect();
            for mask in 0..(1u32 << m) {
                let mut adj = vec![vec![0i8; n]; n];
                for (b, &(i, j)) in pairs.iter().enumerate() {
                    if mask & (1u32 << b) != 0 {
                        adj[i][j] = 1;
                        adj[j][i] = 1;
                    }
                }
                assert_three_way_parity(&adj, &u);
                graphs += 1;
            }
        }
        assert_eq!(graphs, 1 + 2 + 8 + 64 + 1024, "exhaustive graph count");
    }

    // ── Acceptance #2 (b): CURATED small graphs (path, star, complete,
    //    disconnected, weighted, self-loop) — integer weights/field. ───────────
    #[test]
    fn laplacian_eqc_dense_spmv_parity_curated() {
        // Path P4, integer weights.
        let path = vec![
            vec![0i8, 1, 0, 0],
            vec![1i8, 0, 1, 0],
            vec![0i8, 1, 0, 1],
            vec![0i8, 0, 1, 0],
        ];
        // Star S4.
        let star = vec![
            vec![0i8, 1, 1, 1],
            vec![1i8, 0, 0, 0],
            vec![1i8, 0, 0, 0],
            vec![1i8, 0, 0, 0],
        ];
        // Complete K4.
        let k4 = vec![
            vec![0i8, 1, 1, 1],
            vec![1i8, 0, 1, 1],
            vec![1i8, 1, 0, 1],
            vec![1i8, 1, 1, 0],
        ];
        // Disconnected: edge {0,1} + triangle {2,3,4}.
        let disconnected = vec![
            vec![0i8, 1, 0, 0, 0],
            vec![1i8, 0, 0, 0, 0],
            vec![0i8, 0, 0, 1, 1],
            vec![0i8, 0, 1, 0, 1],
            vec![0i8, 0, 1, 1, 0],
        ];
        // Weighted + self-loop (A_11 = 2) — proves self-loop cancellation.
        let weighted_selfloop = vec![vec![0i8, 2, 0], vec![3i8, 2, 1], vec![0i8, 0, 0]];
        for adj in [&path, &star, &k4, &disconnected, &weighted_selfloop] {
            let n = adj.len();
            let u: Vec<i8> = (0..n)
                .map(|i| (((i as i64 * 5 + 1) % 9) - 4) as i8)
                .collect();
            assert_three_way_parity(adj, &u);
        }
    }

    // ── Acceptance #2 (c): LARGE RANDOMIZED CORPUS. 500 integer-weighted
    //    graphs across symmetric / symmetric+self-loop / asymmetric lanes with
    //    random integer fields. Fixed-seed LCG ⇒ fully reproducible, zero deps. ──
    #[test]
    fn laplacian_eqc_dense_spmv_parity_random_corpus() {
        let mut state = 0xDEAD_BEEF_1234_5678u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state
        };
        // Uniform integer in [lo, hi].
        let ui = |lo: i64, hi: i64, r: u64| -> i64 {
            let frac = ((r >> 11) as f64) / ((1u64 << 53) as f64);
            lo + ((hi - lo) as f64 * frac).round() as i64
        };

        const CORPUS: usize = 500;
        for _g in 0..CORPUS {
            let n = 1 + (next() % 12) as usize; // n ∈ [1, 12]
            let lane = next() % 3; // 0=symmetric, 1=symmetric+self-loops, 2=asymmetric
            let density = ui(2, 9, next()) as f64 / 10.0; // edge probability
            let mut adj = vec![vec![0i8; n]; n];
            for i in 0..n {
                for j in 0..n {
                    if i == j {
                        if lane == 1 && ui(0, 9, next()) < 3 {
                            adj[i][j] = ui(1, 3, next()) as i8; // self-loop weight 1..3
                        }
                        continue;
                    }
                    if lane == 2 {
                        if ui(0, 9, next()) < (density * 10.0) as i64 {
                            adj[i][j] = ui(1, 3, next()) as i8;
                        }
                    } else if i < j && ui(0, 9, next()) < (density * 10.0) as i64 {
                        let w = ui(1, 3, next()) as i8;
                        adj[i][j] = w;
                        adj[j][i] = w;
                    }
                }
            }
            let u: Vec<i8> = (0..n).map(|_| ui(-8, 8, next()) as i8).collect();
            assert_three_way_parity(&adj, &u);
        }
    }
}
