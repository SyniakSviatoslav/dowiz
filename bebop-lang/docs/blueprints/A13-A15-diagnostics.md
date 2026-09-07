# A13 + A15 — two compile-time diagnostics in ONE codegen commit

Status: 2026-09-08 SPEC (written by the main session for the worker; ROADMAP rows A13 and A15,
both marked "codegen, diag words only", explicitly to be landed together).

Read `docs/WORKER-CARD.md` first — it is binding (box rules, gates, journal format, traps).
Everything below is the task; do not widen it.

## 0. Why these two are one commit

Both add a `diag_exit` site and its text to `bebop.bp`, both change only *diagnostic* words
(no code is emitted for a program that compiles today and keeps compiling), and both need the
same chain + `--codegen` freeze. Landing them separately doubles a 7-minute chain and two
re-freezes of 70 construct binaries for no isolation benefit: the two sites are in different
functions and cannot interact.

Baseline: `bebop.bin` at fixpoint `7d8262a1` (commit 21e92aa, ROADMAP A14). Start from a clean
tree at that commit.

## 1. A13 — a fn with more than 14 parameters

### What happens today (verified 2026-09-07, docs/TRAPS.md row 8)

`bebop.bp` clamps the parameter count to 14 in **three** places and emits one `brk #8` word
(`em(insns, n, 3558867200)`) into that fn's prologue instead of a diagnostic:

| site (line at 21e92aa) | function | shape |
|---|---|---|
| 4837-4838 | the planning/facts pass | `let pcnt1 = if pcnt0 > 14 then 14 else pcnt0;` + `if pcnt0 > 14 then em(... 3558867200)` |
| 4929-4930 | `compile_fn_at` | `if nparams > 14 then em(... 3558867200)` + `let nb = if nparams > 14 then 14 else nparams;` |
| 5108-5110 | the emission pass | `let nparams = if nparams0 > 14 then 14 else nparams0;` + the same `em` |

So a 15-parameter program **compiles cleanly** and dies silently the first time that fn is
called: no stderr text, no SIGTRAP row, exit 8. That is the worst possible failure mode and the
reason this row exists.

### What to build

1. Reject at **parse time**, before any word is emitted, with a new diagnostic exit code **100**.
   The natural single choke point is `parse_params` (all three sites call it) — put the check
   there if the parameter position is available for `diag_exit(s, <pos>, 100)`; otherwise put it
   at the earliest of the three sites (the planning pass) so the emission pass is never reached.
   Whichever you choose, **all three `em(insns, n, 3558867200)` sites and their clamps must go**:
   with the diag in place they are dead code, and leaving them keeps the silent-death path alive.
2. Add the text to `diag_exit` (bebop.bp:75-…): a line of the form
   `let _ = if code == 100 then diag_text(buf, at, [<ascii>], <len>) else 0;`
   Text: `fn with more than 14 parameters` (31 chars). Derive the ASCII array mechanically:
   `python3 -c "t='fn with more than 14 parameters'; print([ord(c) for c in t], len(t))"`.
   No name interpolation — `diag_exit` already prints `line:col`, which identifies the fn.
3. Same commit, second half of the A13 row: the **fn-table-full** diagnostic currently reuses
   exit 89 and therefore prints `let binding while a call temp is live`, which is a lie.
   Find it with `grep -n "diag_exit([^)]*, 89)" bebop.bp` — it is the site in
   `compile` / `compile_fn` / `compile_program_offs` guarding the `zeros(512)` fnames/fpos/
   sizes/starts arrays (NOT `check_reg_collision`'s, NOT `compile_fn_at`'s `s0 + tsp0 > 256`,
   and NOT the two `sys_exit(89)` at bebop.bp:805 and :2238 — leave all of those alone).
   Give it code **104** with the text `too many fns (cap 512)` (22 chars).
   Consequence to state in the journal: the fuzz category `UNSUPPORTED-89` is unaffected —
   the generator never emits 512 fns.

### Gates for A13

- New construct `bench/parity_constructs/neg/c85_param15.bp`: a 4-line program with a
  15-parameter fn (probes exist in the session-24 scratchpad `argcap/p14-16.bp`; if that
  scratchpad is gone, write one — a fn `f(a1..a15)` returning `a15` and a `main` calling it).
  Register it in `bench/vs_rust/construct_parity.sh` in the **neg** table (the second `case`,
  near `c39_fnmatch`) as `c85_param15) EXPECT=COMPILEFAIL:100;;`.
- A 14-parameter fn must still compile and run (that is the boundary): add it to the same file
  or verify by hand and record the run in the journal GOT.
- `docs/TRAPS.md`: retire row 8 (rewrite it as "was: silent brk #8; since <sha> a compile-time
  exit 100") and add rows 100 and 104 in numeric order.
- `python3 tools/bpref.py` must reject a 15-parameter program the same way if it models it at
  all; if bpref has no parameter limit, say so in the journal instead of inventing one.

## 2. A15 — an unbound symbol

### What happens today (verified 2026-09-07, round-2 shrinker find)

`let _ = v0 = 0 in 0` with `v0` never declared **compiles on every bebop.bin**: the pre-A1
compiler prints 0, `05aa91ac` prints nothing / traps `brk #0` inside `emit_let_chain`, and
`tools/bpref.py` answers 0. All three are wrong: an assignment to an undeclared name is a
program error. Repro: session-24 scratchpad `div3/sh/146987.bp`; if it is gone, the two-line
program above is the whole repro.

### What to build

1. In `bebop.bp`, at the **symbol lookup** in `emit_let_chain`, `emit_compound_stmt` and
   `emit_ident` (the three places that resolve a name to a window/slot entry), when the lookup
   fails: `diag_exit(s, <name position>, 101)`. Today at least one of them falls through to a
   `brk #0` or to emitting a read of an unallocated slot — that fall-through is what you are
   replacing. Do not change how a *declared* symbol resolves; a wrong turn here shows up as a
   fixpoint failure, not a test failure, so check `gen3 == gen4` before anything else.
2. `diag_exit` text for code **101**: `unbound symbol` (14 chars), same mechanical derivation.
3. `tools/bpref.py`: raise the same error (a Python exception whose message contains
   `unbound symbol` and which makes the harness classify the program as COMPILEFAIL:101).
   Match the file's existing error convention — do not invent a second one.
4. `tools/gen.py` (the fuzz generator): it must never emit a write to an undeclared name.
   Find where it picks an assignment target and restrict it to names already bound in scope.
   This is what keeps the whole fuzz corpus valid after this change.

### Gates for A15

- New construct `bench/parity_constructs/neg/c93_unbound.bp` = the two-line repro, registered
  as `c93_unbound) EXPECT=COMPILEFAIL:101;;` in the neg table.
- Every existing construct and std test must still compile: a false positive here (a legal
  program rejected) is the main risk of this row, and `construct parity 70/70` plus
  `std_golden` is what proves its absence.
- `docs/TRAPS.md` row 101.

## 3. Chain, gates, acceptance (both rows)

Run ONE chain for both changes, at the end, in the foreground:

```
OUT=/root/.cache/bebop/a13a15; mkdir -p $OUT
SERIAL=1 PROC_CAP=30 BEBOP_TMP=$OUT bash tools/chain.sh bebop.bp $OUT --codegen
```

Acceptance (all of it, no partial GREEN):

- `chain: fixpoint gen3 == gen4 <md5>`;
- `construct parity: pass=72 fail=0` (70 today + c85_param15 + c93_unbound);
- `diag: 17 pass` becomes 19 if the diag lane picks the new codes up — check `bench/diag/`
  and add `d18_param15` / `d19_unbound` there if that lane is the one that owns diag texts;
- `invariants: GREEN`, `words: PASS`, `ABI ok`, oracles `mismatch=0`;
- census: `bcond` may only increase with a `bench/vs_rust/census_allow.txt` line naming this
  row; `bin_words` growth needs a `bench/parity_constructs/word_budget.txt` line (expect
  +40…80 words for two texts + three checks — that is normal for a diag row);
- **known pre-existing RED, not yours:** `std_golden` gate `store` traps 82 identically on the
  old committed bebop.bin (box restored 2026-09-07). Report it, do not chase it.

Promote with `cp $OUT/gen4.bin bebop.bin.tmp && mv bebop.bin.tmp bebop.bin`, then
`bench/vs_rust/invariants.sh --freeze`, then append ONE journal line to `docs/exp.journal`
(`<epoch> H:… | DID:… | GOT:… | VERDICT:…`) and STOP. **Do not commit** — the main session
commits.

## 4. Traps specific to this task

- The three parameter sites are in three different passes; the planning and emission passes
  must agree on the *fn word count*, so removing an `em` from only one of them changes the
  layout and breaks the fixpoint. Remove all three, or none.
- `diag_text` takes an explicit length — an off-by-one there prints garbage after the message
  and is invisible until someone reads stderr. Count with Python, not by hand.
- Adding a `diag_exit` inside a hot lookup (A15) costs words in every fn that reads a symbol;
  if `bin_words` grows more than ~100 words, you have put the check in the emitted code path
  instead of the compiler's own control flow — re-read where the lookup returns -1.
- `bebop.bp` must stay under 511 fns (`grep -c '^fn ' bebop.bp`).
