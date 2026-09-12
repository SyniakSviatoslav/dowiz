Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row **A16** (REQUIRED since
2026-09-09, `ROADMAP.md:104`). This document SUPERSEDES `docs/blueprints/A16-parsing-annotations.md`
(139 lines, "IMPLEMENTATION BEGUN 2026-09-11"), whose status is not backed by the tree (§0a), and
should land beside it with the old file marked superseded in its first line -- not deleted, because
`ROADMAP.md`'s A16 blueprint column is `--` today and `tools/arch_check.py`'s `max_missing_citations`
counts documents citing files that do not exist. Delivers language items 10 (higher-order functions),
9 (closures) and 8 (generics), in that order, under the agent-first framing (RESEARCH §6).

# A16 -- function values first, explicit environments second, generics as erasure third

## 0. Three corrections this blueprint rests on

**(0a) Nothing of A16 is in the tree, and the document that says otherwise measures parser laxness.**
`docs/LANGUAGE.md:135-138` (added by `4286ec1`, 2026-09-11) says "their SURFACE SYNTAX (annotations,
generic params, closure literals) IS parsed and erased by the compiler as of A16 Phase 1". `grep -n
'annotation\|generic\|closure\|requires\|ensures\|theorem' bebop.bp` returns only comments (`:121`,
`:253`, `:336`, `:2430`, `:5248`, `:5618`, `:6092`, `:6102`). `tools/f8_dt.py`'s four programs compile
because `compile_fn_at:6041` `skip_to(s, pos, 123)` discards everything between `)` and `{` and
`collect_fns:5870` scans only `fn `, so a top-level `theorem ...` line is never seen. That is not
erasure; it is the same accept-and-ignore path that let `let _ = if ok == 0 then 0 else 0` ship (A18).
`docs/VERIFIED-STATE-2026-09-12.md`'s table row for A16 -- "no parser support, NOT IMPLEMENTED" -- is
the correct statement. This blueprint starts from zero and says so.

**(0b) The two facts the design rests on were established by `docs/RESEARCH-PROOF-DESIGN-2026-09-09.md`
and re-checked today.** (i) The emitter has **0 `blr` sites** (`:9`; re-checked: `grep -c blr bebop.bp`
matches only the `sys_run` comment). (ii) A closure packs into one i64 as fn index + environment cell
index (`:81`: "9 bits of fn index ... + 29 bits of environment cell index = 38 of 64 bits"; with a fn
INDEX in words rather than a table slot it is 18 + 29 = 47 bits, still fine). That study proposed
DEFUNCTIONALISATION (an `apply_T` `if`-chain per arrow type, `:85`) to keep the artifact first-order;
this blueprint does not, because (a) it needs a whole-program pass to enumerate lambdas, which the
one-pass thesis forbids without the amendment that study itself said must be WIDENED (`:71`), and (b) a
direct `blr` costs the same 3-5 words and needs no pass. The census gains a `blr` column instead of
staying at 0 by construction.

**(0c) The environment shape the tree already uses IS the closure this row ships.** 22 fns take
`env: [i64]` (grep); `smw.bp` was rewritten to pack everything into one env (journal 1789220830); B6's
blueprint §4 makes it the mandatory parallel-region shape. An explicit environment is what an agent
writes correctly and what the register model already handles (a cell index in one register). Capture
INFERENCE -- finding `a` in `|x| x + a` and building the env -- is the part that needs a text pass
(lambda lifting in the `use_expand` shape) plus escape classification; it is costed in §6 and NOT
scheduled. That is the one human-legibility trade in this row, and it is named.

---

## 1. Scope

### A16 IS

1. **Step 1 -- function values and indirect calls (T50 "functions as cells", HISTORY.md:1749, OPEN).**
   `&f` for a declared fn `f` is a compile-time constant: `f`'s start offset in WORDS from the image base
   (`fntab_lookup:652` at emission; in the planning pass the offset is a placeholder, so the constant is
   emitted as a FIXED 2-word `movz`/`movk` so both passes agree on size). `call_fn(v, a0, ..., ak-1)`
   (k <= 13) delivers `a0..ak-1` to x0..x(k-1) and `v` to x<k> through `vs_deliver(k+1)`, then emits
   `adr x16, #-(4 * n[0])` (the image base, PC-relative; in range because the program cap is 262,144
   words = 1 MiB = `adr`'s reach, `cap_exit:2172`), `add x16, x16, x<k>, lsl #2`, `blr x16`, wrapped in
   `emit_bl`'s x15 save/restore (`:628-651`), result `REG x0` (and x1/x2 under A21). **5-6 words per site**,
   0 elsewhere, no allocation. Position-independent, so it survives `sys_run` of a foreign image.
2. **A range trap** at every `call_fn`: `cmp x<k>, x<words>` against the image length (a fixed constant
   the emitter knows at the END of the program only -- so the check is against the `.bin` footer's word
   count read at run time, ~3 words, or is deferred to the `brk #87`-class "unresolved" text). Chosen:
   `tbnz`-free 3-word compare against a per-program constant patched at `write_lit_cells` time, with a
   new runtime trap text (an A17 step-3 stub change: the trap number from the free set).
3. **Step 2 -- closures = fn value + EXPLICIT environment.** `call_fn(v, env, x)` where `env: [i64]` is an
   array literal (arena cursor, released at `ret`, reset per loop iteration when `loop_alloc_safe:4632`
   holds) or `zeros` when it must escape. Sugar: `pack_fn(v, env)` -> `(v << 40) | env` (one i64, storable
   in a cell or the store) and `call_packed(c, x)` -> two unpack words (`lsr`, `and`) then the step-1
   sequence with env as the first argument. No capture inference. Constructs: a comparator passed to a
   `sort_by`, a reducer with state passed to a `fold`, a packed closure stored in an array and called
   later.
4. **Step 3 -- generics as erasure.** `fn f[T](x: T) -> T` and `fn map[T](a: [T], n: i64, g: i64) -> i64`
   parse: `compile_fn_at` skips `[..]` after the name; `collect_fns` is unaffected (it matches `fn NAME`
   then reads an ident); `parse_params`' type reader (A22 step 2) accepts `T`, `[T]`. `T` is a type
   VARIABLE for `tools/typecheck.py` (A8's oracle) and nothing else, because every value is an i64 or a
   cell index and `[T]` compiles to `[i64]` for every T. **0 words.** Constructs prove a generic `map`
   over two element "types" (i64 and struct indices) runs unchanged and that `typecheck.py` rejects
   `f[i64]("abc")`-class misuse (a `bench/typecheck_neg/` file).
5. **Step 4 -- monomorphisation, only when codegen depends on T.** That is the day A8 lands `[u32]`
   (4-byte loads, `docs/blueprints/A8-typed-tables-u32.md` §3). Then instantiation is textual in the
   `use_expand` shape (`bebop.bp:6869`), 8-12 fns (RESEARCH-PROOF-DESIGN `:155`), keyed `(fn, types)`,
   polymorphic recursion refused with a position (`:93`). **Costed, not scheduled before A8 step 2.**

### A16 IS NOT

1. **Not capture inference / lambda syntax.** `|x| x + a` is costed at ~2 weeks (a lambda-lifting text
   pass + escape classification over `loop_alloc_safe`'s scan + an elaborator diagnostic for an escaping
   closure created in a loop, RESEARCH-PROOF-DESIGN `:89`) and not scheduled. An agent writes `[a]` and
   passes it; the tree already does.
2. **Not defunctionalisation / `apply_T`.** Needs a whole-program pass; `blr` needs none.
3. **Not dependent types, annotations, `requires`/`ensures`, `theorem`, universes.** Those are F8's row
   (`ROADMAP.md:192`) and the F7 kernel's; the old A16 document mixed them in. The f8_dt gate stays what
   it is (a laxness measurement) until F8 gives it a compiler.
4. **Not separate compilation of a HOF against unknown callees** -- Bebop compiles whole programs
   (`use` inclusion), and `&f` needs `f` in the same program.
5. **Not a change to the 14-parameter cap, the x0..x7 window, `emit_bl`, or the epilogue.**

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/c135_fnref.bp` | step 1: `&f`/`&g` in a table, `call_fn` in a loop, a table-driven dispatcher of 8 cells (T50's own DONE-CHECK) |
| `bench/parity_constructs/c136_hof.bp` | step 1: `map`/`fold` taking a fn value; `sort_by` with a comparator; the fn values passed through TWO levels of call |
| `bench/parity_constructs/neg/c137_fnref_range.bp` | step 1: `call_fn(999999, 0)`; `EXPECT=RUNFAIL:<trap>` |
| `bench/parity_constructs/c138_closure_env.bp` | step 2: a reducer with state in `env`, a packed closure stored in an array and called from another fn |
| `bench/parity_constructs/c139_generic_map.bp` + `bench/typecheck_neg/generic_misuse.bp` | step 3 |
| `bench/vs_rust/census.txt`, `tools/census.py` | a `blr` column; `bebop`'s own count stays 0 until generation N+1 |
| `bench/vs_rust/std_golden.sh` | no new gate; `ok=117` unchanged |

### 2.2 The number

`c135`: a table `t = [&f0, &f1, ..., &f7]` of eight fns each computing a distinct affine map; the program
folds `call_fn(t[i & 7], i)` over i in 0..999 -- EXPECT derived by hand as the sum of eight arithmetic
series in the header (the derivation is by residue class, the program is by dispatch: not the same
computation). `c136`'s `sort_by` sorts 64 LCG values with a comparator and folds `v[i] * (i + 1)` --
EXPECT from bpref (bpref implements `fnref`/`call_fn` by name lookup, 30 lines) and cross-checked by a
second comparator that must give the reverse order's fold.

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c135` MATCH at the hand value | `&f` is the planning pass's placeholder (both passes must emit the SAME value: the constant is `fntab_lookup`'s answer on the emission pass and 0 on the planning pass -- the WORD COUNT agrees (2 words) but the VALUE must be right on the emission pass only, which is the one that ships); or `adr`'s base is off by the current word (`n[0]` must be the offset of the `adr` word itself, not of the next) |
| B | `c136` MATCH, and the reversed comparator gives the reversed fold | the second-level pass of a fn value through a `[i64]` param loses it (it cannot: it is an i64) |
| C | `c137` RUNFAIL:<trap> with the A17 line on stderr | the range check is missing or compares against the wrong bound (the footer word count, `fill_size`-class, `:6669`) |
| D | `c138` MATCH; the packed closure stored in a cell and called from another fn returns the same as the direct call | the unpack masks the wrong bits (`v << 40`: env cell indices are < 2^29 by A5's reserve, `docs/blueprints/A5-arena-relative-addressing.md` "Address space"; verify the constant against the reserve actually mapped, 4 GiB or the 1 GiB fallback) |
| E | `c139` MATCH with WORD_DELTA 0 against its non-generic twin | erasure emitted something |
| F | `bebop`'s census row: bcond +k with an allow line, `blr` 0 | the compiler used `call_fn` in the landing commit (two-stage rule) |
| G | WORD_DELTA 0 on 104/104 pre-existing constructs | the ladder in `emit_call_or_ctor` changed for existing names |
| H | a mutant that drops `emit_bl`'s x15 save around `blr` makes `c136` (a caller with > 8 symbols) FAIL | `c136`'s HOF caller does not spill (make it spill: 10 locals) |

---

## 3. Which existing work A16 sits on

| existing | verdict | reason |
|---|---|---|
| `docs/blueprints/A16-parsing-annotations.md` | **SUPERSEDED** (first line edited to say so; kept for the citation ratchet) | its status is not in the tree; its plan starts with dependent-type syntax that belongs to F8 |
| `tools/f8_dt.py` + `docs/LANGUAGE.md:135-138` | **CORRECTED**: LANGUAGE.md's paragraph replaced by the truth (parsed by laxness; F8 owns the syntax); `f8_dt` stays in the battery only if renamed to what it measures, else removed | a gate that passes on laxness is a false green |
| `docs/RESEARCH-PROOF-DESIGN-2026-09-09.md` §3 | **USED** for the packing (`:81`), the 0-`blr` fact (`:9`), the loop/escape hazard (`:89`), the monomorphisation cost (`:155`); its `apply_T` (`:85`) NOT adopted (§0b) | |
| `emit_call_or_ctor` (`bebop.bp:1831-1878`) | **EXTENDED**: two ladder arms (`call_fn`, `pack_fn`/`call_packed`), + `&` handled in `emit_factor` (`:4344`, unary position, like `-`/`!`) | `&f` is a unary form on an identifier |
| `fntab_lookup` (`:652`), `emit_call_resolve` (`:5700`) | **REUSED** for `&f`'s offset | |
| `vs_deliver` (`:935`) | **REUSED** with k+1 operands | |
| `emit_bl` (`:628-651`) | **REUSED** around the `blr` (x15 save/restore under `has_spills`) | |
| the reserved-word table `compile_fn_at:6025-6027` and `tools/builtin_surface.py` | **EXTENDED** (`call_fn`, `pack_fn`, `call_packed` become reserved; F0's 36 -> 39) | the shadowing class F0 found |
| `tools/census.py` | **EXTENDED** with a `blr` column | a new instruction class must be visible to the honesty floor |
| `emit_match_rt` (`:2050-2108`) | the model for an `if`-chain dispatcher if one is ever needed; NOT used | |
| `tools/bpref.py` | **EXTENDED**: `('fnref', name)` evaluates to a table index; `call_fn` looks it up; `pack_fn`/`call_packed` arithmetic mirrored exactly (the same `<< 40`) | |
| `tools/typecheck.py` | **EXTENDED**: `fn` as a type for step 1 (an i64 that came from `&`), type variables for step 3 | |
| `bench/fuzz/gen.py` | **EXTENDED** for step 1 (`&f` of a generated fn, `call_fn` with the right arity) | |
| `selfhost/std/pool.bp`, `smw.bp`, B6's §4 shape | UNCHANGED; they are already the step-2 shape with the fn statically known | |
| `use_expand` (`:6869`) | the shape for step 4's instantiation text | |
| T50 (HISTORY.md:1749, OPEN) | **CLOSED** by step 1 (its DONE-CHECK is `c135`) | |

---

## 4. Constraints as design input

1. **One pass.** `&f` is a constant; `call_fn` is a fixed word sequence; nothing needs the whole
   program. The planning/emission agreement holds because every emitted count is independent of the
   VALUE of the constant (fixed 2-word form) -- the same trick `bl` uses ("a placeholder-target pass
   yields correct sizes", `bebop.bp:6120`).
2. **The register model at a `blr`**: identical to a `bl` -- args in x0..x(k-1), everything else parked
   (`vs_park`), x16 is the scratch the parallel move already owns (`vs_place_args:891` evicts a cycle
   blocker into x16; the `adr`/`add` into x16 happens AFTER the move, so no conflict). x<k> (the fn
   value) is an ARGUMENT register k <= 13 and dies at the call like any argument.
3. **`adr` reach**: +-1 MiB = 262,144 words = the program cap (`cap_exit`); a program at the cap with a
   `call_fn` at its very end is exactly in range. If the cap is ever raised, `adrp`+`add` (2 words) replaces
   `adr`; note it in the ladder arm's comment.
4. **The fn cap 768** is untouched by steps 1-3 (no generated fns). Step 4 generates fns per
   instantiation and is why A16's original fn-budget arithmetic (`ROADMAP.md:104`) matters again then.
5. **Escaping environments are `zeros`** and a `zeros` in a loop is trap-census row 5 (the A6 blueprint's
   own caveat, `A6-aggregates-in-arena-and-frames.md:28`). Step 2's construct exercises a NON-escaping
   env in a loop (array literal, reset per iteration) and an escaping one made once; the LANGUAGE.md
   paragraph for closures says which is which. No GC; the thesis does not want one.
6. **Bootstrap**: two-stage. The compiler uses none of this in the landing commits; generation N+1
   candidates: the 38-arm ladder as a hash-indexed table of `&emit_*` values -- a K5 and `bin_words`
   measurement, not a mandate (a linear `if` chain over 38 hashes is ~114 words and ~40 compares per
   call; a table is one hash-mod and one `blr`).

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- `&f` and `call_fn` (one codegen commit)

**Do:** the unary `&` in `emit_factor`; the two-word constant; the `call_fn` ladder arm; the range check
(its trap text through A17's table + step 3's stub commit -- if A17 step 3 has not landed, the check
uses `brk #87`'s existing "unresolved function" text and the construct's EXPECT is `RUNFAIL:87`, upgraded
later); reserved words; census column; `c135`, `c136`, `c137`; bpref; gen.py.
**Expected:** fixpoint; WORD_DELTA 0 on 104/104; `bebop` +150..+250 words (budget line); bcond +3..+5
(allow line); `blr` column present, `bebop` 0; `construct parity: pass=107 fail=0`.
**Kills:** `c135` wrong (§2.3 A) -- disassemble the `adr` and compute the base by hand from the `.bin`
(`objdump -D -b binary -m aarch64`, the WORKER-CARD method); or rc=201 (window state after `blr` differs
from after `bl`: the push of `REG x0` must claim bit 0, `vs_push:2784`).
**Effort:** 5-7 days. **Gate-days:** 1-2.

### Step 2 -- explicit-env closures and packing (one codegen commit, small)

**Do:** `pack_fn`, `call_packed` (2 unpack words + step 1's sequence); `c138`; LANGUAGE.md paragraph
(escaping vs non-escaping env, with the trap-80 warning); bpref mirror.
**Expected:** fixpoint; `c138` MATCH; `bebop` +30..+60 words.
**Kills:** `c138`'s stored closure returns the direct call's value only when called from the SAME fn
(then the env index was frame-relative -- it must be an arena cell index, A5 step 1b, and an array
literal's index IS one since A6).
**Effort:** 2-3 days.

### Step 3 -- generic syntax as erasure (one codegen commit, parser only)

**Do:** `[T]` after the fn name skipped in `compile_fn_at`; type variables tolerated by `parse_params`'
reader (A22 step 2) and `collect_fns`' return reader (A21); `typecheck.py` type variables with
unification over one level (no inference across calls: A8 §1 "Out: generics, inference across calls"
is amended to "in, one level"); `c139` + `bench/typecheck_neg/generic_misuse.bp`.
**Expected:** `c139` WORD_DELTA 0 against its twin; `typecheck census: 0 findings` on the corpus and
`>= 1` on the neg file (the oracle-equality gate A8 §7 asks for).
**Kills:** a `std_tests` program with `[` after a fn name for another reason (none: grep `fn \w+\[` = 0).
**Effort:** 3 days.

### Step 4 -- monomorphisation (after A8 step 2; its own gate-days)

**Do:** `mono_expand` in the `use_expand` shape: for each `f[T]` called at concrete types whose codegen
differs (`[u32]` vs `[i64]` params only), emit `f__u32`/`f__i64` text; the instantiation cache; a
diagnostic for polymorphic recursion. **Expected:** the fn count grows by exactly the instantiation count
(a census line), `bin_words` by their bodies (budget lines per construct). **Kills:** an instantiation
whose body is byte-identical to another (then erasure sufficed and the instantiation is waste -- refuse
to emit identical twins).
**Effort:** ~2 weeks, 8-12 fns. **Costed, not scheduled** before A8.

---

## 6. The honest ceiling

- **Reachable now without heap:** every HOF whose callee is a declared fn and every closure whose
  environment lives as long as its frame -- comparators, predicates, reducers, per-row kernels, worker
  bodies. The cost is 5-9 words per indirect call and an `env` array the caller writes by hand.
- **Not reachable without a text pass:** implicit capture. Costed at ~2 weeks and not scheduled. This is
  the row's human-legibility trade: a human writes `|x| x + a`; here they write `[a]` and `call_fn(&g,
  env, x)`. Named, deliberate, reversible when someone pays for the lifting pass.
- **Arity of an indirect call is not checked by the emitter** (the callee is a value). The type census
  (A8) checks it where the value's origin is visible; where it is not, a wrong arity is a wrong VALUE
  caught by the construct, and the range trap catches a wrong INDEX loudly. That is the honest limit of
  a one-pass emitter, stated rather than papered over.
- **Generics buy the checker, not the program**, until cell widths differ. Anyone expecting Rust-style
  monomorphisation to exist on day one is told, in LANGUAGE.md, that `[T]` is `[i64]` and why.
- **What this changes for F7**: nothing. The kernel is CIC by fiat (`ROADMAP.md:104`); this row's
  artifact is first-order and monomorphic by construction (a `blr` to a known-arity fn with an i64 env),
  which is what F5's translation validation reasons about -- the study's conclusion survives with the
  `apply_T` chain replaced by a `blr` the validator models as an indirect jump to a fn-table entry.

---

## 7. VERDICT format for an A16 worker

```
VERDICT: GREEN|RED
step: <1-4>
fixpoint: gen3 == gen4 <md5>   bebop words <before> -> <after> (budget line yes|no)
census: bcond <b> -> <a> (allow yes|no)   blr column present <yes|no>, bebop blr = <0>
constructs: pass=<n> fail=<n>; WORD_DELTA 0 on 104/104 pre-existing; c135 <v> == hand <v>; c136 <v> (reverse <v'>); c137 RUNFAIL:<trap>; c138 <v>; c139 WORD_DELTA 0 vs twin
adr base check: `adr` at word <n>, computed base <hex>, footer base <hex> (equal)
mutation: x15 save dropped around blr -> c136 FAIL <yes|no>
typecheck (step 3): corpus 0 findings; generic_misuse >= 1
LANGUAGE.md: A16 paragraph corrected <yes|no>; f8_dt renamed|removed <which>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations, each with the line it deviates from>
```
