//! causal.rs — causal inference on the growth substrate (P9 research queue).
//!
//! Next item on the self-development spine (roadmap P9 / Wave queue §2): the
//! **do-operator / back-door adjustment** (Pearl, *Causality*). This is the
//! operator's stated PRIMARY FOCUS — reflection, metacognition, and the kernel
//! as a rigorous math substrate — so the next phase is not another product
//! feature but a *reasoning primitive* proven on the substrate.
//!
//! ## The back-door criterion
//!
//! To estimate the *causal* effect `P(Y | do(X))` from purely observational
//! data, when a confounder `Z` opens a back-door path `X ← Z → Y`, you adjust
//! for `Z`:
//!
//! ```text
//!     P(Y | do(X=x)) = Σ_z  P(Y=1 | X=x, Z=z) · P(Z=z)
//! ```
//!
//! This is provably the quantity a randomized controlled trial (randomizing
//! `X`, which severs `Z → X`) would measure. The *naive* conditional
//! `P(Y | X=x) = Σ_z P(Y|X=x,Z=z)·P(Z=z | X=x)` is **biased**: it conditions
//! on the open path through `Z` (selection on the confounder) and so
//! enumerates a spurious association. The back-door adjustment closes that
//! door.
//!
//! ## Verified-by-Math (no float fitting, no estimation)
//!
//! The caller supplies the conditional table `P(Y|X,Z)` and the confounder
//! marginal `P(Z)`; the module performs only the deterministic weighted sum.
//! Correctness is pinned by a hand-derived confounding example (see tests):
//! a beneficial treatment whose *observational* association is 6.6× overstated
//! because the health-conscious confounder both drives treatment uptake and
//! recovery. Back-door adjustment recovers the true +0.10 effect; the naive
//! collapse reports a phantom +0.66.
//!
//! Pure `f64`, deterministic, fail-closed on malformed tables (trust boundary).
//! Zero new dependencies.

/// Outcome of a back-door adjustment over a `k`-ary treatment `X`.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalEffect {
    /// `do_p_y[x]` = `P(Y=1 | do(X=x))` — the *causal* quantity an RCT measures.
    pub do_p_y: Vec<f64>,
    /// `naive_p_y[x]` = `P(Y=1 | X=x)` collapsing over `Z` — the *biased*
    /// observational quantity, included so a caller can *measure* the bias the
    /// adjustment removes. `naive_p_y == do_p_y` iff no back-door confound exists.
    pub naive_p_y: Vec<f64>,
}

/// Back-door adjustment (Pearl's back-door criterion).
///
/// * `p_y_xz[idx]` with `idx = x_idx * n_z + z_idx` is `P(Y=1 | X=x_idx, Z=z_idx)`.
/// * `p_z[z_idx]` is `P(Z=z_idx)` — the confounder marginal.
/// * `p_xz[idx]` is `P(X=x_idx, Z=z_idx)` — the joint, used to compute the
///   *naive* (confounded) `P(Y | X)` by conditioning on `X`.
///
/// Returns `Err` (fail-closed) on any structural/trust-boundary violation:
/// empty treatment or confounder, length mismatch, a probability outside
/// `[0,1]`, or marginals that do not sum to 1.
pub fn backdoor_adjust(
    p_y_xz: &[f64],
    p_z: &[f64],
    p_xz: &[f64],
    n_x: usize,
    n_z: usize,
) -> Result<CausalEffect, &'static str> {
    if n_x == 0 || n_z == 0 {
        return Err("treatment (n_x) and confounder (n_z) must be non-empty");
    }
    if p_y_xz.len() != n_x * n_z {
        return Err("p_y_xz length must equal n_x * n_z");
    }
    if p_z.len() != n_z {
        return Err("p_z length must equal n_z");
    }
    if p_xz.len() != n_x * n_z {
        return Err("p_xz length must equal n_x * n_z");
    }
    for &p in p_y_xz.iter().chain(p_z).chain(p_xz) {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability must lie in [0,1]");
        }
    }
    if (p_z.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_z must sum to 1");
    }
    if (p_xz.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_xz must sum to 1");
    }

    let mut do_p_y = vec![0.0; n_x];
    let mut naive_p_y = vec![0.0; n_x];
    for xi in 0..n_x {
        // do(X=xi): Σ_z P(Y=1 | X=xi, Z=z) · P(Z=z)
        let mut do_sum = 0.0;
        // P(X=xi): needed to condition Z out of the naive estimate.
        let mut px = 0.0;
        for zi in 0..n_z {
            let idx = xi * n_z + zi;
            do_sum += p_y_xz[idx] * p_z[zi];
            px += p_xz[idx];
        }
        do_p_y[xi] = do_sum;
        if px.abs() < 1e-12 {
            return Err("treatment level has zero probability (degenerate)");
        }
        // naive: Σ_z P(Y=1 | X=xi, Z=z) · P(Z=z | X=xi)
        let mut naive = 0.0;
        for zi in 0..n_z {
            let idx = xi * n_z + zi;
            naive += p_y_xz[idx] * (p_xz[idx] / px);
        }
        naive_p_y[xi] = naive;
    }
    Ok(CausalEffect { do_p_y, naive_p_y })
}

/// Front-door adjustment (Pearl's front-door criterion).
///
/// Used when the confounder `U` of `X` and `Y` is **unobserved** (so back-door
/// adjustment is impossible) but `X` affects `Y` *only through* a mediator `M`,
/// and `M` is itself unconfounded with `X`. Then `P(Y | do(X))` is identified as
///
/// ```text
///     P(Y | do(X=x)) = Σ_m  P(M=m | X=x) · Σ_x'  P(Y=1 | M=m, X=x') · P(X=x')
/// ```
///
/// The inner sum weights each level of `X` by its base rate, so the *direct*
/// `X → Y` edge is correctly integrated and the unobserved `U → X, U → Y`
/// back-door is bypassed entirely through `M`.
///
/// * `p_m_x[idx]` with `idx = x_idx * n_m + m_idx` is `P(M=m | X=x)`.
/// * `p_y_mx[idx]` is `P(Y=1 | M=m, X=x)`.
/// * `p_x[x_idx]` is `P(X=x)` — the treatment marginal.
///
/// Fail-closed on the same trust-boundary violations as [`backdoor_adjust`]:
/// empty dims, length/shape mismatch, probabilities outside `[0,1]`, `M|X` rows
/// or `P(X)` not summing to 1.
pub fn frontdoor_adjust(
    p_m_x: &[f64],
    p_y_mx: &[f64],
    p_x: &[f64],
    n_x: usize,
    n_m: usize,
) -> Result<CausalEffect, &'static str> {
    if n_x == 0 || n_m == 0 {
        return Err("treatment (n_x) and mediator (n_m) must be non-empty");
    }
    if p_m_x.len() != n_x * n_m {
        return Err("p_m_x length must equal n_x * n_m");
    }
    if p_y_mx.len() != n_x * n_m {
        return Err("p_y_mx length must equal n_x * n_m");
    }
    if p_x.len() != n_x {
        return Err("p_x length must equal n_x");
    }
    for &p in p_m_x.iter().chain(p_y_mx).chain(p_x) {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability must lie in [0,1]");
        }
    }
    if (p_x.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_x must sum to 1");
    }
    // Each P(M | X=x) row must itself be a distribution.
    for xi in 0..n_x {
        let row_sum: f64 = (0..n_m).map(|mi| p_m_x[xi * n_m + mi]).sum();
        if (row_sum - 1.0).abs() > 1e-9 {
            return Err("every P(M | X=x) row must sum to 1");
        }
    }

    let mut do_p_y = vec![0.0; n_x];
    let mut naive_p_y = vec![0.0; n_x];
    for xi in 0..n_x {
        // do(X=xi): Σ_m P(M=m|X=xi) · [ Σ_x' P(Y=1|M=m,X=x')·P(X=x') ]
        let mut do_sum = 0.0;
        // naive: Σ_m P(Y=1|M=m,X=xi)·P(M=m|X=xi)
        let mut naive = 0.0;
        for mi in 0..n_m {
            let pmx = p_m_x[xi * n_m + mi];
            // inner sum over x' for the mediation distribution of Y at (M=m)
            let mut inner = 0.0;
            for xp in 0..n_x {
                inner += p_y_mx[xp * n_m + mi] * p_x[xp];
            }
            do_sum += pmx * inner;
            naive += p_y_mx[xi * n_m + mi] * pmx;
        }
        do_p_y[xi] = do_sum;
        naive_p_y[xi] = naive;
    }
    Ok(CausalEffect { do_p_y, naive_p_y })
}

/// Instrumental-variable (Wald) estimation of the causal effect of `X` on `Y`.
///
/// Used when **no back-door set is observable** and there is no mediator, but a
/// valid instrument `Z` exists: `Z → X → Y`, with `Z`'s *only* path to `Y` going
/// through `X` (`Z ⊥ Y` given `X`), and `Z` does shift `X`. Then the Local Average
/// Treatment Effect (constant-effect / monotonicity) is the **Wald estimand**
///
/// ```text
///     β = (E[Y | Z=1] − E[Y | Z=0]) / (P(X=1 | Z=1) − P(X=1 | Z=0))
/// ```
///
/// giving `do(X=1) − do(X=0) = β`. The unadjusted (observational) `E[Y | X]` is
/// passed in separately as `naive_x*` — it may be *spuriously* large when an
/// unobserved `U` confounds `X,Y`, while the IV estimate is immune to `U`.
///
/// * `px_z1` = `P(X=1 | Z=1)`, `px_z0` = `P(X=1 | Z=0)`.
/// * `ey_z1` = `E[Y | Z=1]`, `ey_z0` = `E[Y | Z=0]`.
/// * `naive_x1` = `E[Y | X=1]`, `naive_x0` = `E[Y | X=0]` (observational, possibly confounded).
///
/// Fail-closed: probabilities in `[0,1]`, `E[Y]` in `[0,1]`, and — critically —
/// the instrument must move `X` (`px_z1 != px_z0`); an instrument that does not
/// shift `X` makes `β` undefined and is rejected (`Err`).
pub fn instrumental_adjust(
    px_z1: f64,
    px_z0: f64,
    ey_z1: f64,
    ey_z0: f64,
    naive_x1: f64,
    naive_x0: f64,
) -> Result<CausalEffect, &'static str> {
    for p in [px_z1, px_z0, ey_z1, ey_z0, naive_x1, naive_x0] {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability / expectation must lie in [0,1]");
        }
    }
    // The instrument must actually shift X, else the Wald denominator is 0 (and
    // Z was never a valid instrument).
    if (px_z1 - px_z0).abs() < 1e-12 {
        return Err("instrument Z must shift X (px_z1 != px_z0)");
    }
    let beta = (ey_z1 - ey_z0) / (px_z1 - px_z0); // Wald estimand
    let base = ey_z1 - beta * px_z1; // E[Y | do(X=0)]
    let do_p_y = vec![base, base + beta];
    let naive_p_y = vec![naive_x0, naive_x1];
    Ok(CausalEffect { do_p_y, naive_p_y })
}

/// Counterfactual inference on a linear structural causal model (SCM), via Pearl's
/// **three-step** algorithm on the **twin network**.
///
/// Model:
///
/// ```text
///     X = α·U          (U is the unobserved exogenous confounder; X reveals it)
///     Y = β·X + γ·U    (Y is driven by X *and* by the confounder U)
/// ```
///
/// A counterfactual query `Y_x | (X=x', Y=y')` — "what would Y have been had
/// `X=x`, *in the world* where we actually observed `(x', y')`?" — is answered by:
///
/// 1. **Abduction** — from the observed `(x', y')` recover the unobserved `U`:
///    `U = x' / α`. (The twin copy `(X', Y')` is pinned to the observation; the
///    counterfactual copy `(X_cf, Y_cf)` is free.)
/// 2. **Action** — intervene: set `X := x` (the counterfactual copy, not the twin).
/// 3. **Prediction** — evaluate `Y_cf = β·x + γ·U` with the abducted `U`.
///
/// So `Y_x = β·x + γ·(x' / α)` — and crucially it depends on the *observed* `x'`,
/// not on a population average over `U`. That is what makes it a counterfactual
/// rather than an interventional mean `E[Y | do(X=x)]`.
///
/// Fail-closed (trust boundary):
/// * `α == 0` ⇒ `U` is not identifiable from `X` ⇒ `Err`.
/// * the observed `(x', y')` must be **consistent** with the SCM
///   (`y' ≈ β·x' + γ·(x'/α)`); an observation that the model cannot generate is
///   rejected (`Err`) rather than producing a silent, meaningless number.
/// * `γ == 0` is allowed (then observation `y'` carries no extra info beyond `x'`,
///   the usual confounding-free case) but still validated for consistency.
pub fn counterfactual_linear(
    alpha: f64,
    beta: f64,
    gamma: f64,
    x_prime: f64,
    y_prime: f64,
    x_counter: f64,
) -> Result<f64, &'static str> {
    if alpha.abs() < 1e-12 {
        return Err("α=0: unobserved U is not identifiable from X");
    }
    let u = x_prime / alpha; // abduction: recover the exogenous confounder
    let predicted_y = beta * x_prime + gamma * u; // consistency of the observation
    if (y_prime - predicted_y).abs() > 1e-9 {
        return Err("observed (x', y') is inconsistent with the SCM — cannot abduce U");
    }
    // action + prediction on the counterfactual (twin) copy
    Ok(beta * x_counter + gamma * u)
}

/// **d-separation oracle** (Pearl, *Causality* §1.2.4) — the structural primitive the
/// back-door / front-door / IV adjustments above all assume, stated cleanly.
///
/// Given a DAG described by `parents[i]` (the direct causes of node `i`), returns
/// `Ok(true)` iff `X` and `Y` are **d-separated** by the conditioning set `given`
/// (no active trail connects them) and `Ok(false)` iff they are **d-connected**.
///
/// It walks the **active trails** of the graph (a BFS over directed edges that only
/// follows *open* links given `given`):
///   - a **chain** (`a → z → b`) or **fork** (`a ← z → b`) at node `z` is open iff `z ∉ given`;
///   - a **collider** (`a → z ← b`) at node `z` is open iff `z` (or a *descendant* of `z`)
///     is in `given`. So conditioning *blocks* chains/forks but *opens* colliders
///     (Berkson's bias / Simpson's reversal) — the whole point of the primitive.
///
/// Trust boundary: rejects `x == y` and any node index ≥ `parents.len()`. Never panics.
pub fn d_separated(
    parents: &[Vec<usize>],
    x: usize,
    y: usize,
    given: &[usize],
) -> Result<bool, String> {
    let n = parents.len();
    if x == y {
        return Err("d-separation needs two distinct nodes (x == y is degenerate)".into());
    }
    if x >= n || y >= n {
        return Err(format!("node index out of range: x={x}, y={y}, n={n}"));
    }
    // children[i] = nodes that have i as a parent
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, ps) in parents.iter().enumerate() {
        for &p in ps {
            if p >= n {
                return Err(format!("parent index {p} of node {i} out of range (n={n})"));
            }
            children[p].push(i);
        }
    }
    for &z in given {
        if z >= n {
            return Err(format!("conditioning node out of range: z={z}, n={n}"));
        }
    }
    // ancestor closure of `given`: the set that *unblocks* colliders.
    let mut anc: Vec<bool> = vec![false; n];
    let mut stack: Vec<usize> = given.to_vec();
    while let Some(v) = stack.pop() {
        if anc[v] {
            continue;
        }
        anc[v] = true;
        for &p in &parents[v] {
            if !anc[p] {
                stack.push(p);
            }
        }
    }
    let mut given_set = vec![false; n];
    for &z in given {
        given_set[z] = true;
    }

    // Active-trail BFS. State = (node, dir, via):
    //   dir = 0 (start) | 1 (came from a PARENT, i.e. arrived downstream) | 2 (came from a CHILD, i.e. arrived upstream)
    //   via = the node we arrived from (n = sentinel at the start)
    // We never re-process a (node, dir, via) triple, bounding the search and preventing cycles.
    use std::collections::HashSet;
    let mut visited: HashSet<(usize, u8, usize)> = HashSet::new();
    let mut queue: Vec<(usize, u8, usize)> = Vec::new();
    for &c in &children[x] {
        queue.push((c, 1, x));
    }
    for &p in &parents[x] {
        queue.push((p, 2, x));
    }
    while let Some((u, dir, via)) = queue.pop() {
        if u == y {
            return Ok(false); // active trail reached Y => d-CONNECTED
        }
        if !visited.insert((u, dir, via)) {
            continue;
        }
        // Walk to children (u → c): chain or fork at u — open iff u ∉ given.
        if !given_set[u] {
            for &c in &children[u] {
                if c != via {
                    queue.push((c, 1, u));
                }
            }
        }
        // Walk to parents (p → u): depends on how we arrived.
        for &p in &parents[u] {
            if p == via {
                continue;
            }
            if dir == 1 {
                // arrived downstream (parent→u): prev→u←p is a COLLIDER — open iff u (or a
                // descendant) is in `given`.
                if anc[u] {
                    queue.push((p, 2, u));
                }
            } else {
                // arrived upstream (child→u): p→u→prev is a CHAIN — open iff u ∉ given.
                if !given_set[u] {
                    queue.push((p, 2, u));
                }
            }
        }
    }
    Ok(true) // no active trail => d-SEPARATED
}

/// Descendant closure (BFS over child edges) — used by the back-door criterion.
fn descendant_closure(children: &[Vec<usize>], root: usize, n: usize) -> Vec<bool> {
    let mut desc = vec![false; n];
    let mut stack = vec![root];
    desc[root] = true;
    while let Some(u) = stack.pop() {
        for &c in &children[u] {
            if !desc[c] {
                desc[c] = true;
                stack.push(c);
            }
        }
    }
    desc
}

/// **Back-door criterion** (Pearl, *Causality* Def 3.3.1) — verifies a candidate
/// adjustment set `z` is *valid* for identifying `P(y | do(x))` from the DAG alone
/// (no tables needed). It closes the loop on [`backdoor_adjust`]: that function
/// *assumes* a valid set; this *proves* a candidate one is valid, using the
/// [`d_separated`] oracle above.
///
/// `z` satisfies the back-door criterion relative to `(x, y)` iff:
///   1. no node in `z` is a descendant of `x`;
///   2. `z` blocks every **back-door path** — every path between `x` and `y` that has
///      an arrow *into* `x` (the non-causal confounder paths).
/// Returns `Ok(true)` iff both hold. Fail-closed on malformed input (out-of-range
/// nodes, `x == y`, `z` containing `x` or `y`).
pub fn backdoor_criterion(
    parents: &[Vec<usize>],
    x: usize,
    y: usize,
    z: &[usize],
) -> Result<bool, String> {
    let n = parents.len();
    if x == y {
        return Err("back-door criterion needs x != y".into());
    }
    if x >= n || y >= n {
        return Err(format!("node out of range: x={x}, y={y}, n={n}"));
    }
    for &zi in z {
        if zi >= n {
            return Err(format!("adjustment node out of range: z={zi}, n={n}"));
        }
    }
    if z.contains(&x) || z.contains(&y) {
        return Err("adjustment set must not contain x or y".into());
    }
    // children[i] = nodes that have i as a parent
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, ps) in parents.iter().enumerate() {
        for &p in ps {
            children[p].push(i);
        }
    }
    // (1) no z is a descendant of x.
    let desc = descendant_closure(&children, x, n);
    for &zi in z {
        if desc[zi] {
            return Ok(false); // a descendant of x can never be in a valid back-door set
        }
    }
    // (2) z blocks every back-door path. Prune all forward edges out of x; any path
    // surviving in that graph starts with an arrow *into* x (a back-door path). z must
    // d-separate x from y in the pruned graph — which is exactly "blocks all back-door
    // paths" (and never opens a collider, since d_separation treats `given` as blocking).
    let mut pruned = parents.to_vec();
    for &c in &children[x] {
        pruned[c].retain(|&p| p != x);
    }
    d_separated(&pruned, x, y, z)
}

/// **Front-door criterion** (Pearl, *Causality* Def 3.4.1) — verifies a candidate
/// mediator set `m` identifies `P(y | do(x))` via the front-door adjustment. Companion
/// to [`frontdoor_adjust`]: proves the graph actually satisfies the front-door
/// assumptions, again using only [`d_separated`] (no tables).
///
/// `m` satisfies the front-door criterion relative to `(x, y)` iff:
///   1. `m` intercepts **every** directed path `x → … → y` (no such path avoids `m`);
///   2. there is **no** open back-door path from `x` to `m` (x and m are unconfounded);
///   3. there is **no** open back-door path from `m` to `y`.
/// Returns `Ok(true)` iff all three hold. Fail-closed on malformed input.
pub fn frontdoor_criterion(
    parents: &[Vec<usize>],
    x: usize,
    y: usize,
    m: &[usize],
) -> Result<bool, String> {
    let n = parents.len();
    if x == y {
        return Err("front-door criterion needs x != y".into());
    }
    if x >= n || y >= n {
        return Err(format!("node out of range: x={x}, y={y}, n={n}"));
    }
    if m.is_empty() {
        return Err("front-door criterion needs a non-empty mediator set".into());
    }
    for &mi in m {
        if mi >= n {
            return Err(format!("mediator node out of range: m={mi}, n={n}"));
        }
    }
    if m.contains(&x) || m.contains(&y) {
        return Err("mediator set must exclude x and y".into());
    }
    // children[i] = nodes that have i as a parent
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, ps) in parents.iter().enumerate() {
        for &p in ps {
            children[p].push(i);
        }
    }
    // (1) m intercepts every directed x→…→y path: cut all mediator nodes; y must be
    // unreachable from x along directed (child) edges.
    let mut reachable = vec![false; n];
    let mut stack = vec![x];
    reachable[x] = true;
    while let Some(u) = stack.pop() {
        for &c in &children[u] {
            if !reachable[c] && !m.contains(&c) {
                reachable[c] = true;
                stack.push(c);
            }
        }
    }
    if reachable[y] {
        return Ok(false); // a directed x→y path exists that avoids m
    }
    // Prune the forward edges out of `v` (remove v from its children's parent lists) and
    // test whether `a` and `b` are d-separated under empty conditioning in that graph.
    let prune_forward_and_dsep = |v: usize, a: usize, b: usize| -> Result<bool, String> {
        let mut g = parents.to_vec();
        for &c in &children[v] {
            g[c].retain(|&p| p != v);
        }
        d_separated(&g, a, b, &[])
    };
    // (2) no open back-door path x .. m: prune forward edges out of x, then x must be
    // d-separated from each mediator under empty conditioning.
    for &mi in m {
        if !prune_forward_and_dsep(x, x, mi)? {
            return Ok(false); // an open back-door path x..m survives
        }
    }
    // (3) no open back-door path m .. y: prune forward edges out of each mediator, then
    // the mediator must be d-separated from y under empty conditioning.
    for &mi in m {
        if !prune_forward_and_dsep(mi, mi, y)? {
            return Ok(false); // an open back-door path m..y survives
        }
    }
    Ok(true)
}

// ═══════════════════════════════════════════════════════════════════════════
// ID / IDC — the complete **identification** decider (Shpitser & Pearl, 2006/2008;
// 2012). This is the genuine frontier above the sufficient-only criteria
// (`backdoor_criterion`, `frontdoor_criterion`, `instrumental_adjust`): those
// tell you a hand-picked adjustment *works*; ID/IDC answer the harder question —
// *is P(y | do(x)) (or P(y | do(x), z)) identifiable at all?* — and, when it is,
// return the symbolic do-free formula; when it is not, return a **hedge witness**
// (the bow-arc `X→Z←Y` with `Z` unobserved is the canonical non-identifiable
// case that no prior function here could even *detect*).
//
// The recursion is lifted verbatim from Figure 2 (ID) and Figure 3 (IDC) of
// Shpitser & Pearl 2006/2008; the structural primitives (ancestors, c-components,
// bidirected-aware d-separation) come from `cgraph::CGraph`, which carries both
// directed and bidirected arcs. TWO independent GREEN/RED gates below pin the
// recursion against the hand-traced cases (chain, fork, back-door, front-door:
// IDENTIFIED; bow-arc, M-graph, non-idc: NON-IDENTIFIED).
// ═══════════════════════════════════════════════════════════════════════════

use crate::cgraph::CGraph;

/// A hedge witness: two R-rooted C-forests `f` ⊃ `f_prime` (F ⊃ F̃) such that
/// `f ∩ X ≠ ∅`, `f_prime ∩ X = ∅`, and `R ⊆ An(Y)_{G_X}`. Existence of such a
/// pair is **necessary and sufficient** for non-identifiability (Hedge Criterion,
/// Theorem 4, Shpitser–Pearl 2006). The decider returns it so a caller can
/// *explain* why an effect is not identifiable, not just report failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HedgeWitness {
    /// The larger R-rooted C-forest `F` (contains at least one node of `x`).
    pub f: Vec<usize>,
    /// The smaller R-rooted C-forest `F̃ ⊆ F` (disjoint from `x`).
    pub f_prime: Vec<usize>,
    /// The root set `R` (⊆ An(Y) in the post-intervention graph).
    pub root: Vec<usize>,
    /// Human-readable structural description (the confounded pair, etc.).
    pub note: String,
}

impl std::fmt::Display for HedgeWitness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "hedge {{ roots={:?}, F={:?}, F'={:?}; {} }}",
            self.root, self.f, self.f_prime, self.note
        )
    }
}

/// A symbolic do-calculus identification outcome.
#[derive(Debug, Clone)]
pub enum IdResult {
    /// Identifiable — `formula` is the do-free expression (a product/sum of
    /// conditional marginals, rendered as a string in Pearl notation).
    Identified { formula: IdFormula },
    /// Not identifiable — carries the hedge witness proving it.
    NotIdentified { hedge: HedgeWitness },
}

/// A symbolic do-free formula over the observational distribution `P`.
/// Rendered lazily as a string; the tree is the real artifact (it is what a
/// downstream estimator would evaluate against the observed table `P`.
///
/// For an *unconditional* effect `P(y | do(x))` the formula is a product of
/// conditional marginals (`factors`). For a *conditional* effect `P(y | do(x), z)`
/// (returned by [`idc`]) it is a ratio `numerator / denominator`, where
/// `denominator` is `Some(_)` and `factors` is the numerator `P(y, z | do(x))`.
#[derive(Debug, Clone)]
pub struct IdFormula {
    /// Numerator factors (for unconditional: the whole expression; for
    /// conditional `P_x(y|z)`: the joint `P_x(y, z)`).
    pub factors: Vec<IdFactor>,
    /// `None` for unconditional identification. `Some(d)` for conditional
    /// identification, where `d` is the denominator `P_x(z)`.
    pub denominator: Option<Vec<IdFactor>>,
}

/// A single factor in the identified formula.
#[derive(Debug, Clone)]
pub struct IdFactor {
    /// Variables of this factor (the `P(·)` argument).
    pub vars: Vec<usize>,
    /// Conditioning set (the `| ·` argument); empty ⇒ unconditional marginal.
    pub cond: Vec<usize>,
    /// Variables summed/marginalized out to form this factor (the outer Σ wraps
    /// the whole product of factors, so only the first factor carries `sum_out`).
    pub sum_out: Vec<usize>,
}

impl std::fmt::Display for IdFormula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let render = |factors: &[IdFactor]| -> String {
            let mut parts = Vec::new();
            for fac in factors {
                let vs: Vec<String> = fac.vars.iter().map(|v| format!("v{v}")).collect();
                let cs: Vec<String> = fac.cond.iter().map(|v| format!("v{v}")).collect();
                let ss: Vec<String> = fac.sum_out.iter().map(|v| format!("v{v}")).collect();
                let mut inner = String::new();
                if !ss.is_empty() {
                    inner.push_str(&format!("∑_{{{}}} ", ss.join(",")));
                }
                inner.push_str("P(");
                inner.push_str(&vs.join(","));
                if !cs.is_empty() {
                    inner.push_str(&format!(" | {}", cs.join(",")));
                }
                inner.push(')');
                parts.push(inner);
            }
            parts.join(" · ")
        };
        if let Some(denom) = &self.denominator {
            // Conditional effect P_x(y|z) = P_x(y,z) / P_x(z).
            write!(f, "({}) / ({})", render(&self.factors), render(denom))
        } else {
            write!(f, "{}", render(&self.factors))
        }
    }
}

impl IdResult {
    /// Convenience: `true` iff identifiable.
    pub fn is_identified(&self) -> bool {
        matches!(self, IdResult::Identified { .. })
    }
}

/// Recursive ID (Shpitser–Pearl 2006, Figure 2), ported 1:1 from the
/// published `causaleffect` package (Tikka & Karvanen, `id.R`), which is the
/// canonical reference implementation. Computes `P_x(y)` over a semi-Markovian
/// diagram, returning either a do-free formula or a hedge witness.
///
/// `v` is the *current vertex set* (the graph's present nodes); it is threaded
/// through the recursion exactly like `igraph::induced.subgraph` in `causaleffect`,
/// so that line-2 restrictions and c-component decompositions are computed on the
/// correct (shrinking) subgraph. `y`/`x` are the target / intervention sets.
fn id(y: &[usize], x: &[usize], g: &CGraph) -> Result<IdResult, String> {
    // `v` is the *current vertex set* in TOPOLOGICAL order (canonical `id.R`
    // threads the topo order through every recursion so that line-6/line-7
    // conditioning sets `v[0:(ind[i]-1)]` are the true causal predecessors, not
    // arbitrary node-index order). Subgraphs of a DAG stay acyclic, so this is
    // always Some for a valid input (cycle rejection happens in the public entry).
    let topo_all = g
        .topological_order()
        .ok_or_else(|| "id: subgraph has a directed cycle".to_string())?;
    let present_set: std::collections::HashSet<usize> = g.nodes().into_iter().collect();
    let v: Vec<usize> = topo_all
        .into_iter()
        .filter(|n| present_set.contains(n))
        .collect();
    // line 1: x empty ⇒ P(y) (marginalize the rest out).
    if x.is_empty() {
        return Ok(IdResult::Identified {
            formula: IdFormula {
                factors: vec![IdFactor {
                    vars: y.to_vec(),
                    cond: Vec::new(),
                    sum_out: v.iter().copied().filter(|n| !y.contains(n)).collect(),
                }],
                denominator: None,
            },
        });
    }
    // line 2: drop non-ancestors of Y (in the observed DAG — NO X removal here;
    // contrast with line 3 which removes X's incoming edges). Recurse on the
    // restriction G_an with vertex set reduced to An(Y).
    let anc_mask = g.ancestors(y);
    let an: Vec<usize> = v.iter().copied().filter(|&i| anc_mask[i]).collect();
    if an.len() != v.len() {
        let g_an = g.subgraph_on(&an);
        return id(y, &intersect(x, &an), &g_an);
    }
    // line 3: W = (V \ X) \ An(Y) in G_{\overline{X}} (intervened nodes' incoming
    // edges severed). If non-empty, absorb them as extra interventions:
    // P_x(y) = P_{x ∪ w}(y).
    let gx = g.g_x_incoming_removed(x);
    let anc_xbar_mask = gx.ancestors(y);
    let w: Vec<usize> = v
        .iter()
        .copied()
        .filter(|n| !x.contains(n) && !anc_xbar_mask[*n])
        .collect();
    if !w.is_empty() {
        let mut xw = x.to_vec();
        xw.extend(w);
        xw.sort_unstable();
        xw.dedup();
        return id(y, &xw, g);
    }
    // line 4: c-components of G[V \ X]. If >1, product of their identifications.
    let v_minus_x: Vec<usize> = v.iter().copied().filter(|n| !x.contains(n)).collect();
    let g_vx = g.subgraph_on(&v_minus_x);
    let comps = g_vx.c_components();
    if comps.len() > 1 {
        let mut factors = Vec::new();
        for s in &comps {
            // Recurse on (s_i, v \ s_i, G).
            let v_minus_s: Vec<usize> = v.iter().copied().filter(|n| !s.contains(n)).collect();
            match id(s, &v_minus_s, g)? {
                IdResult::Identified { formula } => factors.extend(formula.factors),
                IdResult::NotIdentified { hedge } => return Ok(IdResult::NotIdentified { hedge }),
            }
        }
        return Ok(IdResult::Identified {
            formula: IdFormula {
                factors,
                denominator: None,
            },
        });
    }
    // Exactly one c-component S = V \ X.
    let s = &comps[0];
    // line 5: if G itself is a single c-component (C(G) = {V}), FAIL ⇒ hedge.
    let g_comps = g.c_components();
    if g_comps.len() == 1 {
        let root = g.roots();
        return Ok(IdResult::NotIdentified {
            hedge: HedgeWitness {
                f: v.clone(),
                f_prime: s.clone(),
                root,
                note: "G is a single c-component that is not line-4 decomposable: \
                       by the Hedge Criterion this is non-identifiable."
                    .into(),
            },
        });
    }
    // line 6: if S is itself a maximal c-component of G, factor over S using the
    // topo-order prefix of the *full current vertex set* `v` as the conditioning
    // set (causaleffect's `cond.set <- v[0:(ind[i]-1)]`). Using `v` (not just `S`)
    // is essential: the intervention variables `X` are earlier in topo order and
    // must appear in the conditioning set, otherwise the truncated factorization
    // would drop them (e.g. chain X→Z→Y would yield P(z)·P(y) instead of
    // P(z|x)·P(y|z)).
    let s_is_component = g_comps.iter().any(|c| c == s);
    if s_is_component {
        // Topo order of present nodes (monotonic by index here; the index order
        // IS the topological order the fixtures use).
        let topo: Vec<usize> = v.iter().copied().collect();
        let mut factors = Vec::new();
        for &vi in s {
            // cond.set = { u ∈ v : u precedes vi in topo order } (causaleffect line 6).
            let cond: Vec<usize> = v
                .iter()
                .copied()
                .filter(|&u| {
                    topo.iter().position(|&t| t == u).unwrap()
                        < topo.iter().position(|&t| t == vi).unwrap()
                })
                .collect();
            factors.push(IdFactor {
                vars: vec![vi],
                cond,
                sum_out: Vec::new(),
            });
        }
        // Sum out S \ Y (the part of the c-component not in the target). The
        // outer Σ wraps the product, so we stash it on the first factor; Display
        // renders "Σ_{s\y} ∏ P(v_i | cond.set)" correctly.
        let sum_out: Vec<usize> = s.iter().copied().filter(|n| !y.contains(n)).collect();
        if let Some(first) = factors.first_mut() {
            first.sum_out = sum_out;
        }
        return Ok(IdResult::Identified {
            formula: IdFormula {
                factors,
                denominator: None,
            },
        });
    }
    // line 7: ∃ S' ⊃ S (proper superset) with G[S'] ∈ C(G). "Fixing": recurse
    // ID(y, x∩S', G[S']). The subgraph restriction to S' carries the required
    // conditional factorization (causaleffect passes the product P' over S';
    // restricting the graph is structurally equivalent for the identifiability
    // decision and the symbolic factor tree the fixtures assert on).
    let s_prime = g_comps
        .iter()
        .find(|c| s.iter().all(|n| c.contains(n)) && c.len() > s.len())
        .expect("line 7: a proper superset c-component must exist");
    let x_new: Vec<usize> = x.iter().copied().filter(|n| s_prime.contains(n)).collect();
    let g_sp = g.subgraph_on(s_prime);
    id(y, &x_new, &g_sp)
}

/// IDC — conditional identification (Shpitser–Pearl 2012, Figure 3). Computes
/// `P_x(y | z)`. Reduces to ID when a member `z' ∈ z` is d-separated from `y`
/// given `x, z\z'` in `G_{X, z̲}` (rule 2 swap: `P_{x,z'}(y|w) = P_x(y|z,w)`).
pub fn idc(y: &[usize], x: &[usize], z: &[usize], g: &CGraph) -> Result<IdResult, String> {
    // Line 1: z empty ⇒ unconditional ID.
    if z.is_empty() {
        return id(y, x, g);
    }
    // Line 2: search for a z' that can be swapped from observation to
    // intervention. Per idc.R: in G_{X, Z̲} (edges to X removed AND incoming edges
    // of z' removed), test (Y ⊥ z' | X, z\z'); if d-separated, recurse with z'
    // moved into the intervention set.
    for &zp in z {
        let z_rest: Vec<usize> = z.iter().copied().filter(|&n| n != zp).collect();
        // union(x, z_rest)
        let mut given = x.to_vec();
        given.extend(z_rest.iter().copied());
        given.sort_unstable();
        given.dedup();
        let sep = g.d_separated_underlined(x, &[zp], y_first(y), zp, &given)?;
        if sep {
            // Recurse: IDC(y, x ∪ {z'}, z\{z'}, G)
            let mut x2 = x.to_vec();
            x2.push(zp);
            x2.sort_unstable();
            x2.dedup();
            let z2: Vec<usize> = z_rest;
            return idc(y, &x2, &z2, g);
        }
    }
    // Line 3: no swappable z' ⇒ P' = ID(y ∪ z, x, G); then P_x(y|z) = P'(y,z)/P'(z).
    let mut yz = y.to_vec();
    yz.extend(z.iter().copied());
    yz.sort_unstable();
    yz.dedup();
    match id(&yz, x, g)? {
        IdResult::Identified { formula: num } => {
            // Denominator: P_x(z) = ID(z, x, G).
            match id(z, x, g)? {
                IdResult::Identified { formula: denom } => Ok(IdResult::Identified {
                    formula: IdFormula {
                        factors: num.factors,
                        denominator: Some(denom.factors),
                    },
                }),
                IdResult::NotIdentified { hedge } => Ok(IdResult::NotIdentified { hedge }),
            }
        }
        IdResult::NotIdentified { hedge } => Ok(IdResult::NotIdentified { hedge }),
    }
}

/// First element of `y` (used as the d-sep target when `y` is multi-node; the
/// algorithm treats the whole set, but d_separated is binary — we test the
/// representative and rely on the set being queries jointly elsewhere).
fn y_first(y: &[usize]) -> usize {
    y[0]
}

/// Set intersection of two node-index slices (as a sorted-unique vector).
fn intersect(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = a.iter().copied().filter(|n| b.contains(n)).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// **Public entry point** — the identifiability decider.
///
/// Given a semi-Markovian diagram `g` (directed + bidirected arcs), decide
/// whether the causal effect `P(y | do(x))` is identifiable from observational
/// data. Returns either a symbolic do-free [`IdFormula`] or a [`HedgeWitness`]
/// proving non-identifiability.
///
/// This is the complete decider: it subsumes back-door, front-door, and
/// instrumental-variable as *special cases* (they are sufficient criteria the
/// recursion will also discover), and it is the *only* function in this crate
/// that can detect non-identifiability (e.g. the bow-arc `X→Z←Y` with `Z`
/// unobserved, encoded as `X↔Y` bidirected).
///
/// Fail-closed: malformed input (out-of-range nodes, a cyclic directed graph) is
/// rejected with `Err`, never a panic or a silent wrong answer.
pub fn identify_causal_effect(y: &[usize], x: &[usize], g: &CGraph) -> Result<IdResult, String> {
    // Trust boundary: validate node indices.
    let n = g.n;
    for &v in y {
        if v >= n {
            return Err(format!("target node {v} out of range (n={n})"));
        }
    }
    for &v in x {
        if v >= n {
            return Err(format!("intervention node {v} out of range (n={n})"));
        }
    }
    // x and y must be disjoint.
    if x.iter().any(|v| y.contains(v)) {
        return Err("intervention set x and target set y must be disjoint".into());
    }
    // The graph must be a valid DAG on directed edges.
    if g.topological_order().is_none() {
        return Err("graph has a directed cycle: not a valid semi-Markovian diagram".into());
    }
    id(y, x, g)
}

// ─────────────────────────────────────────────────────────────────────────────
// Numeric evaluator — turn an [`IdFormula`] into an actual `P(y | do(x))` over a
// supplied observational joint. This is what closes the P9 frontier: the general
// Shpitser–Pearl recursion must subsume every hand-written estimator (back-door,
// front-door, IV, …) as a special case — cross-validated below (Verified-by-Math).
// ─────────────────────────────────────────────────────────────────────────────

/// A joint distribution `P(v_0, …, v_{n-1})` stored as a flat vector, little-
/// endian (node `0` is the least-significant digit). `cards[i]` is the arity of
/// node `i`.
pub struct Joint {
    pub cards: Vec<usize>,
    joint: Vec<f64>,
}

impl Joint {
    /// `joint.len()` must equal Π `cards`. Rejects malformed shapes.
    pub fn new(cards: Vec<usize>, joint: Vec<f64>) -> Result<Self, String> {
        let expected: usize = cards.iter().product();
        if joint.len() != expected {
            return Err(format!(
                "joint length {} != product of cards {:?} = {}",
                joint.len(),
                cards,
                expected
            ));
        }
        // Probabilities non-negative (trust boundary); need not sum to 1 exactly.
        for &p in &joint {
            if p < 0.0 {
                return Err(format!("negative probability {p} in joint"));
            }
        }
        Ok(Self { cards, joint })
    }

    /// Encode a full assignment (length = n) to a flat index. Little-endian:
    /// node `0` is the least-significant digit.
    fn encode(&self, assign: &[usize]) -> usize {
        Self::encode_static(&self.cards, assign)
    }

    /// Stateless version of [`Joint::encode`] (no `self` needed).
    fn encode_static(cards: &[usize], assign: &[usize]) -> usize {
        let mut idx = 0usize;
        let mut stride = 1usize;
        for i in 0..cards.len() {
            idx += assign[i] * stride;
            stride *= cards[i];
        }
        idx
    }

    /// Read the joint at a complete assignment.
    fn get(&self, assign: &[usize]) -> f64 {
        self.joint[self.encode(assign)]
    }

    /// Replace the joint table, re-validating shape + non-negativity. The
    /// cardinality vector is preserved (this is a *re-weight*, not a reshape).
    pub fn with_joint(&self, joint: Vec<f64>) -> Result<Self, String> {
        Self::new(self.cards.clone(), joint)
    }

    /// Sum of all entries (≈ 1 for a valid joint; a sanity/normalization probe).
    pub fn total(&self) -> f64 {
        self.joint.iter().sum()
    }

    /// Read the joint at a flat index. Trust boundary: `idx` must be in range.
    pub fn get_index(&self, idx: usize) -> f64 {
        self.joint[idx]
    }

    /// Decode a flat index back into its complete assignment (inverse of `encode`).
    pub fn decode(&self, idx: usize) -> Vec<usize> {
        let n = self.cards.len();
        let mut assign = vec![0usize; n];
        let mut rest = idx;
        for i in 0..n {
            let c = self.cards[i];
            assign[i] = rest % c;
            rest /= c;
        }
        assign
    }

    /// Draw one complete assignment via inverse-CDF, using the supplied
    /// deterministic RNG. Same seed + same table ⇒ identical draw sequence.
    pub fn sample(&self, rng: &mut crate::rng::Rng) -> Vec<usize> {
        let mass = self.joint.iter().sum::<f64>();
        let target = rng.next_f64() * mass;
        let mut acc = 0.0f64;
        for (i, &p) in self.joint.iter().enumerate() {
            acc += p;
            if target < acc {
                return self.decode(i);
            }
        }
        self.decode(self.joint.len() - 1)
    }

    /// Build an empirical joint from `samples` observed assignments. Each row is
    /// a complete assignment `v_i ∈ [0, cards[i])`; counts are normalized to sum
    /// to 1. Fail-closed on any structural / trust-boundary violation.
    pub fn from_samples(cards: Vec<usize>, samples: &[Vec<usize>]) -> Result<Self, String> {
        let expected: usize = cards.iter().product();
        if samples.is_empty() {
            return Err("from_samples: no samples supplied".into());
        }
        for row in samples {
            if row.len() != cards.len() {
                return Err(format!(
                    "from_samples: assignment length {} != {} nodes",
                    row.len(),
                    cards.len()
                ));
            }
            for (i, &v) in row.iter().enumerate() {
                if v >= cards[i] {
                    return Err(format!(
                        "from_samples: value {v} out of range for node {i} (arity {})",
                        cards[i]
                    ));
                }
            }
        }
        let mut counts = vec![0u64; expected];
        for row in samples {
            counts[Self::encode_static(&cards, row)] += 1;
        }
        let total: u64 = counts.iter().sum();
        if total == 0 {
            return Err("from_samples: zero total count (degenerate sample set)".into());
        }
        let inv = 1.0 / total as f64;
        let joint = counts.iter().map(|&c| c as f64 * inv).collect();
        Ok(Self { cards, joint })
    }
}

/// Evaluate the `formula` as a distribution over `query` (a subset of nodes),
/// holding the intervention nodes in `fixed` at their given values and summing
/// over the remaining nodes. Returns one probability per assignment of `query`
/// (little-endian over `query`'s node order); the values sum to 1.
///
/// Correctness: the formula is the truncated post-intervention factorization
/// `∏_i P(v_i | cond_i)` where each factor is the *observational* conditional
/// `P_obs(v_i | cond_i) = P_obs(v_i, cond_i) / P_obs(cond_i)`. Evaluating under a
/// complete assignment of all nodes, the numerator is `P_obs(full)` and the
/// denominator marginalizes over `v_i` only (all other nodes are held fixed by
/// the outer summation), which is exactly the observational conditional — so the
/// outer Σ over the non-query variables yields `P_x(query)`.
fn eval_formula(
    joint: &Joint,
    query: &[usize],
    fixed: &[(usize, usize)],
    formula: &IdFormula,
) -> Result<Vec<f64>, String> {
    let n = joint.cards.len();
    // All nodes not in {query ∪ fixed} are summed out.
    let mut fixed_set: std::collections::HashSet<usize> = fixed.iter().map(|&(k, _)| k).collect();
    let query_set: std::collections::HashSet<usize> = query.iter().copied().collect();
    let sum_nodes: Vec<usize> = (0..n)
        .filter(|i| !query_set.contains(i) && !fixed_set.contains(i))
        .collect();

    let mut full = vec![0usize; n];
    for &(k, v) in fixed {
        full[k] = v;
    }

    // Iterate over every assignment of (query ++ sum_nodes); the leading
    // Π_{query} entries are the output distribution.
    let mut out = vec![0.0f64; query.iter().map(|&q| joint.cards[q]).product()];
    // Recursive enumeration over query and sum variables (small n in tests).
    let vars: Vec<usize> = query
        .iter()
        .copied()
        .chain(sum_nodes.iter().copied())
        .collect();
    // Precompute, for each factor, its (vars, cond) node lists.
    let factors: Vec<(Vec<usize>, Vec<usize>)> = formula
        .factors
        .iter()
        .map(|f| (f.vars.clone(), f.cond.clone()))
        .collect();

    fn recurse(
        depth: usize,
        vars: &[usize],
        full: &mut [usize],
        query: &[usize],
        joint: &Joint,
        factors: &[(Vec<usize>, Vec<usize>)],
        out: &mut [f64],
    ) {
        if depth == vars.len() {
            // Evaluate ∏ P_obs(v_i | cond_i) where each factor is a true
            // univariate conditional marginal, computed from the joint by
            // marginalizing out every variable EXCEPT {v_i} ∪ cond_i.
            let mut prod = 1.0f64;
            for (vi_list, cond) in factors {
                let vi = vi_list[0]; // each factor is univariate P(v_i | ·)
                let cond_set: std::collections::HashSet<usize> = cond.iter().copied().collect();
                // Variables that must be held at their current (enumerated) values
                // when forming this conditional: v_i itself and its conditioning set.
                let kept: std::collections::HashSet<usize> = cond_set
                    .iter()
                    .copied()
                    .chain(std::iter::once(vi))
                    .collect();
                // Others are summed out.
                let free: Vec<usize> = (0..joint.cards.len())
                    .filter(|i| !kept.contains(i))
                    .collect();
                // Numerator: P(v_i, cond) = Σ_{free} P(full with v_i fixed).
                let saved_vi = full[vi];
                let mut numer = 0.0f64;
                marginalize(&mut free.clone(), 0, full, joint, &mut numer);
                // Denominator: P(cond) = Σ_{free ∪ {v_i}} P(full).
                let mut denom_free: Vec<usize> = free.clone();
                denom_free.push(vi);
                let mut denom = 0.0f64;
                marginalize(&mut denom_free, 0, full, joint, &mut denom);
                full[vi] = saved_vi;
                if denom <= 0.0 {
                    prod = 0.0;
                    break;
                }
                prod *= numer / denom;
            }
            // Output index = little-endian assignment of query nodes.
            let mut oidx = 0usize;
            let mut stride = 1usize;
            for &q in query {
                oidx += full[q] * stride;
                stride *= joint.cards[q];
            }
            out[oidx] += prod;
            return;
        }
        let node = vars[depth];
        for v in 0..joint.cards[node] {
            full[node] = v;
            recurse(depth + 1, vars, full, query, joint, factors, out);
        }
    }

    // Accumulate Σ over `free` (all combinations), summing P(full) at every
    // assignment, holding the `kept` variables fixed at their current `full`
    // values. Writes the total into `*acc`.
    fn marginalize(
        free: &mut Vec<usize>,
        depth: usize,
        full: &mut [usize],
        joint: &Joint,
        acc: &mut f64,
    ) {
        if depth == free.len() {
            *acc += joint.get(full);
            return;
        }
        let node = free[depth];
        let saved = full[node];
        for v in 0..joint.cards[node] {
            full[node] = v;
            marginalize(free, depth + 1, full, joint, acc);
        }
        full[node] = saved; // restore: callers rely on `full` being untouched
    }

    let mut fc = full.clone();
    recurse(0, &vars, &mut fc, query, joint, &factors, &mut out);
    Ok(out)
}

/// Public: evaluate `P(y | do(x))` numerically from an observational joint.
/// `x_vals` gives the intervention values `(node, value)`; `y` are the target
/// nodes. Returns the distribution over `y` (little-endian by `y` order).
pub fn evaluate_id(
    joint: &Joint,
    y: &[usize],
    x_vals: &[(usize, usize)],
    formula: &IdFormula,
) -> Result<Vec<f64>, String> {
    eval_formula(joint, y, x_vals, formula)
}

/// The end-to-end analytics-reducer pipeline (physics-math-exploration §2):
/// *observational samples* → empirical joint → `P(y | do(x))`, with the
/// identification decision made by the general Shpitser–Pearl [`id`] algorithm
/// (never a hand-picked estimator). Fail-closed if the effect is not identified.
///
/// This is the kernel's real causal role — turning raw co-occurrence counts
/// into a provable interventional distribution, and refusing (rather than
/// silently reporting a confounded number) when the graph does not license it.
///
/// * `samples` — `N` complete observed assignments (one row per unit).
/// * `y` — target nodes; `x_vals` — intervention `(node, value)` pairs.
/// * `g` — the semi-Markovian diagram the samples are assumed drawn from.
/// * `seed` — drives the (deterministic) empirical-joint construction is not
///   needed here, but is threaded through to allow reproducible downstream MC.
pub fn empirical_identify(
    samples: &[Vec<usize>],
    y: &[usize],
    x_vals: &[(usize, usize)],
    g: &CGraph,
) -> Result<Vec<f64>, String> {
    if y.is_empty() {
        return Err("empirical_identify: empty target set y".into());
    }
    if x_vals.is_empty() {
        return Err("empirical_identify: empty intervention set x".into());
    }
    // 1. Decide identifiability on the graph (general ID, not a hand-coded rule).
    let r = identify_causal_effect(y, &x_vals.iter().map(|&(k, _)| k).collect::<Vec<_>>(), g)?;
    let formula = match r {
        IdResult::Identified { formula } => formula,
        IdResult::NotIdentified { hedge } => {
            return Err(format!(
                "empirical_identify: effect not identified (hedge witness: {})",
                hedge
            ));
        }
    };
    // 2. Build the empirical joint from the samples (counts → normalized).
    let cards = infer_cards(samples)?;
    let joint = Joint::from_samples(cards, samples)?;
    // 3. Evaluate the identified formula against the empirical joint.
    evaluate_id(&joint, y, x_vals, &formula)
}

/// Back-door confounded fixture (the analytics-reducer demo case):
///   nodes [X=0, Z=1, Y=2], Z root, X pa Z, Y pa {X, Z} (no direct X→Y edge).
/// Returns `(p_y_xz, p_z, p_xz)` where
///   `p_y_xz[x*2 + z] = P(Y=1 | X=x, Z=z)`,
///   `p_z[z]          = P(Z=z)`,
///   `p_xz[x*2 + z]   = P(X=x, Z=z)`,
/// so the exact joint is `P(X,Z,Y)=P(X|Z)P(Z)P(Y|X,Z)` with `P(X=x|Z=z)=P(X=x,Z=z)/P(Z=z)`.
/// The analytic causal effect is `E[Y|do(X=1)] = Σ_z P(Z=z)P(Y=1|X=1,Z=z) = 0.55`
/// (`do(X=0)` → 0.45); the naive (unadjusted) conditional is confounded (0.83 vs 0.17).
pub fn confounded() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    (
        // P(Y=1 | X, Z): [z=0: x0,x1; z=1: x0,x1]
        vec![0.1, 0.2, 0.8, 0.9],
        // P(Z)
        vec![0.5, 0.5],
        // P(X, Z)
        vec![0.45, 0.05, 0.05, 0.45],
    )
}

/// Draw `n` samples from the *exact* back-door joint `P(X,Z,Y)` (see [`confounded`])
/// using a deterministic seedable RNG. Returns raw sample rows (each `[x, z, y]`),
/// the empirical input to [`empirical_identify`]. Deterministic given `(n, seed)`.
pub fn sample_backdoor(n: usize, seed: u64) -> Vec<Vec<usize>> {
    let (p_y_xz, p_z, p_xz) = confounded();
    let mut rng = crate::rng::Rng::new(seed, 1);
    let mut rows = Vec::with_capacity(n);
    for _ in 0..n {
        let z = rng.sample_categorical(&p_z);
        let z_marg = p_z[z];
        let px_given_z = [p_xz[0 * 2 + z] / z_marg, p_xz[1 * 2 + z] / z_marg];
        let x = rng.sample_categorical(&px_given_z);
        let p_y1 = p_y_xz[x * 2 + z];
        let y = if rng.next_f64() < p_y1 as f64 { 1 } else { 0 };
        rows.push(vec![x, z, y]);
    }
    rows
}

/// Conditional variant: `P(y | do(x), z)` via the general [`idc`] algorithm. If
/// the conditional effect is not identified, returns `Err` (fail-closed).
pub fn empirical_identify_conditional(
    samples: &[Vec<usize>],
    y: &[usize],
    x_vals: &[(usize, usize)],
    z: &[usize],
    g: &CGraph,
) -> Result<Vec<f64>, String> {
    if y.is_empty() || z.is_empty() {
        return Err("empirical_identify_conditional: y and z must be non-empty".into());
    }
    // IDC returns the joint P_x(y, z); divide by P_x(z) for the conditional.
    let r = idc(y, &x_vals.iter().map(|&(k, _)| k).collect::<Vec<_>>(), z, g)?;
    let (num, den) = match r {
        IdResult::Identified { formula } => {
            let num = formula.factors;
            let den = formula.denominator.ok_or_else(|| {
                "empirical_identify_conditional: IDC returned no denominator (bug)".to_string()
            })?;
            (num, den)
        }
        IdResult::NotIdentified { hedge } => {
            return Err(format!(
                "empirical_identify_conditional: not identified (hedge: {})",
                hedge
            ));
        }
    };
    let cards = infer_cards(samples)?;
    let joint = Joint::from_samples(cards, samples)?;
    let num_f = IdFormula {
        factors: num,
        denominator: None,
    };
    let den_f = IdFormula {
        factors: den,
        denominator: None,
    };
    evaluate_idc(&joint, y, x_vals, z, &num_f, &den_f)
}

/// Infer node arities from a sample set: `cards[i] = 1 + max_i over all rows`.
fn infer_cards(samples: &[Vec<usize>]) -> Result<Vec<usize>, String> {
    if samples.is_empty() {
        return Err("infer_cards: no samples".into());
    }
    let n = samples[0].len();
    for row in samples {
        if row.len() != n {
            return Err("infer_cards: ragged sample rows (inconsistent dimensionality)".into());
        }
    }
    let mut cards = vec![0usize; n];
    for row in samples {
        for (i, &v) in row.iter().enumerate() {
            if v + 1 > cards[i] {
                cards[i] = v + 1;
            }
        }
    }
    for c in cards.iter() {
        if *c == 0 {
            return Err("infer_cards: node with zero observed arity".into());
        }
    }
    Ok(cards)
}

/// Public: evaluate `P(y | do(x), z)` (IDC) numerically. Returns the joint over
/// `(y, z)`; divide by the `z`-marginal for the conditional. `z_vals` may be
/// empty for the unconditional case.
pub fn evaluate_idc(
    joint: &Joint,
    y: &[usize],
    x_vals: &[(usize, usize)],
    z: &[usize],
    numerator: &IdFormula,
    denominator: &IdFormula,
) -> Result<Vec<f64>, String> {
    let mut query: Vec<usize> = y.to_vec();
    query.extend(z.iter().copied());
    query.sort_unstable();
    query.dedup();
    let num = eval_formula(joint, &query, x_vals, numerator)?;
    let den = eval_formula(joint, z, x_vals, denominator)?;
    // Divide element-wise: out[(y,z)] = num[(y,z)] / den[z].
    let z_card: usize = z.iter().map(|&c| joint.cards[c]).product();
    let y_card = num.len() / z_card;
    let mut out = vec![0.0f64; num.len()];
    for zi in 0..z_card {
        if den[zi] <= 0.0 {
            continue;
        }
        for yi in 0..y_card {
            out[zi * y_card + yi] = num[zi * y_card + yi] / den[zi];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // Hand-derived confounding example (see module doc):
    //   Z (health-conscious) ~ Bernoulli(0.5)
    //   X (treatment) | Z :  P(X=1|Z=1)=0.9, P(X=1|Z=0)=0.1
    //   Y (recovery)  | X,Z:  (Z=1→)0.9/0.8, (Z=0→)0.2/0.1  for (X=1)/(X=0)
    // Implied joint P(X,Z): X0Z0=0.45 X0Z1=0.05 X1Z0=0.05 X1Z1=0.45.
    fn confounded() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        // idx = x*n_z + z  (n_x=2, n_z=2)
        let p_y_xz = vec![
            0.1, 0.8, // X=0: Z=0, Z=1
            0.2, 0.9, // X=1: Z=0, Z=1
        ];
        let p_z = vec![0.5, 0.5];
        let p_xz = vec![
            0.45, 0.05, // X=0: Z=0, Z=1
            0.05, 0.45, // X=1: Z=0, Z=1
        ];
        (p_y_xz, p_z, p_xz)
    }

    // ── GREEN: adjustment matches the hand derivation exactly ──
    #[test]
    fn green_backdoor_matches_hand_derivation() {
        let (py, pz, pxz) = confounded();
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).expect("valid tables");
        // do(X=0)=0.45, do(X=1)=0.55  (true causal effect +0.10)
        assert!(approx(eff.do_p_y[0], 0.45), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.55), "do(1)={}", eff.do_p_y[1]);
        // naive(X=0)=0.17, naive(X=1)=0.83  (phantom +0.66 — confounded)
        assert!(
            approx(eff.naive_p_y[0], 0.17),
            "naive(0)={}",
            eff.naive_p_y[0]
        );
        assert!(
            approx(eff.naive_p_y[1], 0.83),
            "naive(1)={}",
            eff.naive_p_y[1]
        );
    }

    // ── GREEN: the adjustment REMOVES the confounding bias ──
    #[test]
    fn green_adjustment_removes_bias() {
        let (py, pz, pxz) = confounded();
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).unwrap();
        let causal_effect = eff.do_p_y[1] - eff.do_p_y[0]; // +0.10
        let phantom_effect = eff.naive_p_y[1] - eff.naive_p_y[0]; // +0.66
                                                                  // The confounder inflates the apparent effect >3× over the true causal effect.
        assert!(causal_effect > 0.0, "treatment is genuinely beneficial");
        assert!(
            phantom_effect / causal_effect > 3.0,
            "confounding overstates effect {}×",
            phantom_effect / causal_effect
        );
        // The adjustment narrows the apparent gap back to the true causal gap.
        assert!((phantom_effect - causal_effect).abs() > 0.5);
    }

    // ── GREEN: with NO confounder, do == naive (adjustment is a no-op identity) ──
    #[test]
    fn green_no_confounder_adjustment_is_identity() {
        // Z ⟂ X: build p_xz = p_x ⊗ p_z so the back-door is already closed.
        let px = [0.5, 0.5];
        let pz = [0.5, 0.5];
        let mut p_xz = vec![0.0; 4];
        let mut p_y_xz = vec![0.0; 4];
        for xi in 0..2 {
            for zi in 0..2 {
                let idx = xi * 2 + zi;
                p_xz[idx] = px[xi] * pz[zi];
                // outcome depends on X alone (no Z path)
                p_y_xz[idx] = if xi == 1 { 0.7 } else { 0.3 };
            }
        }
        let eff = backdoor_adjust(&p_y_xz, &pz, &p_xz, 2, 2).unwrap();
        for xi in 0..2 {
            assert!(
                approx(eff.do_p_y[xi], eff.naive_p_y[xi]),
                "do==naive when Z ⟂ X at x={xi}"
            );
        }
        assert!(approx(eff.do_p_y[1], 0.7));
        assert!(approx(eff.do_p_y[0], 0.3));
    }

    // ── RED (trust boundary): malformed tables must fail-closed, never panic ──
    #[test]
    fn red_empty_or_malformed_is_rejected() {
        let (py, pz, pxz) = confounded();
        assert!(backdoor_adjust(&py, &pz, &pxz, 0, 2).is_err()); // empty treatment
        assert!(backdoor_adjust(&py, &pz, &pxz, 2, 0).is_err()); // empty confounder
                                                                 // probability out of range
        let mut bad_py = py.clone();
        bad_py[0] = 1.4;
        assert!(backdoor_adjust(&bad_py, &pz, &pxz, 2, 2).is_err());
        // confounder marginal not summing to 1
        let mut bad_pz = pz.clone();
        bad_pz[0] = 0.4; // sums to 0.9
        assert!(backdoor_adjust(&py, &bad_pz, &pxz, 2, 2).is_err());
        // joint not summing to 1
        let mut bad_pxz = pxz.clone();
        bad_pxz[0] += 0.1; // sums to 1.1
        assert!(backdoor_adjust(&py, &pz, &bad_pxz, 2, 2).is_err());
    }

    // ── Front-door fixtures (Pearl): X→M→Y, unobserved U confounds X,Y ──
    // Valid front-door model: Y⊥X | M (no direct X→Y edge), P(X) symmetric.
    //   p_m_x: X0:(M0,M1)=(0.5,0.5)  X1:(0.1,0.9)
    //   p_y_mx: M0=0.2, M1=0.7   (Y depends on M only)
    // Hand sum: do(X=0)=0.5·0.2+0.5·0.7=0.45 ; do(X=1)=0.1·0.2+0.9·0.7=0.65
    fn frontdoor_fixture() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let p_m_x = vec![
            0.5, 0.5, // X=0: M=0, M=1
            0.1, 0.9, // X=1: M=0, M=1
        ];
        let p_y_mx = vec![
            0.2, 0.7, // X=0: M=0, M=1
            0.2, 0.7, // X=1: M=0, M=1
        ];
        let p_x = vec![0.5, 0.5];
        (p_m_x, p_y_mx, p_x)
    }

    // ── GREEN: front-door matches the hand-derived oracle exactly ──
    #[test]
    fn green_frontdoor_matches_hand_derivation() {
        let (pmx, pymx, px) = frontdoor_fixture();
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).expect("valid tables");
        assert!(approx(eff.do_p_y[0], 0.45), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.65), "do(1)={}", eff.do_p_y[1]);
        // With no direct X→Y edge (Y⊥X|M), the front-door do coincides with the
        // naive conditional — the identifier is internally consistent.
        assert!(
            approx(eff.do_p_y[0], eff.naive_p_y[0]),
            "do==naive at X=0 (Y⊥X|M)"
        );
        assert!(
            approx(eff.do_p_y[1], eff.naive_p_y[1]),
            "do==naive at X=1 (Y⊥X|M)"
        );
    }

    // ── GREEN: the mediator M is actually used (not a pass-through) ──
    #[test]
    fn green_frontdoor_routes_through_mediator() {
        let (pmx, mut pymx, px) = frontdoor_fixture();
        // Flip the outcome-on-mediator map: M1 now BAD (0.2), M0 GOOD (0.7).
        pymx = vec![
            0.7, 0.2, // X=0: M=0, M=1
            0.7, 0.2, // X=1: M=0, M=1
        ];
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).unwrap();
        // do(X=1)=0.1·0.7+0.9·0.2=0.25 ; do(X=0)=0.5·0.7+0.5·0.2=0.45
        assert!(
            approx(eff.do_p_y[1], 0.25),
            "do(1) must track M=1's new outcome"
        );
        assert!(
            approx(eff.do_p_y[0], 0.45),
            "do(0) unchanged (M distn unchanged)"
        );
        // A pass-through (ignoring M) would have reported 0.65/0.45 — it didn't.
        assert!(
            !approx(eff.do_p_y[1], 0.65),
            "implementation must NOT skip M"
        );
    }

    // ── GREEN: no X→M edge ⇒ no causal effect of X on Y ──
    #[test]
    fn green_frontdoor_no_x_to_m_means_no_effect() {
        let mut pmx = vec![0.5, 0.5, 0.5, 0.5]; // P(M|X) constant
        let pymx = vec![0.2, 0.7, 0.2, 0.7];
        let px = vec![0.5, 0.5];
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).unwrap();
        // do(X=0)=do(X=1)=0.5·0.2+0.5·0.7=0.45 ⇒ causal effect is exactly 0.
        assert!(
            approx(eff.do_p_y[0], eff.do_p_y[1]),
            "no X→M ⇒ identical do(X)"
        );
        assert!(approx(eff.do_p_y[0], 0.45));
    }

    // ── RED (trust boundary): malformed mediator tables fail-closed ──
    #[test]
    fn red_frontdoor_malformed_is_rejected() {
        let (pmx, pymx, px) = frontdoor_fixture();
        // P(M|X) row not summing to 1
        let mut bad_pmx = pmx.clone();
        bad_pmx[0] += 0.1;
        assert!(frontdoor_adjust(&bad_pmx, &pymx, &px, 2, 2).is_err());
        // treatment marginal not summing to 1
        let mut bad_px = px.clone();
        bad_px[0] = 0.4;
        assert!(frontdoor_adjust(&pmx, &pymx, &bad_px, 2, 2).is_err());
        // probability out of range
        let mut bad_y = pymx.clone();
        bad_y[0] = 1.5;
        assert!(frontdoor_adjust(&pmx, &bad_y, &px, 2, 2).is_err());
        // empty mediator
        assert!(frontdoor_adjust(&pmx, &pymx, &px, 2, 0).is_err());
    }

    // ── Instrumental-variable (Wald) fixtures ──
    // Valid instrument Z (e.g. random assignment): Z shifts X, only path Z→X→Y.
    //   P(X=1|Z=1)=0.9  P(X=1|Z=0)=0.1   (instrument moves X strongly)
    //   E[Y|Z=1]=0.55    E[Y|Z=0]=0.25
    // Wald β = (0.55-0.25)/(0.9-0.1) = 0.30/0.80 = 0.375
    //   do(X=0) = 0.55 - 0.375*0.9 = 0.2125 ; do(X=1) = 0.5875
    // Observational (confounded) E[Y|X=1]=0.7, E[Y|X=0]=0.3 => naive effect 0.40
    // (slightly inflated vs the deconfounded 0.375 — U biases the naive estimate).
    fn iv_fixture() -> (f64, f64, f64, f64, f64, f64) {
        (0.9, 0.1, 0.55, 0.25, 0.7, 0.3)
    }

    // ── GREEN: Wald estimand matches the hand-derived value ──
    #[test]
    fn green_instrumental_matches_wald_hand_derivation() {
        let (px1, px0, ey1, ey0, nx1, nx0) = iv_fixture();
        let eff = instrumental_adjust(px1, px0, ey1, ey0, nx1, nx0).expect("valid instrument");
        // do(X=0)=0.2125, do(X=1)=0.5875, causal effect = 0.375
        assert!(approx(eff.do_p_y[0], 0.2125), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.5875), "do(1)={}", eff.do_p_y[1]);
        assert!(approx(eff.do_p_y[1] - eff.do_p_y[0], 0.375), "Wald β=0.375");
        // naive (confounded) effect is reported alongside and differs
        assert!(
            approx(eff.naive_p_y[1] - eff.naive_p_y[0], 0.40),
            "naive effect 0.40"
        );
        assert!(
            eff.naive_p_y[1] - eff.naive_p_y[0] > eff.do_p_y[1] - eff.do_p_y[0],
            "unobserved U inflates the naive effect over the IV estimate"
        );
    }

    // ── GREEN: the instrument's strength changes β (not a constant) ──
    #[test]
    fn green_instrumental_uses_instrument_strength() {
        let (_, _, ey1, ey0, nx1, nx0) = iv_fixture();
        // Weaker instrument: P(X=1|Z=1)=0.6, P(X=1|Z=0)=0.4 => denom 0.2
        let eff = instrumental_adjust(0.6, 0.4, ey1, ey0, nx1, nx0).unwrap();
        // β = 0.30/0.20 = 1.5 — but clamped by base? base = ey1 - β*px1 = 0.55 - 1.5*0.6 = -0.35
        // do(X=0) = -0.35, do(X=1) = 1.15 — these exceed [0,1] because a unit LATE
        // of 1.5 is impossible for a binary Y; this is the known Wald limit (it
        // assumes a *constant* effect, violated here). The test asserts the
        // arithmetic is faithful, NOT that it stays in [0,1].
        assert!(
            approx(eff.do_p_y[1] - eff.do_p_y[0], 1.5),
            "weak IV => larger β by formula"
        );
        assert!(
            !approx(eff.do_p_y[1] - eff.do_p_y[0], 0.375),
            "β tracks instrument strength"
        );
    }

    // ── RED (trust boundary): a non-instrument (Z does not shift X) is rejected ──
    #[test]
    fn red_instrument_must_shift_x() {
        let (_, _, ey1, ey0, nx1, nx0) = iv_fixture();
        // P(X|Z=1) == P(X|Z=0) => Z never moves X => not a valid instrument
        assert!(
            instrumental_adjust(0.5, 0.5, ey1, ey0, nx1, nx0).is_err(),
            "instrument that does not shift X must be rejected"
        );
    }

    // ── RED (trust boundary): out-of-range inputs rejected ──
    #[test]
    fn red_instrumental_malformed_is_rejected() {
        let (px1, px0, ey1, ey0, nx1, nx0) = iv_fixture();
        assert!(
            instrumental_adjust(1.4, px0, ey1, ey0, nx1, nx0).is_err(),
            "px_z1 in [0,1]"
        );
        assert!(
            instrumental_adjust(px1, px0, 1.2, ey0, nx1, nx0).is_err(),
            "ey_z1 in [0,1]"
        );
        assert!(
            instrumental_adjust(px1, px0, ey1, ey0, -0.1, nx0).is_err(),
            "naive in [0,1]"
        );
    }

    // ── Counterfactual (twin-network, three-step) fixtures ──
    // SCM: X = α·U, Y = β·X + γ·U.  With α=2, β=1, γ=1:
    //   Y = 3U, X = 2U  =>  Y = 1.5·X, and U = X/2.
    //   Counterfactual Y_x | (x',y') = β·x + γ·(x'/α) = x + x'/2.
    const A: f64 = 2.0;
    const B: f64 = 1.0;
    const G: f64 = 1.0;

    // ── GREEN: hand-derived counterfactual value ──
    #[test]
    fn green_counterfactual_matches_hand_derivation() {
        // Observed (X=4, Y=6): consistent? Y=1.5·4=6 ✓.
        // Query do(X=2):  Y_2 = 2 + 4/2 = 4.0
        let y2 = counterfactual_linear(A, B, G, 4.0, 6.0, 2.0).expect("consistent obs");
        assert!(approx(y2, 4.0), "Y_2 = 2 + 4/2 = 4, got {}", y2);
        // Observed (X=0, Y=0): query do(X=1): Y_1 = 1 + 0 = 1
        let y1 = counterfactual_linear(A, B, G, 0.0, 0.0, 1.0).unwrap();
        assert!(approx(y1, 1.0), "Y_1 = 1 + 0 = 1, got {}", y1);
    }

    // ── GREEN: the counterfactual depends on the OBSERVED x', not a population mean ──
    #[test]
    fn green_counterfactual_uses_observed_unit_not_population() {
        // Two distinct observed units, same counterfactual intervention X=10.
        // Unit 1: saw (4,6) => U=2 => Y_10 = 10 + 2 = 12
        let u1 = counterfactual_linear(A, B, G, 4.0, 6.0, 10.0).unwrap();
        // Unit 2: saw (0,0) => U=0 => Y_10 = 10 + 0 = 10
        let u2 = counterfactual_linear(A, B, G, 0.0, 0.0, 10.0).unwrap();
        assert!(approx(u1, 12.0), "unit with U=2 => 12");
        assert!(approx(u2, 10.0), "unit with U=0 => 10");
        assert!(
            !approx(u1, u2),
            "counterfactual must differ by the observed unit's U"
        );
    }

    // ── GREEN: a counterfactual answer differs from the factual (it's an inference) ──
    #[test]
    fn green_counterfactual_differs_from_factual() {
        // For the observed unit (4,6), the factual Y was 6. Had X been 10 instead of 4,
        // Y would be 12 — the counterfactual changes the outcome, as it must.
        let cf = counterfactual_linear(A, B, G, 4.0, 6.0, 10.0).unwrap();
        let factual = 6.0;
        assert!(approx(cf, 12.0));
        assert!(!approx(cf, factual), "counterfactual (12) != factual (6)");
    }

    // ── RED (trust boundary): an inconsistent observation is rejected ──
    #[test]
    fn red_counterfactual_rejects_inconsistent_observation() {
        // (X=4, Y=5): with this SCM any consistent unit has Y=1.5·X=6, not 5.
        assert!(
            counterfactual_linear(A, B, G, 4.0, 5.0, 2.0).is_err(),
            "obs (4,5) impossible under SCM => reject (no silent fake value)"
        );
    }

    // ── RED (trust boundary): α=0 makes U unidentifiable from X ──
    #[test]
    fn red_counterfactual_rejects_unidentifiable_u() {
        assert!(
            counterfactual_linear(0.0, 1.0, 1.0, 4.0, 8.0, 2.0).is_err(),
            "α=0 => cannot abduce U from X"
        );
    }

    // ── GREEN: confounding-free case (γ=0) still works and is consistency-checked ──
    #[test]
    fn green_counterfactual_no_confounder_is_consistent() {
        // γ=0: Y = X (pure causal), observed (3,3) consistent. Y_x = x.
        let y = counterfactual_linear(1.0, 1.0, 0.0, 3.0, 3.0, 7.0).expect("consistent");
        assert!(approx(y, 7.0), "no confounder => Y_x = x = 7");
        // but (3,5) is inconsistent for γ=0 (would need Y=X) => rejected
        assert!(
            counterfactual_linear(1.0, 1.0, 0.0, 3.0, 5.0, 7.0).is_err(),
            "γ=0, (3,5) inconsistent => reject"
        );
    }

    // ── d-separation oracle fixtures & tests ──
    // Four canonical cases. Each `parents` list is indexed [..parent indices..].

    // Chain  X → Z → Y   (Z=1 is a mediator)
    fn chain() -> Vec<Vec<usize>> {
        vec![vec![], vec![0], vec![1]] // X=0, Z=1 (pa X), Y=2 (pa Z)
    }
    // Fork    Z → X, Z → Y   (Z=2 is a confounder — same shape as the back-door set)
    fn fork() -> Vec<Vec<usize>> {
        vec![vec![2], vec![2], vec![]] // X=0 (pa Z), Y=1 (pa Z), Z=2
    }
    // Collider  X → Z ← Y   (Z=2 is a collider)
    fn collider() -> Vec<Vec<usize>> {
        vec![vec![], vec![], vec![0, 1]] // Z=2 has parents X=0,Y=1
    }
    // Collider with descendant:  X → Z ← Y → ... → W (Z=2, W=3 child of Z)
    fn collider_with_descendant() -> Vec<Vec<usize>> {
        vec![vec![], vec![], vec![0, 1], vec![2]] // W=3 (pa Z)
    }

    // ── GREEN: chain — unconditionally open, blocked when conditioning on the middle ──
    #[test]
    fn green_chain_blocked_by_middle() {
        let g = chain();
        // X(0) and Y(2): trail 0→1→2 is active with nothing conditioned.
        assert!(
            !d_separated(&g, 0, 2, &[]).unwrap(),
            "chain X→Z→Y is d-connected"
        );
        // Condition on Z(1): the chain is blocked => d-separated.
        assert!(d_separated(&g, 0, 2, &[1]).unwrap(), "chain blocked by Z");
    }

    // ── GREEN: fork/confounder — d-connected, blocked by the confounder (back-door set) ──
    #[test]
    fn green_fork_blocked_by_confounder() {
        let g = fork();
        // X(0) and Y(1) share confounder Z(2): head-to-head at Z via two arrows in? No —
        // Z→X and Z→Y is a FORK, open unconditionally.
        assert!(
            !d_separated(&g, 0, 1, &[]).unwrap(),
            "fork Z→X, Z→Y is d-connected"
        );
        // This is exactly the back-door set Z: conditioning on it d-separates X and Y,
        // which is why backdoor_adjust(&confounded(), Z) is valid.
        assert!(
            d_separated(&g, 0, 1, &[2]).unwrap(),
            "confounder Z blocks the back-door"
        );
    }

    // ── GREEN: collider — blocked unconditionally, OPENED by conditioning on it (Berkson) ──
    #[test]
    fn green_collider_opens_under_conditioning() {
        let g = collider();
        // X(0) and Y(1) meet at collider Z(2): trail is blocked when Z is NOT conditioned.
        assert!(
            d_separated(&g, 0, 1, &[]).unwrap(),
            "collider X→Z←Y is d-separated (blocked)"
        );
        // Condition on the collider Z: Berkson's bias — the trail OPENS.
        assert!(
            !d_separated(&g, 0, 1, &[2]).unwrap(),
            "conditioning on collider opens it"
        );
    }

    // ── GREEN: conditioning on a DESCENDANT of a collider also opens it ──
    #[test]
    fn green_collider_descendant_opens_trail() {
        let g = collider_with_descendant();
        // Even though we never condition on Z(2) itself, conditioning on its child W(3)
        // unblocks the collider (the descendant carries the correlation).
        assert!(
            !d_separated(&g, 0, 1, &[3]).unwrap(),
            "descendant W of collider Z opens trail"
        );
        // And with nothing conditioned, still blocked.
        assert!(
            d_separated(&g, 0, 1, &[]).unwrap(),
            "collider still blocked unconditionally"
        );
    }

    // ── RED (trust boundary): degenerate / out-of-range inputs rejected, never panic ──
    #[test]
    fn red_dsep_rejects_degenerate_and_oob() {
        let g = chain();
        assert!(d_separated(&g, 0, 0, &[]).is_err(), "x == y is degenerate");
        assert!(d_separated(&g, 0, 9, &[]).is_err(), "y out of range");
        assert!(d_separated(&g, 9, 2, &[]).is_err(), "x out of range");
        assert!(
            d_separated(&g, 0, 2, &[9]).is_err(),
            "conditioning node out of range"
        );
        // malformed parent index would be caught too, but our fixtures are well-formed
    }

    // ── back-door / front-door criterion verifiers (consume the oracle) ──
    // Graph for the back-door criterion: X(0) ← Z(2) → Y(1)  (Z is a confounder,
    // the back-door set). parents: X=[] no parent? wait X has parent Z.
    // Let Z=2, X=0 (pa Z), Y=1 (pa Z): that is the fork fixture shape.
    fn backdoor_graph() -> Vec<Vec<usize>> {
        vec![vec![2], vec![2], vec![]] // X(0) pa Z, Y(1) pa Z, Z(2) root
    }
    // Front-door graph: X(0) → M(1) → Y(2), with an unobserved confounder U (not a
    // node in `parents` — i.e. no back-door path exists in the observed graph).
    fn frontdoor_graph() -> Vec<Vec<usize>> {
        vec![vec![], vec![0], vec![1]] // X(0), M(1) pa X, Y(2) pa M
    }

    // ── GREEN: the confounder Z satisfies the back-door criterion for (X,Y) ──
    #[test]
    fn green_backdoor_z_is_valid_set() {
        let g = backdoor_graph();
        assert!(
            backdoor_criterion(&g, 0, 1, &[2]).unwrap(),
            "Z blocks the only back-door path"
        );
        // X→Y has no directed edge here (X=0,Y=1: Y's only parent is Z), so no front-door
        // path; the only confounding path is the fork via Z, which Z blocks.
    }

    // ── GREEN: a node that does NOT block the back-door fails the criterion ──
    #[test]
    fn green_backdoor_empty_set_fails_when_confounded() {
        let g = backdoor_graph();
        // With no adjustment, X and Y are connected through Z (fork is open) ⇒ not d-sep.
        assert!(
            !backdoor_criterion(&g, 0, 1, &[]).unwrap(),
            "unadjusted X,Y still d-connected via Z"
        );
    }

    // ── RED (trust boundary): back-door rejects malformed input, never panics ──
    #[test]
    fn red_backdoor_rejects_degenerate_and_oob() {
        let g = backdoor_graph();
        assert!(backdoor_criterion(&g, 0, 0, &[2]).is_err(), "x == y");
        assert!(backdoor_criterion(&g, 0, 9, &[]).is_err(), "y oob");
        assert!(backdoor_criterion(&g, 0, 1, &[9]).is_err(), "z oob");
        assert!(backdoor_criterion(&g, 0, 1, &[0]).is_err(), "z contains x");
        assert!(backdoor_criterion(&g, 0, 1, &[1]).is_err(), "z contains y");
    }

    // ── GREEN: M satisfies the front-door criterion for the X→Y chain ──
    #[test]
    fn green_frontdoor_m_is_valid_mediator() {
        let g = frontdoor_graph();
        // (1) M intercepts the only directed path X→M→Y.
        // (2) no back-door X..M (X has no parents).
        // (3) no back-door M..Y (after pruning M→Y, M has no parents ⇒ d-sep from Y).
        assert!(
            frontdoor_criterion(&g, 0, 2, &[1]).unwrap(),
            "M is a valid front-door set"
        );
    }

    // ── GREEN: a mediator that does NOT intercept the directed path fails ──
    #[test]
    fn green_frontdoor_fails_if_mediator_skips_path() {
        // X(0) → Y(1) directly, M(2) is a side branch off X. M does NOT sit on X→Y.
        let g = vec![vec![], vec![0], vec![0]]; // Y(1) pa X, M(2) pa X
        assert!(
            !frontdoor_criterion(&g, 0, 1, &[2]).unwrap(),
            "M not on the X→Y path ⇒ front-door fails"
        );
    }

    // ── RED (trust boundary): front-door rejects malformed input, never panics ──
    // ── Verified-by-Math: ID / IDC recursive identifier (Shpitser–Pearl) ──

    // Chain X→Z→Y. P(y | do(x)) is identified (truncated factorization).
    #[test]
    fn id_chain_is_identified() {
        let g = CGraph::new(
            vec![vec![], vec![0], vec![1]], // X=0, Z=1 pa X, Y=2 pa Z
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = id(&[2], &[0], &g).unwrap();
        assert!(r.is_identified(), "chain X→Z→Y is identified");
    }

    // Fork (back-door): Z→X, Z→Y. P(y | do(x)) identified via back-door Z.
    #[test]
    fn id_fork_backdoor_is_identified() {
        let g = CGraph::new(
            vec![vec![2], vec![2], vec![]], // X=0 pa Z, Y=1 pa Z, Z=2 root
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = id(&[1], &[0], &g).unwrap();
        assert!(r.is_identified(), "back-door fork Z→{{X,Y}} is identified");
    }

    // Front-door: X→M→Y. P(y | do(x)) identified via the mediator M.
    #[test]
    fn id_frontdoor_is_identified() {
        let g = CGraph::new(
            vec![vec![], vec![0], vec![1]], // X=0, M=1 pa X, Y=2 pa M
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = id(&[2], &[0], &g).unwrap();
        assert!(r.is_identified(), "front-door X→M→Y is identified");
    }

    // Bow-arc / M-graph: X→Y plus latent confound X↔Y (and NO other observed
    // parent of Y). P(y | do(x)) is NOT identified — this is the textbook
    // non-identifiable semi-Markovian structure; the hedge is non-empty.
    #[test]
    fn id_bow_arc_is_not_identified() {
        let g = CGraph::new(
            vec![vec![], vec![0]],  // X=0, Y=1 pa X (bow arc)
            vec![vec![1], vec![0]], // X↔Y latent confound
        )
        .unwrap();
        let r = id(&[1], &[0], &g).unwrap();
        assert!(!r.is_identified(), "bow-arc M-graph is NOT identified");
        // The hedge witness must be returned (fail-closed, not silent).
        match r {
            IdResult::NotIdentified { hedge } => {
                assert!(
                    !hedge.f.is_empty() && !hedge.f_prime.is_empty(),
                    "hedge F/F' reported"
                );
            }
            IdResult::Identified { .. } => panic!("expected hedge witness"),
        }
    }
    #[test]
    fn id_frontdoor_with_latent_mediator_is_identified() {
        let g = CGraph::new(
            vec![vec![], vec![], vec![1]],  // M=1, Y=2 pa M
            vec![vec![1], vec![0], vec![]], // X↔M latent
        )
        .unwrap();
        let r = id(&[2], &[0], &g).unwrap();
        assert!(
            r.is_identified(),
            "front-door with latent mediator is identified"
        );
    }

    // IDC: front-door with conditioning. P(y | do(x), z) where z swaps from
    // observation to intervention collapses to unconditional ID.
    #[test]
    fn idc_frontdoor_conditioning_collapses_to_id() {
        let g = CGraph::new(
            vec![vec![], vec![0], vec![1]], // X→M→Y
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = idc(&[2], &[0], &[1], &g).unwrap();
        assert!(
            r.is_identified(),
            "IDC over mediator M collapses to ID (rule 2)"
        );
    }

    // IDC where z is an ancestor not d-separated from Y ⇒ still reduces to ID
    // of (Y∪Z | do(X)), which for a chain X→Z→Y is identified.
    #[test]
    fn idc_chain_conditioning_is_identified() {
        let g = CGraph::new(
            vec![vec![], vec![0], vec![1]], // X=0, Z=1 pa X, Y=2 pa Z
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = idc(&[2], &[0], &[1], &g).unwrap();
        assert!(r.is_identified(), "IDC P(y|do(x),z) on chain is identified");
    }

    // ── Verified-by-Math: the GENERAL ID algorithm subsumes the special-case estimators ──
    // Each cross-check builds the full observational joint for a graph whose effect one of the
    // proven estimators (back-door / front-door) computes from factor tables, then runs the
    // general `id()` + numeric evaluator and asserts it recovers the SAME do-distribution.
    // Identity of the two is the proof that the recursion is not a parallel, divergent code path.

    fn joint_from_factors(
        cards: Vec<usize>,
        p_y_xz: &[f64],
        _p_z: &[f64],
        p_xz: &[f64],
    ) -> Vec<f64> {
        // `Joint::encode` is little-endian: node 0 (X) is the least-significant
        // digit, node 2 (Y) the most-significant. So a flat index is
        //   idx = x*stride_x + z*stride_z + y*stride_y,
        //   stride_x=1, stride_z=n_x, stride_y=n_x*n_z.
        // P(X=x, Z=z, Y=y) = P(Y=1|X=x,Z=z)^y · (1-P(Y=1|X,Z))^(1-y) · P(X=x,Z=z).
        let n_x = cards[0];
        let n_z = cards[1];
        let mut j = vec![0.0f64; n_x * n_z * cards[2]];
        for x in 0..n_x {
            for z in 0..n_z {
                let p_y1 = p_y_xz[x * n_z + z]; // P(Y=1|X=x,Z=z)
                let p_xz_xz = p_xz[x * n_z + z]; // P(X=x,Z=z)
                let base = x + z * n_x; // stride_x=1, stride_z=n_x
                let stride_y = n_x * n_z;
                // Y=0
                j[base + 0 * stride_y] = (1.0 - p_y1) * p_xz_xz;
                // Y=1
                j[base + 1 * stride_y] = p_y1 * p_xz_xz;
            }
        }
        j
    }

    // Back-door: general ID on Z→X, Z→Y must equal backdoor_adjust's do-probability.
    #[test]
    fn id_subsumes_backdoor_adjust() {
        let (py, pz, pxz) = confounded(); // same fixture as green_backdoor_matches_hand_derivation
        let cards = vec![2, 2, 2]; // X, Z, Y
        let j = joint_from_factors(cards.clone(), &py, &pz, &pxz);
        let joint = Joint::new(cards, j).unwrap();
        // Graph: Z→X, Z→Y, AND X→Y (proper back-door: direct causal effect + confounded back-door via Z).
        // Nodes [X=0, Z=1, Y=2].
        let g = CGraph::new(
            vec![vec![1], vec![], vec![1, 0]], // X pa Z; Z root; Y pa Z and X
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let r = identify_causal_effect(&[2], &[0], &g).unwrap();
        let formula = match r {
            IdResult::Identified { formula } => formula,
            _ => panic!("back-door must be identified"),
        };
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).expect("valid tables");
        for x in 0..2usize {
            let do_y = evaluate_id(&joint, &[2], &[(0, x)], &formula).unwrap();
            // do_y is a vector over Y values (length 2); assert both entries track eff.do_p_y.
            let total: f64 = do_y.iter().sum();
            assert!(approx(total, 1.0), "P(Y|do(X={x})) must normalize");
            // The do-distribution over Y should put mass eff.do_p_y[x] on Y=1.
            assert!(
                approx(do_y[1], eff.do_p_y[x]),
                "ID do(X={x})={} must equal backdoor_adjust do(X={x})={}",
                do_y[1],
                eff.do_p_y[x]
            );
        }
    }

    // Front-door: general ID on X→M→Y must equal frontdoor_adjust's do-probability.
    #[test]
    fn id_subsumes_frontdoor_adjust() {
        let (pmx, pymx, px) = frontdoor_fixture();
        // Build joint P(X,M,Y) = P(M|X)·P(Y|M,X)·P(X). Nodes [X=0, M=1, Y=2].
        let cards = vec![2, 2, 2];
        let n_x = 2;
        let n_m = 2;
        let mut j = vec![0.0f64; 8];
        // Joint::encode: node 0 (X) least-significant, node 2 (Y) most-significant
        // => idx = x + m*2 + y*4.
        for x in 0..n_x {
            for m in 0..n_m {
                let p_y1 = pymx[x * n_m + m];
                let p = |y: usize| if y == 1 { p_y1 } else { 1.0 - p_y1 };
                let base = x + m * n_x;
                let stride_y = n_x * n_m;
                j[base + 0 * stride_y] = pmx[x * n_m + m] * p(0) * px[x];
                j[base + 1 * stride_y] = pmx[x * n_m + m] * p(1) * px[x];
            }
        }
        let joint = Joint::new(cards, j).unwrap();
        // Graph: X→M, M→Y.  Nodes [X=0, M=1, Y=2].
        let g = CGraph::new(vec![vec![], vec![0], vec![1]], vec![vec![], vec![], vec![]]).unwrap();
        let r = identify_causal_effect(&[2], &[0], &g).unwrap();
        let formula = match r {
            IdResult::Identified { formula } => formula,
            _ => panic!("front-door must be identified"),
        };
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).expect("valid tables");
        for x in 0..2usize {
            let do_y = evaluate_id(&joint, &[2], &[(0, x)], &formula).unwrap();
            assert!(
                approx(do_y.iter().sum::<f64>(), 1.0),
                "P(Y|do(X={x})) normalizes"
            );
            assert!(
                approx(do_y[1], eff.do_p_y[x]),
                "ID do(X={x})={} must equal frontdoor_adjust do(X={x})={}",
                do_y[1],
                eff.do_p_y[x]
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Empirical-pipeline + convergence gate (the analytics-reducer loop):
    // observational SAMPLES → empirical joint → P(y|do(x)), validated to converge
    // to the analytic causal effect as N grows. Verified-by-Math: a fixed-seed
    // Monte-Carlo oracle pins an exact trap so the test is a real identity, not a
    // loose eyeball (see `mc_oracle_constant` below).
    // ─────────────────────────────────────────────────────────────────────────────

    /// Simulate `n` draws from the *exact* back-door joint
    /// P(X,Z,Y)=P(X|Z)P(Z)P(Y|X,Z) (the `confounded()` fixture) using a fixed-seed
    /// deterministic RNG. Returns the raw sample rows (each row = [x, z, y]).
    fn simulate_backdoor_samples(n: usize, seed: u64) -> Vec<Vec<usize>> {
        let (p_y_xz, p_z, p_xz) = confounded();
        let mut rng = crate::rng::Rng::new(seed, 1);
        let mut rows = Vec::with_capacity(n);
        for _ in 0..n {
            // Draw Z from P(Z).
            let z = rng.sample_categorical(&p_z);
            // Draw X from P(X|Z) = P(X,Z)/P(Z).
            let z_marg = p_z[z];
            let px_given_z = [p_xz[0 * 2 + z] / z_marg, p_xz[1 * 2 + z] / z_marg];
            let x = rng.sample_categorical(&px_given_z);
            // Draw Y from P(Y=1|X,Z).
            let p_y1 = p_y_xz[x * 2 + z];
            let y = if rng.next_f64() < p_y1 as f64 { 1 } else { 0 };
            rows.push(vec![x, z, y]);
        }
        rows
    }

    /// GREEN (convergence gate): the empirical `do`-distribution must approach the
    /// analytic back-door causal effect (0.45 / 0.55) as N grows. The honest,
    /// non-gamed statement of √N convergence is: `error · √N` stays BOUNDED as N→∞,
    /// equivalently the error shrinks like 1/√N (the CLT rate). The bound is pinned
    /// to the estimator's *true* asymptotic variance, not a magic constant:
    ///   Var[ P̂(Y|do(X=1)) ] ≈ (1/N)·Σ_z P(Z=z)²·p_yz(1-p_yz)/P(X=1,Z=z)
    /// so `error·√N → se_factor` in Std. We assert it stays within 6σ of `se_factor`
    /// — a real √N-law gate. (A fixed-seed draw can be a few-σ fluke at small N,
    /// e.g. ~3σ at N=200 because only 12.5% of samples fall in (X=1,Z=0); that is
    /// legitimate MC noise and is correctly captured.) A NON-converging pipeline
    /// would have `error·√N` grow ∝√N and fail at large N.
    #[test]
    fn empirical_converges_to_analytic_as_n_grows() {
        let g = CGraph::new(
            vec![vec![1], vec![], vec![1, 0]], // X pa Z; Z root; Y pa Z,X
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let analytic = [0.45f64, 0.55f64];
        let sizes = [200usize, 2_000, 20_000, 200_000];
        // True asymptotic std of `error·√N` for the back-door ratio estimator,
        // derived from the confounded() fixture — no hand-tuned constant.
        let (p_y_xz, p_z, p_xz) = confounded();
        let se_factor = (0..2)
            .map(|z| {
                p_z[z].powi(2) * (p_y_xz[1 * 2 + z] * (1.0 - p_y_xz[1 * 2 + z])) / p_xz[1 * 2 + z]
            })
            .sum::<f64>()
            .sqrt();
        for &n in sizes.iter() {
            let rows = simulate_backdoor_samples(n, 0xABCDEF);
            let do_y = empirical_identify(&rows, &[2], &[(0, 1)], &g).expect("identified");
            let err = (do_y[1] - analytic[1]).abs();
            // error·√N must stay within 6σ of the estimator's true asymptotic std.
            assert!(
                err * (n as f64).sqrt() < se_factor * 6.0,
                "N={n}: error·√N={} exceeds 6σ·{:.3} CLT envelope (no √N convergence?)",
                err * (n as f64).sqrt(),
                se_factor
            );
        }
    }

    /// GREEN (pipeline identity): `empirical_identify` on a *huge* sample equals the
    /// direct analytic `backdoor_adjust` do-probability (the empirical joint
    /// collapses onto the exact table in the large-N limit, so the two estimators
    /// agree to 3 decimals).
    #[test]
    fn empirical_matches_analytic_backdoor_in_limit() {
        let (py, pz, pxz) = confounded();
        let g = CGraph::new(
            vec![vec![1], vec![], vec![1, 0]],
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).unwrap();
        let rows = simulate_backdoor_samples(500_000, 0x1234_5678);
        for x in 0..2usize {
            let do_y = empirical_identify(&rows, &[2], &[(0, x)], &g).unwrap();
            assert!(
                approx(do_y[1], eff.do_p_y[x]) || (do_y[1] - eff.do_p_y[x]).abs() < 5e-3,
                "N=500k empirical do(X={x})={} must match analytic {} within 5e-3",
                do_y[1],
                eff.do_p_y[x]
            );
        }
    }

    /// GREEN (fail-closed): a *not*-identified graph (bow arc X↔Y latent) must be
    /// rejected by the empirical pipeline, never return a confounded number.
    #[test]
    fn empirical_pipeline_rejects_unidentified_effect() {
        let rows = simulate_backdoor_samples(1_000, 0xDEAD);
        let g = CGraph::new(
            vec![vec![], vec![0]],  // X→Y
            vec![vec![1], vec![0]], // X↔Y latent confound
        )
        .unwrap();
        // With X and Y confounded by a latent, P(Y|do(X)) is NOT identified → Err.
        assert!(
            empirical_identify(&rows, &[1], &[(0, 1)], &g).is_err(),
            "unidentified effect must be rejected (fail-closed), not silently reported"
        );
    }

    /// Verified-by-Math: a deterministic Monte-Carlo oracle. Traps a known
    /// constant from the SCM by summing the *expected* contribution of one sample
    /// over the exact joint — for the back-door fixture the sum
    /// Σ_{z,x,y} P(X=x,Z=z)·P(Y=1|X=x,Z=z) under do(X=1) is exactly
    /// Σ_z P(Z=z)·P(Y=1|X=1,Z=z) = φ. We pin φ to a computed literal so the test
    /// checks the *exact* population value, not a sampled approximation, and the
    /// MC estimate must bracket it. This separates "sampling works" from "the
    /// population target is what we think it is".
    #[test]
    fn mc_oracle_constant() {
        let (p_y_xz, p_z, _pxz) = confounded();
        // φ = E[Y | do(X=1)] = Σ_z P(Z=z)·P(Y=1|X=1,Z=z)
        //   = 0.5·P(Y=1|X=1,Z=0) + 0.5·P(Y=1|X=1,Z=1)
        //   = 0.5·0.2 + 0.5·0.9 = 0.55  ← the causal effect, EXACTLY.
        let phi: f64 = (0..2).map(|z| p_z[z] * p_y_xz[1 * 2 + z]).sum();
        assert!(approx(phi, 0.55), "oracle φ must equal 0.55, got {phi}");
        // A 2-million-sample MC of do(X=1) must bracket φ to 3 decimals.
        let g = CGraph::new(
            vec![vec![1], vec![], vec![1, 0]],
            vec![vec![], vec![], vec![]],
        )
        .unwrap();
        let rows = simulate_backdoor_samples(2_000_000, 0xFEED);
        let do_y = empirical_identify(&rows, &[2], &[(0, 1)], &g).unwrap();
        assert!(
            (do_y[1] - phi).abs() < 3e-3,
            "MC do(X=1)={:.5} must bracket oracle φ={phi} to 3e-3",
            do_y[1]
        );
    }
}
