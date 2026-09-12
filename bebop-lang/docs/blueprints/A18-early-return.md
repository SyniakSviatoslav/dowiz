Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Numbers are quoted
with `path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row A18 (new). Delivers
language item 3 (early return / real control flow) and the agent-framing lint against the no-op
conditional.

# A18 -- `return` and `break` in expression position, and the no-op `if` refused

## 0. Three corrections this blueprint rests on

**(0a) `return` and `break` are not missing; they are STATEMENT-ONLY, and that is the whole defect.**
T99 landed them on 2026-09-04 (`HISTORY.md` T99: "`return e;` = e in x0 + one `b` patched to the fn
epilogue"). They are recognised by `emit_body_classify` (`bebop.bp:5481-5484`) at the START of a body
item and dispatched by `emit_body:5518-5521`. `if` is an EXPRESSION (`LANGUAGE.md:75`, "only the taken arm
runs"); its arms are parsed by `emit_cmp` inside `emit_cond_branch:3803` and `:3831` and delivered into
one register `d`. A `return` inside an arm therefore reaches `emit_factor:4344-4355`, which knows the
keywords `if`, `let`, `match` and nothing else, falls to `emit_ident`, and exits **101 unbound symbol**
(`emit_var_or_ctor:1884`). There is no way to say "leave now if `ok == 0`".

**(0b) What the author wrote instead is measured, and it shipped a defect.** `let _ = if ok == 0 then
0 else 0;` occurs 15 times in `bench/parity_constructs/c70_qdsl.bp:190-303`, 24 times in
`selfhost/std/qdsl.bp`, 70 times tree-wide (4 files). It computes the flag and discards it, so
`qdsl_parse("invalid query that should fail")` returns a tree and `c70_qdsl_neg` is frozen at **1**
(`construct_parity.sh:158`) against its own header's 89 (`c70_qdsl_neg.bp:1`). A second author hit the
same wall and chose a different workaround: `selfhost/tcheck_kernel.bp:13-15` -- "`return` is a STATEMENT
ONLY ... which is why `return` appears in ZERO selfhost sources ... every branch is branchless error
propagation with uid 0 as an absorbing sentinel". (Checked: one file under `selfhost/` + `std_tests/` has
a line-initial `return`; the claim is nearly exact.)

**(0c) The machinery an arm-position `return` needs already exists.** `emit_cond_branch` snapshots the
window count, free mask, cs mask and slot cursor before the then-arm (`:3786-3789`) and RESTORES them
before the else-arm (`:3813-3817`), so an arm that never falls through leaves nothing the other arm
depends on. `emit_return_stmt:4708-4723` already delivers the value to x0 (`vs_deliver(1)`), records a
`b` placeholder in the per-fn list `fntab[5247..]` and `compile_fn_at:6089` patches every placeholder to
the epilogue. The `ret` restores callee-saved registers, so cs temps parked around the `if` need no
release on the early path. `break` is symmetric on `fntab[5265..]` and `emit_while_stmt`'s exit patch.

---

## 1. Scope

### A18 IS

1. **`return e` and `break` as expressions of type "never"**, legal wherever an expression is:
   `if ok == 0 then return 0 else 0`, `let x = if c then return 7 else f(y)`, a `match` arm, a `let ...
   in` body. The emitted form is exactly today's statement form (deliver + one `b`) followed by one
   dummy operand so the enclosing expression's plumbing (the arm's `vs_to(d)`) has something to consume.
2. **A lint that REFUSES the no-op shape**: a discarded `if` whose two arms are both integer literals
   (`let _ = if c then K1 else K2;` and the bare item `if c then K1 else K2;`) is a compile-time
   diagnostic with a free code below 99 and the text `conditional with constant arms and no effect --
   did you mean return?`. It is the exact shape that shipped the accepting parser.
3. **The pending-jump caps raised** from 17 `return` / 18 `break` per fn (`emit_return_stmt:4715`,
   `emit_break_stmt:4729`, exit 98) to 64 each, because arm-position returns multiply them; the lists move
   from the 38-cell "jumps" zone (`check_abi.py:206`, `fntab[5247..5284]`) to a 130-cell window in the
   free 5601+ zone (verify with `check_abi.py --fntab`).
4. **One positive construct and one `neg/` construct**, EXPECTs derived by hand (§2), and `bench/fuzz/
   gen.py` taught to emit `return`/`break` inside arms (today it avoids them entirely, `gen.py:10`).
5. **The B7 unblock stated as a consequence, not a gate**: after this row `qdsl_query` can be rewritten
   with real early exits and `c70_qdsl_neg` re-derived to 89 -- that number is B7's, and it goes GREEN
   only when B7's worker rewrites the parser.

### A18 IS NOT

1. **Not statement-level `if c { ... }` without `else`.** A block-arm parser is ~200 lines and a second
   syntax for `if`; `if c then return x else 0` is explicit and an agent writes it. Costed, not
   scheduled.
2. **Not `else`-less expression `if`.** The mandatory `else` (`emit_cond_branch:3811-3812` skips 4 chars
   for `else`) stays; an `if` that yields a value must yield one on both arms.
3. **Not `continue`.** Not asked for; a `while` body's tail already discards its value, and `break` plus a
   flag covers it. Costed at the same ~40 lines as `break` if ever wanted.
4. **Not a rewrite of `qdsl.bp`.** That is B7 step 1's repair. This row provides the construct it needs.
5. **Not a change to `return`'s value semantics.** `return e` inside a `while` inside an `if` still
   patches to the fn epilogue; `break` still targets the innermost `while`; T43's `x27` reset at the loop
   exit still runs (`c36_break` proves it for the statement form and is re-frozen unchanged).

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/c124_condreturn.bp` | positive construct; `EXPECT` hand-derived (§2.2) |
| `bench/parity_constructs/neg/c125_noopif.bp` | `EXPECT=COMPILEFAIL:<code>`; the `c70` idiom in four lines |
| `bench/vs_rust/construct_parity.sh` | two `case` rows (or, after A24, two headers) |
| `bench/diag_neg/d11_noopif.bp` | the same program with its `// EXPECT line:col code` header, so the lint's POSITION is gated too |
| `bench/fuzz/gen.py` | emits `return`/`break` in arm position; the fuzzer's own summary line then counts them |

### 2.2 The number

`c124_condreturn` computes, for `n` in 0..99, `first_div(n)` = the first `k` in 2..n-1 dividing `n`
found by a loop that RETURNS from inside an `if` arm inside the `while`, or `n` itself when none exists
(1 for n < 2 by an arm-position return before the loop); plus a `break`-in-arm loop that sums `i` until
the first `i` with `i * i > 50` and breaks from the then-arm. EXPECT = the sum over n of `first_div(n)`
times 1000 plus the break sum, derived BY HAND from the prime table (the smallest prime factor of each n
< 100 is a table anyone can check; the break sum is 0+1+...+7 = 28) -- not from bpref and not from a
closed form the program could compute. The header states the derivation.

`c125_noopif`:

```
fn f(ok: i64) -> i64 {
  let _ = if ok == 0 then 0 else 0;
  ok
}
fn main() -> i64 { f(1) }
```

`EXPECT=COMPILEFAIL:<code>`, code from the free set (66-79; 65 is F3's).

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c124` MATCH at the hand value | the arm-position `return` falls through (value = the else arm), the `b` lands on the wrong epilogue offset (a second fn's), or `break` leaves the OUTER loop |
| B | `c124` compiles with `selfcheck_exit` silent (rc != 201) | the dummy operand is not pushed (window count off by one at the item boundary -> self-check 8) or the mask is left dirty after the never-returning arm (self-check 5/7) |
| C | `c125` COMPILEFAIL:<code> with no `.bin`, `d11` at the hand-counted position | the lint misses the shape or fires at the wrong position |
| D | `c05_if`, `c35_return`, `c36_break`, `c70_csel`, `c71_csel_impure` WORD_DELTA 0 | the change altered the statement forms or the csel path (`emit_cond_csel:3737` must not see `return` as a pure arm -- `arm_is_pure:4916` must return 0 for it) |
| E | a mutant that drops the `b` word in the expression path makes `c124` fail | the construct is not sensitive to the branch (it would be if `first_div` had no composite input) |
| F | fuzz batch of 300 seeds on the promoted binary: `DIVERGE=0`, `return`/`break` present in >= 10 % of programs | gen.py did not learn the shape, or bpref and bebop disagree on a corner (an arm-position `return` inside `let ... in`) |

---

## 3. Which existing work A18 sits on

| existing | verdict | reason |
|---|---|---|
| `emit_return_stmt` (`bebop.bp:4708-4723`), `emit_break_stmt` (`:4725-4737`), `patch_jumps` (`:4741-4755`) | **KEPT, split**: the deliver+placeholder part becomes `emit_return_core`/`emit_break_core` called from both the statement site and the new expression site | the `;` consumption at `:4720-4721` is statement-only |
| `emit_factor` keyword dispatch (`:4331-4349`) | **EXTENDED** with `return`/`break` recognised the way `match` is (`:4326-4331`, five chars + a non-ident follower, the same test `emit_body_classify:5481-5484` uses) | the one place expression-position keywords enter |
| `emit_cond_branch` state save/restore (`:3786-3789`, `:3813-3817`) | KEPT untouched | it already isolates the arms |
| `emit_cond_csel` / `arm_is_pure` (`:3737`, `:4916-4961`) | **EXTENDED**: an arm containing `return`/`break` is impure (`kw_bad_at:4864` gets two more words) | csel cannot express a branch away |
| `emit_let_stmt` discard path (`:5405`, `is_discard`) | **EXTENDED** with the lint: when `is_discard` and the RHS text starts with `if` and both arms are literals (a text scan in the `arm_end` style, `:4832`) -> `diag_exit(code)` | the no-op shape |
| `emit_body` expression-item path (`:5555`) | same lint for the bare item form | |
| `d09_returns` (`bench/diag_neg`, exit 98) | KEPT, cap 17 -> 64 means its program must be regenerated at 65 | |
| `c35_return`, `c36_break` | KEPT, re-frozen with WORD_DELTA 0 | statement forms unchanged |
| `selfhost/tcheck_kernel.bp` sentinel style | untouched | a later cleanup, not this row |
| `bench/fuzz/gen.py:10` ("no return / break") | **CHANGED** | otherwise the new path is never fuzzed |

---

## 4. Constraints as design input

1. **Every expression emitter leaves exactly one window entry** (REGISTER-MODEL-BLUEPRINT §1.2). A
   `return` in expression position emits the branch and then pushes `CONST 0` (`vs_push kind 1`, no
   word); the enclosing `vs_to(d)` then materialises one dead `mov xd,#0` after the `b`. 1 dead word
   per use. Acceptable; the alternative (teaching every consumer about "never") is a type.
2. **Planning and emission must agree on word counts** (§4 of the register-model blueprint). The dummy
   is a CONST, so both passes emit the same words; the `b` is one word in both.
3. **`b` reach**: `patch_jumps` writes a 26-bit offset (`335544320 + (target - p)`); a fn is capped at
   65,536 words (`cap_exit`), in range.
4. **The cs mask at the item boundary** (`emit_body:5573-5580` self-checks) sees the state AFTER the
   if-expression joins, which `emit_cond_branch` already restores; the early path never reaches the
   boundary. No change.
5. **`loop_alloc_safe` (`:4632`) and the hoist scan (`hoist_scan:5042`)** scan body TEXT; a `return`
   inside an arm inside a loop body is text they already tolerate at statement level (`c35_return`'s
   `early` fn). `hoist_scan` bails on match/call bodies (A14); it must also bail on a body containing
   `return` in an arm -- or the hoisted pair's re-take after the loop is skipped by the early exit
   (harmless: `ret` restores the registers, and nothing after the loop runs).
6. **Bootstrap:** two-stage. `bebop.bp` must not use arm-position `return` in the landing commit. From
   the next commit on, the compiler's 316 `let _ = if ... then ... else 0;` conditional-effect lines are
   candidates -- worth a `bin_words` measurement, not a mandate.

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- the expression forms (one codegen commit)

**Do:** split the two statement emitters; add the `emit_factor` arms; make `arm_is_pure` refuse them;
raise the caps and move the lists; write `c124`; re-freeze.
**Expected:** `chain: fixpoint gen3 == gen4`; `construct parity: pass=105 fail=0`; WORD_DELTA 0 on all
104 existing constructs; `bebop` words +100..+160 with one `word_budget` line; census bcond +2..+4 with
an allow line (the two keyword tests in `emit_factor` are impure ifs: the same class A14b recorded).
**Kills the step:** WORD_DELTA != 0 on `c05_if`/`c70_csel` (the csel path changed); rc=201 on `c124`
(window/mask invariant broken by the dummy); `c124` MATCH at a value other than the hand value.
**Effort:** 2 days. **Gate-days:** 1.

### Step 2 -- the lint (same commit or the next; diag words only)

**Do:** the `is_discard`+literal-arms scan in `emit_let_stmt` and `emit_body`; text via A17's table;
`c125`, `d11`.
**Expected:** `construct parity: pass=106 fail=0`, `diag: 17 pass`. Every existing construct and every
`std_tests/*.bp` still compiles -- **false positives are the risk**: run `bench/vs_rust/std_golden.sh`
in full; the 70 tree-wide occurrences (`selfhost/std/qdsl.bp`, `selfhost/compile.bp`, both `c70` files)
become COMPILE ERRORS by design and must be rewritten in the same commit (B7's repair for `qdsl.bp`
lands here or B7's row accepts the red).
**Kills the step:** a `std_tests` program with a legitimate constant-arm discarded `if` (none found by
grep today; if one exists, it is rewritten, not exempted).
**Effort:** 1 day.

### Step 3 -- fuzz the shape

**Do:** `gen.py` emits `return`/`break` in arm position at ~10 % of `if`s inside fn bodies (not inside
`let ... in` chains that bpref evaluates lazily -- see §6); 300 seeds foreground.
**Expected:** `DIVERGE=0`.
**Kills the step:** a DIVERGE -- shrink it, freeze it as a construct, and it is a compiler defect until
proven an oracle defect (the 2026-09-12 precedent cuts both ways).

---

## 6. The honest ceiling

- **bpref parity for `return` in expression position** is the risk: bpref implements `return` as a
  Python exception (`ReturnSignal`, `bpref.py:57`) raised from the statement runner (`run_body:466`).
  Raising it from inside `ev()` works for `if` arms (Python unwinds), but a `return` inside a `let x = ...
  in` RHS that bpref evaluates before binding must unwind identically -- it does, by construction; the
  fuzzer decides.
- **The dead `mov xd,#0` per use** is 1 word; the A2b peepholes do not remove it (they fold
  arithmetic, not dead stores). ~15 uses in `qdsl.bp` = 15 words. Not worth a mechanism.
- **What stays impossible to express without a block form:** a sequence of statements under a
  condition without an `else` arm. Today's answer is `let _ = if c then (let _ = a in let _ = b in 0)
  else 0;` -- verbose, unambiguous, and an agent writes it. A human wants `if c { a; b }`; costed at ~200
  lines, not scheduled, and it is the one place this row trades human legibility.

---

## 7. VERDICT format for an A18 worker

```
VERDICT: GREEN|RED
step: <1-3>
fixpoint: gen3 == gen4 <md5>   bebop words <before> -> <after> (+<n>, budget line yes|no)
constructs: pass=<n> fail=<n>; WORD_DELTA 0 on 104/104 pre-existing; c124 <value> == hand <value>; c125 COMPILEFAIL:<code>
diag: <pass> pass <fail> fail (d11 at <line:col>)
census: bcond <before> -> <after> (allow line yes|no)
fuzz: N=300 DIVERGE=<n> arm-return share <pct>
mutation: dropped-`b` mutant -> c124 FAIL <yes|no>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations, each with the line it deviates from>
```
