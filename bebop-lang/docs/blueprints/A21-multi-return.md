Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row A21 (new). Delivers
language item 1 (tuples / multiple return values) as a REGISTER CALLING CONVENTION, not a value.

# A21 -- multiple return values in x0..x2, destructured by `let (a, b) =`

## 0. Three corrections this blueprint rests on

**(0a) The pack idiom is not one file's habit; it is the language's only way to return two things.**
`(p << 32) | node` and `& 0xffffffff` in `c70_qdsl.bp` (`:42,46,61,65,101,114,117,152` / `:77,98,109,
118,145,...`) are 15 + 14 sites in one file; tree-wide, `<< 32` packing occurs at **75** sites in 12
selfhost files and the unpack mask at **68**. The compiler itself does it: `emit_call_resolve` returns
`pb * 2 + okres` (`bebop.bp:5723`), `emit_body` returns `n[0] * 2 + lastf` (`:5612`), and
`compile_fn_at_facts` packs FIVE facts into one word decoded by `fw_vc/fw_alloc/fw_cshi/fw_tsp/fw_marks`
(`:5955-5959`). Every one of these halves the range of both values and writes the unpack at every
consumer.

**(0b) The register model already delivers k operands to x0..x(k-1) for every builtin.** `vs_deliver`
(`:935-951`) parks everything below the top `nops` entries and places them by the same parallel move a
call uses (`vs_place_args:873`). `emit_return_stmt:4713` calls it with 1. Calling it with 2 is the callee
side of this row, and it is zero new machinery.

**(0c) After a `bl`, x1..x7 are free and dead, and nothing restores them.** `emit_bl_call:855-857`:
"x1..x7 are free (parked); x0 holds the result", then `vs_push(kind 2, x0)`. `emit_bl` (`:628-651`)
saves/restores only x15. `emit_epilogue_sized` (`:4551-4560`) restores x19..x26, sp, fp, lr. So a value
the callee leaves in x1 survives to the caller's next instruction, and pushing `REG x1` is legal in the
model at that point. The convention costs **0 words** on the caller side.

---

## 1. Scope

### A21 IS

1. **`return (e0, e1[, e2])` and a tail expression `(e0, e1[, e2])`** deliver k <= 3 values to x0..x(k-1)
   through `vs_deliver(k)`. k = 1 is today's `return e`; `(e)` stays a parenthesised expression
   (`emit_paren:3657`), a `,` after the first expression is what makes it a tuple, and only in return/tail
   position -- elsewhere `emit_paren`'s exit 95 stands.
2. **`let (a, b[, c]) = f(args);`** binds k symbols from x0..x(k-1): after `emit_bl_call` pushes `REG x0`,
   the destructuring site pushes `REG x1` (and `REG x2`), then binds top-down through `vs_bind` exactly
   as a `let` does. The same form works after `call_fn` (A16) since it ends in the same push.
3. **Arity recorded and checked at zero words.** `collect_fns` (`:5870`) runs before any fn is compiled;
   it reads the header's `-> (` and counts commas into a per-fn arity cell (a 768-cell zone, e.g.
   `fntab[7000+i]`, verify free with `check_abi.py --fntab`). `let (a, b) = f()` compares its pattern
   width with `f`'s recorded arity and `diag_exit`s on mismatch; `return (a, b)` in a fn declared `-> i64`
   likewise. The check exists so that a silent x1 read cannot happen.
4. **bpref, typecheck, gen.py, Lean** each get the form: a `('tuple', [..])` node returned as a Python
   tuple, `('lettuple', names, call)`; typecheck's return type `(i64, i64)`; gen.py emits `let (a, b) =`
   at ~5 % of calls to a tuple-returning fn; one Lean rule.
5. **Two constructs**: `c129_multiret` (positive, hand-derived) and `neg/c130_multiret_arity`
   (COMPILEFAIL).

### A21 IS NOT

1. **Not a first-class tuple value.** A tuple cannot be stored, passed, bound to one name, or nested.
   `let t = (a, b)` is exit 95 as today. A stored pair is a two-field struct (A22) or two cells.
2. **Not k > 3.** x0..x2 covers every pack idiom found (all are pairs; the compiler's five-fact word is
   the outlier and is a struct's job). Above 3 is a diagnostic, not a register-pressure surprise.
3. **Not a change to the argument convention** (14 params, x8..x13 for 9-14: `parse_params:297`).
4. **Not a change to `emit_bl`'s x15 save/restore** or to the epilogue.
5. **Not variadic or optional returns**: a fn returns exactly its declared arity on every path, and the
   arity check is what makes that true.

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/c129_multiret.bp` | positive; `divmod(n, d) -> (i64, i64)` returning `(n / d, n % d)` via `return` inside a loop and via a tail; a `minmax(a, b)` tail tuple; a 3-tuple `split3`; each used in a loop of 200 values |
| `bench/parity_constructs/neg/c130_multiret_arity.bp` | `let (a, b) = g(1)` where `g -> i64`; `EXPECT=COMPILEFAIL:<code>` |
| `bench/vs_rust/construct_parity.sh` | two rows |
| `bench/diag_neg/d12_arity.bp` | position-gated twin of c130 |

### 2.2 The number

`c129` EXPECT is the sum over n in 1..200 of `q * 1000 + r` for `(q, r) = divmod(n, 7)`, plus `min *
1000000 + max` over the pair `(17, 4)`, plus the 3-tuple's `a + 2b + 3c` for `split3(123456)` = digits
by hundreds. **Derived by hand in the header**: the divmod sum is `1000 * sum(n // 7) + sum(n % 7)` with
`sum(n // 7) = 7 * (0+1+...+27) + 28 * 4 ... ` -- the worker writes the arithmetic out, and it must NOT be
the same expression the program computes (the program divides; the derivation counts residue classes).

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c129` MATCH at the hand value | x1 is clobbered between the callee's tail and the caller's bind -- by a spill reload of the caller's own symbol (`vs_bind` of a spilled symbol emits `str x0,[x15..]` only, `:2937`; but the `has_spills` `ldr x15,[sp],#16` at `emit_bl:648` writes x15, not x1: fine) or by the T43 loop reset word at a back-edge (`ldr x27`, fine). The first real risk is `vs_place_args` for the SECOND push: `vs_push(kind 2, x1)` takes bit 1 of the free mask (`vs_push:2784`) -- if the mask still shows x1 owned by a stale arg entry, `vs_mask_take` self-checks (rc 201) |
| B | `c130` COMPILEFAIL:<code>, no `.bin`; `d12` at the hand-counted position | arity not recorded (a `-> i64` fn destructured to 2 compiles and reads garbage) |
| C | WORD_DELTA 0 on `c08_call`, `c23_spillcall`, `c53_param9`, `c58_callmix`, `c61_arrcall` | the call path changed for single-value calls |
| D | `c129` re-run with `divmod` REORDERED after `main` in the file | `collect_fns` did not record arity order-independently (it must: it scans the whole source before any fn compiles) |
| E | a mutant that pushes `REG x0` twice instead of `x0, x1` makes `c129` FAIL | the construct's second component equals its first somewhere -- it must not (divmod with d = 7 guarantees q != r for n >= 8) |
| F | fuzz 300 seeds, `DIVERGE=0`, tuple calls present | bpref's tuple evaluation order differs from the emitted (`vs_deliver` evaluates left to right: `emit_return_stmt` parses e0 then e1; bpref must too) |

---

## 3. Which existing work A21 sits on

| existing | verdict | reason |
|---|---|---|
| `vs_deliver` (`:935`), `vs_place_args` (`:873`) | **REUSED unchanged** | k-operand delivery exists |
| `emit_return_stmt` (`:4708`) | **EXTENDED**: after `emit_cmp`, if the next char is `,` (inside parens), parse up to 2 more, deliver k | |
| `compile_fn_at` tail (`:6085-6086`, `vs_to(x0)` + `vs_pop`) | **EXTENDED**: a tuple tail delivers k via `vs_deliver(k)` instead | the tail path is one place |
| `emit_paren` (`:3657-3665`) | **EXTENDED** with a tuple branch gated by a "tail/return position" flag in a fntab cell (set by `emit_return_stmt` and the tail site, cleared after) | elsewhere `,` stays exit 95 |
| `emit_bl_call` (`:807-871`) | KEPT; the second/third push happens at the DESTRUCTURING site, not here | single-value calls stay byte-identical |
| `emit_let_stmt` (`:5313`) | **EXTENDED**: `let (` -> read k names, `sym_bind` each, parse the call, push `REG x1..`, bind top-down | the F3 length side channel (`:5324`) is written per name |
| `collect_fns` (`:5870-5922`) | **EXTENDED**: after `read_ident`, skip the param list (`skip_to_delim` shape) and read `-> (` commas | order-independent by construction |
| `tools/bpref.py` `program()` (`:166-246`, reads `-> [`/`ref`/ident) | **EXTENDED** for `-> (` | |
| `tools/typecheck.py` BUILTIN/sigs | **EXTENDED** | |
| `bench/fuzz/gen.py` | **EXTENDED** | |
| the compiler's own packs (`:5722`, `:5612`, `:5955-5959`) | UNTOUCHED in this row; candidates for generation N+1 with a `bin_words` measurement | two-stage bootstrap |

---

## 4. Constraints as design input

1. **One pass, planning/emission agreement.** The destructuring site emits words that depend only on
   k (from the pattern) and on the symbols' locations (register or slot) -- both known in both passes.
   The arity CHECK reads `collect_fns`'s zone, filled before either pass.
2. **The window after a `bl`**: x0 is `REG` (owned); pushing `REG x1` must find bit 1 free. It is:
   every arg entry was popped at `:851-854` and `vs_park` moved everything else to cs/slots. If a
   builtin ever leaves a `MULC` on x1 -- none does after a call.
3. **`vs_settle_flags` on push** (`vs_push:2753`): a FLAGS entry cannot be on top after a `bl` (the call
   consumed everything). Fine.
4. **The `.use` positions and `diag_exit`** for the arity error: the position is the `let (` site.
5. **14-parameter cap** and the return convention do not interact: returns use x0..x2, args x0..x13 --
   the SAME registers, at different times (args are dead after the `bl`).
6. **Bootstrap:** two-stage. `bebop.bp` must not use `let (a, b) =` or a tuple tail in the landing
   commit.

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- arity table (no words; one commit with step 2 or alone as byte-identical)

**Do:** `collect_fns` records arity; a `diag_exit` site in `emit_let_stmt` for `let (`.
**Expected:** gen2 == gen3 == gen4 (byte-identical: no program uses the form) or gen3 == gen4 with a
`bebop` budget line for the new parser words. `c130` COMPILEFAIL.
**Kills:** a fn whose header spans a line break or has a comment between `)` and `{` is mis-read --
`collect_fns` must use the same `skip_ws`/`skip_line_comment_n` discipline as `compile_fn_at:6041`.

### Step 2 -- the forms (one codegen commit)

**Do:** `emit_return_stmt`, the tail, `emit_paren`'s gated branch, the destructuring `let`; bpref;
`c129`; re-freeze.
**Expected:** fixpoint; `construct parity: pass=106 fail=0`; WORD_DELTA 0 on all pre-existing
constructs; `bebop` +150..+250 words, one budget line; bcond +2..+4 with an allow line.
**Kills:** any pre-existing construct WORD_DELTA != 0; `c129` at a value other than the hand value; rc
201 on `c129`.
**Effort:** 3-4 days. **Gate-days:** 1.

### Step 3 -- fuzz (foreground, 300 seeds)

**Expected:** `DIVERGE=0`, tuple calls in >= 5 % of seeds. **Kills:** a DIVERGE -> shrink, freeze, fix.

---

## 6. The honest ceiling

- **Zero-word on the caller side is real only when the destructured names are register symbols.** A
  fn with > 8 symbols binds the tuple's components into spill slots: `str x1,[x15,#..]` -- one word each,
  the same as a scalar `let`. No regression, no gain.
- **A tuple-returning fn called in EXPRESSION position** (`g(divmod(n, 7))`) yields x0 only; the
  second value is silently dropped. Agent-first says refuse it: `diag_exit` when a fn of arity > 1 is
  called outside a destructuring `let` or a `return`. Included in step 1's check (it is the same arity
  table). Cost: one more `diag_exit` in `emit_bl_call`.
- **What this does not give:** a tuple in a store cell, a tuple as an argument, a nested tuple. Those
  are structs; A22 step 1 makes a two-field struct's offsets nameable, and a pair stored in a cell stays
  a pack for values that fit 32 bits -- with the `<< 32` idiom then confined to storage, where its range
  limit is a documented layout choice rather than a calling-convention accident.

---

## 7. VERDICT format for an A21 worker

```
VERDICT: GREEN|RED
step: <1-3>
fixpoint: gen3 == gen4 <md5>   bebop words <before> -> <after> (budget line yes|no)
constructs: pass=<n> fail=<n>; WORD_DELTA 0 on 104/104 pre-existing; c129 <value> == hand <value>; c130 COMPILEFAIL:<code>
diag: d12 at <line:col>
order test: divmod after main -> c129 <value> (must equal)
mutation: double-x0 push -> c129 FAIL <yes|no>
fuzz: N=300 DIVERGE=<n> tuple share <pct>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations>
```
