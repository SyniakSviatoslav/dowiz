> **SUPERSEDED (2026-07-17)** — see `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (canonical
> roadmap, phases P01–P30) and `CORE-ROADMAP-INDEX.md` + `CORE-ROADMAP-2026-07-17/` (the Layer A–I
> execution structure). Preserved for historical/audit-trail purposes only. The P-I audit (§2.3)
> found this doc's findings the best-absorbed of all five older masters — every concrete item has a
> named carrier in P01–P30 (Kalman→P04, autodiff→P17, backup organ→P12, eigensolver
> dual-authority→P11 + the eigenvector refactor plan, recall@5 engine→P02/R1-D, etc.); zero
> would-be-lost items.

# Master Integration Plan — Research Findings → Integration Steps (2026-07-14)

> Consolidates this session's deep-research (TensorFlow · Transformer/attention · circuit-impedance ·
> Duplicati/OpenScholar/fileexploree · HMM/Lorentz/Kalman · codebase grounding) **plus** the prior
> cycles (math-first, retrieval/memory, physics-UI/quantum/capture). Tags: **PROVEN** (measured/cited)
> · **PATTERN-TO-APPLY** · **AVOID** · **BUG** (real defect found). Companions: `math-first-architecture-
> blueprint.md`, `internal-retrieval-living-memory-blueprint.md`, `physics-ui-capture-blueprint.md`.

## 0. The through-line (one paragraph)

Every thread converged on the same posture: **reverse-engineer the pattern, never vendor the
runtime** (dowiz's zero-dep / local-first / Rust-native invariant); use the extraordinary math
(resolvent / spectral / Kalman / diffusion) where it is genuinely *unifying and true*, and flag the
rest as notation-only; the **highest-leverage "resistance" is the LLM token economy plus wiring
organs that are already built**, not new kernel micro-optimisation; and **keep friction (a fuse) on
money / auth / RLS** — minimising resistance is not a universal good.

## 1. This cycle's research verdicts (condensed, honest)

### 1.1 TensorFlow + Transformer + LLM-from-scratch
- **APPLY:** (a) a **minimal micrograd-shaped reverse-mode autodiff** (~150–300 LOC Rust, arena
  Wengert tape) — fills the one real empty gap, for the capture-redraw *small-parameter* fitting
  (SIREN/Gaussian-splat); (b) **extend `eqc` to a named-equation IR graph** (`organ.eq.py`) — the
  honest analog of TF's SavedModel/GraphDef, already on eqc's own roadmap, unlocks dead-organ-elim +
  cross-organ CSE; (c) **attention = learned graph diffusion** — `softmax(QKᵀ/√d)V` is *exactly*
  `markov.rs`'s row-stochastic SpMV `Âv`; use as a **retrieval lens** (it proves the cheaper
  fixed-operator PPR/heat-kernel choice is right, not an oversight).
- **AVOID:** vendoring TF/XLA (their problems — dispatch overhead, kernel-launch latency — don't
  exist; LLVM already fuses eqc's inlined output); trainable multi-head attention; a from-scratch LLM
  (no NL task; needs tensor autodiff, out of scope).

### 1.2 Circuit / impedance as a resource framework
- **Genuine framework where it reduces to queueing theory** (Little's Law `L=λW`, M/M/1
  `W=1/(μ−λ)`, **Kingman's VUT** — its `(Ca²+Cs²)/2` variability term is the *rigorous* meaning of
  "impedance ≠ resistance", i.e. burstiness cost), **decoration elsewhere** (literal max-power-transfer
  "impedance matching" misleads — software wants ρ<1 with margin + backpressure; "voltage" isn't
  measurable). `absorbing.rs`'s `N=(I−Q)⁻¹` is literally resistor-network algebra (Doyle-Snell) — with
  the honest caveat that the duality needs reversible chains.
- **Actionable:** the dominant "resistance" today is the **LLM token economy** ($/action + latency),
  not kernel micro-opts (the `Vec<Vec>` matmul is correctly *low* priority — 10×10 fits L1, Roofline
  says not bandwidth-bound). Use the circuit lens as a **diagnostic checklist** (series/parallel?
  bursty/steady? hot/cold boundary? pool headroom?), not a source of new equations. 🔴 **Never apply
  it to money/auth/RLS — there the friction is a fuse.**

### 1.3 Duplicati · OpenScholar · conaticus/FileExplorer — all REVERSE-ENGINEER
- **Duplicati** → a small **native Rust backup organ** alongside pgrust: SHA-256 content-addressed
  blocks + a **remote-rebuildable index** + AES-per-volume. Its *fixed*-block dedup weakness
  **validates the FastCDC choice**; its `compact` orphaned-hash bugs **harden the ATTIC pattern**
  (gate on a waste-fraction tolerance, strictly two-phase: write-attic → verify → drop live).
- **OpenScholar** → no live API, GPU + 250M-vector index disproportionate. The one portable idea:
  **treat a verification failure as a retrieval trigger** (upgrade the `deep-research` skill — one
  feedback sentence → one targeted re-search → re-verify that claim, capped ≤2 rounds).
- **conaticus/FileExplorer** → independently migrated to a **hand-rolled zero-dep prebuilt index
  (Adaptive Radix Tree)** — real-world confirmation of the blueprint's trigram-over-tantivy bet.

### 1.4 HMM + Lorentz → Kalman (the "Brain + Body" synthesis)
- **Kalman is the fusion** — a continuous-state HMM with linear-Gaussian dynamics: **predict**
  (geometric state transition = Body; `A` can be an `SE(3)` matrix) + **update** (Bayesian correction
  = Brain). **`geo.rs::ema_next` IS the steady-state 1D Kalman** (proven: EMA α = the fixed-point
  gain). **Build order: Kalman first** (real waiting consumer — live courier tracking; port bebop's
  `kalman.rs`, generalize `ema_next` to `[lat,lng,v]`), `SO(3)/SE(3)` **second** (Galilean, *not*
  Lorentz — couriers ≪ c), Viterbi/HMM **last** (research queue). The invariance principle →
  **money is a non-covariant invariant scalar** (a Noether framing of `money_guard`; a docs note).

## 2. Real findings that change the plan (BUGs + hazards)

- **BUG — 3 broken backup scripts.** `scripts/backup-{verify,restore,drill}.ts` import from the
  now-deleted `apps/api/src/workers/backup/*` (the C2-declutter commit `e1505e1d` moved that tree to
  `attic/apps-api/...` and repointed *some* scripts, not these three). The backup logic
  (dump/encrypt/upload/verify/R2) is fully written + tested; only the wiring is stale.
- **Hazard — cross-repo dual-authority eigensolver.** Three independent eigensolvers now exist with
  no parity gate: dowiz Faddeev-LeVerrier+Durand-Kerner (`kernel/src/spectral.rs`), bebop2 Jacobi
  (`kalman.rs`), bebop2 Francis QR (`lyapunov::eigenvalues_general`, wrapped in
  `agent-governance-wasm`). This is the exact hazard `markov.rs:1-8` already fixed once (Python↔Rust),
  re-opened cross-repo. (Note: the two repos are already structurally coupled — `agent-governance-wasm`
  has a path dep into `bebop-repo`.)
- **Stranded asset — the `recall@5=1.0` living-knowledge engine** is fully built at commit `545f37df`
  on `origin/backup-wip-2026-07-08` / `origin/feat/sovereign-core-phase-zero` but **absent from the
  current branch** (only `node_modules` on disk). Biggest "built, not wired" item.
- **Gap — `SpectralKalman` is predict-only** (covariance propagation in the eigenbasis; no gain `K`,
  no `H`/`R` observation model). Extending it to a full filter is a bounded add using its own
  `invert()`.
- **Perf — `Vec<Vec<f64>>` matmul** in `spectral.rs`/`absorbing.rs`; the **flat-`Vec<f64>` fix already
  exists** in `kalman.rs` (mechanical back-port).

## 3. Integration roadmap (ordered by leverage × proof)

**Tier A — wire what's already built (highest ROI, lowest risk):**
- **A1. Fix the 3 broken backup scripts** — repoint imports to `attic/apps-api/...` (or promote that
  tree out of attic). [BUG · trivial · restores a tested backup path]
- **A2. Resurrect the living-knowledge engine** — cherry-pick the 18-file `545f37df` forward onto the
  canonical branch, re-point at the 174-file living-memory corpus. [recall@5=1.0, no new math]
- **A3. Reconcile the 3 eigensolvers** → one canonical (the general Francis QR handles complex
  eigenvalues like dowiz's FL+DK); others delegate; add a cross-repo parity gate. [dual-authority]
- **A4. Back-port the flat-`Vec<f64>` matmul** (`kalman.rs` pattern) into `spectral.rs`/`absorbing.rs`
  — DOD/cache win, mechanical. (Ties to math-first S3 + the CSR plan.)

**Tier B — the new proven organs (patterns to apply):**
- **B1. Kalman — the state-estimation organ.** Extend bebop `SpectralKalman` predict→full filter
  (gain `K`, obs `H`/`R`, innovation, reuse `invert()`); generalize `geo.rs::ema_next` → a
  `[lat,lng,v_lat,v_lng]` constant-velocity courier Kalman. Deterministic (fixed `dt`), real
  consumer (live tracking). *Answers the operator's "Viterbi vs geometric" question: Kalman first.*
- **B2. Minimal reverse-mode autodiff** (~150–300 LOC, arena tape) — for capture-fitting's small
  parameter fits. GPU real-time fitting stays quarantined.
- **B3. `eqc` → named-equation IR graph** (`organ.eq.py`) — the SavedModel analog; wire eqc's
  fixed-point emission into `dmd.rs` RLS / `enrich.rs` Adam for a **deterministic online learner**.
- **B4. Native Rust backup organ** (Duplicati pattern) — content-addressed blocks + remote-rebuildable
  index + AES-per-volume + FastCDC dedup + two-phase compaction. Closes the ops **3-2-1-1-0 off-Hetzner
  gap**; unifies with the memory ATTIC compaction.
- **B5. L0 trigram search layer** (blueprint spec, ~400–900 LOC, `regex-automata`/`memchr` verify;
  conaticus-confirmed) + an ART secondary index.

**Tier C — process / framework upgrades (cheap, high-value):**
- **C1. deep-research: verify-failure → retrieval-trigger** (OpenScholar pattern; one conditional
  edge, ≤2 rounds; mirrors the N=3 loop cap).
- **C2. Circuit/resource diagnostic checklist** as a standing lens — token-economy-first; replicate
  the `VertexBridge` (0-JSON/1-writeBuffer) + Supavisor-pool-headroom patterns; 🔴 never on money/auth/RLS.
- **C3. Attention-as-diffusion lens** cited in the L3 retrieval reasoning.
- **C4. Invariance-principle docs note** — money as a non-covariant scalar (Noether framing of
  `money_guard`).

**Tier D — deferred / research (honest):**
- **D1. Viterbi/HMM** agent-hidden-state (research queue; no demonstrated gap `markov.rs` doesn't cover).
- **D2. GPU-heavy capture fitting** (quarantined; live-video fit = a research contribution, not integration).
- **D3. Trainable attention / from-scratch LLM / TF-XLA vendoring** — AVOID.

## 4. The last summary, folded in (physics-UI + quantum + capture)

Prior cycle (committed `12434ed2`): the **resolvent/Green's-function spine** (`f(L)` spectral
calculus in Dirac notation) is the honest "extraordinary math"; the **grand unification** = ONE
Laplacian `L` across memory-recall + salience-decay + UI-layout + UI-motion + UI-blur (blur ≡ heat
kernel); the **physics-GPU UI** (wgpu sole dep, SDF + Slug text, no-DOM via AccessKit); **capture→redraw**
(zero-dep capture + DCT floor; Gaussian-splat/SIREN offline; redraw solved via web-splat; the novel
"store forces `S(t)` not frames" codec). **This plan supplies the missing "body" systems around that
"field" core:** Kalman (state estimation), autodiff (the fitting seam), the backup organ (durability
floor), and the token-economy governor (the dominant resistance).

## 5. The unifying picture (one operator, one governor, one floor)

- **One graph operator `L`, its spectral calculus `f(L)`** (resolvent / heat / attention / quantum-walk)
  = memory recall + internal search relatedness + UI field + blur. Computed as **CSR-SpMV +
  deterministic fixed-point** (math-first S3).
- **Kalman** (predict = geometric-transform *Body* + update = probabilistic *Brain*) = state estimation
  *over* that operator's world (couriers, the agent's own loop-state).
- **Autodiff** = the differentiable seam (capture-fitting, deterministic online learning via eqc).
- **The circuit/queueing lens** = the resource governor — token-economy-first, Kingman-aware, with the
  `VertexBridge`/pool-headroom reference patterns.
- **Backup + ATTIC + dedup** = the durability floor — never-delete, remote-rebuildable, two-phase.
- All **zero-dep**, all **deterministic where gated**, **friction-as-fuse on money/auth/RLS**.

## 6. Immediate next step (recommendation)

Highest ROI with lowest risk is **Tier A** — and two of its items are *real defects/stranded assets*,
not speculative features:
1. **A1** — fix the 3 broken backup scripts (a genuine defect; ~minutes).
2. **A2** — resurrect the `recall@5=1.0` living-knowledge engine onto the canonical branch.
Then **B1 (Kalman)** as the first *new organ* (it directly answers your build-order question and has a
live consumer). Say the word and I'll start with A1+A2 (safe, proven) or jump to B1 (Kalman), on
`feat/kernel-fsm-graph-analysis` or a fresh branch.

---

### Key sources
TensorFlow / OpenXLA / SavedModel docs · Vaswani 2017 (Attention) · nanoGPT · Raschka LLMs-from-scratch
· karpathy/micrograd · "Transformers are GNNs" (Joshi 2020) · Little's Law · Kingman VUT · Jackson
networks · Roofline · Doyle-Snell (random walks ↔ electric networks) · Duplicati docs (dblock/dindex/
dlist) · OpenScholar (arXiv:2411.14199) · conaticus/FileExplorer (ART) · Rabiner HMM tutorial · Viterbi
· Kalman/Särkkä · Lorentz group / Galilean contraction · Noether. Codebase: `tools/eqc/`,
`kernel/src/{spectral,absorbing,markov,geo}.rs`, `bebop2/core/src/{kalman,dmd,vsa}.rs`,
`crates/bebop/src/{enrich,memory,lanes}.rs`, `bebop2/proto-wire/src/bpv7.rs`, `engine/src/bridge.rs`,
`scripts/backup-*.ts` + `attic/apps-api/src/workers/backup/*`, `spikes/living-knowledge/` (@`545f37df`),
`deploy/pgrust.*`, `packages/config/src/index.ts`.
