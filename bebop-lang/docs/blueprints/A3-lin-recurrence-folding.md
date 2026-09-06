Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A1 (tags, vs_binop with MULC/madd forms, B5 loop shape) and A2 step 0 (fn cap 512)

# A3 LIN: folding linear recurrences over k = 2/4 iterations

## 0. Goal

A `while` whose body is an affine recurrence over its accumulators and counter (`s = a*s + b(i)`, `i = i - c`) is emitted as a folded loop that performs k iterations per trip with the composed affine map (exact in wraparound i64), followed by the original loop for the tail. Gates: **k1h_ms <= 0.5 x Rust**, **k4_ms <= 0.6 x Rust** (honest.sh), K3H row reported, bpref parity on every std_test (LCG/hash loops), constructs c73-c75. Expected (RESEARCH-DEPS §1b(4)): K1H 3 -> ~1 cycle/iter, K4 4.5 -> ~2.3, K3H 4 -> ~2.

## 1. Scope

In: `emit_while_stmt` (bebop.bp:3593, verified) gains a shape detector (text scan of the body between `{` and the matching `}`) + an affine mini-parser + a folded-loop emitter that uses the existing tag primitives (vs_push CONST/SYM, vs_binop mul/add/sub, vs_bind). k = 4 when there is one accumulator and every composed coefficient fits a single movz (< 65536 in magnitude) after folding, else k = 2. Out: any body with a call, array access, string, `if`, nested `while`, `match`, `return`, `break`, div/mod/shift/comparison/bitwise ops in an accumulator RHS, an accumulator assigned twice, a counter used in a non-affine way, a condition that is not one of `i > L`, `i >= L`, `i < U`, `i <= U` with L/U a literal or a loop-invariant symbol. Fixed points: the non-folded path is byte-identical (WORD_DELTA 0 on c07_while, c33-c36); bpref untouched (the source is unchanged, parity is by construction); no new fntab cells except the pair table below.

## 2. Preconditions

A1 landed (MULC materialisation table §2, `madd` form, retarget); A2 step 0 (fn cap 512, fntab relayout); honest kernels k1h/k3h/k4 with Rust twins (bench/vs_rust/kernels/, rust_once/, honest.sh:17 verified); `skip_while_cond` (bebop.bp:3561) as the scanner model; the §3.14 statement-boundary invariant (the folded loop must leave mask 255 / cs 0).

## 3. Design

**Shape.** Body items (emit_body_classify bebop.bp:3791 recognises them; the detector re-scans text): only `let NAME = RHS;` items and a final `0`. Exactly one counter item `let i = i - c;` / `let i = i + c;` (c a literal, c > 0). Every other `let` binds an accumulator s_j (a symbol bound BEFORE the loop -- `sym_is_outer` bebop.bp:2990-style check) with an affine RHS over {s_1..s_m, i, invariants, literals} using only `+ - *` and parentheses, where every `*` has at least one literal side after constant folding (affine). m <= 3. Condition (skip_while_cond gives its text): `i OP B` with OP in `> >= < <=`, B a literal or an outer symbol not assigned in the body; direction consistent (decreasing counter with `>`/`>=`, increasing with `<`/`<=`).

**Composition.** State vector z = (s_1..s_m, i, 1). One iteration = z' = M z with integer M ((m+2)x(m+2)); the counter row is (0..0, 1, -c) and the last row (0..0, 0, 1). M^k by repeated squaring/multiplying in wraparound i64 (bebop's `*`/`+` are exactly that) -- computed at compile time, both passes identical. Folded body = for each accumulator j: `s_j_new = sum_l M^k[j][l] * s_l + M^k[j][m] * i + M^k[j][m+1]`, computed into window temps from the OLD values (simultaneous update), then bound; counter `i = i - k*c`.

**Guard.** The folded trip is legal iff all k iterations would have passed the original test. For a monotone counter and a threshold test that is exactly: decreasing, `i > L`: guard `i - (k-1)*c > L`; `i >= L`: `i - (k-1)*c >= L`; increasing `i < U`: `i + (k-1)*c < U`; `<=` likewise. Emit the guard as the folded loop's own bottom test (B5 shape), then the ORIGINAL loop (parsed from text as today) handles 0..k-1 remaining iterations. Both loops share the T43 mark logic only if the body allocates -- it does not (shape excludes allocation), so the folded loop emits no pmark/reset words.

**Emission via tags** (no new word forms: everything is vs_push/vs_binop/vs_bind): for K1H (s' = 3s + i, i' = i - 1, k = 4): M^4 row = (81, 40, -18): `push SYM s; push CONST 81; mul -> MULC(s,81)` (81 not special -> materialised as movz+mul or, better, consumed by the following add as madd); `push SYM i; push CONST 40; mul -> MULC(i,40) = 32+8 -> add-shifted + lsl`; `add`; `push CONST -18; add -> sub #18`; bind s (retarget). Counter: `sub i,i,#4`. Guard: `cmp i,#3 ; b.gt`. Expected ~8 words per 4 iterations. K4 (k = 2): row (9, 84, -65): `84 = 64 + 16 + 4` is three powers -> general movz+mul path; fine (2 words). K3H inner (k = 2): (9, 8, 12, -3) over (a, x, y): x is an invariant symbol -> a SYM operand, 8 = pow2 -> add-shifted.

**Coefficient table for readers** (the worker verifies with bpref on a 10-iteration probe, not by trusting this): K1H x2 = (9, 4, -1), x4 = (81, 40, -18); K4 x2 = (9, 84, -65); K3H inner x2 = (9a, 8x, 12y, -3).

**Where the state lives.** The detector runs before `pjump`; it needs: m, the accumulator symbol registers, the counter register, c, k, M^k as (m)x(m+2) i64 cells. Store in fntab `3836..3899`? No -- 3890-3903 are taken (struct guard, literals); use a local `zeros(64)` cell array in emit_while_stmt (one allocation per while STATEMENT, outside any loop body of the compiler -- L8 is about loops inside the compiler's own while bodies; emit_while_stmt's body is a function body, fine) and pass it to the folded emitter.

**Invariants.** Same word count in the planning and emission passes (text-directed, no register-dependent decisions); mask 255 / cs 0 at the statement boundary after the folded loop (all temps bound or popped); the original loop after the folded one is emitted by the unchanged path.

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:emit_while_stmt | detector call before `pjump`; folded loop emission; then the existing path | bebop.bp:3593 |
| bebop.bp: new `lin_detect(s, b0, b1, cond_pos, stab, out)` | shape + counter + condition scan | new, after skip_while_cond bebop.bp:3561 |
| bebop.bp: new `lin_affine(s, pos, end, names, m, coef)` | affine mini-parser over `+ - * ( )` returning coefficient cells or 0 | new |
| bebop.bp: new `lin_matpow(M, m, k)` + `lin_emit_fold(...)` | compose and emit via tags | new (~6 fns; A2 step 0 raised the cap) |
| bench/parity_constructs c73-c75, construct_parity.sh, word_budget.txt (the folded loops GROW some constructs: c07_while stays 0, K-kernel bins are not constructs), docs/PERF.md rows k1h/k4 | -- | construct_parity.sh:86 area |

## 5. Steps

1. Detector + affine parser only (no emission change): a debug counter `fntab[3836] = number of loops detected` and a diag print behind an env-free flag is NOT allowed in emitted words -- instead validate the detector with a python twin: `tools/lin_census.py` that applies the same rules to every std_test and to bebop.bp and prints the loops it would fold (expect: k1h, k3h inner, k4, k8h's x-only? no -- k8h's acc depends on a branch, the whole body is rejected; hash loops `h = h*131 + ch` are rejected because `ch` comes from `char()` = a call). The bebop detector must agree with the census on every file (a construct-like gate: `tools/lin_census.py --check <bin>` compiles each probe with the candidate and reads a per-loop marker... simpler: emit nothing differently yet, but add the count of folded loops to the journal line via the existing `fntab[4000]`-style budget? Keep it simplest: step 1 = python census + the bebop detector returning its decision; the decision is exercised in step 2 through the constructs.)
2. Folded emission, k = 2 only; constructs c73 (K1H shape, n = 0/1/2/3/5/1000 in one program summed), c74 (K3H shape), c75 (K4 shape with a negative start); chain `--codegen`; word_budget lines for the kernel-shaped constructs; honest.sh rows.
3. k = 4 for m = 1; chain; honest.sh; gate check.
Each step a chain-gated commit; leave uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c73_lin1 | `fn f(n: i64) -> i64 { let s = 0; let i = n; while i > 0 { let s = s * 3 + i; let i = i - 1; 0 }; s } fn main() -> i64 { f(0) + f(1)*7 + f(2)*11 + f(3)*13 + f(5)*17 + f(1000) }` | bpref | trip counts 0..5 and 1000: guard exactness for k = 2 and 4 |
| c74_lin3 | K3H shape (two accumulators over an invariant x) with x = 300, y = 7 | bpref | m = 2 map, invariant operand |
| c75_lin4 | `let v = -5; let i = 2000; while i > 0 { let v = (v + i * 7) * 3 - 11; let i = i - 1; 0 }` + an increasing-counter twin `while i < n` | bpref | parenthesised affine RHS, negative start, `<` direction |
| c76_lin_reject | body with `i / 2` and one with a call | bpref | must NOT fold: WORD_DELTA 0 vs the unfolded twin |

Twins: k1h/k3h/k4 honest rows (existing). Parity: `tools/std_par.sh` over all std_tests (part of the battery) -- LCG generators (sgraph.bp:14 `lcg`, verified) are affine with huge constants and DO fold when they sit in such a loop shape; the battery proves parity.

## 7. Gates

- `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen` GREEN (std_golden 99, constructs, oracles, fuzz batch); WORD_DELTA lines recorded, growth budgeted.
- `bench/vs_rust/honest.sh`: k1h <= 0.5x Rust, k4 <= 0.6x Rust; k3h reported (expect 0.3-0.5x); k2h/k8h unchanged.
- docs/PERF.md: k1h_loopwords/k4_loopwords rows (the metric counts the loop the kernel runs most -- confirm perf.py's loop finder picks the folded loop; if it picks the tail loop, add a note, do not change the metric).
- RED: any construct or std_test value mismatch = guard or composition error (probe: print M^k from bpref's python twin `tools/lin_census.py --matrix`), or gen2 != gen3.

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| guard off by one | c73 with n = k-1, k, k+1 | wrong sum for small n |
| accumulator read after write within the fold (not simultaneous) | K3H shape where s_2's RHS reads s_1 | value drift |
| wraparound of composed coefficients treated as overflow | LCG loop in std_tests | parity fails only for big constants |
| counter also used in RHS with a different sign convention | `let s = s + i` with `i = i + 1` | wrong |
| planning/emission disagreement | the detector reads text only; no register-dependent decision | gen2 != gen3 |
| loop words metric picks the tail loop | inspect docs/PERF.md loop finder note | k1h_loopwords unchanged while ms drops |

## 9. VERDICT format

```
VERDICT: GREEN|RED
fixpoint: <md5>
folded loops in bebop.bp / std_tests: <n> / <n> (census == detector: yes|no)
loopwords: k1h <b> -> <a>; k3h; k4
honest: k1h <bebop> vs <rust> = <x>x (gate 0.5); k4 = <x>x (gate 0.6); k3h = <x>x
constructs: c73-c76 EXPECT + WORD_DELTA; word_budget lines
battery: <line>
journal: <line>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, HEAD, A1/A2 facts, the harness commands, the trap list (never pkill -f literal, no script edits while running, `S=` prefix, nested if as call arg, `&&`-then-`&`, exit 95 nesting, L8, fn cap 512 after A2 step 0). </context>
<constraints> shape rules of §3 exactly; the non-folded path byte-identical; no new asm forms (tags only); python census first; one chain commit per step; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> implement A3 steps 1-3 and report. </task>
