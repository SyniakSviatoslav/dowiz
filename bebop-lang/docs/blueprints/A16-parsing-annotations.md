# A16 — Closures, Generics, Dependent Types: Parsing & Annotation Support

**Status:** IMPLEMENTATION BEGUN (2026-09-11)
**Scope:** Parsing/annotation support first. Full CIC, closures, generics, higher-order functions.
**Approach:** ONE grammar, elaboration as a compiler PHASE (not a second language).

---

## Current State Assessment

### What exists today
- **Language surface:** `docs/LANGUAGE.md` — integer-only, first-order, no closures/generics/dependent types
- **Compiler:** `bebop.bp` (7691 lines) — single-pass, one grammar, emits AArch64 words
- **Parser entry:** `compile_program(s)` → `compile_program_to(s, insns, n)` → per-fn `compile_fn(s)`
- **Statement classification:** `emit_body_classify` at bebop.bp:5445 — classifies `let`/`while`/`return`/`break`/compound/expr
- **Expression parsing:** `emit_cmp` → `emit_add` → `emit_mul` → `emit_unary` → `emit_postfix` → `emit_primary`
- **Pattern matching:** `emit_match_lit` for compile-time constructor patterns only
- **F6 certificate checker:** `selfhost/tcheck.bp` (529 lines) — standalone, own 511-fn cap
- **F7/F8:** Not started — these depend on A16's parsing/annotation foundation

### What A16 requires (from ROADMAP.md line 104-105, F8 line 192)
1. **Closures** — first-class function values capturing environment
2. **Generics** — parameterized types/functions
3. **Higher-order functions** — functions as arguments/returns
4. **Dependent types over infinite universe hierarchy** — full CIC
5. **Annotations** — `[T; n]`, `{x : T | p}`, `requires`/`ensures`/`invariant`, `theorem`
6. **Elaboration** — compiler PHASE producing internal rep + proof term (NOT a second language)

### Constraint from ROADMAP
- Defunctionalisation/monomorphisation survive but are re-scoped: compiled artifact stays first-order/monomorphic
- Translation-validation layer reasons about the monomorphic artifact
- Surface types are elaborated AWAY during compilation (erased before emission)
- VCs (verification conditions) emitted into F4/F5's side zone

---

## Phase 1: Parsing & Annotation Support (THIS PHASE)

### 1.1 Update LANGUAGE.md
Remove "closures, generics" from "What is NOT in the language" (line 132-134).
Add annotation syntax to the grammar spec.

### 1.2 Annotation Scanner (new function in bebop.bp)
- `scan_annotations(s, pos, strn, fntab)` — scans for annotation prefixes before fn/struct/enum declarations
- Recognized annotations:
  - `[T; n]` — length-indexed array type (dependent)
  - `{x : T | p}` — subset type (refinement)
  - `requires(expr)` — precondition
  - `ensures(expr)` — postcondition  
  - `invariant(expr)` — loop invariant
  - `theorem name(args) -> type { body }` — theorem declaration
  - `forall`, `exists` — quantifiers (for future use)
- Annotations are PARSED into fntab side-channel slots, then ERASED (not emitted)
- Must not break existing constructs (gen3 == gen4 fixpoint preserved)

### 1.3 Type Annotation Parser (extension of skip_to_field_delim)
- Extend `parse_params` to handle:
  - Generic parameters: `fn foo[T](x: T) -> T { ... }`
  - Dependent types in params: `fn bar(n: i64, arr: [i64; n]) -> i64 { ... }`
  - Return type annotations with dependencies
- Type syntax extension:
  - `TYPE := ... | '[' TYPE ';' expr ']' | '{' NAME ':' TYPE '|' expr '}' | TYPE '#' u64` (universe level)
- Must parse WITHOUT requiring the expression to be fully evaluated (annotations are compile-time only)

### 1.4 Closure Syntax (parsing only, emission deferred)
- `fn (x: T) -> T { body }` — anonymous closure literal (capture by value)
- `fn name[T](x: T) -> T { ... }` — generic function declaration
- Closure capture analysis: which locals need to be captured?
- Closure value representation: `(code_ptr, env_ptr)` pair — defer code gen

### 1.5 fntab Side-Channel Slots for Annotations
Reserve new fntab slots:
- `fntab[5550..555F]` — annotation storage (16 slots)
- Per-function annotation list: linked list in fntab
- Annotation kinds: REQUIRES=1, ENSURES=2, INVARIANT=3, THEOREM=4, DEP_TYPE=5, CLOSURE=6, GENERIC=7

### 1.6 Elaboration Phase Skeleton
- `elab_annotations(s, pos, fntab)` — called after `compile_fn` parses the signature
- Walks annotated positions, emits VC placeholders into a side buffer
- Does NOT change code generation — annotations are erased from the emitted stream
- Proof terms are structural (F7 kernel will check them later)

### 1.7 Universe Level Parser
- Parse `Type 0`, `Type 1`, ..., `Type u` (universe levels)
- Universe `Prop` (impredicative, for CIC)
- Level arithmetic: `max(u, v)`, `u + 1`
- Store levels in fntab side channel for F7 kernel

---

## Files to Modify

| File | Change | Risk |
|------|--------|------|
| `docs/LANGUAGE.md` | Remove closures/generics from "NOT in language"; add annotation syntax | LOW — doc only |
| `bebop.bp` | Add `scan_annotations`, extend `parse_params`, add fntab slots | MEDIUM — must preserve fixpoint |
| `tools/bpref.py` | Add annotation AST nodes (if they appear in bpref mirror) | LOW |
| `tools/typecheck.py` | Extend type grammar for dependent types | LOW |

## Files to Create

| File | Purpose |
|------|---------|
| `docs/blueprints/A16-parsing-annotations.md` | This plan, frozen |
| `docs/LANGUAGE.md` (amended) | Updated language surface spec |
| `selfhost/elab.bp` (stub) | Elaboration phase scaffold (A16 Phase 2 prep) |

---

## Done-When (Phase 1)

1. `docs/LANGUAGE.md` no longer lists closures/generics under "NOT in the language"
2. `bebop.bp` parses `[T; n]`, `{x:T|p}`, `requires/ensures/invariant/theorem` annotations without error
3. Annotations are ERASED (not emitted) — existing `self_check()` ctors still pass
4. `gen3 == gen4` fixpoint unchanged (annotation parsing adds zero words to the compiler compiling itself)
5. Generic parameter syntax `fn foo[T](...)` parses
6. Closure literal syntax `fn (x) -> T { ... }` parses (emission deferred)
7. bpref.py handles the new AST nodes (annotation nodes are inert)
8. typecheck.py recognizes the extended TYPE grammar

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Annotation parsing breaks existing fixpoint | Parse annotations as NO-OP when not recognized; existing code has none |
| fntab slot collision with existing uses | Verify fntab[5550..555F] are free (scan bebop.bp for `fntab\[55[0-9a-f]\d\]`) |
| Universe level syntax conflicts with existing `Type` | `Type` is not a keyword today — check first |
| Too much parsing complexity too early | Keep Phase 1 to PARSING ONLY — no semantic checking, no code gen changes |

---

## Measurement

- **Annotation parse time:** < 1 ms per function (annotation scanning is a single sequential pass)
- **Fixpoint impact:** 0 words added to bebop.bin (annotations erased before emission)
- **Construct parity:** +N annotation constructs (one per annotation kind), all EXPECT=p Ass until F7 checks them
- **bpref parity:** annotation nodes are inert in bpref; parse tree includes them but evaluation ignores them
