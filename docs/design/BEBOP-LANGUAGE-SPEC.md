# Bebop Language Specification — v0.2

> Phase 0 deliverable (revised). Authoritative design reference.
> Author: Sviatoslav Syniak · License: AGPL-3.0-or-later · 2026-08-17
> Doctrine: MANIFESTO C8 — over-engineering is the #1 ally. Extension: `.bp`.

---

## 1. Identity

Bebop is a **native, glyphic systems language**. There are **no words** in its surface — only **glyphs and calculus**. A program is a geometric composition of vector symbols; the terminal renders each glyph as pixels (braille/half-block), an editor renders it as a δ-outline. ASCII is only a *fallback rendering* of a glyph, never the source of truth.

The lexicon is **cosmic**: types are constellations, functions are stars, values are worlds, memory is a galaxy.

Bebop is written **natively, not in Rust**: the bootstrap compiler is a minimal native C core; the compiler self-hosts in Bebop; the backend emits **machine code directly** (aarch64+NEON, x86_64+AVX) with **zero runtime, zero external dependencies** (no LLVM).

---

## 2. Surface: glyphs, not words

### 2.1 The glyph is the token
A program is a sequence of **glyphs**. Each glyph is a named vector symbol (δ-encoded outline on a pixel grid, §2.4 of v0.1). There is no `fn`/`struct`/`match` — there is `★`/`◇`/`△`.

### 2.2 Cosmic lexicon (core glyph alphabet)

**Structure (the "sky"):**
| Glyph | Meaning | Cosmic name |
|---|---|---|
| `◈` | module / namespace | star-system |
| `★` | function definition | star |
| `◇` | record / struct | diamond |
| `△` | inductive / data (sum) | constellation |
| `◉` | value / term | world |
| `⊙` | contract / invariant | halo |
| `◎` | quotient type | eclipse |

**Calculus (the "orbits"):**
| Glyph | Meaning |
|---|---|
| `λ` | abstraction |
| `→` | function type / arrow |
| `∏` | dependent product (Π-type) |
| `∑` | dependent sum (Σ-type) |
| `:` | type annotation |
| `≡` | definitional equality |
| `≅` | quotient equivalence |
| `≈` | propositional equality |
| `⊢` | typing judgement |
| `⊨` | contract obligation (entails) |

**Quantities (QTT rig `0/1/ω`):**
| Glyph | Quantity | Meaning |
|---|---|---|
| `∅` | 0 | erased (proof / type) |
| `·` | 1 | linear (exactly once) |
| `∞` | ω | unrestricted |

**Fields & NTT (dowiz core):**
| Glyph | Meaning |
|---|---|
| `𝔽ₚ` | prime field mod p |
| `ℤₘ` | ring mod m |
| `ωₙ` | primitive n-th root of unity |
| `⟲` | NTT / forward transform |
| `⟳` | inverse NTT |
| `⊛` | circular convolution |
| `⧉` | hypervector (1024-bit) |
| `⊕` | hypervector bundling |
| `⊗` | hypervector binding |

**Quantum (hybrid state):**
| Glyph | Meaning |
|---|---|
| `ψ` | quantum state |
| `|ψ⟩` | ket |
| `⟨ψ|` | bra |
| `⨁` | superposition |
| `⨂` | entanglement |
| `𝐇` | Hadamard |
| `𝐌` | measurement |

**Memory & relations (living-memory foundations):**
| Glyph | Meaning |
|---|---|
| `⌾` | node |
| `⤳` | edge |
| `⋈` | relational join |
| `⊑` | containment |
| `≺` | precedence |

**Parallelism & atomics:**
| Glyph | Meaning |
|---|---|
| `∥` | parallel composition |
| `⋉` | fork |
| `⋊` | join |
| `⚛` | atomic |
| `⤫` | branchless (predicated) |

**Logic:**
| Glyph | Meaning |
|---|---|
| `∧` `∨` `¬` | and / or / not |
| `∀` `∃` | forall / exists |

### 2.3 The glyphic program
A Bebop function is written as a glyph composition:
```
◈ dowiz·ntt

★ ⟲ ◉(xs: 𝔽ₚ*) → 𝔽ₚ*
⊙  requires  (◉ n) ≡ 2ᵏ
⊙  ensures  ⟳(⟲(xs)) ≈ xs
{
  △ n { 1 → xs · n → ◉ let (e,o) = decimate(xs) ⋈ butterflies(⟲ e, ⟲ o, ωₙ) }
}
```
(ASCII shown only because the terminal renders glyphs as pixels; the true source is the glyph sequence.)

---

## 3. Core calculus (QTT) — unchanged, glyph-notated

The calculus of v0.1 §4 stands, rendered in glyphs: `∏(x:ᵍ A) → B` for dependent products with quantity `ᵍ ∈ {∅,·,∞}`. Linear = `·`, erased = `∅`, unrestricted = `∞`. The borrow checker is the quantitative analysis; erasure deletes `∅`-terms at runtime.

---

## 4. Contracts (SPARK/Ada model) — glyph-notated

`⊙ requires` / `⊙ ensures` / `⊙ invariant` / `⊙ decreases`, plus `ghost` (`∅`-quantified, erased). VCs lower to SMT. `⚛ bit_identical` proves SIMD ≡ scalar.

---

## 5. Native execution model

1. **no_std, zero-runtime** — Bebop core emits freestanding object code; no allocator unless `·`-linear regions demand it (arena).
2. **Direct machine code** — the backend emits aarch64 (with **NEON**) and x86_64 (with AVX) instructions directly; **no LLVM, no third-party backend**. NEON is the reference vector target (the host is aarch64).
3. **`⚛ atomic`** — primitives are indivisible: single-instruction where the ISA provides it (CAS, LDADD), else lock-free sequences.
4. **`⤫ branchless`** — the hot path (NTT butterflies, hypervector bundling/binding, field mul) compiles to **predicated/select** code (CSEL/CMOV), no conditional branches. The compiler emits branchless lowering by default for the Bebop subset.

---

## 6. Concurrency & deep parallelism (first-class)

- `∥` composes two computations in parallel (data-parallel / task-parallel).
- `⋉ e` forks `e`; `⋊` joins; a **PID-dynamic spawner** (dowiz `dynamic_spawner`) is the built-in scheduler — the language's parallelism is depth-first by default, width-adaptive under load.
- `Vector<W,T>` + `⧉` hypervector operations are **implicitly parallel** across lanes (NEON/SVE/AVX) and **bit-identical** to the scalar path.

---

## 7. Core methods — dowiz's best algorithms are Bebop's methods

The most valuable dowiz modules become **built-in methods**, not libraries:

| dowiz algorithm | Bebop method |
|---|---|
| `hypervector.rs` (1024-bit, bundling/binding, shift-invariant similarity) | `⧉` type + `⊕` `⊗` `·` methods |
| `ntt.rs` (exact integer NTT/INTT, convolution) | `⟲` `⟳` `⊛` on `𝔽ₚ`/`ℤₘ` |
| `fft.rs` / `modular.rs` | `ωₙ` root synthesis, `ℤₘ` ring ops |
| `living_memory.rs` (code-graph, vector search, persistence) | `⌾`/`⤳`/`⋈` relational memory |
| `quantum.rs` (hybrid quantum state) | `ψ` hybrid state, self-prediction (§8) |
| `arena` / `slot_arena` | `·`-linear regions (no heap) |
| `money.rs` (i64 minor units, no-float) | `◉ Money` with `∅float` effect law |
| `order_machine.rs` / `causal.rs` (event-sourced, fold) | `△` state machine, `Σ = fold(events)` |
| `simd.rs` (bit-identical SoA lanes) | `Vector<W,T>` with `⚛ bit_identical` |

**Every Bebop method is `O(n)` zero-overhead**: linear time where linear is the floor, no hidden `O(n²)`, no allocation, no bounds-check cost on the subset (proved out by contracts).

---

## 8. Self-prediction: hybrid quantum state + relational memory

Bebop programs **predict themselves**. Two coupled mechanisms:

1. **Hybrid quantum state (`ψ`)** — every Bebop item carries a hybrid classical+quantum state (dowiz `quantum.rs`). The quantum part holds a superposition over likely next-actions; measurement collapses it. This is *not* AI — it is a deterministic `ψ` with caller-supplied entropy (MANIFESTO C10: RNG-free hot path).

2. **Relational memory (`⌾`/`⤳`/`⋈`)** — living-memory's code-graph is a built-in relation: nodes are items, edges are call/data-dependency, `⋈` joins them. The program **queries its own past** (`⌾ path`, `⤳ neighbors`, `⋈ join`) to predict the next hot path, prefetch hypervectors, and re-rank.

Together: `ψ ⊗ ⌾` predicts the next branch/lookup **branchlessly** (§5.4) and prefetches it before it is demanded — the language's own `comptime` baking (§9) plus its runtime relational memory.

---

## 9. Compile-time baking (zero cold-start)

`comptime` + pervasive `const fn` bake the living-memory index, NTT twiddle factors, and code-graph into `.rodata`. Runtime = pointer deref. Combined with `ψ`-prediction, the daemon's cold-start disappears **by construction**.

---

## 10. Bootstrap & backend (native, not Rust)

- **Stage 0 (bootstrap):** `bebopc` written in **C** (native, minimal) — glyphic lexer → parser → QTT elaborator → **direct aarch64/x86_64 codegen**. No Rust, no LLVM.
- **Stage 1 (self-host):** `bebopc` rewritten in **Bebop** (`◈ bebopc ★ compile`), once Bebop compiles itself.
- **Stage 2 (silicon):** `⚛ hardware` items → Calyx/CIRCT → Verilog (NTT butterfly, hypervector bundling, SHA-3 round).

---

## 11. Appendix — worked example (glyphic, ASCII-rendered)

```
◈ dowiz·ntt

★ ⟲ ◉(xs: ⧉ 𝔽ₚ) → ⧉ 𝔽ₚ
⊙  requires  n ≡ 2ᵏ
⊙  ensures  ⟳(⟲ xs) ≈ xs
⊙  decreases n
{
  △ n {
    1     → xs
    n     → ◉ let (e, o) = decimate xs ⋈
              butterflies(⟲ e ∥ ⟲ o, ωₙ)        ⚛ · ⤫
  }
}
```

Notes: `∥` runs the two sub-transforms in parallel; `⚛` marks the butterfly atomic; `⤫` forces branchless predication; the round-trip `ensures` is discharged by SMT for fixed `n`.

---

## 12. Open items (carried forward)
1. Full glyph outline set (Phase 1) — format fixed in v0.1 §2.6; the 300-glyph cosmic alphabet is drawn in Phase 1.
2. `ψ` collapse semantics (measurement basis) — Phase 4.
3. Direct codegen register allocator (aarch64/x86_64) — Phase 3.
4. `⤫` branchless guarantee for arbitrary user code vs the Bebop subset only — Phase 3.
5. Self-host ordering — post-Phase-3.
