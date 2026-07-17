# BATCH 6 — DecisionUnit-JIT / RGB-Seed Codec / Mipmap-LOD / Equilibrium: research + audit findings (2026-07-17)

> Research + audit (NOT a blueprint) for the four dialogue concepts that the earlier batch-split
> under-scoped (`00-SOURCE-PROMPT.md`, `01-RAW-DIALOGUE-PART-A.md`), plus a mandatory completeness
> sweep. Operator flagged that omission will be treated as sabotage ("нічого не опускай з промпту,
> враховуй усе, буквально усе") — this batch errs toward over-listing every named concept and its
> coverage. Complexity/rewrite-size is **not** a valid rejection ground for this arc (source-prompt
> lines 15-16, 23); only physics / correctness / determinism-contract / absent-consumer is. Every
> load-bearing claim carries a `file:line`, a live-command ground, or an explicit epistemics tag
> (Anu/Ananke discipline, `AGENTS.md`).

## Epistemics tags

- `[VERIFIED-CODE]` — read the live working tree at that `file:line` this session.
- `[BLUEPRINT-AUTHORITY]` — the concept is already promoted to a landed design doc in this repo;
  that doc is the design authority and this batch defers to it, adding only what it did not cover.
- `[PRIOR-ART-ADJUDICATED]` — a sibling batch (10–14) or blueprint already gave this a verdict; do
  not re-litigate. Cited so the reader can trace it.
- `[NEW-GROUND]` — a genuine extension the prior authority did **not** cover; safe to carry forward.
- `[PHYSICS-REJECT]` — rejected on determinism/hardware/domain, the allowed kind (never complexity).
- `[DEFER-WITH-TRIGGER]` — sound but no consumer at today's scale; a measurable condition flips it.
- `[GAP]` — a named dialogue concept no batch (this one included) has audited as a coherent unit.
- `[U]` — unmeasured / open risk carried forward honestly.

---

## §0 — Headline (read first)

Three of the four concepts land on **already-decided authority**, and the audit's value is drawing
the precise line between what that authority covers and the genuinely new ground the dialogue adds:

1. **DecisionUnit gossip = the "Decision Compiler."** CONFIRMED **the same idea**, already promoted
   to the **primary recommendation** of `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md`
   §2, with all four repo precedents verified live this session. Yesterday's blueprint is the design
   authority. The dialogue adds three pieces that blueprint left thin: **epoch-versioning**,
   **Proof-of-Quality gating before gossip-propagation**, and **Merkle-DAG rollback versioning** —
   and Batch 4's rejections do **not** kill the gate, because gating a *compiled artifact once at
   import* is architecturally verify-before-persist, not the per-transaction optimistic fraud-proof
   that was rejected. §1.
2. **RGB-seed procedural encoding = transmit a GENERATOR, not a DELTA.** Genuinely distinct from
   Batch 1 (sparse) and Batch 2 (delta) — but **REJECT for general tensor state on physics**: the
   dialogue's `color = f(seed, harmonics)` uses transcendentals (sin/cos), and this repo's own
   determinism audit proves transcendental float paths are **not** cross-target bit-identical
   (`rng.rs:22-28`). The one *sound* generator-encoding — truncated spectral reconstruction
   `W ≈ U_k Λ_k U_kᵀ` — already exists as Phase-28 rung 1. §2.
3. **Pixel/mipmap LOD token compression** — **mostly ALREADY EXISTS** under other names (retrieval
   tiers, spectral coarsening, CDC dedup, and the Repowise skeleton→symbol→range LOD the reader is
   using right now). Batch 1 A10 already redirected the literal pixel-mipmap to spectral coarsening.
   The one genuinely-absent piece is a kernel-level *multi-resolution summary/pooling* primitive over
   a live token stream. §3.
4. **Equilibrium / zero-point (no-supervisor structural safety)** — pushing one level deeper than
   Batch 3: the operator's literal ideal ("no external process watches; bad states are
   unrepresentable") **is genuinely achieved** for internal-arithmetic invariants (budget, money,
   drift-reject, fuel) and **is NOT** achieved for the tamper leg — `integrity_check` (`hydra.rs:180`)
   and `boot_verify` (`hydra.rs:253`) **are** processes checking a condition (a poll and a
   self-administered `assert!`). That boundary is not an accident: you can make an unstable *number*
   unrepresentable by type; you cannot make external *tamper* or a *crash-loop* unrepresentable —
   detecting them requires a check, and self-checking is the exact RC-2 gap the Hermetic audit named.
   §4.

The completeness sweep (§5) finds **one significant uncovered concept** — *predictive tensor handoff
for moving physical assets* (drone-following tiles / ghost-tile prefetch / atomic pointer swap /
shadow-tile multi-hypothesis simulation with probability-weighted pruning) — plus four minor
named-but-unverdicted items.

---

## §1 — DecisionUnit gossip as a distributed JIT compiler

**Verdict: CONFIRMED same idea as the "Decision Compiler"; yesterday's blueprint is authority;
the dialogue adds three real pieces of new ground; the PoQ gate SURVIVES Batch 4's rejections in
one specific form (import-time independent replay), not the rejected form (per-transaction optimistic
fraud-proof).** `[BLUEPRINT-AUTHORITY]` + `[NEW-GROUND]`

### 1.1 Are these the same idea? — CONFIRMED, with the four precedents re-verified live

The dialogue's core insight — one hub pays the LLM-inference cost **once** to compile a native,
tested, provenance-stamped "DecisionUnit" for a recurring question-shape; every other node then
decides in nanoseconds with zero network / zero tokens — is **verbatim** the Decision Compiler
already ruled the primary recommendation of the latency blueprint
(`BLUEPRINT-LATENCY-ELIMINATION…:37-53, §2:129-183`). That blueprint's mechanism line: "for a
**recurring question shape**, query the LLM **once** … answer every subsequent instance in-process
in nanoseconds, with a typed escalation path back to the LLM" (`:131-136`). Same object, same
economics, same "LLM moves from the request path to the build path."

The four repo precedents that blueprint cites (`§2.2:169-183`) are **real** — verified this session,
not from memory:

| Precedent | Live cite (this session) | Status |
|---|---|---|
| `is_redline(path)` | `tools/ci-truth/src/main.rs:237` (fn), consumed `:386`, tests `:718-734` | `[VERIFIED-CODE]` — the existence proof for "compile a judgment into a native fn" |
| `Scope::touches_red_line()` + `RedLinePolicy::check` | `dowiz-agentic-mesh/kernel/src/ports/agent/scope.rs:244`, policy `:254-270`, `AgentBridge=0x12` `:70` | `[VERIFIED-CODE]` — a closed-enum match that structurally refuses red-line scopes |
| hermes `gov_route` EV table | `tools/telemetry/governance.sh:50` (`gov_route`), natively ported in `tools/telemetry/native-trackers/src/main.rs:333,379` ("Replicate gov_route's EV pick") | `[VERIFIED-CODE]` — derived from harvested `track_record.jsonl`, serves in microseconds |
| `skillspector-rs` rules pipeline | `tools/skillspector-rs/build.rs` + `gen_rules.py` (both present, live `ls`) | `[VERIFIED-CODE]` — Python analyzer regenerated into native Rust at build time when the source-of-truth changes |

**Conclusion:** same idea, four times proven by hand. The latency blueprint already named,
systematized, and DECART'd it (`§2.5:259-279`, choosing LLM-generated native Rust over data tables,
classifiers, or status-quo). **Treat `BLUEPRINT-LATENCY-ELIMINATION…` §2 as the design authority.**
This batch does not re-design the compiler; it audits what the *dialogue* adds on top.

### 1.2 What NEW ground does the dialogue add that the blueprint did NOT cover?

The blueprint is a single-host document — its §2.4 invalidation rides GapWire, its registry is
`decision-units/` in-process, and cross-hub sharing is deferred to a one-line speculative item
(`§5 S7:509-515`, `§3.6:423-434`). The dialogue extends it into a **distributed** setting, and three
of its pieces are genuinely uncovered:

**(a) Epoch-versioning for DecisionUnits.** `[NEW-GROUND]` The blueprint's invalidation is binary —
a `GapEvent` on a watched-input flips a unit to `Stale`, which then answers `Escalate`
unconditionally (`§2.4(d):248-257`). It has **no version identity** for a unit across recompiles, so
two hubs that independently recompile the same shape have no way to say "my unit is newer than
yours." The dialogue's epoch clock supplies exactly this: a monotone epoch stamped into each unit's
provenance header makes "which compiled unit wins on gossip" a deterministic `max`-merge (the same
logical-epoch primitive Batch 2 §8 and Batch 5 §1.1 already recommend for the gossip roster —
`bebop-repo/bebop2/proto-wire/src/discovery.rs` gossip path, epoch as a Lamport-style counter). This
is the **compose point** the blueprint gestured at but did not design: DecisionUnit epoch == gossip
epoch, one counter, no wall-clock (the HLC caveat from Batch 2 §7 applies — logical only).

**(b) Proof-of-Quality gating before gossip-propagation.** `[NEW-GROUND]` The blueprint's
verification is local (`§2.4(c):233-247`: RED→GREEN property tests + independent replay + operator
gate on red-line shapes). It does **not** design *what a receiving hub checks before importing a
foreign compiled unit*. The dialogue's PoQ hybrid is that missing import gate — but see §1.3 for the
critical constraint on which PoQ form survives.

**(c) Merkle-DAG versioning for rollback.** `[NEW-GROUND]` The blueprint can invalidate a unit
(→`Stale`) but has no *history* of a unit's compiled versions to roll back to. The dialogue's
Merkle-DAG gives each shape a content-addressed lineage of compiled versions, so a hub that discovers
its newest unit is wrong can revert to the last-known-good compiled version rather than falling all
the way back to the slow LLM path. **Caveat from Batch 2 §4/§11** `[PRIOR-ART-ADJUDICATED]`: a *new*
Merkle-DAG **authority** is the exact dual-authority hazard the RCI Triadic Council overturned
(`ADR-realtime-change-intelligence.md:44-50`) — so the rollback history must be the *same*
content-addressed event log the units already register through (`decision-units/` provenance keyed
by the existing sha3 content-address), not a parallel patch-DAG. The rollback concept is sound; a
second hash-DAG to hold it is not.

### 1.3 Does Batch 4's rejection kill the PoQ gate? — NO, for a compiled artifact; YES, for the rejected forms

The task's sharp question: Batch 4 rejected per-message ZK-proofs and speculative/optimistic
execution — does that also kill the "Proof-of-Quality via optimistic fraud-proof" gating this
dialogue proposes for DecisionUnit propagation? **The answer turns on "once per artifact" vs "per
transaction," and it is a real, load-bearing distinction.**

- **The rejected forms stay rejected.** `[PRIOR-ART-ADJUDICATED]`
  - *Statistical PoQ* (accept a gossiped unit by how many peers "vote quality") = a reputation/quorum
    aggregate — falls under Batch 4 §1's `NO-COURIER-SCORING` contradiction + Cheng–Friedman (2005)
    impossibility (`13-BATCH4…:150-156, §2.3:181-185`). Blocked on the operator ruling; do not build.
  - *Optimistic fraud-proof PoQ* (propagate now, let a challenger prove fraud later) = the
    optimistic-execution class Batch 4 §2.5 rejects **outright** as *degrade-open* in a degrade-closed
    architecture (`13-BATCH4…:210-227`; R1 §6: no permissionless fraud proof worked on any optimistic
    rollup for ~3 years). And Batch 4 §2.5's SSR-2020 batch-verify finding (`6541ae8`,
    `sign.rs:971`) proves the "cheap rollback / cheap quorum verify" premise is *false* — batching
    signature checks is not free, correctness forces one full verify per item.
- **But gating a compiled artifact ONCE is not a transaction gate — it is verify-before-persist.**
  `[NEW-GROUND]` A DecisionUnit is imported **once** and then serves millions of decisions. The
  correct gate is therefore the repo's already-blessed pattern: **import-time independent
  re-execution** — the receiving hub replays the unit's harvested instance-set through the unit and
  through its own oracle; disagreement ⇒ reject. This is:
  - the **semantic-contract PoQ = `WorkReceipt`** half Batch 4 *accepted* (`13-BATCH4…:164-173`,
    checked by the counterparty via `hybrid_gate.rs::HybridGate::check`, `:346`), not the optimistic
    half it rejected;
  - identical in shape to `commit_after_decide_drift_gate` — **verify BEFORE persist**
    (`dowiz/kernel/src/event_log.rs:389`, tests `:650,:677`) `[VERIFIED-CODE]`, the validity-first
    alternative Batch 4 says to keep;
  - identical in shape to the P06 **key_V independent re-execution** the Hermetic audit names as the
    one fix that closes the whole self-certification family (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:196-204`,
    RC-2) — the author-hub's own GREEN is never the certificate; a different party re-runs it.

  The cost asymmetry is the whole point: an *optimistic fraud-proof per transaction* imports the
  challenge-window / funded-challenger / censorship machinery R1 §6 shows nobody made work, to save a
  sub-millisecond check on the network's own 10–100 ms floor (Batch 4 §2.5). An *independent replay
  once per compiled artifact* is a one-time import cost amortized over the artifact's entire lifetime
  — exactly what the highest-trust operation (importing foreign executable decision logic, latency
  blueprint `S7:509-515`) should pay. **Verdict: the PoQ gate survives as import-time replay; it does
  not survive as an optimistic fraud-proof or a statistical vote.** The one-artifact/per-transaction
  distinction is meaningful and decisive.

### 1.4 Honest ceiling and residual risk

`[U]` The compiler only wins for the judgment-shaped call class (≈1/3 of calls, 4% of decode tokens —
latency blueprint `§2.6:281-301`); it cannot touch open-ended synthesis. And gossiping compiled units
is capability-theater until a second live hub exists (`§3.6:432-434`, one-host session, B2 unbuilt).
The named risk is unchanged and load-bearing: a wrong compiled rule is *worse* than a slow LLM (fast,
confident, invisible), which is precisely why the import gate must be independent replay, never
self-certification, and red-line-adjacent shapes stay operator-gated (`§2.4(c).5:242-243`).

---

## §2 — RGB-seed procedural function encoding ("Universal Codec: Procedural State Encoding")

**Verdict: transmit-a-GENERATOR is genuinely distinct from Batch 1 (sparse) and Batch 2 (delta), but
REJECT for general tensor state on cross-architecture float non-determinism; the ONE sound
generator-encoding (truncated spectral reconstruction) already exists as Phase-28 rung 1.**
`[PHYSICS-REJECT]` (general) + `[BLUEPRINT-AUTHORITY]` (the sound form)

### 2.1 Is it a distinct compression strategy? — YES

A seed/parametrized-function encoding transmits a **generator** (`state = f(seed, params)`), and a
receiving node regenerates state locally. This is categorically different from:
- **Batch 1's sparsity** (`10-BATCH1…` A1, `csr.rs:39-54`) — stores the *nonzeros* of the actual
  data;
- **Batch 2's delta/patch** (`11-BATCH2…` §5, `anti_entropy.rs:75-107`) — ships the *difference*
  from a known base.

A generator ships *neither the data nor a diff* — it ships a recipe. So the dialogue is right that
this is a third strategy worth its own verdict. `[VERIFIED-CODE]` for the two contrasts.

### 2.2 Is it physically sound? — NO for the proposed harmonic form (the decisive finding)

The dialogue's own analogy is `color(x,y,z) = f(seed, harmonics)` — a **harmonic** (trigonometric)
basis. That is exactly the failure surface. This repo has **no fixed-point arithmetic** (grep: the
only "fixed" hits are *fixed summation order*, not Q-format integers — §C2a this session); it relies
on f64 with pinned op-order, and its **own determinism audit already proved the boundary**:

> `kernel/src/rng.rs:22-28` `[VERIFIED-CODE]`: "The 'bit-identical across runs, platforms, and
> builds' claim … is earned by the **integer** generator … It does **not** extend to any
> transcendental float path (`ln`/`sin`/`cos`/`atan2`/`hypot`/etc.) … those are reproducible
> *per-target*, not *cross-target*: IEEE-754 does not mandate identical rounding for transcendental
> functions across different libm implementations/platforms."

This is the Hermetic Cause-and-Effect Finding C, ranked finding #20 (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:292`):
"only the integer RNG fully earns 'bit-identical across platforms'." A harmonic seed-generator
evaluated on x86_64 and on ARM will produce **different bits**, so any DecisionUnit / mesh peer that
regenerates state from a harmonic seed and then content-addresses or signs it (the mesh's whole
authenticity model) would compute a **different content-id on different hardware** — a mesh-breaking
P0 effect (Cause-and-Effect ranks idempotency-keys/signatures as P0, `:100-105`). **This is a physics
rejection, the allowed kind — not complexity.**

The counter-example that proves the rule: money is generator-safe **because it is integer-exact** —
`money.rs:151-152` does ALL→EUR via `rate_scaled = (rate * 1e9).round() as i128` (`[VERIFIED-CODE]`),
scaled-integer arithmetic, never a float channel (`money_guard.rs` red-line, Batch 4 §2.6). A
generator is sound **iff** it is a pure integer/rational function (like `rng.rs` SplitMix64→PCG64,
which *is* cross-target bit-identical). A trig/harmonic generator is not.

### 2.3 Does it connect to anything that exists? — YES, three live anchors

`[VERIFIED-CODE]` The "regenerate from a seed" pattern is already in the tree:
- **`kernel/src/rng.rs`** — a deterministic, seedable PRNG (SplitMix64→PCG64), the *integer*
  generator that genuinely is cross-target reproducible; the sound substrate for any seed-encoding.
- **`spectral_cache.rs:107`** (`h ^= x.to_bits()`) + `DecompCache` — already stores a decomposition
  keyed by content-address, so "regenerate vs cache" is a live tradeoff the repo makes.
- **`backup.rs:426`** (LCG deterministic bytes, "reproducible, no entropy") and **`domain.rs:536-540`**
  (SplitMix-style hash with the `0x9E3779B97F4A7C15` constant) — deterministic byte generation from a
  seed already appears twice.

### 2.4 The one sound generator-encoding already exists

The mathematically honest form of "ship a generator that regenerates the tensor" is **low-rank
spectral reconstruction**: transmit `U_k, Λ_k` and regenerate `W ≈ U_k Λ_k U_kᵀ`. That IS a
generator (the receiver reconstructs `W` from a small parametrization), it composes with the mesh's
determinism contract **if** the solver is fixed-K/fixed-order, and it is **already Phase-28 rung 1**
(`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA…:347`, re-homed to `spectral::topk_symmetric` per the
eigenvector-refactor addendum `:588-599`) `[VERIFIED-CODE]`. Batch 1 A10 reached the same redirect
for the mipmap analogy (`10-BATCH1…:337-352`). **Verdict: REJECT the literal harmonic-seed codec for
general state (cross-arch transcendental non-determinism + no consumer — most kernel state is
event-sourced *facts*, not procedurally-generable noise); ADOPT-as-existing the integer-PRNG seed for
reproducible sampling and the truncated-spectral generator for compressible relation structure.**
Neither rejection is on complexity.

---

## §3 — Pixel/mipmap hierarchical (LOD) token compression

**Verdict: mostly ALREADY EXISTS under other names; the literal pixel-mipmap is already redirected by
Batch 1 A10 to spectral coarsening; the one genuinely-absent piece is a kernel-level multi-resolution
summary/pooling primitive over a live token stream.** `[PRIOR-ART-ADJUDICATED]` + `[GAP]` (narrow)

### 3.1 The idea, and where it already lives

The dialogue frames context/token streams as "pixels" with Level-of-Detail tiers (raw tokens =
full-res, summarized/pooled = low-res), where the mesh queries low-res first and streams high-res on
demand. Honest audit against the tree: **this shape recurs all over the repo already**, under four
names —

1. **Retrieval tiers (coarse→fine).** `[VERIFIED-CODE]` The kernel's `retrieval/` layer is literally
   a multi-resolution query stack: `bm25.rs`/`trigram` (coarse lexical) → `diffusion.rs`/`ppr` (graph
   recall) → embeddings (fine, Layer B). MEMORY's internal-retrieval arc names it a 4-layer
   trigram/BM25/HNSW/diffusion stack. "Query low-res first, refine on demand" *is* the retrieval
   ladder.
2. **Spectral coarsening = the mip pyramid.** `[BLUEPRINT-AUTHORITY]` Batch 1 A10 already made this
   call (`10-BATCH1…:337-352`): the top-k eigenmodes of `W ≈ U_k Λ_k U_kᵀ` (Phase-28 rung 1) **are**
   the low-frequency ("coarse mip") content of the relation graph; a Laplacian graph-coarsening
   operator on top gives a principled pyramid. This is the correct math for "downsample a stream."
3. **Content-defined hierarchical dedup.** `[VERIFIED-CODE]` `kernel/src/chunker.rs` (Buzhash rolling
   CDC, content-address via `sha3_256`, per Batch 1 A10) already compresses a byte stream so "a
   one-byte change re-hashes only the local block" — the streaming/hierarchical half of the idea.
4. **The Repowise skeleton→symbol→range LOD (tooling).** The `get_context` skeleton is "~37% of a
   full Read" (`.claude/CLAUDE.md` Repowise section) — a literal low-res tier queried first, with
   `get_symbol` streaming the high-res body on demand. This is the dialogue's exact "query low-res
   first, stream high-res on demand" pattern, *already in production as the index the reader is using*
   — but it is dev-tooling, not kernel/mesh code.

Adjacent: the **headroom compression proxy** (MEMORY, `ANTHROPIC_BASE_URL→127.0.0.1:8787`, ~1,619
tok/req saved) and P26's `BoundedStore`/cache work (`BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS…`)
are the token-economy siblings, but neither is a *multi-resolution* representation.

### 3.2 The honest gap

`[GAP]` (narrow) What does **not** exist is a **kernel-level primitive that produces a summarized /
mean-pooled low-resolution representation of a live token/context stream** that the mesh can query
before deciding whether to stream the raw tokens. The retrieval layer *indexes documents*; it does
not emit a multi-resolution *summary* of an in-flight token stream. This is real but small, and it
overlaps heavily with Batch 1 A10's spectral-coarsening redirect — so the honest framing is: **not a
fresh architectural gap, a re-frame.** Mean-pooling/summarization of a token stream is also an
LLM-generation task (producing the "low-res" summary costs decode tokens), which puts most of its
weight back into the latency blueprint's decode-discipline territory (`§3.7.1`), not into a
zero-token kernel primitive. **Verdict: the LOD idea is ~85% already-built under other names;
adopt-as-existing (retrieval tiers + spectral coarsening + CDC + skeleton tooling); the residual
"token-stream pooling primitive" is a narrow gap that should be evaluated *after* Phase-28's spectral
ladder lands, since that ladder is the coarse tier it would sit on.** Not rejected on complexity.

### 3.3 Bridge to Part B (flagged for completeness)

The operator attached Part B's signal-processing primitives as "candidate primitives for token-stream
filtering." The LOD idea is where they connect: **Nyquist–Shannon** (Batch 1 B14) says a stream
sampled below 2× its bandwidth **aliases** — i.e. a low-res tier that pools/downsamples a token
stream too aggressively will alias high-frequency content into spurious low-frequency summary, exactly
the failure a mipmap's box-filter is designed to pre-empt. Batch 1 B14 already carries this to the
mesh epoch clock; extending it to the token-stream LOD is the honest home for Part B's
signal-processing framing. Minor, flagged not built.

---

## §4 — Equilibrium / "Zero-point" state: structural (no-supervisor) self-healing & self-termination

**Verdict: the operator's literal ideal is ACHIEVED for internal-arithmetic invariants and
ASPIRATIONAL for the tamper/restart legs — and the boundary is structural, not incidental. Batch 3
established the three-way split; this batch pushes one level deeper to the exact place a "check"
survives.** `[VERIFIED-CODE]` + `[PRIOR-ART-ADJUDICATED]`

Batch 3 (`12-BATCH3…` §6) already mapped the operator's three-way Self-Healing / Self-Termination /
Snapshot-Re-entry split to file:line and found "no supervisor" honored on all three legs but built
unevenly. **This batch does not repeat that.** The task asks something sharper: is "no supervisor at
all" — instability *literally cannot occur* because the math/types make bad states unrepresentable —
actually achieved, or does even the best degrade-closed pattern still have SOME process checking SOME
condition?

### 4.1 Where the ideal is genuinely REAL (bad state is unrepresentable; no check exists)

`[VERIFIED-CODE]` For **internal-arithmetic invariants**, the correction is baked into the *same
computation* as the normal path — there is no separate watcher, because the unsafe state does not
typecheck / does not execute:
- `ComputeBudget::debit` refuses past the ceiling and records **no** spend; the over-budget path
  returns `Err(BudgetExceeded)` — "proceed anyway" is not representable (`budget.rs:14` calls
  degrade-closed "the load-bearing word", per Batch 3 §6a). No supervisor polls the budget; the debit
  *is* the gate.
- **drift-gate = verify-before-persist**: an `Unstable`-spectrum mutation is rejected *pre-persist*
  (`event_log.rs:389`, `[VERIFIED-CODE]` via Batch 4 §2.5). The check is not a separate process
  watching the log — it *is* the commit path. The bad mutation never lands; the prior topology is
  intact by construction ("Survival = endurance, not exclusion", `event_log.rs:383`).
- **fuel exhaustion** stops a compute-bomb via a wasmtime `OutOfFuel` trap
  (`dowiz-agentic-mesh/agent-adapters/src/fuel.rs`, Batch 3 §2) — the runtime/hardware stops the
  guest; no supervising process polls-and-kills.
- **money = discrete integer channel** (`money_guard.rs`, `money.rs`) — a fractional/interpolated
  money value is *unrepresentable*, not *detected-and-corrected*.

For this class the operator's ideal is **literally true**: instability cannot occur because the type
system / arithmetic makes the bad state unrepresentable, and no external process watches. This is the
"physics, not bureaucracy" pole the dialogue itself argues for (Batch 3 §0).

### 4.2 Where a "check" SURVIVES (the deeper finding — the ideal is aspirational here)

`[VERIFIED-CODE]` The single place the operator's ideal breaks is **tamper/integrity**, and it breaks
**because it must**:
- `Hydra::integrity_check` (`hydra.rs:180-193`) flips `Live`↔`Locked` on the predicate
  `rho < 1.0 && rho.is_finite()`. **This is a process checking a condition** — a poll. It is not baked
  into a normal write path; the commit path *calls* it (`hydra.rs:227`: "if
  self.integrity_check() == Locked"). A tamper-corrupted baseline is **detected by a check**, not made
  unrepresentable.
- `boot_verify` (`hydra.rs:253-263`) runs a self-administered `assert!` (`:258`) that **halts** on an
  unstable baseline. That is a supervisor-style condition-check the node runs on itself at boot — and
  Gender V-3 already flagged it self-administered, with "silence-before-witnessing uncovered"
  (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:181-183`, finding #15).
- **Restart-intensity** (Batch 3 §2 gap): there is **no** structural form at all — a crash-looping
  drainer relaunches forever; the only fix named is a *policy* supervisor (systemd MaxR/MaxT). The one
  place the dialogue's "distributed watchdog" is genuinely needed is precisely where no structural
  invariant can replace it.

**Why the boundary is structural, not a missing feature:** you can make an unstable *number*
unrepresentable with a type (budget, money, drift). You **cannot** make external *tamper* or a
temporal *crash-loop* unrepresentable — the corrupt baseline and the relaunch both *are*
representable values/events; the only way to catch them is to *check a condition*, and checking a
condition is a process. So `integrity_check`/`boot_verify` are not a design failure to be refactored
into structure — they are the irreducible residue of "a system cannot make its own compromise
unrepresentable to itself." And self-checking one's own integrity is exactly the RC-2
self-certification gap the Hermetic audit says only the **P06 key_V independent re-execution** path
closes (`:196-204`) — an *independent* party checks, because the compromised node cannot be trusted to
check itself. `[PRIOR-ART-ADJUDICATED]`

### 4.3 Precise answer to the task's question

Is "no supervisor at all" achieved anywhere? **Yes — genuinely, for every internal-arithmetic
invariant (§4.1), where bad state is unrepresentable and no process watches.** Does even the best
degrade-closed pattern still have SOME process checking SOME condition? **Yes — for the tamper leg
(`integrity_check`, `boot_verify`) and the restart leg, a check survives, and it is forced: external
and temporal threats cannot be typed out of existence.** So the operator's ideal is **not uniformly
aspirational and not uniformly real** — it is *earned exactly on the arithmetic invariants and
aspirational exactly on the externally-caused ones*, and that split line is the same physics-vs-
bureaucracy line (Hermetic §4 verdict, `:315-317`: the in-process Rust core earns the principles;
what must cross the author/verifier divide only aspires). The honest one-sentence resolution: **a
system can make its own bad math unrepresentable, but it cannot make its own compromise
unrepresentable to itself — that is why key_V (an independent second party), not a cleverer type,
is the load-bearing fix for the one leg that still needs a check.**

---

## §5 — COMPLETENESS SWEEP (mandatory)

Re-read `01-RAW-DIALOGUE-PART-A.md` end-to-end (including the compressed bracket, lines 25-53) and
`00-SOURCE-PROMPT.md` Part A (27-36) + Part B (38-59). Every named concept mapped to its owning batch
below. Batch files present in this directory (all read this session): 10-BATCH1, 11-BATCH2,
12-BATCH3, 13-BATCH4, 14-BATCH5, plus this 15-BATCH6. **No batch file is missing.**

### 5.1 Coverage ledger — every dialogue concept, its owner (nothing silently dropped)

| # | Concept (dialogue term) | Owner | Verdict there |
|---|---|---|---|
| 1 | Market-based micro-negotiation / bid-priority scheduling | Batch 4 §2.1 | REJECT-as-drafted; sealed-batch form narrow |
| 2 | JIT recursive proofing / self-auditing inline witnessing | Batch 4 §2.2 | self-audit REJECTED (RC-2); counterparty-verified WorkReceipt accepted |
| 3 | Emergent swarm / flocking / "pulsing organism" | Batch 5 §1.1 (gossip) + Batch 4 §2.4 (spectral) | EXTEND gossip; spectral convergence is the real "swarm-converges" math — **but "flocking" as a distinct behavior model is not independently verdicted → §5.2 gap iii** |
| 4 | Speculative / optimistic state execution + rollback | Batch 4 §2.5 | REJECTED outright (degrade-open) |
| 5 | Eventual-consistency vs fast-finality / finality-tiering | Batch 4 §2.4 + Batch 2 §0 | local explicit finality; commutative/non-commutative split |
| 6 | Priority-tagged transitions / priority dispatcher | Batch 4 §2.6 | nested TokenBucket envelopes; no new machinery |
| 7 | **DecisionUnit gossip / JIT-compile swarm intelligence** | **Batch 6 §1** (+ latency blueprint §2 authority) | CONFIRMED = Decision Compiler; 3 new-ground pieces |
| 8 | **Epoch-versioning for DecisionUnits** | **Batch 6 §1.2(a)** | NEW-GROUND; epoch == gossip epoch (logical) |
| 9 | Proof-of-Quality (4 forms + hybrid) | Batch 4 §2.2 + **Batch 6 §1.3** | statistical/optimistic REJECTED; import-time replay SURVIVES |
| 10 | State journaling + rollback pseudocode | Batch 2 §10 | checkpoint/restore built; rolling-truncation council-gated |
| 11 | Reputation-weighted bisection dispute resolution | Batch 4 §1/§2.3 + Batch 2 §6 | bisection OK; reputation weighting BLOCKED (red-line) |
| 12 | Determinism reqs (fixed-point, no SystemTime, no thread_rng) | **Batch 6 §2.2** (fixed-point/FP) + Batch 2 §7 (SystemTime) + Batch 1 B12 (thread_rng) | fixed-point absent, FP transcendental cross-arch non-determinism is the codec-killer |
| 13 | ZK-proof anchoring (EZKL/risc0/sp1) | Batch 4 §2.7 | REJECT per-message; checkpoint/light-client only |
| 14 | Sparse tensor graphs (COO/CSR/Z-order/Morton/block-SIMD) | Batch 1 A1/A2/A3 | EXTEND-EXISTING; Morton DEFER |
| 15 | Branchless (cmov/masking/sentinel padding) | Batch 1 A3 | ALREADY-EQUIVALENT + EXTEND |
| 16 | Memory-wall (HugePages/THP, arena, repr(align), prefetch, tiling) | Batch 1 A5–A9 | DEFER-WITH-TRIGGER via port template; arena ADOPT (Phase-28) |
| 17 | 3D-spatial memory mapping (tile = HugePage) | Batch 1 A7/A9 | DEFER-WITH-TRIGGER |
| 18 | **Token "pixel"/mipmap LOD compression** | **Batch 6 §3** + Batch 1 A10 | mostly ALREADY-EXISTS; narrow pooling-primitive gap |
| 19 | Distributed shared memory / RDMA / AF_XDP / DPDK | Batch 5 §1.3–1.5 | REJECT-on-physics (hardware absent) |
| 20 | Custom 32-byte L2 Ethernet framing | Batch 5 §1.2 | REJECT-on-physics; fields → signed envelope |
| 21 | **Predictive tensor handoff for moving physical assets** (drone-following tiles, ghost-tile prefetch, atomic pointer swap, shadow-tile multi-hypothesis sim + probability-weighted pruning) | **NO OWNER** | **→ §5.2 gap i (significant)** |
| 22 | Hybrid Logical Clocks (HLC), no clock master | Batch 2 §7 | REJECT physical-clock half; logical half already-equivalent |
| 23 | Gossip-based epoch propagation | Batch 2 §8 + Batch 5 §1.1 | EXTEND-EXISTING |
| 24 | Circuit-breaker / watchdog / Mesh Panic Handler (hard-stop, logical breaker, eval-gate soft-stop) | Batch 3 §1/§2/§3 | breaker designed-not-built; panic-handler = Locked (built); **"eval-gate soft-stop" not named → §5.2 gap iv** |
| 25 | Rolling snapshot / checkpoint-restore + adaptive epoch | Batch 2 §10 | PARTIAL; truncation council-gated; adaptive-epoch DEFER |
| 26 | "Monocoque" safety-as-structure | Batch 3 §0/§7 + **Batch 6 §4** | ALREADY-DOCTRINE; §4 deepens the no-supervisor boundary |
| 27 | Hard invariants > RLHF ethical bureaucracy (AGI safety) | Batch 3 §0/§7 | = repo's own §4 verdict; REINFORCES |
| 28 | FPV-hardware grounding (physical vs ethical failure) | Batch 3 §0 (framing) | framing for #27 |
| 29 | Hermetic Polarity (Faithfulness vs Physics camps) | Batch 3 §0 + **Batch 6 §4** | Polarity P4, `HERMETIC…:66-78` |
| 30 | Descartes-square decision method | (meta-method, used throughout; not a buildable) | not a gap |
| 31 | Self-healing w/o watchdog as emergent flow topology | Batch 3 §6 + **Batch 6 §4** | honored on no-supervisor axis; §4 gives the exact residue |
| 32 | Self-Heal / Self-Terminate / Snapshot-Re-entry 3-way split | Batch 3 §6 | Self-Term fully built; other two partial |
| B1 | Laplace transform of Dirac δ (`L{δ}=1`) | Batch 1 B11 | REJECT (domain-mismatch) |
| B2 | Dim-reduction table (PCA/t-SNE/UMAP/Isomap/LLE) | Batch 1 B12 | PCA adopt-as-existing; stochastic embedders REJECT (determinism) |
| B3 | Kuen surface | Batch 1 B16 | REJECT-as-decorative |
| B4 | Z-transform integration property | Batch 1 B13 | EXTEND (analytic lens) |
| B5 | Nyquist–Shannon sampling | Batch 1 B14 + **Batch 6 §3.3** | EXTEND (real bound); bridge to token-stream LOD |
| B6 | Laplace integration property | Batch 1 B15 | REJECT (domain-mismatch); discrete form = B4 |

### 5.2 Named gaps — concepts NO batch has audited as a coherent unit (over-listed, per operator warning)

**i. `[GAP]` — Predictive tensor handoff for moving physical assets (SIGNIFICANT).** The dialogue's
cluster of *drone-following tiles · ghost-tile prefetch · atomic pointer swap · shadow-tile
multi-hypothesis simulation with probability-weighted pruning · predictive tensor handoff for a
physically-moving asset* is **not audited as a coherent concept by any batch**. Partial touches only:
Batch 5 §1.6 routes "a tile to the right worker" as a userspace admission decision; Batch 1 A6 covers
generic software prefetch; Batch 4 §2.5 rejects *speculative execution* generically. But the coherent
idea — a predictive spatial-tensor handoff that tracks a moving physical asset and prefetches the
tile it is about to need — is unverdicted. This is a live adjacency to two standing arcs the auditor
must flag rather than drop:
  - the **Gaussian-Splatting address-picker arc** (MEMORY: 2-stage pin→3D-scene, per-job rented GPU,
    courier-photo bootstrap) — "moving physical asset + spatial tiles" is that arc's territory;
  - the **Kalman integration arc** (MEMORY: `geo.rs::ema_next` IS a 1D Kalman; "SpectralKalman
    predict-only") — "shadow-tile multi-hypothesis + probability-weighted pruning" is a **particle
    filter / beam search**, and the repo *already has* Kalman prediction to build it on. Note it is
    **not** a NO-SCORING violation: the "probability weight" is over *state hypotheses*
    (predict-track), not over *agents* (Batch 4 §1) — a physics prior, not a reputation score. **This
    is the one concept in the whole dialogue that deserves its own research pass and got none.**

**ii. `[GAP]` — Atomic pointer swap / lock-free tile handoff (sub-item of i).** Batch 1 audits arena,
SIMD, alignment, prefetch — but **not** RCU/hazard-pointer/atomic-swap lock-free memory reclamation,
which is the specific mechanism the "ghost-tile / atomic pointer swap" handoff names. No `AtomicPtr`
audit exists. Small, real, unowned.

**iii. `[GAP]` (minor) — "Flocking / boids" as a distinct coordination model.** Concept #3 dissolves
into gossip (Batch 5) + spectral convergence (Batch 4), which is the correct reduction — but no batch
verdicts *flocking-as-such* (local-neighbor alignment rules producing global order). If the operator
wants boids-style local rules audited as a primitive (vs. the anti-entropy gossip that subsumes them),
that verdict does not yet exist. Likely dissolves, but flagged not silently assumed.

**iv. `[GAP]` (minor) — "eval-gate soft-stop" (3rd Mesh-Panic tier) by name.** Batch 3 verdicts the
hardware hard-stop (fuel) and the logical breaker/Locked-state, but the dialogue's *third* tier — an
**evaluation gate that soft-stops** (degrade to advisory rather than halt) — is not verdicted by that
name. It most likely maps to the drift-gate / admission soft-refuse (Batch 3 §4 Survival-Mode), but
the mapping is not made explicit anywhere.

**v. `[GAP]` (framing) — Part B primitives as token-stream FILTERS.** Batch 1 §3 verdicts each Part-B
math object in isolation (adopt/reject), but the operator's stated *purpose* — "signal (token-stream)
filtering, stream-architecture" — as an integrated filtering pipeline over the token/context stream
is not designed anywhere. Batch 6 §3.3 opens the Nyquist bridge; the full "token stream as a sampled
signal you anti-alias / band-limit / integrate" architecture is unbuilt and unowned. Small,
re-frame-shaped, flagged.

**None of i–v is rejected here** — they are named so the operator sees them; i warrants a real
research pass, ii–v are small or dissolve. Per the operator's explicit warning, over-listed rather
than under-listed.

---

## §6 — Anu / Ananke check

**Anu (derivable, not asserted):** every "same idea" claim in §1 is a live `file:line` read of the
four precedents (`is_redline` :237, mesh `scope.rs:244`, `gov_route` `governance.sh:50` +
`native-trackers:379`, `skillspector build.rs`), not memory; the §2 codec rejection derives from the
repo's *own* written determinism boundary (`rng.rs:22-28`) plus the integer-money counter-example
(`money.rs:151`), not from taste; §3's "already exists" is four cited live surfaces; §4's deeper
finding is a direct read of the *calling* relationship (`hydra.rs:227` calls `integrity_check`, vs
`event_log.rs:389` where the check *is* the commit path) — the distinction between "a check the path
calls" and "a check the path is" is grounded, not asserted. Weakest links, named: the epoch/Merkle-DAG
new-ground (§1.2) is design-shaped extension of a landed blueprint, not yet code (tagged NEW-GROUND,
not VERIFIED); the §5.2-i handoff gap is a research *absence* I assert by having read all six batch
files, which is falsifiable by pointing at a batch that covers it (none does).

**Ananke (structural, not hoped):** the batch adds no machinery — it routes each concept to an
existing authority (latency blueprint §2, Phase-28, Batch 3/4, the Hermetic §4 verdict) and states
exactly what is left uncovered. The one load-bearing structural correction it makes is §1.3's
per-artifact-vs-per-transaction distinction: it keeps the PoQ gate **structurally identical to
verify-before-persist / key_V** (an independent second party checks once, at import) rather than
letting it drift into the optimistic-fraud-proof / statistical-vote forms the repo's theorems and CI
gates forbid — so the good outcome (a safe import gate for foreign compiled decision logic) is forced
by reusing a proven pole, not by a new mechanism. §4's resolution is likewise structural: it names
*why* one leg (tamper) irreducibly needs a check (external threats cannot be typed out of existence)
and points that check at the already-designed independent verifier (key_V), rather than pretending a
cleverer type could remove it.

---

*Batch 6 complete. Research + audit only — no product code, no blueprint written. The four
under-scoped concepts: (1) DecisionUnit gossip IS the Decision Compiler (latency blueprint §2 is
authority; epoch/PoQ-import-gate/Merkle-rollback are the new ground; the gate survives as
once-per-artifact independent replay, not per-transaction optimism); (2) RGB-seed codec is a genuine
third strategy but REJECT-on-cross-arch-transcendental-determinism for general state, sound only as
the integer-PRNG seed and the truncated-spectral generator that already exist; (3) mipmap-LOD is
~85% already-built under other names; (4) the no-supervisor ideal is real for arithmetic invariants
and irreducibly aspirational for the tamper leg. Completeness sweep: one significant unowned concept
(predictive tensor handoff for moving physical assets) plus four minor named gaps, over-listed per
the operator's anti-omission warning.*
