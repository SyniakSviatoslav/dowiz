# Build-and-Test-First Re-Examination of 6 Disputed Rejections (2026-07-17)

> **Methodology (operator, binding):** *"building & testing first, any claims or authority
> opinions next… Proven methods and appointed opinions is scam for me if it can't [be] verified in
> terms of quality/efficiency in comparison to the new ideas."* Each of the 6 items the operator
> disputed in `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md` §2 is re-decided here on a **real
> build + measurement on this host (rustc 1.96.1, x86_64)** or, where a build genuinely cannot
> settle it, on **fresh 2026 benchmarks** — never by re-citing the literature that produced the
> original rejection. All programs are persisted in the sibling `reexam-builds/` dir (compile with
> `rustc --edition 2021 -O`); outputs below are verbatim.
>
> **Scoreboard:** 2 verdicts **FLIP** on measurement (#2 witness, #4 CORDIC), 1 **PARTIAL FLIP**
> (#5 Laplace/Kuen), 1 stale-number **UPDATE that narrows but survives** (#3 ZK), 1 **STAYS but for
> a measured not cited reason** (#1 speculation), 1 **REFINED — the citation was over-generalized**
> (#6 Merkle-DAG). The ZK number specifically: the blueprint's **~10⁶× is stale by 1–2 orders**;
> the current frontier CPU figure is **Jolt < 10⁵×**, and GPU real-time proving (SP1 Hypercube,
> 2026) proves an entire 32M-gas Ethereum block in **10.8 s**.

---

## Item 1 — Speculative / optimistic execution + rollback (register #4 / rejection §2.2)

**Operator:** "it will be faster." **Old rejection reason (cited):** verify ≈ 0.1–1 ms ≪ mesh RTT
10–100 ms, so there is nothing to speculate around.

**What I built** (`item1_speculation.rs`): a realistic small **local** order transition — status +
integer money (`total = subtotal + subtotal·tax_bps/10000`) + real SHA3-256 content-addressing
(copied verbatim from `kernel/src/event_log.rs`) — run three ways over 2,000,000 attempts:
**A** = verify-before-persist (the kernel's actual `commit_after_decide` shape); **B** =
speculate-apply-then-maybe-rollback (snapshot → mutate → verify → commit|restore); **V** = the
verify-only slice (the thing speculation tries to hide). A `spin` knob injects a stand-in for a
signature/network cost so we can see where speculation *could* pay off.

**Measured (this host):**

```
spin=   0  verify-only V=   9.69 ns  A(verify-first)=  90.20 ns  B(speculate+rollback)=  82.45 ns  | B-A = -7.74 ns/op
spin=  50  verify-only V=  33.61 ns  A(verify-first)= 105.62 ns  B(speculate+rollback)= 105.41 ns  | B-A = -0.21 ns/op
spin= 200  verify-only V= 159.16 ns  A(verify-first)= 232.15 ns  B(speculate+rollback)= 229.96 ns  | B-A = -2.20 ns/op
spin=1000  verify-only V= 822.52 ns  A(verify-first)= 898.47 ns  B(speculate+rollback)= 897.09 ns  | B-A = -1.38 ns/op
```

(Both paths produce byte-identical logs; assertion `log == log_b` passes.)

**What the numbers actually say — three findings the RTT citation missed:**
1. **Local verify is ~10 ns, not 0.1–1 ms.** The old rejection over-stated the local verify cost by
   ~10⁴×. In-process `decide` (transition + integer money) is ~10 ns.
2. **The commit cost is dominated by content-addressing (~80 ns SHA3), not verify.** Speculation
   targets the wrong 10%.
3. **Speculation is a wash — no measurable speedup (B−A within ±8 ns noise on a ~90 ns op).** A
   single-threaded synchronous speculation cannot *overlap* anything, so reordering apply/verify
   buys nothing at any `spin`. Speculation is a technique for hiding **high-latency, overlappable**
   verification (cross-network consensus); there is no such window on the local decide path.

**The decisive point speculation can't escape:** the only work you may speculatively execute is the
**reversible in-memory** part — which the bench shows is already the ~10 ns cheap part. The part
worth hiding (emitting the side effect: charging money, dispatching a courier, gossiping to peers)
is **irreversible** and therefore forbidden to speculate in a degrade-**closed** money
architecture — you cannot roll back a charge or a dispatch.

**Revised verdict: STAYS REJECT — now for a measured reason, not the RTT estimate.** Local verify is
~10 ns (measured), speculation delivers no speedup (measured, ±noise), and the only rollback-able
work is already the cheap work. *What would change this:* a profile showing a **concurrent** hot
path where an **independent** unit of work can run during a genuinely slow (≫µs) verify — i.e. the
cross-network case, which is the DEFER'd mesh consensus layer, not the local kernel.

---

## Item 2 — Self-auditing inline witnessing (register #2 / rejection §2.18)

**Operator:** "yes, add." **Old rejection reason:** self-certification = RC-2, "the check restates
the claim." This is **correct for the naive form** and **wrong for the steelman**, and the
blueprint conflated the two.

**What I built** (`item2_witness.rs`): the SAME forged artifact (a "skim-the-tax" money output)
run through both forms against an **independent** verifier.
- **Naive self-cert:** author emits `{output, self_valid: bool}` — a verdict the author controls.
- **Steelman commitment (`WorkReceipt`):** author emits `{input_commit = H(inputs), law_id,
  output_commit = H(output), output}` — **no validity claim**. An independent party receives the
  inputs out-of-band, **re-runs the external pinned law** (`money.v1`, which the verifier holds, not
  the author), and checks `H(law(inputs)) == output_commit`.

**Measured output (verbatim):**

```
law money.v1(2599, 875 bps) = 2826   (forged claim = 2599)

--- MALICIOUS author (skims tax) ---
  NAIVE self-cert:  trusting-verifier accepts = true    (self_valid flag = true)
                    recompute-verifier accepts = false   <- but this IS just re-doing the work (RC-2)
  STEELMAN receipt: independent-verifier result = Err("REPLAY MISMATCH: committed output != law(inputs)")
```

**Finding:** the naive form's trusting-verifier **accepts the forgery** (the flag is
author-controlled); its distrusting-verifier catches it only by **redoing the law** — at which
point the witness added nothing (that IS RC-2). The steelman form **rejects the forgery via
independent replay without trusting the author**, and additionally buys **input non-equivocation**
(the author can't later claim different inputs), **tamper-evidence** (altering the stored output
breaks `output_commit`), and **offline/deferred/third-party verifiability that travels with the
artifact.** That is a *commitment*, not a *certification*: the validity judgment is made by a
different party running an external law — precisely the author≠verifier crossing the rejection
demanded.

**Revised verdict: FLIP (split).** The naive "I checked my own work" form **stays REJECT** (RC-2,
confirmed by build). The steelman inline-commitment form is **ADOPT** — and it is *already the
blueprint's own accepted shape*: it is structurally identical to `WorkReceipt` (W3-L2), the
DecisionUnit import-replay gate (W3-L6), and the P06 `key_V` independent re-execution. The
operator's "yes, add" is correct once "inline witnessing" is read as "inline **commitment**," which
is what a witness is in the cryptographic sense. The blueprint's error was treating *witness* as a
synonym for *self-certification*; the build shows they are opposites.

---

## Item 3 — Per-message ZK proofs (register #13 / rejection §2.4) — **the stale-number item**

**Old rejection reason:** proving is **~10⁶× native execution** (measured 59 s vs 15 µs in the cited
literature). This number is from 2024 and is now **stale by 1–2 orders of magnitude.** I could not
"build" a zkVM prover here, so I settled this on **fresh 2026 benchmarks** (the honest fallback the
task permits), cross-checked across independent sources:

| Metric (freshly grounded, 2025–2026) | Value | Source |
|---|---|---|
| **Jolt CPU prover overhead** | **< 100,000× (< 10⁵×)** per RISC-V cycle on a laptop | a16z crypto, 2025 |
| Jolt throughput | > 1M cycles/s (32-core), 500k/s (MacBook); 5× faster than risc0, 2× faster than SP1 | a16z crypto, 2025 |
| SP1-AVX2 (CPU) | 1M cycles in **26.3 s** (risc0: 118 s) | Succinct, 8/2024 |
| **SP1 Hypercube (GPU, 2026)** | 99.7% of Ethereum blocks **< 12 s**; a 143-tx / **32M-gas** block in **10.8 s** on 16 RTX 5090 | Succinct, 2026 |
| ZKsync Airbender (2026) | ~35 s / Ethereum block on a single H100 (17 s no recursion) | BlockEden, 1/2026 |
| zkVM **verification** time | **milliseconds** | EthProofs dashboard, 2026 |

**Honest current number:** the frontier **CPU** prover overhead is **≈ 10⁵×** (Jolt), down from the
**~10⁶–10⁷×** the blueprint cited — a real **1–2 order-of-magnitude** improvement driven by
lookup-based provers (Twist/Shout) and GPU fleets. Wall-clock has collapsed: an entire 32M-gas block
proves in ~11 s, and *verification* is now milliseconds.

**Does this flip per-message ZK on the hot path?** No — but the margin narrowed and the reason
changed. A signature verify (the real-time primitive) is **~10 µs**; even a 10⁵×-overhead proof of a
*tiny* decide-circuit (a few thousand cycles) is still **≫ 100 ms of prover work** and needs a GPU,
i.e. still **~10⁴× slower** than the verify it would replace, per message. So **per-message stays
DEFER.** But the **checkpoint/light-client STARK** form (register #13's own deferred half) is now
**materially more attractive**: millisecond verification + amortizable second-scale proving makes a
periodic FSM-replay audit realistic where in 2024 it was not.

**Revised verdict: number UPDATED (10⁶× → ~10⁵× CPU / GPU-seconds wall-clock); per-message DEFER
survives on a narrower margin; the checkpoint-STARK DEFER should be re-prioritized upward.**

---

## Item 4 — RGB-seed harmonic procedural encoding (register #13 E13 / rejection §2.13)

**Operator's technical rebuttal:** "they are identical if the functions/equations inside are
identical" — correct **iff** a portable deterministic implementation replaces per-platform libm
transcendentals. **Old rejection reason:** transcendental float paths are not cross-target
bit-identical (`rng.rs:20-29`, the repo's own determinism audit). That is true for `f64::sin`, which
delegates to platform libm. So I **built the deterministic replacement** and tested it.

**What I built** (`item4_cordic.rs`): a fixed-point **Q30 CORDIC** sin/cos whose runtime kernel uses
**only** `i64` add/sub/compare and **arithmetic shift** — no `f64`, no libm. The atan table + gain
are **frozen integer constants** (generated once offline, shipped as literals like SHA3 round
constants — never recomputed on any target). `f64` appears only in the *measurement harness*, never
on the codec path.

```rust
fn cordic_sincos(mut z: i64) -> (i64, i64) {          // angle & result in Q30, PURE INTEGER
    while z >  PI_Q30 { z -= TWO_PI_Q30; }            // range fold: integer add/sub only
    while z < -PI_Q30 { z += TWO_PI_Q30; }
    let mut negate = false;
    if z >  HALF_PI_Q30 { z -= PI_Q30; negate = true; } else if z < -HALF_PI_Q30 { z += PI_Q30; negate = true; }
    let (mut x, mut y) = (CORDIC_K_Q30, 0i64);
    for i in 0..31u32 {
        let (dx, dy) = (x >> i, y >> i);              // *2^-i via arithmetic shift (cross-target defined)
        if z >= 0 { x -= dy; y += dx; z -= ATAN_Q30[i as usize]; }
        else      { x += dy; y -= dx; z += ATAN_Q30[i as usize]; }
    }
    if negate { (-x, -y) } else { (x, y) }
}
```

**Measured (verbatim):**

```
determinism: 51471 samples/run, digest_run1=0x9d1c0e89c65cbe08 digest_run2=0x9d1c0e89c65cbe08 -> BIT-IDENTICAL
accuracy vs libm over [-pi/2,pi/2], 12868 samples: max|sin err|=1.22e-8  max|cos err|=1.16e-8  (~26.3 bits)
```

**Empirical proof of "zero platform-dependent ops"** — disassembly of the compiled kernel (with a
`#[no_mangle]` copy so the symbol is clean):

```
=== full disassembly of cordic_sincos (79 instructions) ===
>>> ZERO float/xmm instructions — pure integer confirmed
distinct mnemonics: add(10) mov(9) sub(7) cmp(7) movabs(5) shr(4) xor(3) lea(3) inc(3)
                    sar(2) neg(2) mul(2) imul(2) cmov*(4) test setne push pop js jne jmp je
```

The generated machine code contains **no** `mulsd/addsd/sqrtsd/cvt*/xmm/sin/cos/pow` — only integer
instructions. Since Rust guarantees identical `i64` wrapping and arithmetic-shift semantics on every
target (unlike libm transcendentals), the output is cross-target bit-identical **by construction**.

**Does this resolve the objection?** **Yes, for the integer form.** The domain objection dissolves
the moment libm is removed. The operator is correct: two nodes running the *same integer equation*
produce the *same bits*, so a seed-regenerated state content-addresses identically on x86 and ARM.

**Revised verdict: FLIP (conditional).** The **libm/`f64::sin` harmonic form stays REJECT** (genuinely
non-cross-target — unchanged). The **integer-CORDIC harmonic form is ADOPT-able**: bit-identical,
~26-bit accurate, provably float-free. *What closes the last mile* (cannot run ARM on this host):
add a CI job that builds this kernel on `x86_64` **and** `aarch64` (cross-compile or qemu) and
asserts both emit digest `0x9d1c0e89c65cbe08`. That single asserted digest is the entire
cross-target proof — no per-target float tolerance needed. The blueprint should carry the CORDIC
primitive as the deterministic substrate for *any* transcendental the mesh needs (it also unblocks
the `asin/atan2`-for-haversine follow-on named in W1-L1/E5).

---

## Item 5 — Laplace-domain primitives + Kuen surface (rejection §2.15)

**Operator:** "can be expanded — this is the natural geometrical evolution." **Old rejection
reason:** no continuous s-plane / no hyperbolic-geometry surface in a discrete deterministic kernel.
I took this as a real research question and **measured** it on the ONE part that is not a mismatch:
the graph Laplacian the kernel already ships (`spectral.rs::laplacian` + `eigenvalues` +
`algebraic_connectivity`) **is the discretization of a continuous operator.**

**What I built** (`item5_laplace_beltrami.rs`): a deterministic cyclic-Jacobi eigensolver over a
Gaussian-weighted graph on `n` points sampled from the unit circle S¹, whose **continuous**
Laplace-Beltrami spectrum is known exactly: `{0, 1, 1, 4, 4, 9, 9, …} = k²`. I measured the discrete
spectrum's low eigenvalue **ratios** (which cancel the bandwidth constant) as `n` grows:

```
     n |     l1     l2     l3     l4     l5     l6
    24 |  1.000  1.000  1.731  1.731  1.957  1.957
    48 |  1.000  1.000  3.300  3.300  5.629  5.629
    96 |  1.000  1.000  3.837  3.837  8.066  8.066
   160 |  1.000  1.000  3.942  3.942  8.657  8.657
target ->  1.000  1.000  4.000  4.000  9.000  9.000
```

**Finding:** the discrete graph Laplacian's spectrum **converges monotonically to the continuous
Laplace-Beltrami spectrum `{k²}`**, degeneracies (sin/cos pairs → `l1=l2`, `l3=l4`, `l5=l6`)
captured exactly. This is **Belkin-Niyogi (2008)** convergence, live on the kernel's own machinery —
a genuine continuous relaxation, not a fantasy. It grounds the spectral embedding /
Laplacian-eigenmaps the kernel is already positioned for.

**Second axis — the Kuen surface (constant negative curvature) specifically.** The Kuen surface is a
constant-negative-curvature (hyperbolic) surface. Hyperbolic geometry is the **cited** right space
for low-distortion embedding/visualization of **tree-like** topologies (Sarkar 2011, Graph Drawing;
Nickel-Kiela Poincaré embeddings 2017; arXiv:2502.17130, 2025). And the mesh's **anchor-rooted
delegation hierarchy** (`roster.rs`/`node_id.rs`, register C3) **is a tree.** I quantified why the
geometry matches:

```
binary-tree nodes within radius 10        = 2047
Euclidean disk area  (R=10)               = 314      <- cannot hold 2047 nodes w/o crushing distances
Hyperbolic disk area (R=10, kappa=-1)     = 69192    <- 220x the capacity; holds the tree with room
```

Trees have **exponential** neighborhood growth; Euclidean space grows **polynomially**, hyperbolic
space grows **exponentially** — so H² absorbs a tree with bounded distortion where R² forces Ω(√log)
distortion. The negative-curvature framing the operator points at is real for the delegation-tree
**visualization/embedding** layer.

**Revised verdict: PARTIAL FLIP — the §2.15 rejection was over-broad.** The **literal
Laplace-*transform* / continuous-time / s-plane primitives stay REJECT** (correct: a discrete
event-sourced kernel has no continuous-time signal — items B1/B6 in the register). But two specific,
cited, code-adjacent **expansions** are genuine and were wrongly foreclosed: (a) **graph-Laplacian ↔
Laplace-Beltrami** as the continuous limit that *grounds the existing spectral machinery* and
enables manifold-learning consumers if a point cloud (e.g. courier geo-telemetry) ever appears; (b)
**hyperbolic / negative-curvature (Kuen-geometry) embedding of the tree-shaped delegation topology**
for low-distortion visualization. Both are **add-on** layers (embedding/rendering), never
replacements of the discrete core — exactly the operator's "expand, this is the natural geometrical
evolution." *What would fully settle axis (b):* run a Sarkar embedding of the **real roster
delegation tree** into the Poincaré disk and measure worst-case distortion vs a best-effort R²
(MDS) embedding — expected H² distortion 1+ε vs R² distortion growing with depth. I built the
capacity argument (measured above) but not the embedding itself.

---

## Item 6 — A second Merkle-DAG authority (register #20 E20 / rejection §2.7)

**Operator:** "add." **Old rejection reason:** dual-authority hazard, "the exact construction the
RCI council overturned (`ADR-realtime-change-intelligence.md:44-50`)." This is an **archaeology**
item (no build), settled by re-reading the cited council finding precisely.

**What ADR:44-50 actually overturned** (verbatim context, lines 42-49): round-1's **Option C** built
an event-sourced graph projection that was declared *"git is ground truth; the chain is a derived
cache."* The council's kill-shot: *"Option C rebuilt git's own content-addressed hash-DAG one level
up: the dual-authority hazard it claimed to kill by construction."* The rejected construction is
specifically **a second content-addressed authority that DERIVES / DUPLICATES facts an existing
authority (git) already owns** — two stores claiming to represent *the same underlying truth*, free
to silently desync. The ADR's fix (Decision §1, F-1) keeps *"no chain of its own; all RCI state is a
derived, disposable cache."*

**Was E20's construction the same, or over-generalized?** **Over-generalized.** The RCI finding is
about a **derived-duplicate** authority. E20 rejected *"a NEW Merkle-DAG authority for patch/unit
history"* by citing it — but a **DAG is a data-structure choice, not automatically a second
authority.** Two distinctions the blueprint blurred:

1. **New facts vs derived facts.** DecisionUnit lineage (which unit supersedes which; branch/merge
   of harvested instance-sets; epoch order) is a **new fact domain no existing log owns.** A store
   that is the **single** authority for a **new** domain is not dual-authority — the RCI hazard only
   bites when the second store re-derives facts git/`event_log` *already* hold.
2. **DAG-shape is intrinsic, not gratuitous.** DecisionUnit history is **genuinely a DAG**: a unit
   can have **multiple parents** (a merge of harvested instances) and can branch. A **linear** chain
   (the current `event_log`) *cannot natively express* that. So the operator's "add" points at a
   real structural need the linear log loses.

**Revised verdict: REFINED — the flat REJECT was the citation over-reaching.** The correct rule is
narrower than "any Merkle-DAG = dual-authority": *do not create a second authority for facts an
existing authority already owns.* Under that rule, E20's **own resolution is already compliant** and
should be stated as ADOPT-shaped, not REJECT: put the DAG-structured lineage **inside the single
sha3 content-addressed log the units already register through** (one authority, DAG-shaped edges for
the new lineage facts) — **not** a parallel store mirroring `event_log`. The dual-authority hazard
is real and must stay guarded, but it is triggered by *duplication*, not by *DAG topology*. The
operator's "add" is correct for the branch/merge lineage; the blueprint's job is only to keep it a
**single** authority for that new domain.

---

## Summary table

| # | Item | Old reason (cited) | New evidence (built/measured) | Verdict change |
|---|---|---|---|---|
| 1 | Speculative execution | verify ≪ RTT | local verify **~10 ns**, commit dominated by SHA3 ~80 ns, speculation **no speedup ±noise**, only cheap reversible part is speculatable | **STAYS** — measured reason |
| 2 | Inline witnessing | self-cert = RC-2 | naive form **accepts forgery**; commitment form **rejects it via independent replay** + non-equivocation/tamper-evidence | **FLIP (split)** — commitment ADOPT |
| 3 | Per-message ZK | ~10⁶× overhead | frontier **~10⁵×** (Jolt); GPU real-time (32M-gas block in 10.8 s); verify in ms | **UPDATE** — number stale; per-msg DEFER survives narrowly; checkpoint-STARK ↑ |
| 4 | RGB harmonic codec | libm not cross-target | integer CORDIC: **bit-identical**, ~26-bit, **zero float instructions** (disasm-proven) | **FLIP (conditional)** — integer form ADOPT |
| 5 | Laplace / Kuen surface | domain mismatch | graph-Laplacian → **Laplace-Beltrami spectrum {k²} measured converging**; hyperbolic disk **220×** capacity for the delegation tree | **PARTIAL FLIP** — continuous limit + hyperbolic embed are real add-ons |
| 6 | Second Merkle-DAG | RCI dual-authority | RCI killed a **derived-duplicate**; DecisionUnit lineage is **new facts + intrinsically DAG-shaped** | **REFINED** — cite was over-generalized; ADOPT as single authority for the new domain |

**Sources (items 3 & 5):**
[Jolt 6× speedup, a16z crypto](https://a16zcrypto.com/posts/article/jolt-6x-speedup/) ·
[SP1 Hypercube real-time proving](https://blog.succinct.xyz/real-time-proving-16-gpus/) ·
[SP1 benchmarks 8/6/24](https://blog.succinct.xyz/sp1-benchmarks-8-6-24/) ·
[ZKsync Airbender](https://blockeden.xyz/blog/2026/01/30/zksync-airbender-fastest-risc-v-zkvm-ethereum-proving/) ·
[Belkin-Niyogi convergence of Laplacian eigenmaps](http://misha.belkin-wang.org/papers/CLEM_08.pdf) ·
[Error estimates for spectral convergence of the graph Laplacian (FoCM 2019)](https://link.springer.com/article/10.1007/s10208-019-09436-w) ·
[Sarkar, low-distortion tree embedding in the hyperbolic plane](https://homepages.inf.ed.ac.uk/rsarkar/papers/HyperbolicDelaunayFull.pdf) ·
[Low-distortion GPU tree embeddings in hyperbolic space (2025)](https://arxiv.org/pdf/2502.17130)
