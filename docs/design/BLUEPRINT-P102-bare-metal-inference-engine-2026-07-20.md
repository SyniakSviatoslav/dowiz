# BLUEPRINT P102 — Bare-Metal Native Inference Engine: zero-dependency, two-model real-time crosswire (2026-07-20)

**Status: BLUEPRINT / PLAN — no code written, no crate created, no model pulled.**
**Date:** 2026-07-20
**Component:** AGENT (engine crate) / CORE (discipline + port seams) — mobile half feeds DELIVERY P52/P71.
**Numbering:** P102 — next free after P101 (P99/P100 permanently skipped per the
latency-percentile lexical-collision ruling recorded in P101's own header).

## 0. Operator rulings this blueprint executes (recorded, closed — not reopened below)

Three operator rulings landed 2026-07-20, in sequence, all final ("no further discussions"):

- **R-1 — Build the fully custom bare-metal inference engine, zero external crates.** No
  candle, no mistral.rs, no llama-cpp bindings/FFI, no ONNX runtime. Everything hand-written
  in Rust: GGUF loader, SIMD compute kernels, KV-cache/attention, tokenizer, sampler,
  scheduler. This takes the repo's existing pure-`std`/zero-external-dependency default
  philosophy (`engine/` is offline-clean; the kernel default build is pure-`std`/serde-free;
  PQ crypto is implemented in-repo and KAT-gated rather than imported) to its full conclusion
  for the inference subsystem. The engine is the deliberate long-term replacement for the
  Ollama serving path.
- **R-2 — Exactly two concrete, fully native offline models, crosswired in real time:**
  **LFM2.5-VL-450M** and **SmolVLM-256M-Instruct**. Not primary/fallback — both run
  concurrently, actively complementary ("real time crosswired"). One pair, used everywhere:
  the same two models are the mobile courier-app VLM choice *and* the two models the server
  engine runs. Not an abstract N-slot tier system — the engine is designed around exactly
  these two.
- **R-3 — O-1 (the LFM Open License question P101 §3.3 raised) is RESOLVED: "clear to
  ship."** LFM2.5-VL-450M is explicitly authorized to ship under the LFM Open License v1.0.
  The license facts stay on record (§3.1) per this repo's decisions-get-recorded-not-erased
  discipline; the decision itself is closed.

What remains genuinely open for design — and what this document does — is the *engineering*:
the crosswire dataflow, the engine architecture, the scheduling on this exact box, and a
phased build plan with falsifiable per-phase acceptance ("verified, not claimed"). The
research pass that preceded these rulings (Ollama-vs-raw-engine overhead measurements, GGUF
format internals, PagedAttention applicability, this box's ISA reality) is used below as
engineering input for building the engine correctly.

## 1. Grounding facts — the physics the engine is designed against

All live-verified, none assumed:

| Fact | Value | Source |
|---|---|---|
| CPU | AMD EPYC-Milan (Zen3), 4 physical cores × SMT2 = 8 vCPU, 1 socket/NUMA node | P101 §1 (`lscpu`, live 2026-07-20) |
| SIMD | `avx`, `avx2`, FMA present; **no AVX-512** (arrived Zen4/Genoa), **no AMX** (`grep -c amx /proc/cpuinfo` = 0) | P101 §1, live-verified |
| RAM | 30 GiB total, ~28 GiB available | P101 §1 |
| GPU | none | P101 §1 |
| Measured LLM ceiling on this host | `llama3.1:8b` Q4: **flat 9.21 / 9.36 / 9.80 tok/s at 1/2/4 concurrent streams** — ~4.9 GB weights per token ⇒ host already moving ~49 GB/s. Decode is **memory-bandwidth-bound**; concurrency does not scale past one 8B stream | `BLUEPRINT-P-F-local-ai-mesh.md` §0 + re-measurement (its line 536) |
| Kernel latency tier | `place_order` decide/fold p99 = 140 ns — pure in-memory FSM, zero I/O | kernel benches |
| Ollama wrapper tax (external measurement) | raw llama.cpp 77.0 tok/s vs Ollama 69.1 tok/s (**10.3%** throughput penalty); single-turn REST adds **12–18%** latency; Go runtime ~**1.2 GB** extra RAM | Inventive HQ benchmark (research pass) |

Three consequences, stated once and built on everywhere below:

1. **Target AVX2, not AVX-512.** The original proposal's AVX-512/AMX framing does not apply
   to this server — Zen3 provably lacks both. All hand-written kernels target
   `std::arch::x86_64` AVX2+FMA with runtime `is_x86_feature_detected!` dispatch and a
   permanent scalar reference path (§4.3). An ISA seam stays open for AVX-512 (Zen4+ hosts)
   and NEON (the mobile half, §7 Phase 7) — added later behind the same dispatch, never
   assumed.
2. **The determinism dividend is the honest headline, not nanoseconds.** LLM decode is
   bounded below by weights-bytes-per-token ÷ DRAM bandwidth — milliseconds per token on any
   engine, forever; the 140 ns `place_order` tier is a different physics regime and this
   document never compares the two. What a hand-written engine *can* deliver that the
   Go-daemon path cannot: **same weights + same prompt + same seed + same ISA path + same
   thread partition ⇒ bit-identical token stream, run-to-run**; zero GC pauses; zero heap
   allocation in the steady-state decode loop (measured under `count-allocs`, §4.6); bounded
   p99/p50 inter-token jitter. That is kernel-grade *discipline* applied to inference — plus
   the measured wrapper savings (throughput/latency/RAM, table above) and one fewer non-Rust
   daemon in the sovereignty story.
3. **The two-model ruling makes true concurrency physically viable on this box.** Two 8B
   streams could not scale (flat 9.21→9.36 tok/s — the bandwidth wall). The ruled pair is
   ~0.5 GB (LFM2.5-VL-450M ≈ Q8) + ~0.27 GB (SmolVLM-256M ≈ Q8) of weights: both decoding
   simultaneously at T tok/s each demand ≈ 0.75·T GB/s — at the measured ~49 GB/s class
   ceiling that is an upper bound near ~65 tok/s *each, concurrently* (upper bound, not a
   promise; small-model decode picks up per-token overhead that Phase 0 measures). R-2's cap
   at exactly two small models is what turns "real time crosswired" from a bandwidth fantasy
   (at 8B scale) into a workload this box can actually run. §3.3 designs the scheduler around
   exactly this.

## 2. What the engine replaces, and what it must not touch

**Replaces (long-term):** the Ollama daemon path on the hub — Go control plane, HTTP/JSON
per-request serialization, CGO boundary, LRU model scheduling. P101's Ollama-native topology
(residency plan, static `TaskClass` routing, intake cascade) is the **shipped near-term
truth and stays authoritative until each lane's cut-over gate is green** (§6.1, §7 Phase 6).
This blueprint does not un-ship P101; it defines what eventually supersedes it, per R-1.

**Must not touch:**

- **The kernel's default build.** `dowiz-kernel` gains no dependency and no inference code.
  The engine is a new standalone crate (`infer/`, `dowiz-infer`, §4.1) that path-depends on
  `dowiz-kernel` for port types only — the same direction of dependency `llm-adapters` has.
  `kernel/src/attention.rs`'s own stance ("the kernel stays non-AI") holds: inference lives
  at the edge, behind ports.
- **The money/order process boundary.** The engine links into the agent-lane service process
  (`agent-loop` service / a dedicated serving binary), never into `native-spa-server` or any
  process holding order/money authority — the P40 DECART-B firewall extends to `dowiz-infer`
  verbatim. Worth stating: because the engine is pure Rust with a handful of audited unsafe
  islands (arena, SIMD intrinsics, optional syscall seam) and **zero C/C++ FFI**, the
  crash-isolation argument that forces daemon separation for a C++ engine is structurally
  weaker here — but the firewall is kept anyway; defense in depth costs nothing.
- **`llm-adapters`' crate boundary.** The local-model-wiring blueprint §1.1 locked
  `llm-adapters` as a sync HTTP client with no in-process ML runtime — that *crate-level*
  lock stands untouched: no tensor code enters `llm-adapters` or the kernel. Its
  *deployment-level* corollary ("local model wiring is an HTTP-client problem, never
  in-process") is superseded **at end-state only, by R-1, scoped to the dedicated `infer/`
  crate** — recorded here rather than silently contradicted, and flagged for a DECISIONS.md
  entry (§8). Until Phase 6 cut-overs, the HTTP path remains exactly as locked.

## 3. The two-model crosswire — the core design

### 3.1 The pair (facts + license record)

| | **SmolVLM-256M-Instruct** ("S") | **LFM2.5-VL-450M** ("L") |
|---|---|---|
| Role in the crosswire | Fast perception lane: first-pass scene/OCR gist, framing checks, instant preview | Agentic lane: tool selection, structured extraction, grounding/pointing |
| License | Apache-2.0 (fully permissive) | **LFM Open License v1.0** — Apache-2.0-based, **not OSI**; free commercial use while the using company's annual revenue < $10M USD, above that a Liquid AI commercial agreement is required; nonprofits exempt. **O-1 RESOLVED 2026-07-20 — operator ruling, verbatim: "clear to ship."** Authorized to ship under these terms; the revenue-cap clause stays recorded here for future awareness (a >$10M-revenue mesh participant would need its own Liquid agreement). Decision closed. |
| Measured evidence (research pass) | DocVQA 58.3, OCRBench ≈526/1000; proven phone deployments (<1 GB single-image); **no demonstrated tool-calling** | OCRBench **684**/1000 (strongest doc-OCR in class), RefCOCO-M 81.28 (grounding), **BFCLv4 21.08** — the only sub-1B model with *measured* agentic evidence at all ("measurable, not reliable" — the suites decide, §7); vendor sub-250 ms edge claim (unreproduced — Phase 0/4 marks it reproduced-or-not, Moonshine-precedent wording) |
| Architecture (to be pinned from GGUF tensor maps at Phase 0, never from datasheets) | SigLIP-family ViT vision tower + **SmolLM2-135M llama-family text decoder** (RMSNorm/RoPE/GQA/SwiGLU) | **LFM2 hybrid backbone** (gated short-convolution blocks + grouped-query attention) + SigLIP2-family vision tower |
| Weights (Q8_0 class) | ≈0.27 GB | ≈0.5 GB |

Two structural gifts of exactly this pair, worth naming because they shape the build order:

- **Both vision towers are SigLIP-family** — one hand-written ViT encoder kernel set serves
  both models (§7 Phase 3 builds it once, Phase 4 reuses it).
- **S's text side is pure llama-family** — the best-documented decoder architecture, so it
  doubles as the engine's reference architecture for the KAT discipline (§4.5) before the
  more exotic LFM2 hybrid is attempted. The build order S-then-L is engineering sequencing,
  not a priority ranking — both ship, crosswired, per R-2.

P101 §3.2 ranked these same two models as primary-vs-fallback for mobile. **R-2 supersedes
that framing: both ship, concurrently, as one crosswired unit, on both surfaces** (mobile
courier app and server engine). P101's scenario-suite discipline (§3.5 there) carries over
unchanged — it now scores the pair *and* the crosswired composite, not a winner.

### 3.2 Crosswire dataflow — two modes, one deterministic comparator, zero LLM-judging

"Real time crosswired" is designed as two concrete modes, chosen statically per call site.
Both keep the models concurrently hot and genuinely complementary; neither is a fallback
chain (no "S failed, try L" rung anywhere):

- **Mode A — preview + verify (concurrent).** Both lanes receive the same input
  simultaneously. S (fast) returns a structured `PerceptionDraft` — caption gist, extracted
  fields (OCR lines, label text), framing verdict — typically well before L completes. The
  caller surfaces S's draft immediately (courier capture UX: sub-second framing/preview
  feedback while L reasons). L's output — tool call / structured extraction — is the
  **authoritative** result. On L's completion a **deterministic comparator** (plain Rust
  field logic: exact/normalized equality on overlapping extracted fields — order IDs,
  label strings, line items; *never* an LLM judging an LLM) checks S's draft against L's
  result:
  - agree → `CrosswireOutcome::Corroborated(result)`;
  - disagree on a required field → `CrosswireOutcome::Disagreement { field, s_value,
    l_value }` — surfaced as the **refusal form** (re-capture prompt / human path), never a
    silent pick of either model. Same negative-case-first discipline as the golden suites: a
    confidently wrong "package present" is worse than "can't tell". Two small models
    disagreeing is *signal*, and the pair yields it for free.
- **Mode B — draft-conditioned (pipelined).** S runs first-pass perception; its
  `PerceptionDraft` is injected into L's prompt (structured, delimited, schema-pinned) so L
  reasons *over* S's fast perception rather than from pixels alone. Per-request this is a
  pipeline (L waits ~S's small latency); under any sustained load both lanes stay
  continuously busy on different requests — concurrency at the system level. Used where the
  output is a single structured artifact and there is no interactive preview to serve
  (server-side intake assist, comms drafting with an attached image).

Call-site mapping (P101 §1's real call sites, not invented ones):

| Call site | Mode | Notes |
|---|---|---|
| P52 K4 courier capture assist (PoD photo sanity, receipt/menu/label OCR) — mobile | **A** | S draft = instant framing/preview; L = field extraction + capture-tool call; comparator on extracted fields |
| CS-4 intake assist (P48-INTAKE) | **B** | S perceives/normalizes, L extracts; the deterministic sufficiency oracle stays the pure `IntentParser` (P101 §4.4) — if the crosswired output still re-parses `Ambiguous`, the path is the existing human backstop. The crosswire *replaces* P101's S-then-G escalation rung inside the pair; the human backstop is unchanged |
| CS-5 comms drafting (D14, opt-in human-gated) | **B** | Draft-conditioned; human gate unchanged |
| CS-1 agent loop | gated | L is the tool-selection lane; **cut-over from `llama3.1:8b` only if the pair clears CS-1's golden-suite bar** (§6.1). BFCLv4 21.08 is "measurable, not reliable" — the suite decides, not this document |
| CS-2 code lane, CS-3 embedding lane | out of pair | See §6.1 residuals — a 450M VLM is not a code model and neither model is an embedder; these lanes stay on the P101 topology until their own dispositions |

### 3.3 Scheduling — exactly two lanes on four cores, static by design

R-2's "exactly 2" collapses the scheduler to something small enough to verify:

- **Two permanent lanes**, one per model. Both models load at boot into their own
  never-reset weight arenas (§4.2) and stay resident for the process lifetime. **No
  eviction, no LRU, no load queue, no N-slot registry — that machinery has no reason to
  exist for a fixed pair** and is therefore not built (unrepresentable > forbidden, same
  spirit as the routing enums omitting `Ord`).
- **Static thread partition, v1: L = 3 worker threads, S = 1**, on the 4 physical cores
  (SMT siblings left to the OS; measured, not assumed — §7 Phase 5's matrix includes a 4+4
  SMT cell). Partition is fixed at boot (policy-as-data in config), no work stealing, no
  dynamic rebalancing in v1 — determinism and measurability first. The
  `kernel/src/core_pinning.rs` seam (today a deliberate no-op; its DECART found no locality
  win on single-socket) gets its first real justification here — **partition isolation and
  jitter**, not locality — and Phase 5 measures pin-vs-no-pin before the no-op is replaced.
- **Concurrency ceiling: two decode streams, total** — one per lane, matching the locked
  dispatcher `workers: 2` and P101 §2's bandwidth argument. Within a lane, requests queue
  FIFO (bounded, degrade-closed `Busy` past the bound, mirroring `WorkerSlots`).
- **KV-cache: one static ring-slab per lane** (operator's own layer-3 design — correct for
  this workload). Allocated once at boot for the configured `n_ctx`, reused across
  turns/sessions via `O(1)` reset (the `BumpArena::reset` discipline), **overflow is a typed
  `InferError::CtxExhausted`, never silent eviction/truncation** — forbidden transitions are
  errors, not no-ops, same as the order FSM. **PagedAttention is deliberately not built:**
  it solves KV fragmentation across *many concurrent variable-length sessions* competing
  for a constrained pool (vLLM-scale multi-tenant serving; <4% waste vs 60–80%). This box
  runs exactly two low-concurrency lanes with statically sized contexts — the static slab
  is sufficient and far easier to verify. **Named re-open trigger:** if dowiz ever serves
  many concurrent independent inference sessions per hub (multi-tenant chat-scale), paged
  KV becomes worth reconsidering — that trigger, not taste, reopens it.

## 4. Engine architecture — `infer/` (`dowiz-infer`)

### 4.1 Crate shape and dependency law

New standalone crate at repo root (sibling of `kernel/`, `engine/`, `llm-adapters/`; no
workspace, `cd infer && cargo test`):

- `[dependencies]`: **`dowiz-kernel` (path, port types only). Nothing else. Ever.** The
  crate-level test that enforces it: `cargo tree -e no-dev` must list exactly two nodes
  (dowiz-infer, dowiz-kernel) — wired as a CI grep in the same style as the kernel's
  serde-absence check.
- Features (all off by default): `mmap-syscall` (§4.2), and the consumer-side
  `count-allocs` passthrough to the kernel's counting allocator. AVX2 is **runtime**
  dispatch, not a feature — one binary serves any x86_64 host, scalar fallback always
  compiled.
- Modules: `gguf` (parser/loader) · `tok` (tokenizers) · `kern` (compute kernels: scalar +
  AVX2) · `arch` (model graphs: `llama_family`, `siglip`, `lfm2`) · `kv` (ring-slab) ·
  `sample` (greedy/temp/top-p + hand-rolled seeded PCG32) · `sched` (two-lane runtime) ·
  `xwire` (crosswire modes + comparator) · `ports` (kernel port impls).

### 4.2 Loader — one-time static allocation, `BumpArena` reused as specified

- **GGUF v3 parser, hand-written, pure std:** magic/version, metadata KV section (arch,
  alignment, tokenizer vocab/merges/scores, chat template), tensor infos (name, dims,
  quant type, offset), data section aligned per `general.alignment` (default 32). Typed
  errors for every malformation (`GgufError::*`), **never a panic on untrusted bytes**; the
  parser is fuzzed with deterministic seeds in Phase 1 acceptance.
- **Weights land in one never-reset `BumpArena`-backed region per model** — the existing
  `kernel/src/arena.rs` primitive, reused, not reinvented: page-aligned via the arena's
  address-rounding (`alloc_slice` aligns the *address*, item-52 Miri finding), one-time
  allocation at boot, degrade-closed `None` on exhaustion (boot fails typed, never
  half-loads). Scratch (dequant tiles, logits, tokenizer buffers) uses per-lane reset
  arenas — the textbook phase/region shape arena.rs documents.
- **Phase 1 path: `std::fs` read into the arena.** One copy at boot, then zero-copy
  forever. Honesty about the original mmap framing: the layer-1 mmap design was sized for
  multi-GB weights; R-2's pair totals **<1 GB**, so a one-time boot read (sub-second from
  page cache) achieves "page-aligned, one-time static allocation" with strictly less
  `unsafe`. **The mmap seam is kept, not dropped:** feature `mmap-syscall` (Phase 5+) adds
  raw `mmap`/`madvise` via stable-`asm!` x86_64-Linux syscall stubs — zero crates, audited
  unsafe island — and gives `HugePageHint` its first real backend (`MADV_HUGEPAGE`): the
  seam's own documented trigger, a persistent tensor region >2 MB, now genuinely fires.
- **Quantization support, phased:** Q8_0 (34 B/32 weights: f16 scale + 32×i8) → Q4_0
  (18 B/32) → K-quants Q4_K (144 B/256 superblock: 2×f16 super-scales + 12 B of 6-bit
  sub-scales/mins + 128 B nibbles) and Q6_K (210 B/256). On-the-fly dequant fused into the
  GEMV kernels, per the operator's layer-2 spec — no dequantized-weights copy ever
  materializes.

### 4.3 Compute kernels — AVX2 + permanent scalar reference

- Hand-written `std::arch::x86_64` kernels: quantized GEMV via the integer dot path
  (`_mm256_maddubs_epi16` + `_mm256_madd_epi16`, f32 FMA accumulation per block scale),
  dequant, RMSNorm, LayerNorm+GELU (SigLIP side), softmax, RoPE, SwiGLU, and the LFM2
  gated-short-conv block. Runtime `is_x86_feature_detected!("avx2")`+FMA dispatch; scalar
  reference implementations are **permanent, not scaffolding** — they are the parity oracle
  and the portable floor (wasm/NEON later).
- **Threading = row partitioning, no cross-thread reductions.** GEMV output rows are
  independent dot products; each row is computed wholly by one thread, so thread count
  changes *scheduling*, never *arithmetic*. Within a row, SIMD accumulation uses a fixed
  lane count and a fixed tree-reduce — deterministic run-to-run on a given ISA path.
- **Stated divergence from `kernel/src/simd.rs`'s bit-identity rule** (vectorize across
  rows, never within a row's reduction, so SIMD replays scalar op order exactly): a
  bandwidth-bound GEMV row dot *requires* in-row vectorization; forfeiting it forfeits the
  engine. The discipline adapts rather than breaks: scalar-vs-AVX2 parity is **ULP-bounded
  property-tested** (hand-rolled deterministic PRNG loops, no proptest crate), each ISA
  path is *individually* bit-deterministic, and token-stream KATs are pinned **per ISA
  path** (§4.5). Greedy argmax uses a deterministic tie-break (lowest token id) so
  ≤ULP logit ties cannot produce nondeterminism within a path.

### 4.4 Tokenizers and sampler — zero-allocation on the steady path

- Both tokenizers are byte-level BPE families read from GGUF metadata (S: SmolLM2/GPT2-style
  vocab+merges; L: LFM2's own vocab). Hand-written merge loop over caller-provided `&[u8]`
  with arena scratch and pre-sized token buffers; round-trip encode/decode parity is pinned
  against Phase 0 fixture corpora per tokenizer. Chat templates and image-token splicing
  layouts (S's sub-image grid tokens; L's own layout) are pinned as fixtures at Phase 0 —
  template drift is the classic silent integration bug, and the golden suites are the
  detector.
- Sampler: greedy (default for gate-relevant lanes) + temperature/top-p behind a hand-rolled
  seeded PCG32. Deterministic by construction; seed is part of the request, echoed in
  telemetry.
- **Image decode stays outside the engine.** The engine consumes decoded RGB8 (`&[u8]` +
  dims) at the port boundary; PNG/JPEG decoding belongs to the caller (mobile app / serving
  binary). Scope cut that keeps zero-dep honest — a from-scratch JPEG decoder is a separate
  security-sensitive project this blueprint refuses to smuggle in.

### 4.5 Correctness discipline — KAT-gated like the PQ stack, parity-pinned like eqc_gen

The engine's correctness story copies the repo's two strongest precedents:

- **Token-stream KATs (the ACVP pattern):** Phase 0 generates fixture corpora — temp-0
  greedy continuations (token IDs), tokenizer round-trips, chat-template renderings, and
  per-layer activation traces for the first tokens — using a reference runtime as a
  **dev-time oracle only** (run on the operator's box to produce committed fixtures; never
  a build/runtime/CI dependency — exactly how NIST vectors gate `kernel/src/pq/` without
  NIST code in the tree). Every fixture header records oracle name/version/quantization.
  Phases 1–4 must reproduce pinned token IDs **bit-exactly per ISA path**; activation traces
  exist for bisecting the first divergence when a KAT goes red.
- **Self-check style:** the loader validates tensor-map completeness against the arch graph
  (missing/extra/mis-shaped tensor = typed boot error); the two-lane scheduler's state
  machine gets the same forbidden-transition-is-an-error treatment as the order FSM.
- The deterministic-AI-inference synthesis (2026-07-19) is this arc's discipline ancestor —
  its grounding (attention.rs lens, simd.rs bit-identity, mat.rs, CORDIC float-rejection
  precedent, literal-zero-dep constraint) applies here verbatim. Its item-34 toy-pilot ruling
  (own-kernel *fixed-point* inference pipeline proven on a synthetic classifier first) is a
  **separate, unmodified track**: P102 is f32-accumulation LLM inference at the edge behind
  ports; the kernel-internal fixed-point arc keeps its own sequence. Neither blocks the other.

### 4.6 Falsifiable resource claims

- **Zero heap allocations per generated token, steady state, dowiz-side** — asserted under
  the kernel's `count-allocs` counting allocator (`arena::counting_alloc`), snapshot before /
  after an N-token decode, N scaling with allocations pinned at 0. (Load/boot phase is
  explicitly exempt; the claim is the decode loop.)
- **Resident memory budget:** weights (<1 GB pair) + 2 KV slabs + fixed scratch — asserted
  as an RSS ceiling in the Phase 5 matrix, and compared against the Ollama path's RSS for
  the same pair (the ~1.2 GB Go-runtime saving, verified not quoted).
- **Jitter:** p99/p50 inter-token latency ratio per lane recorded in every Phase 5+ run;
  the determinism dividend must be *visible* there vs the Ollama path or it is not claimed.

## 5. Port and composition seams (Phase 6 detail, named early so nothing drifts)

- **Ports:** the engine implements the kernel's `LlmBackend` (`kernel/src/ports/llm.rs` —
  id/caps/chat/embed/rerank/health; `embed`/`rerank` return `Err(Unsupported)` honestly,
  §6.1 E-1) for text-shaped calls, plus the **`VlmModel`-style sibling port** P101 §3.4
  already prescribed for vision inputs (AsrModel precedent: typed errors mirroring
  `LlmError`, offline fixture stub satisfying the contract with no model present). If a
  vision port lands in the kernel it is its own small reviewed change — this blueprint does
  not smuggle kernel edits.
- **Composition:** one new fail-closed arm in the `AiMode` family (`native`), wired through
  the wiring blueprint's Phase-1 `from_config` seam, off by default; unset env keeps today's
  behavior bit-identical. Default builds of kernel/llm-adapters/native-spa-server are
  untouched (provable: their lockfiles/`cargo tree` do not change at all — the engine is
  consumed only by the agent-lane serving binary behind its own feature).
- **Routing:** static, per R-2 and the HK-05/HK-09 non-duplication ruling — call sites name
  a crosswire mode; there is no learned router, no cascade rung, no model registry.

## 6. Reconciliation with the sibling blueprints (explicit, no silent contradiction)

### 6.1 P101 (local/mobile model selection + Ollama topology) — near-term truth, plus residuals

- **Sequencing, stated plainly: P101 ships now; P102 replaces it lane-by-lane, gated.**
  P101's Ollama residency/routing/cascade remains the operating topology while Phases 0–5
  build. Each lane cuts over to the native engine **only** when the pair-under-native
  meets that lane's own suite bar (P101 §3.5 scenario suite; wiring §3.2 golden
  tool-calling suite), scored against the lane's current Ollama-served model. A lane that
  does not clear its bar stays on P101's topology and says so in the scorecard — capability
  honesty is a *measurement*, not a veto on the build.
- **Superseded within P101:** the §3.2 primary-vs-fallback framing (R-2: both ship,
  crosswired) and the §4.4 S-then-G intake escalation *inside the pair* (Mode B + the
  unchanged human backstop replace it). The O-1 ruling P101 §8.1 requested is **resolved**
  (R-3, recorded in §3.1 here). P101 gets dated append-only annotations pointing here (done
  in the same commit series as this file).
- **Untouched by P102 and honestly named as residuals of "replace Ollama entirely":**
  - **E-1 (embedding lane, CS-3):** neither ruled model is an embedder;
    `nomic-embed-text`/`qwen3-embedding` (encoder-family) stay Ollama-served. Full Ollama
    retirement requires either a native encoder-arch phase (post-Phase 6, own ruling) or a
    standing minimal Ollama/embed sidecar. Open, tracked, not silently absorbed.
  - **CS-2 (code/dev-harness lane):** `qwen2.5-coder:7b` work is out of the pair's class;
    dev-lane stays Ollama until its own disposition. Not product-path; not a launch coupling.
- P101's Phase A/B measurement machinery is *consumed*, not duplicated: its scenario suite
  becomes the pair's mobile gate; its Phase-B matrix discipline is the template for
  Phase 5's concurrency matrix.

### 6.2 Local-model-wiring blueprint — crate lock kept, deployment stance superseded at end-state

Covered in §2: crate-level lock intact forever; "never in-process" deployment stance
superseded by R-1 for the dedicated `infer/` crate only, effective at Phase-6 cut-overs;
recommend recording as a DECISIONS.md entry (§8). Its Phase 1 (`AiMode` composition switch)
and Phase 2 (golden suite) are **prerequisites** P102 consumes — nothing here blocks them.

### 6.3 Deterministic-inference synthesis + P-F physics

§4.5 records the discipline lineage and the separate toy-pilot track. P-F's §0 physics
ruling ("gossip the compiled OUTPUT of inference, not the inference") is untouched — P102
changes *how a hub runs its own inference*, not what crosses the mesh.

## 7. Phased build plan — RED→GREEN, each phase measured before the next starts

Honest scale statement up front: hand-writing a correct GGUF loader, two tokenizers, an
AVX2 kernel set, a ViT encoder, a llama-family decoder, an LFM2 hybrid decoder, and a
two-lane deterministic runtime is a **multi-month, person-months-class build** even at this
deliberately narrowed scope (exactly 2 models, exactly 2 quant families at first, no GPU,
no server framework). The phases are ordered so every one lands something independently
verified and useful, and no phase starts before the prior phase's GREEN is committed.

- **Phase 0 — baselines, fixtures, and targets (measurement only; no engine code).**
  - Pure-std STREAM-triad-style bandwidth probe (`tools/`) → this box's real B GB/s.
  - Pull both GGUFs; measure both via the existing external path on this box (and, when the
    P52/P71 timeline starts, ≥1 real mid-range Android device — P101 Phase A hardware rule):
    decode tok/s, prefill tok/s, first-token p50/p99, RSS. These numbers are the engine's
    published targets; the vendor sub-250 ms claim gets its reproduced-or-not verdict here.
  - Generate + commit the KAT fixture corpus (§4.5) for both models with provenance headers.
  - Run the P101 §3.5 scenario suite + wiring §3.2 golden suite against the pair (via the
    existing serving path) → the capability scorecard that Phase 6's cut-over gates compare
    against, and first real data for crosswire mode tuning.
  - **GREEN:** committed scorecard + fixture corpus + bandwidth number. RED is trivial
    (none of it exists).
- **Phase 1 — GGUF loader + both tokenizers + scalar llama-family decoder; S text-side
  end-to-end (Q8_0).**
  - **GREEN:** parser round-trips both models' metadata/tensor tables and rejects ≥3
    corrupted fixtures with typed errors (no panic under a deterministic-seed fuzz pass);
    tokenizer round-trip parity on both fixture corpora; **64-token temp-0 continuations
    bit-equal to pinned KAT token IDs for ≥10 prompts** on SmolLM2-135M text (scalar path);
    boot-load wall-clock recorded; weight/scratch arenas in place with `high_water`
    telemetry.
- **Phase 2 — AVX2 kernel layer + threading.**
  - **GREEN:** every kernel ULP-parity-tested vs scalar; AVX2-path token KATs pinned and
    passing; 100 consecutive runs bit-identical per path; **0 allocs/token** under
    `count-allocs`; measured decode ≥ **60%** of Phase-0 external-runtime tok/s for the same
    model+quant (intermediate bar — the gap is then a named optimization list, not a shrug).
- **Phase 3 — SigLIP vision tower + pixel pipeline → SmolVLM-256M complete.**
  - **GREEN:** multimodal KATs (image+prompt → pinned tokens); P101 §3.5 scenario suite run
    natively scores ≥ the same model's Phase-0 external-runtime scores; S-lane preview
    latency ≤ 1.25× its Phase-0 external baseline on this box.
- **Phase 4 — LFM2 hybrid backbone → LFM2.5-VL-450M complete (encoder reused).**
  - **GREEN:** same gate structure as Phase 3 for L (KATs, scenario suite, golden
    tool-calling suite native-vs-external parity-or-better); K-quant (Q4_K/Q6_K) dequant
    lands here with its own scalar/AVX2 parity tests.
- **Phase 5 — the crosswire runtime.** Two lanes, static partition, ring-slab KV, both
  modes, comparator, typed outcomes.
  - **GREEN:** a 6-cell measured matrix on this box (P101 §4.5 discipline): S-solo · L-solo
    · S∥L concurrent · Mode-A end-to-end · Mode-B end-to-end · 3+1-vs-SMT partition cell.
    Decision rule mirrored from P101: if concurrent operation degrades either lane's decode
    >25% vs solo, Mode A serializes at the scheduler for that surface and the number is
    recorded; else the concurrent shape ships. Plus: comparator disagreement fixtures →
    typed refusal (never a silent pick); `CtxExhausted` fixture → typed error; RSS ceiling
    assertion; **p99/p50 jitter ≤ the Ollama-path equivalent for the same pair** (the
    determinism dividend, made falsifiable or not claimed).
- **Phase 6 — ports, composition, lane cut-overs.**
  - **GREEN:** port contract tests (including the no-model fixture stub); `AiMode` native
    arm fail-closed with default behavior bit-identical when unset; per-lane cut-over
    scorecards vs the Phase-0/P101 baselines with each lane's verdict recorded
    (cut-over / stays-Ollama); E-1 + CS-2 residual dispositions written down; Ollama
    decommission checklist exists and is gated on every cut-over lane soaking green.
- **Phase 7 — NEON port (aarch64) for the mobile half.** Same scalar reference, same KATs,
  same suites on ≥1 real device; rides the P52/P71 timeline; app-edge port discipline per
  P101 §3.4. (The pair-everywhere ruling R-2 is what makes this a port, not a second
  engine.)

## 8. Non-goals (explicit, closed list)

- No GPU path; no AVX-512/AMX code (this box lacks both — ISA dispatch seam stays open for
  future hosts; hand-written intrinsics remain AVX2+scalar only until a real Zen4+ host
  exists to measure on).
- No PagedAttention (static ring-slab per §3.3; named multi-tenant trigger to reopen).
- No third model, no N-slot registry, no eviction machinery (R-2: exactly two).
- No training/fine-tuning of any kind (P54 deferral holds, P101 §5).
- No learned/real-time router, no cascade rungs (HK-05/HK-09 non-duplication; R-2
  crosswire is static per call site).
- No LLM-as-judge anywhere — the crosswire comparator is plain field logic.
- No HTTP surface on the engine (in-process ports only; serving stays with the existing
  binaries).
- No image-format decoding in the engine (RGB8 at the boundary, §4.4).
- No model weights in git; no auto-pull (wiring §2.3 stance).
- No kernel dependency additions; no code in `llm-adapters`; no change to the money/order
  process boundary.
- No distributed inference (P-F physics ruling untouched).

## 9. Risks — owned, with mitigations (the build proceeds; these are managed, not debated)

| Risk | Mitigation |
|---|---|
| Correctness surface of a from-scratch engine (quant math, attention, templates) | §4.5 KAT discipline: bit-exact token pins per ISA path + per-layer activation traces for bisection; suites that can go RED at every phase |
| LFM2 hybrid architecture is the least-documented piece | Build order S-then-L; Phase 0 pins the real tensor graph from the GGUF itself + activation fixtures; Phase 4 has its own full gate |
| Server reasoning capability of a 450M-class lane vs the incumbent 8B | Phase 6 cut-over gates are per-lane and measured; lanes keep P101's topology until the pair clears their bar — direction is ruled, cut-over is earned |
| Perf gap vs a decade-tuned reference on prefill/vision GEMM (compute-bound, unlike decode) | Split targets: decode is bandwidth-bound (ceiling reachable with straightforward AVX2); prefill/encoder get the 60%→parity ladder with a named optimization list per phase |
| Maintenance of hand-written kernels over time | R-2 bounds the surface permanently (2 models, 2 arch families + 1 shared encoder, 2 quant families at first); bench_track A/B guards regressions |
| Unsafe islands (arena, intrinsics, optional syscalls) | Existing house discipline: Miri where applicable (item-52 precedent), audited SAFETY comments, degrade-closed exhaustion, zero C/C++ FFI anywhere |

## 10. Registration + follow-ups

- Registered in `ROADMAP.md` (§9 chronological, §10.2 index) and `CORE-ROADMAP-INDEX.md`
  §10, same pass as this file.
- P101 annotated (append-only, dated) at §3.2/§4.4/§8: R-2 crosswire supersedes
  primary-vs-fallback; O-1 resolved per R-3.
- Recommended (operator's normal process, not done here): a DECISIONS.md entry recording
  R-1/R-2/R-3 — the in-process-inference end-state supersession of the wiring lock, the
  two-model crosswire, and the LFM license authorization — so the rulings live in the
  decisions ledger, not only in a blueprint header.

---

*End of blueprint. Nothing is built; Phase 0 is startable immediately and touches no
dependency, no crate, and no serving path — it produces the numbers and fixtures every
later phase is gated on.*
