# Bebop — the Cosmic Glyph Alphabet (v0.1)

> Phase 1 foundation. The complete glyph lexicon — the only surface of Bebop.
> Every symbol below is a **glyph** (δ-encoded vector outline on a pixel grid);
> ASCII is a terminal fallback, never the source of truth.
> Author: Sviatoslav Syniak · AGPL-3.0-or-later · 2026-08-17

## The law
> **There are no words in Bebop.** Every token is a glyph. A program is a
> geometric composition of these symbols. The compiler reads glyphs; the editor
> draws them; the terminal rasterizes them to braille/half-block pixels
> (dowiz `pixel_snapshot.rs`). This table is the complete surface vocabulary.

Each entry: `GLYPH` = `name` — meaning (*cosmic name*). The `name` is the
ASCII fallback token; the glyph is canonical.

---

## 1 · Structure — the sky

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `◈` | `mod` | module / namespace | star-system |
| `★` | `fn` | function definition | star |
| `◇` | `struct` | record / product | diamond |
| `△` | `data` | inductive / sum | constellation |
| `◉` | `val` | value / term | world |
| `⊙` | `contract` | pre/post/invariant | halo |
| `◎` | `quotient` | quotient type | eclipse |
| `✦` | `trait` | typeclass / interface | binary star |
| `♁` | `impl` | implementation | moon |
| `⌘` | `type` | type alias | comet |
| `⇐` | `use` | import / bring | orbit-entry |
| `¤` | `const` | constant | fixed star |
| `◐` | `let` | let-binding | half-world |
| `◑` | `mut` | mutable binding | waxing-world |
| `❖` | `match` | pattern match | prism |
| `◒` | `if` | conditional | terminator-line |
| `↻` | `loop` | loop | orbit |
| `↺` | `while` | while | retrograde-orbit |
| `♒` | `for` | for-each | stream |
| `↯` | `return` | return | reentry |
| `⤼` | `break` | break | escape-velocity |
| `⤿` | `continue` | continue | slingshot |

## 2 · Calculus — the orbits

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `λ` | `lambda` | abstraction | wave |
| `→` | `arrow` | function type | trajectory |
| `∏` | `pi` | dependent product | field-of-orbits |
| `∑` | `sigma` | dependent sum | union-of-orbits |
| `:` | `colon` | type annotation | casts-light |
| `::` | `cons` | member / cons | docking |
| `≡` | `eqdef` | definitional equality | same-star |
| `≅` | `eqquot` | quotient equivalence | same-orbit |
| `≈` | `eqprop` | propositional equality | near-orbit |
| `⊢` | `turnstile` | typing judgement | emits |
| `⊨` | `models` | contract obligation | satisfies |
| `=` | `assign` | binding/assignment | lock |
| `.` | `dot` | field access | land |
| `;` | `semi` | separator | tick |
| `,` | `comma` | separator | breath |

## 3 · Quantities — the QTT rig

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `∅` | `q0` | quantity 0 (erased/proof) | void |
| `·` | `q1` | quantity 1 (linear) | photon |
| `∞` | `qω` | quantity ω (unrestricted) | cosmos |
| `⊕` | `plusq` | quantity join | merge |
| `⊗` | `timesq` | quantity tensor | entangle |

## 4 · Numeric tower — the matter

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `ℤ` | `z` | integers | bedrock |
| `ℕ` | `n` | naturals | pebble |
| `𝔽ₚ` | `fp` | prime field mod p | gem |
| `ℤₘ` | `zm` | ring mod m | ore |
| `ℚ` | `q` | rationals | dust |
| `ℝ` | `r` | reals (gated) | continuum |
| `ℂ` | `c` | complex (gated) | spectrum |
| `i64` | `i64` | 64-bit int (money) | coin |
| `u64` | `u64` | 64-bit uint | counter |
| `f64` | `f64` | float (no_std-gated) | mirage |
| `♾` | `inf` | infinity value | horizon |

## 5 · NTT & fields — the signal

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `ωₙ` | `root` | primitive n-th root of unity | pole-star |
| `⟲` | `ntt` | forward NTT | redshift |
| `⟳` | `intt` | inverse NTT | blueshift |
| `⊛` | `conv` | circular convolution | fold |
| `×` | `mul` | field multiply | fuse |
| `＋` | `add` | field add | accrete |
| `−` | `sub` | subtract | shed |
| `÷` | `div` | divide | split |
| `∤` | `mod` | modulo | remainder-orbit |
| `≪` | `shl` | shift left | outbound |
| `≫` | `shr` | shift right | inbound |
| `⤳` | `permute` | bit-reversal permute | scatter |

## 6 · Hypervector — the mind

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `⧉` | `hv` | hypervector (1024-bit) | mind |
| `⊕` | `bundle` | bundling (sum) | superpose |
| `⊗` | `bind` | binding (product) | entangle |
| `⊖` | `unbind` | unbind | release |
| `∘` | `shift` | circular shift | spin |
| `⊚` | `sim` | similarity (cosine/dot) | resonance |
| `⋔` | `xor` | XOR (Hamming) | antipode |
| `⋏` | `majority` | majority/threshold | consensus |
| `∥` | `norm` | vector norm | magnitude |

## 7 · Quantum — the hybrid state

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `ψ` | `state` | quantum state | wavefunction |
| `|⟩` | `ket` | ket vector | emission |
| `⟨|` | `bra` | bra vector | absorption |
| `⨁` | `superpose` | superposition | interference |
| `⨂` | `tensor` | entanglement | correlation |
| `𝐇` | `hadamard` | Hadamard gate | beam-split |
| `𝐌` | `measure` | measurement | collapse |
| `⨯` | `cross` | cross product | gyre |

## 8 · Memory & relations — the galaxy (living-memory foundations)

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `⌾` | `node` | memory node | star-cluster |
| `⤳` | `edge` | dependency edge | filament |
| `⋈` | `join` | relational join | merge-galaxy |
| `⊑` | `subset` | containment | within-orbit |
| `≺` | `precede` | precedence/order | before |
| `⋈` | `path` | graph path | light-path |
| `⤴` | `neighbor` | neighbors | local-group |
| `↺` | `recall` | recall record | retrieve |
| `⇝` | `search` | search (vector/keyword) | probe |

## 9 · Parallelism & atomics — the clock

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `∥` | `par` | parallel composition | simultaneity |
| `⋉` | `fork` | fork | branch |
| `⋊` | `join` | join | merge |
| `⚛` | `atomic` | atomic operation | indivisible |
| `⤫` | `branchless` | branchless/predicated | predestined |
| `⟕` | `spawn` | spawn task | birth |
| `⟖` | `sync` | synchronize | rendezvous |
| `⧗` | `barrier` | barrier | equinox |
| `⧖` | `wait` | wait | patience |

## 10 · Logic — the proof

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `∧` | `and` | conjunction | conjunction |
| `∨` | `or` | disjunction | disjunction |
| `¬` | `not` | negation | negation |
| `∀` | `forall` | universal | all-orbit |
| `∃` | `exists` | existential | some-orbit |
| `⊃` | `implies` | implication | causation |
| `⊥` | `bottom` | false / empty | singularity |
| `⊤` | `top` | true / unit | cosmos-all |

## 11 · Effects & capabilities — the atmosphere

| Glyph | Name | Meaning | Cosmic name |
|---|---|---|---|
| `⏱` | `clock` | time capability | chronos |
| `⚄` | `rng` | entropy capability | chaos |
| `⬡` | `env` | environment capability | weather |
| `⟡` | `float` | float capability (gated) | shimmer |
| `⌁` | `net` | network capability | aether |
| `⚙` | `io` | io capability | machinery |
| `∅` | `pure` | purity (empty capability) | vacuum |

## 12 · Cosmic names — the poetry

The compiler's diagnostics, type errors, and traces speak in these terms:
type = *constellation* · function = *star* · value = *world* · module = *star-system* ·
memory = *galaxy* · proof = *orbit* · contract = *halo* · bug = *singularity* ·
compile = *forge* · link = *docking* · run = *flight* · crash = *supernova*.

---

## Encoding note
Each glyph's canonical form is a δ-outline: `[(dx,dy), …]` pen-deltas + fill rule
(spec v0.2 §2.4). The 300-glyph outline corpus is drawn in the font sub-phase of
Phase 1; this table fixes **name → meaning → cosmic-name** and the **glyph set**.
ASCII names above are the terminal fallback (`--render=ascii`), never the parse input.

## Completeness rule
The alphabet is **closed** — any construct not in this table has no Bebop
surface form (and vice versa: every glyph in this table must parse). Adding a
construct = adding its glyph here first, RED+GREEN (MANIFESTO C7).
