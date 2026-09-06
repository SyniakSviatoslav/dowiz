Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 (Phase A/B roadmap) with the A1 register-model worker tree (bebop.bp uncommitted, 250 fns); depends on A1 (register model landed: FLAGS/REG tags, vs_* primitives, emit_cond as in REGISTER-MODEL-BLUEPRINT §3.5)

# A2 `csel` for pure `if` arms, then (optional) hoisting of 64-bit loop-invariant constants

## 0. Goal

Remove the branch from every `if c then a else b` whose arms are pure expressions, emitting `csel x<d>,x<ta>,x<tb>,<cond>` instead of `b.<inv> / b / join`; gate **K8H <= 1.2x Rust** (bench/vs_rust/honest.sh row `k8h`, today 4.5x; K8 control showed the mispredict is ~55 % of K8). If K8H is still > 1.2x after commit 1, commit 2 hoists the two 64-bit LCG constants (2 x movz+3 movk = 8 words/iter, RESEARCH-DEPS §5.2) out of the loop body.

## 1. Scope

In: `emit_cond` (bebop.bp:2805, verified) gets a pre-scan + csel path; a text scanner `arm_is_pure(s, pos)` in the style of `skip_while_cond` (bebop.bp:3561, verified); the FLAGS tag is consumed by `csel` (cond field) exactly like `b.<cond>`; constructs; PERF row `k8h_loopwords`. Commit 2 (conditional): `emit_while_stmt` (bebop.bp:3593) pre-scans the body text for integer literals >= 2^16, materialises them once before `pjump` into free callee-saved registers, and the body's `CONST` tags for those values become `CS r` tags.
Out: any `if` with a call, a `let`, a `while`, a nested `if`, an array literal, a string literal, a struct/enum ctor in either arm (all stay on the branch path); `match`; short-circuit `&&`/`||` (T125, still undefined). Fixed points: emit_cond's branch path words for impure arms stay byte-identical (WORD_DELTA 0 on c05_if, c46_andor); B5's loop shape; bpref semantics (the value is identical, only the taken-arm laziness disappears -- both arms are evaluated, so purity is what makes that unobservable).

## 2. Preconditions

A1 landed and promoted (push_words == 0, c55-c64 frozen); `vs_cmp` pushes `FLAGS cond` with the real AArch64 condition number (REGISTER-MODEL §2); `vs_alloc`/`vs_pop`/`vs_to` and the ownership invariants (§3.14, mask helpers with sites); K8H honest row exists (bench/vs_rust/kernels/k8h.bp, rust_once/k8h.rs, honest.sh:17 `for k in k1h k2h k3h k4 k8h`, verified). The fn count is 250 against the old compiler's cap of 256 (bebop.bp:4465 `if cnt[0] >= 256 then diag_exit(s,0,89)`, verified): **step 0 below raises the cap first, as its own commit.**

## 3. Design

**Step 0 (own commit, byte-identical codegen): fntab relayout, fn cap 512.** The old compiler builds gen2 and enforces 256 fns (bebop.bp:3221, 3237, 4109, 4465 verified; `fnames/fpos/sizes/starts/offs = zeros(256)` at 4457-4462). A2 adds ~4 fns and A3 ~6, so the cap must move before them. New layout, `fntab = zeros(8192)` at every `zeros(4096)` site: fn zones `[cnt][names][offsets][srcpos]` for cnt <= 512 = 1..1537, enum zone + ft_cache (2*128) after it (max ~1900); per-fn facts `2200 + i` (i < 512); own-fn cells 1800-1802 -> 2720-2722; window entries `3000 + 3i` (128 entries, 3000..3383) instead of 2000+3i; everything at 3655+ unchanged (3661, 3662+, 3680+, 3797/3798, 3823-3827, 3890-3892, 3899-3903, 4000); literal table `3903 + i` (cap 193) -> `5000 + i` (cap 1000, `fntab[3899..3902]` headers stay). tools/check_abi.py zone tuples updated (its line ~167: the `(2000,2383,"window")` tuple becomes `(3000,3383,"window")`, add `(2200,2711,"facts")`). Gate: `tools/chain.sh bebop.bp $OUT` WITHOUT `--codegen` must print gen3 == gen4 and the constructs must show 0 WORD_MISMATCH (the emitted words do not depend on table addresses; the only word that changes is `fntab = zeros(8192)`'s own literal inside bebop.bin, so bin_words changes by 0 and the self-compile md5 changes -- that is expected, note it in the journal line). Also the new source keeps <= 255 fns until this commit is promoted (the OLD compiler compiles it).

**Commit 1: csel.** In `emit_cond`, after `emit_cmp` of the condition and BEFORE the branch word is chosen, pre-scan the source text from the current `pos` (just after `then`) to the matching `else` and from `else` to the end of the else-arm (the arm ends where `emit_cmp` would stop: the grammar `cmp` at paren depth 0 -- reuse the scanner discipline of `skip_while_cond`: track `()`/`[]` depth, skip strings; stop at `else` at depth 0 for the then-arm, and for the else-arm at the first `;`, `)`, `,`, `}`, `in`, `then`, `else` at depth 0 or end of input). `arm_is_pure` returns 1 iff the arm text contains none of: an identifier followed by `(` (a call or builtin), the keywords `let`/`while`/`if`/`match`/`return`/`break`, `[` (array literal or index -- index would be fine but keep it out: `ldr` can fault on a not-taken path with a bad index), `"`, `{`, `.` (field access). Both arms pure -> csel path:

```
emit_cmp(cond)                      -- top: FLAGS c (or REG r / CONST)
cond not FLAGS: materialise r, `cmp x<r>,#0` -> FLAGS ne        (cbz would need a branch)
then-arm: emit_cmp -> ta = vs_reg(top)      (may be CONST: materialise; may be SYM: use directly)
else-arm: emit_cmp -> tb = vs_reg(top)      -- the FLAGS entry is now NOT the top: §1.2's rule
                                             "FLAGS only at the top" must be suspended here: emit_cond
                                             holds the cond entry below two arm entries and NOTHING in
                                             the arms may emit a flag-setting word (purity guarantees
                                             it: no cmp, no T118 trap, no call); assert with the
                                             existing FLAGS-materialise path disabled for this window
d = vs_pick_dest (reuse ta or tb when they are window temps, else vs_alloc)
csel x<d>,x<ta>,x<tb>,<c>            -- cond field = the real condition number (as+objdump)
vs_pop x3 (tb, ta, FLAGS); push REG d, n[0]-1 (retargetable: csel rd is bits 0-4)
```
Comparisons inside a pure arm (`if a then (b < c) else 0`) would clobber the flags: exclude `<`, `>`, `==`, `!=` from pure arms (the scanner rejects them). `!x` in an arm also emits `cmp` -> rejected. Word forms (asm text; derive with as+objdump): `csel xd,xn,xm,<cond>`; `cmp xr,#0`. The census gains `csel` -- add a column only if census.py counts it (it does not today; no allow line needed unless b.cond falls, which it will: record the allow line).

**Commit 2 (only if K8H > 1.2x): constant hoisting.** In `emit_while_stmt` before `pjump` (loop entry, executed once): scan the body text [b0, b1) for integer literals with value >= 65536 or < 0 (a `-` followed by digits) at paren depth 0 or any depth, dedupe, take up to `26 - 19 - stab[0] - cs_in_use` free callee-saved registers (cs mask 3823 must be 0 here: statement context, verified by the §3.14 invariant), materialise each with movz/movk into x<cs>, mark the cs bit taken for the whole loop (a "loop cs temp": released at `endl`), and record (value, register) pairs in fntab cells `3828 + 2k` (k < 4, cells 3828-3835 are free -- verify with `grep -o 'fntab\[38[2-9][0-9]' bebop.bp`). `emit_lit`/`vs_push` of a CONST whose value matches a live pair pushes `CS r` instead of `CONST c` -- but only inside the loop body (the pairs are cleared at `endl`). Prologue pairs: `cs_hi` fact must include these registers (`vs_cs_take` already updates 3825). The planning pass and the emission pass see the same text, so the decision is identical in both (§4 agreement).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp: every `zeros(4096)` for fntab, `fntab[1500 + i]`, `fntab[1800..1802]`, `2000/2001/2002 + 3*i` (vs_entry_*, vs_set_entry), `3903 + i` (emit_str, scan_one_lit, write_lit_cells), the `>= 256` guards, `zeros(256)` fn arrays | step 0 relayout + cap 512 | bebop.bp:616, 773, 1795-1798, 2936, 3211-3237, 4090-4109, 4242, 4447, 4457-4465 (verified) |
| tools/check_abi.py zones | window/facts tuples | check_abi.py:~167 |
| bebop.bp:emit_cond | csel path + purity pre-scan | bebop.bp:2805 |
| bebop.bp: new `arm_is_pure(s, from, to)` + `arm_end(s, pos)` | scanners | after skip_while_cond bebop.bp:3561 |
| bebop.bp:vs_settle_flags | must not fire inside the csel window | bebop.bp:2058 |
| bebop.bp:emit_while_stmt (commit 2) | literal pre-scan + hoist + pair table 3828+ | bebop.bp:3593 |
| bebop.bp:emit_lit / vs_push CONST (commit 2) | CONST -> CS when a live pair matches | vs_push bebop.bp:2065 |
| bench/parity_constructs, construct_parity.sh, census_allow.txt, word_budget.txt, docs/PERF.md | constructs, allow lines, rows | -- |

## 5. Steps

0. Relayout commit (§3 step 0): chain without `--codegen`, gen3 == gen4, 0 WORD_MISMATCH; journal line; promote; the main session commits.
1. Scanners + csel path; constructs c70/c71 (§6); `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen`; census_allow line for the b.cond drop; honest.sh K8H row; journal line; leave uncommitted -> main session.
2. If K8H > 1.2x: hoisting commit as above; c72; chain; honest.sh; journal.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c70_csel | `fn main() -> i64 { let a = 7; let b = 3; let i = 0; let acc = 0; while i < 1000 { let acc = acc + (if i * 7 % 3 == 1 then a + i else b - i); let i = i + 1; 0 }; acc }` | derive with `python3 tools/bpref.py` | pure arms with SYM/temp operands, cond from `==` |
| c71_csel_impure | same shape but the then-arm calls `g(i)` | bpref | must stay on the branch path (WORD_DELTA vs c70 shows the difference) |
| c72_hoist (commit 2) | K8H's loop shape with two 64-bit constants | bpref | two loop cs temps, cs_hi in the prologue |

Register in bench/vs_rust/construct_parity.sh (`c70_csel) EXPECT=<v>;;` next to c53, verified line 86) and freeze with the chain's FREEZE=1. Twin: bench/vs_rust/kernels/k8h.bp vs rust_once/k8h.rs via honest.sh (existing).

## 7. Gates

- Step 0: `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT` -> `chain: fixpoint gen3 == gen4`, `battery: GREEN`, no WORD_DELTA lines.
- Commit 1: chain `--codegen` GREEN; `k8h_loopwords` in docs/PERF.md falls (expect 40 -> ~30); `bench/vs_rust/honest.sh` K8H <= 1.2x; K1H-K4 rows unchanged within noise; c05_if/c46_andor WORD_DELTA 0.
- Commit 2: K8H row again; `k8h_loopwords` -8.
- RED looks like: a construct value mismatch (flags clobbered inside an arm -> the purity scanner missed a comparison), or gen2 != gen3 (planning/emission disagreement: the pre-scan read different text in the two passes -- it cannot, both read `s` at the same pos; if it happens, `pos` was advanced by the scanner: scanners must restore pos).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| an arm emits a flag-setting word (cmp, T118 trap, `!`) | c71 + `if c then !x else y` must take the branch path | wrong arm value |
| FLAGS not on top while the arms push | §1.2 rule suspended only inside emit_cond's csel window; assert via the boundary invariant after the join | exit 102 |
| csel with CONST arms materialised into the same register | vs_pick_dest / distinct temps for ta and tb | csel picks the same value twice |
| hoisted register collides with a `let` inside the loop | cs temps are above stab[0] at hoist time; a later `let` inside the body binds 19+stab[0] -- the loop cs temps must sit ABOVE the highest register the body will bind: pre-count the body's `let` names (text scan, like the literal scan) and start hoisting at 19 + stab[0] + new_lets | value clobbered mid-loop -> c72 wrong |
| census b.cond drop not allowed | invariants lane RED | add the allow line |

## 9. VERDICT format

```
VERDICT: GREEN|RED
step0: fixpoint <md5>, WORD_MISMATCH 0, fn cap 512
csel: fixpoint <md5>; k8h_loopwords <before> -> <after>; K8H honest <ms> vs <rust ms> = <x>x (gate 1.2x)
hoist: done|skipped (K8H after csel = <x>x); k8h_loopwords -> <after>
constructs: c70/c71/c72 EXPECT + WORD_DELTA lines; c05/c46 delta 0
census: <allow line>
journal: <line>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo + this blueprint path; HEAD; A1 landed facts; harness commands (chain, battery, FREEZE/WORD_DELTA/word_budget, census_allow, words.objdump, check_abi, reap, never cp over bebop.bin, proc cap 30, fuzzd paused); the fn-cap note (step 0 first, own commit). </context>
<constraints> one variable per commit (relayout | csel | hoist); no other codegen change; scanners restore pos; asm words via as+objdump into $OUT/words.objdump before editing; leave each step uncommitted for the main session; reap after every run. </constraints>
<output_format> the §9 block. </output_format>
<task> implement A2 step 0, then commit 1, then commit 2 if the K8H gate is not met; report. </task>
