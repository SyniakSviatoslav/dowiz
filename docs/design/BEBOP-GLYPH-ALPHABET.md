# Bebop — the Glyph Alphabet (v0.2, ordinary lexicon)

> Phase 1 foundation. The closed surface vocabulary of Bebop.
> **Law:** glyphs are **vector δ-outlines, never emoji**. The Unicode char in the left
> column is only a *terminal-render placeholder*; the canonical form is the δ-outline
> on a pixel grid (drawn in Phase 1). ASCII names are the fallback tokens agents parse.
> Lexicon is **ordinary** — technical terms, not metaphor.

## 1. Structure
| Glyph | Name | Meaning |
|---|---|---|
| `◈` | `mod` | module / namespace |
| `★` | `fn` | function definition |
| `◇` | `struct` | record / product type |
| `△` | `data` | inductive / sum type |
| `◉` | `val` | value / term |
| `⊙` | `contract` | precondition / postcondition / invariant |
| `◎` | `quotient` | quotient type (equivalence class) |
| `✦` | `trait` | interface / typeclass |
| `♁` | `impl` | implementation |
| `⌘` | `type` | type alias |
| `⇐` | `use` | import / bring into scope |
| `¤` | `const` | constant |
| `◐` | `let` | let-binding |
| `◑` | `mut` | mutable binding |
| `❖` | `match` | pattern match |
| `◒` | `if` | conditional |
| `↻` | `loop` | loop |
| `↺` | `while` | while |
| `♒` | `for` | for-each |
| `↯` | `return` | return |

## 2. Calculus
| Glyph | Name | Meaning |
|---|---|---|
| `λ` | `\` | lambda / abstraction |
| `→` | `->` | arrow / function type |
| `∏` | `Pi` | dependent product (forall) |
| `∑` | `Sigma` | dependent sum (exists/pair) |
| `:` | `:` | type annotation |
| `::` | `::` | cons / member |
| `≡` | `===` | definitional equality |
| `≅` | `~=` | quotient equivalence relation |
| `≈` | `~~` | propositional equality |
| `⊢` | `|-` | typing judgement |
| `⊨` | `|=` | proof obligation / models |
| `=` | `=` | assignment / let |
| `.` | `.` | field access |
| `;` | `;` | separator |
| `,` | `,` | separator |

## 3. Quantities (QTT rig)
| Glyph | Name | Meaning |
|---|---|---|
| `∅` | `0` | quantity 0 — erased / proof-only |
| `·` | `1` | quantity 1 — linear / owned |
| `∞` | `w` | quantity ω — unrestricted / shared |

## 4. Numeric tower
| Glyph | Name | Meaning |
|---|---|---|
| `ℤ` | `Z` | integers |
| `ℕ` | `N` | naturals |
| `𝔽ₚ` | `Fp` | prime field (NTT modulus) |
| `ℤₘ` | `Zm` | ring mod m |
| `i64` | `i64` | signed 64-bit (money) |
| `u64` | `u64` | unsigned 64-bit |
| `f64` | `f64` | float (gated) |
| `♾` | `inf` | infinity |

## 5. NTT & fields
| Glyph | Name | Meaning |
|---|---|---|
| `ωₙ` | `w_n` | primitive n-th root of unity |
| `⟲` | `ntt` | number-theoretic transform |
| `⟳` | `intt` | inverse NTT |
| `⊛` | `conv` | circular convolution |
| `×` | `*` | multiply |
| `＋` | `+` | add |
| `−` | `-` | subtract |
| `÷` | `/` | divide |
| `∤` | `%` | modulo / remainder |
| `≪` | `<<` | shift left |
| `≫` | `>>` | shift right |

## 6. Hypervector (1024-bit)
| Glyph | Name | Meaning |
|---|---|---|
| `⧉` | `hv` | hypervector (D=1024) |
| `⊕` | `bundle` | bundling (sum/consensus) |
| `⊗` | `bind` | binding (xor/product) |
| `⊖` | `unbind` | unbind (release) |
| `∘` | `shift` | circular permutation |
| `⊚` | `sim` | cosine / hamming similarity |
| `⋔` | `xor` | bitwise xor |
| `⋏` | `maj` | majority / count-ones |

## 7. Quantum (hybrid state)
| Glyph | Name | Meaning |
|---|---|---|
| `ψ` | `state` | hybrid quantum state |
| `|⟩` | `ket` | ket vector |
| `⟨|` | `bra` | bra vector |
| `⨁` | `super` | superposition (sum) |
| `⨂` | `tensor` | entanglement (tensor product) |
| `𝐇` | `H` | Hadamard |
| `𝐌` | `M` | measurement |

## 8. Memory & relations (living-memory)
| Glyph | Name | Meaning |
|---|---|---|
| `⌾` | `node` | memory node |
| `⤳` | `edge` | graph edge |
| `⋈` | `join` | relational join |
| `⊑` | `sub` | subset / containment |
| `≺` | `pre` | precedence / order |
| `⤴` | `neigh` | neighbor / path |
| `⇝` | `search` | semantic search |

## 9. Parallelism & atomics
| Glyph | Name | Meaning |
|---|---|---|
| `∥` | `par` | parallel composition |
| `⋉` | `fork` | fork |
| `⋊` | `join` | join |
| `⚛` | `atomic` | atomic operation |
| `⤫` | `branchless` | branchless predication |
| `⟕` | `spawn` | spawn task |
| `⟖` | `sync` | synchronize |
| `⧗` | `barrier` | barrier |

## 10. Logic
| Glyph | Name | Meaning |
|---|---|---|
| `∧` | `&&` | and |
| `∨` | `||` | or |
| `¬` | `!` | not |
| `∀` | `forall` | forall |
| `∃` | `exists` | exists |
| `⊃` | `=>` | implies |
| `⊥` | `bottom` | bottom / absurd |
| `⊤` | `top` | top / unit-truth |

## 11. Effects (capabilities)
| Glyph | Name | Meaning |
|---|---|---|
| `⏱` | `clock` | time |
| `⚄` | `rng` | randomness (caller-supplied) |
| `⬡` | `env` | environment |
| `⟡` | `float` | floating point |
| `⌁` | `net` | network |
| `⚙` | `io` | I/O |
| `∅` | `pure` | pure (no capability) |

## 12. Agentic surface (agents + humans, one grammar)
- **ASCII fallback is total**: every glyph has a stable ASCII token (`fn`, `mod`, `⊕`→`bundle`, …),
  so an agent parses `.bp` unambiguously with no vision.
- **Contracts = specs**: `⊙` clauses are machine-checkable, so an agent verifies its own
  output before a human reviews it.
- **Living-memory relational** (`⌾⤳⋈⇝`): an agent navigates the codebase semantically.
- **Deterministic, no_std**: an agent reasons about behavior without ambient state.
