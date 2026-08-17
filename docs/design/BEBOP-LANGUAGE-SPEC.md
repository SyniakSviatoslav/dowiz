# Bebop Language Specification — v0.3

> Phase 0 deliverable (revised). Authoritative design reference.
> Author: Sviatoslav Syniak · License: AGPL-3.0-or-later · 2026-08-17
> Doctrine: MANIFESTO C8 — over-engineering is the #1 ally. Extension: `.bp`.

---

## 1. Identity

Bebop is a **native, glyphic, agentic systems language**. There are **no words** in
its surface — only **glyphs and calculus**. A program is a geometric composition of
vector symbols; the terminal renders each glyph as pixels (braille/half-block), an
editor renders it as a δ-outline. ASCII is only a *fallback rendering* of a glyph,
never the source of truth.

**Glyphs are vector outlines, not emoji.** Every symbol is a hand-drawn δ-outline on a
pixel grid (spec §2.4); Unicode pictographs/emoji are never the canonical form — the
canonical form is the δ-outline, and the terminal fallback is the ASCII name.

**The lexicon is ordinary** — technical terms (`module`, `function`, `record`, `sum`,
`contract`, …), not metaphor.

**Bebop is agentic** — a first-class surface for AI agents *and* humans:
- a complete, stable ASCII fallback so agents tokenize unambiguously;
- contracts are machine-checkable specs an agent verifies against its own output;
- deterministic, no_std semantics an agent can reason about;
- living-memory relational navigation (`⌾⤳⋈`) — an agent asks the codebase, it answers;
- hybrid-quantum self-prediction (`ψ`) lets an agent (and the runtime) prefetch its next
  hot path. Agents and humans share one surface, one grammar, one type system.

**Bebop carries all the methods of C, Rust, and Lean 4:**
- **C** — pointers, manual memory, bit-level control, freestanding/no-runtime, ABI, `volatile`, inline asm.
- **Rust** — ownership/borrowing, traits, no_std, zero-cost abstractions, const-eval, macros, unsafe scoping.
- **Lean 4** — dependent types, inductive + quotient types, proof terms, termination, universe hierarchy.

Bebop is written **natively, not in Rust**: the bootstrap compiler is a minimal native
C core; the compiler self-hosts in Bebop; the backend emits **machine code directly**
(aarch64+NEON, x86_64+AVX) with zero runtime, zero external dependencies (no LLVM).

---

## 2. Surface: glyphs, not words

### 2.1 The glyph is the token
A program is a sequence of **glyphs**. Each glyph is a named vector symbol (δ-encoded
outline on a pixel grid). There is no `fn`/`struct`/`match` keyword — there is a glyph;
its ASCII name is the fallback token. The full alphabet is in
`BEBOP-GLYPH-ALPHABET.md` (closed: every glyph parses, every construct has a glyph).

### 2.2 Glyph lexicon (core, ordinary)

| Glyph | Name | Meaning |
|---|---|---|
| `◈` | `mod` | module / namespace |
| `★` | `fn` | function definition |
| `◇` | `struct` | record / product |
| `△` | `data` | inductive / sum |
| `◉` | `val` | value / term |
| `⊙` | `contract` | pre/post/invariant |
| `◎` | `quotient` | quotient type |
| `✦` | `trait` | interface / typeclass |
| `♁` | `impl` | implementation |
| `⌘` | `type` | type alias |
| `⇐` | `use` | import |
| `¤` | `const` | constant |
| `◐` | `let` | let-binding |
| `◑` | `mut` | mutable binding |
| `❖` | `match` | pattern match |
| `◒` | `if` | conditional |
| `↻` | `loop` | loop |
| `↺` | `while` | while |
| `♒` | `for` | for-each |
| `↯` | `return` | return |

**Calculus:** `λ` lambda · `→` arrow · `∏` pi · `∑` sigma · `:` colon · `::` cons ·
`≡` definitional-equality · `≅` quotient-equivalence · `≈` propositional-equality ·
`⊢` typing · `⊨` obligation · `=` assign · `.` field · `;`/`,` separator.

**Quantities (QTT rig):** `∅` = 0 (erased) · `·` = 1 (linear) · `∞` = ω (unrestricted).

**Numeric tower:** `ℤ` ints · `ℕ` naturals · `𝔽ₚ` prime field · `ℤₘ` ring mod m ·
`i64`/`u64`/`f64` (float gated) · `♾` infinity.

**NTT & fields:** `ωₙ` root-of-unity · `⟲` ntt · `⟳` intt · `⊛` convolution ·
`×` `＋` `−` `÷` `∤` · `≪` `≫` shifts.

**Hypervector (1024-bit):** `⧉` hypervector · `⊕` bundle · `⊗` bind · `⊖` unbind ·
`∘` shift · `⊚` similarity · `⋔` xor · `⋏` majority.

**Quantum (hybrid state):** `ψ` state · `|⟩` ket · `⟨|` bra · `⨁` superposition ·
`⨂` tensor · `𝐇` Hadamard · `𝐌` measure.

**Memory & relations (living-memory):** `⌾` node · `⤳` edge · `⋈` join · `⊑` subset ·
`≺` precede · `⤴` neighbor · `⇝` search.

**Parallelism & atomics:** `∥` par · `⋉` fork · `⋊` join · `⚛` atomic ·
`⤫` branchless · `⟕` spawn · `⟖` sync · `⧗` barrier.

**Logic:** `∧` and · `∨` or · `¬` not · `∀` forall · `∃` exists · `⊃` implies ·
`⊥` bottom · `⊤` top.

**Effects (capabilities):** `⏱` clock · `⚄` rng · `⬡` env · `⟡` float · `⌁` net ·
`⚙` io · `∅` pure.

> The glyph in the left column is the terminal-render placeholder for the vector
> δ-outline (the canonical form). ASCII names are the fallback tokens an agent parses.

---

## 3. Core calculus — QTT

The core is **Quantitative Type Theory** (Atkey 2018; McBride 2016; Idris 2, Brady 2021):
a single type theory that fuses **Rust's ownership** (linearity) and **Lean 4's dependent
types** (erasure/proofs) via quantity annotations `0`/`1`/`ω`.

Quantities form the rig `{0,1,ω}`:
- `0 + p = p`, `1 + 1 = ω`, `ω + p = ω` (join)
- `0 · p = 0`, `1 · p = p`, `ω · p = ω` (tensor)

Judgements (`Γ ⊢ t : A`, usage-tracked):
```
────────── VAR     x :ᵖ A ∈ Γ,  p ≠ 0
Γ ⊢ x : A

Γ, x :ᵖ A ⊢ t : B
────────────────── LAM
Γ ⊢ λx. t : (x :ᵖ A) → B

Γ ⊢ f : (x :ᵖ A) → B    Γ ⊢ s : A
───────────────────────────────── APP
Γ ⊢ f s : B[s/x]

Γ ⊢ A : Typeᵢ    Γ, x :ᵖ A ⊢ B : Typeⱼ
────────────────────────────────────── PI
Γ ⊢ (x :ᵖ A) → B : Type₍ᵢ⊔ⱼ₎

──────────── TYPE
Γ ⊢ Typeᵢ : Typeᵢ₊₁
```

- `1` (linear) = move/ownership (Rust's affine discipline).
- `ω` (unrestricted) = shared borrow / `Copy`.
- `0` (erased) = proof / type-level only (Lean's `Prop`).
- Termination: structural recursion + a `decreases` well-founded measure (Lean parity).

---

## 4. Contracts (Spark/Ada parity)

```
⊙ fn add(a: i64, b: i64) → i64
  requires a ≥ 0 ∧ b ≥ 0
  ensures  result = a + b ∧ result ≥ 0
  invariant ...
```

- `requires` (precondition), `ensures` (postcondition), `invariant` (data + loop),
  `ghost` (verification-only code), `decreases` (termination measure).
- Verification conditions (VC) are generated weakest-precondition style, then discharged
  to **SMT** (Z3/CVC5). The verifiable subset mirrors SPARK: no unbounded recursion
  without a measure, explicit frame (`reads`/`writes`).
- `⚛ bit_identical` — a contract that SIMD and scalar paths emit identical bits
  (dowiz `simd.rs` design rule, made a compiler-checked law).
- `◉ no_float` — a type-level guarantee that a float never reaches money (dowiz C5).

---

## 5. Effects & capabilities

A `pure` function has an empty capability set. Capabilities (`⏱` clock, `⚄` rng,
`⬡` env, `⟡` float, `⌁` net, `⚙` io) are tracked like regions: a `pure` fn cannot call
anything needing a capability it lacks. This is MANIFESTO C2 (pure core: no
clock/RNG/env/float/network vocabulary) as a type-level effect.

---

## 6. Concurrency, atomics, branchless

- `∥` parallel composition, `⋉`/`⋊` fork/join, `⟕` spawn (PID-dynamic scheduler from
  dowiz `dynamic_spawner`), `⚛` atomics (CAS/LDADD), `⤫` branchless predication
  (CSEL/CMOV). O(n) zero-overhead methods; deep parallelism is a first-class effect.

---

## 7. Core methods (dowiz algorithms = Bebop methods)

NTT `⟲⟳`, hypervector `⧉⊕⊗`, FFT/modular `ωₙℤₘ`, living-memory `⌾⤳⋈`, quantum `ψ`,
arena (linear regions), money `◉ no_float`, event-sourced `△ fold`. All are **built-in
methods** of the language, O(n) zero-overhead, no_std-native, NEON/AVX-lowered.

---

## 8. Compile-time evaluation

`comptime` + pervasive `const fn`. Living-memory index, NTT twiddle factors, code-graph
are baked into `.rodata` at compile time — runtime is a pointer dereference. Zero
cold-start, no daemon needed.

---

## 9. Backends (native)

1. **Direct machine code** — aarch64 (**NEON**) + x86_64 (**AVX**), zero runtime, zero deps, `no_std`.
2. **Calyx/CIRCT** — `⚛ hardware` items → synthesizable Verilog (hypervector bundling, NTT butterfly, SHA-3 round).
3. **Bootstrap in C, self-host in Bebop** — Stage 0 `bebopc` is a minimal native C core.

---

## 10. Open items (for Phase 1+)

1. Full glyph outline corpus (300 glyphs) — drawn as δ-outlines in Phase 1.
2. Trait coherence model (Rust vs Lean typeclasses) — Phase 2.
3. SMT solver integration (Z3/CVC5 FFI vs embedded) — Phase 3.
