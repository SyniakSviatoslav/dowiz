# Bebop Language Specification — v0.1

> Phase 0 deliverable. Authoritative design reference for the compiler.
> Author: Sviatoslav Syniak · License: AGPL-3.0-or-later · 2026-08-17
> Doctrine: MANIFESTO C8 — over-engineering is the #1 ally.

---

## 1. Overview

Bebop is a **systems language with dependent types, linearity, and machine-checked contracts**, designed as the substrate of the dowiz delivery OS. One surface syntax elaborates into a **Quantitative Type Theory (QTT)** core, then lowers to **LLVM** (machine code) and — for `#[hardware]` items — to **MLIR/Calyx** (silicon).

Four pillars, each first-class:

1. **QTT** — quantities `0/1/ω` unify Rust's ownership (linearity) and Lean 4's dependent types (erasure).
2. **Contracts** — `requires`/`ensures`/`invariant`/`ghost`, verified by SMT (SPARK/Ada model).
3. **Determinism** — `#[bit_identical]` (SIMD ≡ scalar), purity effects, `no_float` money.
4. **Glyphs** — every symbol is a δ-encoded vector glyph on a pixel grid.

The verifiable fragment of Bebop is called the **Bebop subset** (cf. SPARK subset).

---

## 2. Lexical structure

### 2.1 Source encoding
UTF-8, no BOM. Line endings LF. A program is a sequence of items (like Rust) plus, optionally, a `module` declaration.

### 2.2 Tokens
```
ident        := [A-Za-z_][A-Za-z0-9_]* | operator-name
int_lit      := dec | hex | bin   (suffixed: i8 i16 i32 i64 u8 u16 u32 u64 usize)
float_lit    := dec '.' dec exp?   (gated: only in `!pure` regions, §7)
string_lit   := "..."  (UTF-8, escapes)
char_lit     := 'c'
glyph_lit    := ◇name◇   (a symbol written by glyph, not ASCII — see §2.6)
```

### 2.3 Keywords (each carries a vector glyph, §2.6)
```
fn  data  struct  enum  trait  impl  module  use  type  let  mut
match  if  else  loop  while  for  return  break  continue
pure  ghost  comptime  const  inline  extern
requires  ensures  invariant  reads  writes  decreases
where  self  Self  pub  priv
true  false  nil
0 1 ω            (quantities — reserved as types/annotations)
unsafe  hardware  as
```

### 2.4 Operator names
Arithmetic/bitwise/logical operators reuse Rust spellings (`+ - * / % << >> & | ^ ! && || == != < > <= >=`). User-defined operators are out of v0.1 scope.

### 2.5 Comments
`//` line, `/* */` block, and `///`/`//!` doc comments (rendered with glyphs in the doc tool).

### 2.6 The glyph alphabet (δ-encoded vector glyphs)

Every keyword, operator, delimiter, and built-in type has a canonical **vector glyph**: a closed outline on an `M×M` pixel grid (default `M=16`), encoded as a sequence of **δ deltas** — signed pen-move pairs — plus a fill rule.

**Glyph encoding (binary + text form):**
```
glyph   := "◇" name "◇"
outline := (dx₀,dy₀) (dx₁,dy₁) ... (dxₙ,dyₙ)  # δ deltas, each in [-M, M]
fill    := 0 (stroke) | 1 (nonzero-fill) | 2 (even-odd)
```
Delta form is chosen because glyphs are **self-delimiting under affine transform** (translate/scale/rotate a glyph by transforming deltas, not absolute points) — the same property TrueType hint deltas exploit, but pixel-grid-native.

**Renderer tiers** (both must render the identical glyph):
- **Terminal fallback** — reuse `pixel_snapshot.rs` braille (8 bit/2×4) and half-block (2 bit/1×2) to rasterize the glyph grid into text-safe pixels (this is the dowiz "agent sees a buffer as pixels" path).
- **Vector tier** — the δ-outline rasterized at any resolution (editor, LSP hover, error output).

**Alphabet scope (v0.1):** all keywords in §2.3, all operators in §2.4, the delimiters `( ) { } [ ] < > , ; : -> => = |`, and the built-in type names `Bool Int UInt I64 U64 F64 String Unit Field Zmod Vector Hypervector Money Type`. The font is the bounded Phase-1 deliverable; this section fixes the *format*, not the final outlines.

---

## 3. Surface syntax (EBNF, summary — full in Appendix A)

### 3.1 Items
```bebop
module dowiz.money

// A function with a contract.
fn add(a: I64, b: I64) -> I64
  requires  b >= 0
  ensures   result == a + b
{
  a + b
}

// A record.
struct Money { minor: I64 }

// An inductive type (sum), GADT-capable (dependent indices allowed).
data OrderState
  | Created
  | Picked(order_id: I64)
  | Delivered(proof: DeliveredProof)

// A quotient type: Field<P> = I64 / ~ (congruence by P).
quotient Field<const P: I64> = I64 by (a ~ b => (a - b) % P == 0)

// A contract (reusable named spec, SPARK-style).
contract Invariant<T> {
  fn valid(&self) -> Bool
  invariant { self.valid() }
}
```

### 3.2 Quantities in signatures
A binding's quantity is written after `:` where it matters:
```bebop
fn use_once(x: 1 T) -> U {}        // linear: x used exactly once
fn shared(x: ω T) -> U {}          // unrestricted: x shared freely
fn erased(x: 0 T) -> U {}          // erased: x absent at runtime (proof/type)
```
Default quantity is `ω` for function parameters (matches Rust's shared-borrow ergonomics); explicit `1` opts into linear/move semantics; `0` opts into erasure.

### 3.3 Ownership sugar (Rust parity)
- `fn take(x: T)` — `T` is linear by default in **move position** (quantity `1`).
- `fn borrow(x: &T)` — `&T` desugars to a quantity-`ω` reference with a lifetime.
- `fn borrow_mut(x: &mut T)` — unique linear borrow.
- `Copy` bound ≡ quantity-`ω` usage of an owned value (no move).
Elaboration (§5) lowers these to QTT quantities; the borrow checker is the quantitative resource analysis.

---

## 4. Core calculus (QTT)

### 4.1 Quantities and the rig
Quantities `Q = {0, 1, ω}` form a rig:

| `+` | 0 | 1 | ω |    | `·` | 0 | 1 | ω |
|-----|---|---|---|    |-----|---|---|---|
| 0   | 0 | 1 | ω |    | 0   | 0 | 0 | 0 |
| 1   | 1 | ω | ω |    | 1   | 0 | 1 | ω |
| ω   | ω | ω | ω |    | ω   | 0 | ω | ω |

- `0` = zero (erased: proofs, types, compile-time), `1` = unit (linear: exactly once), `ω` = infinity (unrestricted).
- `+` = combine uses, `·` = scale uses (e.g. a function used `p` times scales its argument's use by `p`).

### 4.2 Core syntax
```
q       ::= 0 | 1 | ω
term    ::= x                                  variable
          | λ(x :q A). t                       lambda
          | t u                                application
          | (x :q A) → B                       dependent product
          | Type i                             universe
          | data ctor / eliminator             inductive (desugared)
          | let x = t in u                     let
          | □                                  erased box (quantity 0)
          | [t]                                linear box (quantity 1)
ctx     ::= ∅ | Γ, x :q A
```

### 4.3 Typing rules (declarative, Atkey 2018 / Brady 2021)
Judgements: `Γ ⊢ t : A` (well-typed) and the usage-tracking refinement `Γ ⊢ t :ᵖ A`.

```
──────────── VAR                          x :q A ∈ Γ
Γ ⊢ x : A

Γ, x :q A ⊢ t : B
─────────────────────────── LAM          (q is the usage of x in t)
Γ ⊢ λ(x:q A). t : (x :q A) → B

Γ ⊢ f : (x :q A) → B      Γ ⊢ s : A
─────────────────────────────────────── APP
Γ ⊢ f s : B[s/x]

Γ ⊢ A : Type i      Γ, x :q A ⊢ B : Type j
────────────────────────────────────────── PI
Γ ⊢ (x :q A) → B : Type (i ⊔ j)

──────────── TYPE
Γ ⊢ Type i : Type (i+1)

Γ ⊢ t : A      Γ, x :q A ⊢ u : B
──────────────────────────────────── LET
Γ ⊢ let x = t in u : B

Γ ⊢ A ≅ B      Γ ⊢ t : A
──────────────────────── CONV
Γ ⊢ t : B
```

**Linearity discipline:** if `(x :1 A) → B` and `x` occurs in `B`, then a well-typed body `t` uses `x` **exactly once** (the quantitative check on `LAM`). A `ω`-bound variable may be used any number of times. A `0`-bound variable is absent at runtime (erasure); it may appear only in types/proofs.

**Substitution lemma + subject reduction** hold; the core is normalizing on the verifiable subset (termination, §4.7).

### 4.4 Universes
Cumulative `Type 0 : Type 1 : …`, `Type i ⊆ Type j` for `i ≤ j`. Impredicativity is **not** assumed in v0.1 (proof-relevant `Type` is predicative; a proof-irrelevant `Prop` universe may be added later for Lean-style impredicativity).

### 4.5 Inductive types
`data` introduces a strictly-positive inductive family with dependent indices (GADTs). Each constructor gets a usage-correct type; eliminators (pattern matching) are checked for:
- **strict positivity** (no `T` left of a `→` in its own definition),
- **coverage** (exhaustive patterns),
- **termination** (§4.7).

### 4.6 Quotient types
`quotient Q = A by (x ~ y => R)` introduces `Q` with a canonical projection `[-] : A → Q`, and an eliminator that requires the well-definedness obligation `R x y → f x = f y` (proof that the function respects the equivalence). This is what types `Field<P>` and `Zmod<M>` (§7.2) so NTT/FFT are typed against their ring, not raw integers.

### 4.7 Termination
All functions in the **Bebop subset** must pass the termination checker: a structural `decreases` argument, or an explicit well-founded measure (the `decreases` clause). Non-terminating code is allowed only in `!pure` effect regions (§6).

---

## 5. Elaboration (surface → core)

1. **Desugar** ownership sugar (§3.3) into explicit quantities and boxes.
2. **Resolve traits/typeclasses** (Rust coherence vs Lean typeclass — see §5.1).
3. **Insert erasure** for quantity-`0` bindings (no runtime repr).
4. **Generate contract obligations** (§6) alongside the core term.
5. **Check** the elaborated term against §4.3.

### 5.1 Trait system (decision deferred to §8-risk, resolved here for v0.1)
Bebop v0.1 uses **typeclasses with coherence** (Lean-style instances + Rust-style coherence condition: at most one instance per type-and-trait, enforced by orphan rules). `trait` declares a typeclass; `impl` provides an instance; dependent `where` clauses give Lean-style instance parameters. This is the "universe-polymorphic trait" middle ground: enough for `Field<P>`, `Semiring`, `Copy`, `Eq`.

---

## 6. Contracts & verification (SPARK/Ada model)

### 6.1 Syntax
```bebop
fn f(x: A) -> B
  requires  P(x)                       // precondition
  ensures   Q(x, result)               // postcondition (binds `result`)
  reads     g1 g2                      // frame: globals read
  writes    g3                         // frame: globals written
  invariant { /* data/loop invariant */ }
  decreases x                          // termination measure
```

- `ghost` items and bindings exist only for proof; they erase to nothing.
- `invariant` on a `data`/`struct` is a **data invariant** (holds at every well-formed construction).
- `invariant` inside a `loop` is a **loop invariant** (checked on entry + preserved by the body).

### 6.2 Verification-condition (VC) generation
Weakest-precondition calculus over the **Bebop subset**:
- `wp(x := e, Q) = Q[e/x]`
- `wp(if c then a else b, Q) = (c → wp(a,Q)) ∧ (¬c → wp(b,Q))`
- `wp(loop, Q)` uses the loop invariant + `decreases` for termination VC.
- `requires`/`ensures` of a callee become the assumption/obligation at the call site.

### 6.3 SMT translation
VCs are translated to **SMT-LIB** and discharged by Z3 (primary) / CVC5 (fallback):
- `Bool` → SMT `Bool`, `I64`/`U64` → `(_ BitVec 64)`, `I64` arithmetic with overflow checks → bit-vector ops + range obligations.
- `Field<P>` → `Int` with mod-bound, or bit-vector for prime `P`.
- Inductive/dependent structures → algebraic datatypes + uninterpreted functions (UF) when decidable fragments suffice.
- Quantified `ensures` over finite domains → bounded quantifier expansion; unbounded → SMT `forall` (may be incomplete — hence the *subset*).

### 6.4 The Bebop subset (verifiable fragment)
Deterministic subset with: no unbounded recursion without `decreases`, no aliasing that breaks `reads`/`writes` framing, integer/bit-vector arithmetic, finite-field ops, and first-order quantification over finite/bounded domains. Outside the subset, contracts are checked but may be deferred as **proof obligations** (reported, not silently dropped).

### 6.5 `#[bit_identical]` (the determinism contract)
```bebop
#[bit_identical]
fn softmax_batch(xs: &[F64; n]) -> [F64; n]
```
The compiler generates **two** lowerings (SIMD and scalar) and proves — or, in debug builds, emits a runtime equality check — that both emit identical bits. For the subset, bit-identity is established *by construction* (the SIMD lowering is forced to replay the exact scalar op order per lane, exactly the rule already in `simd.rs` §6), so the check is a compiler invariant, not a per-call proof.

---

## 7. Effect system (purity & regions)

Every function has an **effect** drawn from a capability lattice. Capabilities:
```
capability ::= clock | rng | env | float | network | io | alloc
```
- `pure fn` — empty capability set: **cannot** name `clock/rng/env/float/network/io/alloc` anywhere in its transitive call graph. This is MANIFESTO C2 (§1.5 unrepresentability) as a *type*, not a comment.
- A function may list capabilities explicitly: `fn f(x: T) !{float, alloc} -> U`.
- Effects are **region-quantified**: a `pure` function cannot call a `!{rng}` function; capability polymorphism (`!{e}`) is supported but not inferred beyond the simple case in v0.1.

**Money law (C5) as an effect + type:** `Money` is a newtype over `I64`; its constructors are `pure`, and any function producing `Money` is proven `!{float}`-free. A `float` capability cannot flow into a `Money`-producing context — the compiler rejects it.

---

## 8. Numeric tower

| Type | Meaning | Notes |
|---|---|---|
| `I64` | signed 64-bit (money minor units) | overflow = VC obligation |
| `U64`/`U32`/`U16`/`U8` | unsigned | bit-vector ops |
| `I32`/`I16`/`I8` | signed | |
| `usize` | address size | |
| `F64` | IEEE-754 binary64 | **gated** to `!{float}` regions only |
| `Field<const P: I64>` | prime field mod `P` | quotient type (§4.6); NTT ring |
| `Zmod<const M: I64>` | ring mod `M` | |
| `Fixed<p>` | fixed-point Q-format | permitted non-integer math, no float |
| `Bool`, `Unit` | — | |

No `From<F64>` for money (C5). `crate::math` equivalents (round/sqrt/exp/powi as fixed-point or bit-exact integer algorithms) are provided so the pure core needs no `F64`.

---

## 9. SIMD / vector types

```bebop
Vector<const W: usize, T>       // W-lane vector, portable (LLVM vector IR)
Hypervector                      // 1024-bit = Vector<16, U64> + bundling/binding ops
```

- Lowering: `Vector<W,T>` → LLVM `<W x T>`; portable across AVX-512 / NEON / SVE / RVV.
- `Hypervector` primitives (bundling `⊕`, binding `⊗`, similarity `·`) are `#[hardware]`-eligible and `#[bit_identical]`.
- NTT butterfly `(a, b) ↦ (a+b, (a−b)·ω)` is a vector primitive.

---

## 10. Compile-time evaluation (zero cold-start)

- `comptime { ... }` — arbitrary pure computation at compile time (Zig-style), evaluated by the compiler's interpreter.
- `const fn` — pervasive const-evaluation for pure functions.
- **Baking:** `comptime` blocks build the living-memory index, NTT twiddle factors, and code-graph; the results are emitted as static arrays in `.rodata`. Runtime does pointer dereference only — the daemon's cold-start disappears by construction.

---

## 11. Backend model

### 11.1 LLVM (v1)
- QTT → LLVM: monomorphise generics, **erase quantity-`0`** terms, lower `Vector` to LLVM vector IR, `no_std`/freestanding target support.
- Contracts have no runtime cost in the subset (proved away); outside the subset, `ensures`/`requires` become debug `assert!` (removed in release).

### 11.2 `#[hardware]` → MLIR/Calyx (v2)
- `#[hardware] fn` is lowered to a Bebop MLIR dialect → **Calyx** → synthesizable Verilog (hypervector bundling, NTT butterfly, SHA-3 round). Same source as the CPU tier; the compiler picks the target.

---

## 12. Appendix A — full EBNF

```
program     ::= { item }
item        ::= module_decl | fn | data | struct | enum | quotient
              | trait | impl | contract | use | type_alias
fn          ::= [attrs] "fn" ident generics? "(" params ")" ["!" effect]
                ["->" type] contract_clauses? block
data        ::= "data" ident indices? "|" ctor { "|" ctor }
ctor        ::= ident [ "(" [type {"," type}] ")" ]
quotient    ::= "quotient" ident "=" type "by" "(" ident "~" ident "=>" expr ")"
contract    ::= "contract" ident [generics?] "{" { fn_sig invariant } "}"
contract_clauses ::= { requires | ensures | reads | writes | decreases | invariant }
block       ::= "{" { stmt } "}"
stmt        ::= let | expr ";" | return expr | if | match | loop | while | for | break | continue
expr        ::= literal | ident | glyph_lit | unop expr | expr binop expr
              | expr "(" [expr {"," expr}] ")" | "if" expr block ["else" block]
              | "match" expr "{" { pat "=>" expr } "}" | "let" pat "=" expr
pat         ::= ident | ctor pat | "_" | literal
type        ::= ident | "Type" | "Vector<" const "," type ">" | "Field<" const ">"
              | "Zmod<" const ">" | type "->" type | "(" x ":" q type ")" "->" type
              | "&" ["mut"] type
```

---

## 13. Appendix B — worked example (NTT with a round-trip contract)

```bebop
module dowiz.ntt

// Prime field and primitive root for the length-1024 NTT.
const P: I64 = 2^64 - 2^32 + 1          // Goldilocks prime, illustrative
const ROOT: Field<P> = [...omega...]

pure fn ntt(x: &[Field<P>; n]) -> [Field<P>; n]
  requires  n.is_power_of_two()
  ensures   |result| == |x|
  ensures   inverse(ntt(ntt(x))) == x            // round-trip, proved not tested
  decreases n
{
  match n {
    1 => x,
    n => {
      let (even, odd) = decimate(x)             // even-indexed / odd-indexed
      let e = ntt(even)
      let o = ntt(odd)
      butterflies(e, o, ROOT)                   // vector primitive, §9
    }
  }
}

pure fn inverse(y: &[Field<P>; n]) -> [Field<P>; n]
  ensures ntt(inverse(y)) == y
{ /* conjugate twiddles, same butterfly */ }
```

The `ensures inverse(ntt(ntt(x))) == x` becomes an SMT obligation that — for a fixed `n` — discharges by bit-vector/field reasoning; for symbolic `n` it is a bounded-induction VC over `decreases n`.

---

## 14. Open items carried into implementation
1. Glyph outlines (Phase 1) — format fixed here, actual δ-outlines drawn in Phase 1.
2. `Prop` vs proof-relevant `Type` (impredicativity) — v0.2 decision.
3. Capability inference breadth (`!{e}` polymorphism) — v0.2.
4. SMT solver selection heuristic (Z3 vs CVC5 per fragment) — Phase 3.
5. Self-hosting order (`bebopc` in Bebop) — post-Phase-3 milestone.
