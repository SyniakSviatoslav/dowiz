# Bebop — the language of dowiz: maximal rewrite plan

> **Author:** Sviatoslav Syniak · **License:** AGPL-3.0-or-later · **Date:** 2026-08-17
> **Doctrine:** Over-engineering is the #1 ally (MANIFESTO C8, amended). No slices, no gating — full language + full rewrite, at maximum speed.

**Goal:** Build **Bebop** — a native, glyphic, **agentic** systems language fusing **C's** low-level control, **Rust's** ownership, **Lean 4's** dependent types, **SPARK/Ada's** contracts, first-class SIMD/NTT, an FPGA/ASIC synthesis path, and vector-glyph source (not emoji) — then rewrite **all of dowiz** in it, bit-identical to the Rust reference.

**Architecture:** One **glyphic** surface (no words, ordinary lexicon, vector glyphs not emoji) → **QTT** kernel → **verification** (Lean 4 kernel: every statement proven) + **mandatory compilation, all optimizations** (direct aarch64+NEON / x86_64+AVX-512 machine code, **WebAssembly** for browser, **GPU** compute, no LLVM) + **Calyx/CIRCT** (silicon). Contracts → SMT. Built-in methods: NTT, hypervector, living-memory, hybrid quantum state. Atomic & branchless. **Agentic** (agents + humans, one surface).

**Stack:** QTT (Idris-2 model), direct native codegen, Calyx/CIRCT, Z3/CVC5, **C (bootstrap) → self-host in Bebop**, 12-way swarm.

---

## 1. Naming & doctrine (settled)

| Term | Meaning |
|---|---|
| **Bebop** | **the language** (this document) |
| **dowiz** | the **delivery OS**, written in Bebop |
| **bebop-llm** | the LLM component |
| **bebop-agent** | the agent component |
| **bebop2-protocol** | the protocol (post-quantum delivery) |

**MANIFESTO C8 is amended:** *"Over-engineering is the #1 ally — the deep stack (Bebop language, PQ/mesh, spatial/ASIC tiers) is built first-class, not gated."* The language is not a bet to hedge; it is the substrate.

---

## 2. Measured velocity — the extrapolation

Real git numbers, 2026-08-10 → 08-17 (7 days, 12-way swarm):

```
commits:        202        (~29 / day)
files touched:  632        (~90 / day)
insertions:     255,757    (~36.5K LOC / day)
deletions:      226,653
current tree:   216,614 LOC Rust
```

**Throughput rule:** mechanical work (front-ends, glyphs, stdlib porting, differential tests, KAT vectors, docs) scales **linearly with swarm width** at ~36.5K LOC/day. Design-heavy work (type-theory kernel, SMT lowering, HLS) is the **critical path** — bounded by reasoning, not typing — and runs at a fraction of that, but is itself small in LOC.

| Deliverable | Est. LOC | At measured velocity |
|---|---|---|
| Front-end (lexer/parser/AST) | ~6K | <1 day (mechanical) |
| Vector-glyph font + renderer | ~5K | <1 day (mechanical) |
| QTT kernel (elab/unif/quant/termination) | ~15K | 2–4 days (critical path) |
| LLVM codegen (monomorph/erasure/no_std) | ~8K | 2–3 days |
| Contracts → SMT lowering | ~6K | 2–4 days (critical path) |
| SIMD/vector + `Hypervector` + `Field<P>` | ~4K | ~1 day |
| FPGA/ASIC HLS (MLIR→Calyx) | ~10K | 1–2 weeks (critical path) |
| Tooling (LSP, glyph errors, fmt) | ~6K | 2–3 days |
| dowiz stdlib port (Rust→Bebop, zero-dup) | ~180–200K | 6–10 days (swarm) |
| Differential tests + KAT | ~20K | 3–5 days |

**Total ≈ 300K LOC-equivalent.** Pure LOC throughput ≈ 8 days; but the ~20% critical path (QTT kernel + SMT + HLS) is design-bound, so realistic wall-clock is **6–10 weeks** with the swarm at full width. Not years.

---

## 3. Language design (Bebop)

### 3.1 Core calculus — QTT
Quantities `0 / 1 / ω`. This one calculus delivers both halves:
- **Rust parity** — `1` = move/linear (ownership), `ω` = shared `&T`, `&mut` = unique linear borrow; the borrow checker *is* the quantitative analysis. `Copy` = `ω`.
- **Lean 4 parity** — dependent products `Π (x:A)→B x`, cumulative universes, inductive + quotient types, `0` = proof/erased.

### 3.2 Contracts (SPARK/Ada parity)
`requires` / `ensures` / `invariant` / `ghost` / `reads`·`writes`, on a verifiable subset, lowered to SMT:
```bebop
fn ntt(x: &[Fp<P>; n]) -> &[Fp<P>; n]
  requires  n.is_power_of_two()
  ensures   inverse(ntt(ntt(x))) == x
```

### 3.3 Determinism & purity contracts (the killer features)
- `#[bit_identical]` — SIMD ≡ scalar, proven by the compiler (formalizes `simd.rs`).
- Effect/region types for C2 "pure core": `pure` cannot name `clock`/`rng`/`env`/`float`/`network` (§1.5 unrepresentability as a type).
- `Money : Z` newtype with `no_float` (C5).

### 3.4 Numeric tower
`i64` minor-unit money · `Fp<P>` finite fields · `Zmod<M>` · fixed-point · explicit `no_std` `f64`.

### 3.5 SIMD / vector — first-class
`Vector<W,T>` (portable AVX-512/NEON/SVE/RVV via LLVM). `Hypervector` (1024-bit = `Vector<16,u64>`) with bundling/binding as primitives. `#[bit_identical]` across widths.

### 3.6 Compile-time baking (Zig comptime parity)
`comptime` + pervasive `const fn`. Living-memory index, NTT twiddle factors, code-graph baked into `.rodata` — runtime = pointer deref. Subsumes the daemon (zero cold-start, no daemon needed).

### 3.7 Glyphic surface (no words)
The surface is **glyphs only** — no keywords, no words. Every symbol is a vector glyph (δ-outline); ASCII is a terminal fallback, never the source. **Ordinary lexicon**: `★`=fn, `◇`=struct, `△`=data, `⊙`=contract. Glyphs are vector δ-outlines, **not emoji**. Full alphabet in `BEBOP-GLYPH-ALPHABET.md`.

### 3.8 Concurrency & deep parallelism (first-class)
`∥` parallel composition, `⋉`/`⋊` fork/join, PID-dynamic spawner (dowiz `dynamic_spawner`) as built-in scheduler. Vector/hypervector ops implicitly parallel + `⚛ bit_identical`.

### 3.9 Core methods (dowiz algorithms = Bebop methods)
NTT `⟲⟳`, hypervector `⧉⊕⊗`, FFT/modular `ωₙℤₘ`, living-memory `⌾⤳⋈`, quantum `ψ`, arena (linear regions), money `◉no_float`, event-sourced `△fold`. All **O(n) zero-overhead**.

### 3.10 Atomic & branchless
`⚛` atomic primitives (CAS/LDADD), `⤫` branchless predication (CSEL/CMOV) for the hot path. no_std native + **NEON** reference target.

### 3.11 Self-prediction (hybrid quantum state + relational memory)
`ψ` hybrid state ⊗ living-memory `⌾⤳⋈` relations let a program predict + prefetch its own next hot path — deterministic, RNG-free (C10).

### 3.12 Agentic (agents + humans, one surface)
Total ASCII fallback (agents tokenize without vision), contracts as machine-checkable specs, living-memory `⌾⤳⋈⇝` semantic navigation, deterministic no_std semantics. Built for AI agents as first-class users.

### 3.13 Morse identifiers (density)
Names are written in Morse (ITU codebook), translatable back to ASCII — ~2–3× shorter than ASCII, symbolically compact. `.`/`-` dot/dash, space = letter sep, `/` = word sep.

---

## 4. Backends (native, not Rust, not LLVM; mandatory compilation, all optimizations)
1. **v1 — direct machine code** — aarch64 (**NEON**) + x86_64 (**AVX-512**), zero runtime, zero deps, `no_std`. Compilation is **mandatory**; all optimizations mandatory.
2. **v2 — WebAssembly** — browser rendering with no extra compilers: Bebop → `.wasm` directly.
3. **v3 — GPU compute** — Vulkan/Metal/WebGPU shaders (hypervector bundling, NTT convolution, mass search).
4. **v4 — Calyx/CIRCT** (FPGA/ASIC). `⚛ hardware` items → synthesizable Verilog.
5. **v5 — physical ceiling** — PIM/Compute-SRAM + photonic (speed-of-light NTT/FFT). Targets, not blockers.
6. **Bootstrap in C, self-host in Bebop** — Stage 0 `bebopc` is a minimal native C core; Stage 1 rewrites it in Bebop.

---

## 5. dowiz stdlib map (what Bebop must express)

| dowiz domain | Bebop feature |
|---|---|
| `order_machine`/`causal` (event-sourced) | dependent-type state machine; C3 as a type |
| `money` | `Money` + `no_float` (C5) |
| `hypervector` | `Hypervector` + `#[bit_identical]` |
| `ntt`/`fft`/`modular` | `Field<P>` + verified NTT round-trip |
| `pq` (x25519/aes_gcm/ML-DSA) | KAT-gated, `#[hardware]` targets |
| `living_memory`/`code_graph` | compile-time-baked index + relational memory (`⌾⤳⋈`) |
| `quantum` (hybrid state) | `ψ` self-prediction + relational memory |
| `dynamic_spawner`/`parallel_patterns` | `∥`/`⋉`/`⋊` deep parallelism |
| `arena`/`slot_arena` | linear-region allocation (quantity-1) |
| `simd` | portable vectors + bit-identity contract |

---

## 6. The maximal plan (no gates, full speed)

**Phase 0 — Spec (2–3 days).** `BEBOP-LANGUAGE-SPEC.md`: EBNF grammar, QTT typing rules, contract semantics, effect system, glyph map. Verify on paper: a hand-written NTT example type-checks.

**Phase 1 — Glyphic front-end + font (3–5 days).** Native `bebopc` (C bootstrap): glyphic lexer, recursive-descent parser → AST, 300-glyph alphabet, ordinary lexicon (δ-outlines) + braille/vector renderer, `bebopc fmt` on `.bp`. Verify: fmt round-trips; full alphabet renders. (The Rust `bebop-lang/` scaffold is reference-only, superseded by the native C bootstrap.)

**Phase 2 — QTT kernel (1–2 weeks, critical path).** Elaborator, quantitative resource analysis, dependent elaboration, termination checker, quotient types. Verify: NTT example from Phase 0 type-checks for real.

**Phase 3 — Native codegen + contracts (1–2 weeks, critical path).** QTT → **direct aarch64(NEON)/x86_64(AVX) machine code** (monomorphisation, quantity-0 erasure, branchless `⤫` + atomic `⚛` lowering, register allocator), `no_std`, contract extraction → SMT-LIB → Z3/CVC5, `⚛ bit_identical`. **Milestone:** `bebopc build ntt.bp` → binary bit-identical to Rust NTT, `ensures` discharges in Z3.

**Phase 4 — SIMD + stdlib core (3–5 days).** `Vector`, `Hypervector`, `Field<P>`; port `hypervector.rs` + `ntt.rs` + `modular.rs` + `money.rs`. Verify: differential vs Rust, 10k random inputs, bit-for-bit.

**Phase 5 — FPGA/ASIC HLS (1–2 weeks, critical path).** `#[hardware]` → MLIR → Calyx/CIRCT → Verilog for hypervector bundling + NTT butterfly + SHA-3; simulate + synthesize. Verify: HLS NTT == software NTT on KAT.

**Phase 6 — Tooling (2–3 days).** LSP server, errors rendered in glyphs, docs.

**Phase 7 — Full dowiz rewrite (1–2 weeks, swarm).** 216K LOC → Bebop (zero-dup, contracts replace defensive code). Priority: `money` → `order_machine` → `ntt`/`modular` → `hypervector` → `living_memory` → `pq` → everything else. 12-way swarm, disjoint-file. Differential vs Rust bit-for-bit.

**Phase 8 — Verification parity (1 week).** C1–C13 as contracts; Rust reference frozen as differential oracle; full RED+GREEN + KAT green.

**Total: ~6–10 weeks.**

---

## 7. Files to create

- `docs/design/BEBOP-LANGUAGE-SPEC.md`
- `bebop-lang/` — native compiler (C bootstrap → self-hosted Bebop): glyphic lexer/parser/elab/core/native-codegen/contracts/glyphs/hls
- `bebop-lang/glyphs/` — vector-glyph font + renderer
- `crates/bebop-std/` — stdlib (hypervector, ntt, money, …) mirrored from dowiz-core
- `crates/dowiz-core/src/*.bp` — ported modules (alongside Rust `.rs` during transition)
- `kernel/tests/bebop_differential.rs` — differential harness

---

## 8. Tests / validation
- **Type-level:** contracts discharge via Z3/CVC5 (NTT round-trip, money no-float, legal transitions).
- **Differential:** Bebop == Rust bit-for-bit across all ported modules.
- **KAT:** shared NTT/FFT/PQ known-answer vectors.
- **`#[bit_identical]`:** SIMD == scalar, compile-time.
- **Glyph:** full-alphabet render + round-trip.

---

## 9. Execution model (how we hit the velocity)

1. **12-way swarm at all times** — each phase decomposed into disjoint-file tasks (front-end files, stdlib modules, test suites); subagents touch only their own `.bp`/`.rs`, never shared `lib.rs`.
2. **Critical path isolated** — QTT kernel + SMT + HLS are the only serialized segments; everything else fans out.
3. **Differential-oracle discipline** — the Rust reference is the source of truth at every step; Bebop output must match bit-for-bit before a module is "done".
4. **RED+GREEN per C7** — every ported function carries a failing-then-passing assertion.
5. **Zero dup/dead code** — binding rule; the Bebop tree is leaner than the Rust tree, contracts replacing runtime checks.

---

## 10. Risks (engineering challenges — not stop-conditions)
1. **QTT kernel correctness** — the hard core; mitigated by building it against the Phase 0 spec + a small KAT corpus from day one.
2. **SMT coverage** — not every contract auto-proves; the "verifiable subset" is a deliberate inner language.
3. **Trait system** — Rust coherence vs Lean typeclasses vs both; decided in Phase 0 spec.
4. **FPGA synthesis** — HLS backend writable + simulable without hardware; real silicon gated on equipment only.
5. **Bootstrap** — first `bebopc` is in C (native); self-hosting in Bebop is Stage 1, a milestone not a blocker.

---

## 11. Interaction with current work
The daemon + plugin (`9164c20`) stays as the runtime path until Phase 7 compile-time baking replaces it. This plan does not block in-flight living-memory / workspace-split / PID-concurrency work — it is the forward track that absorbs them.
