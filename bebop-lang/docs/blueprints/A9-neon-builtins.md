Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A1 (vs_deliver operand delivery for builtins); `scan` any time after A1 (measured-first), `cmp_mask`/`fill` after A8 (u32 tables), `umulh` any time

# A9 NEON / hardware builtins: `scan`, `cmp_mask`, `sum64`, `fill`, `umulh`

## 0. Goal

Five builtins in the emit_sys_* pattern (hand words derived by as+objdump, bpref stub, construct gate, check_abi allowlist) that give the parser and the store's scan loops the memory-width the ISA has (NEON 2 x i64 lanes / 16 bytes per `ld1`, no gathers, no 64-bit vector multiply -- RESEARCH-TENSOR §2, /proc/cpuinfo `asimd asimddp crc32`, no `sve`): gates **K5 -10 %** from `scan` alone (measured by hand on one `skip_ws` first; threshold decides whether the builtin lands), **K6 nn.bp ns/row <= 4** with `cmp_mask` + `sum64` against a Rust scan twin in the same honest report, each builtin a construct (c78-c82) with a bpref stub.

## 1. Scope

In: `scan(s, pos, class)` -- advance `pos[0]` over bytes of class `class` (0 = whitespace (space/tab/nl/cr), 1 = identifier chars [A-Za-z0-9_], 2 = not-quote-not-backslash, 3 = not-newline) using `ld1 {v0.16b}` + `cmeq`/`cmhi` range compares + `umaxp`/`shrn` mask extraction + `rbit/clz` to find the first non-matching byte; returns the new pos; `cmp_mask(t, n, c, op)` over a `[u32]` or `[i64]` table -> a bitmap (cells) of `t[i] op c` (op: lt/le/eq/ge/gt, compile-time constant), 2 lanes i64 or 4 lanes u32 per step, `cmgt/cmge/cmeq` + narrowing to bits; `sum64(t, n)` -> sum of i64 cells (`ld1 {v0.2d,v1.2d}` + `add v.2d` + `addp`); `fill(t, n, v)` (`dup v0.2d` + `st1`); `umulh(a, b)` -> high 64 bits of a*b (one scalar word, `umulh xd,xn,xm`; Q32 fixed-point `plus-times` for B-phase kernels). Out: auto-vectorisation; a (file, lane, width) tag payload; `smull`-based 64-bit vector multiply (rejected: >= 8 instructions per 2 lanes). Fixed points: the register model (builtins are call-like: vs_deliver parks everything, operands in x0..x2, result in x0); bpref semantics (stubs compute the same values in python).

## 2. Preconditions

A1 landed (vs_deliver bebop.bp:855, verified; the hand-word blocks may clobber x0..x13 and flags); emit_hvham (bebop.bp:878) as the NEON precedent (`ldp q0,q1`, `eor`, `cnt`, `addv`); check_abi's `sys_allow` reads every `em(insns, n, <lit>)` inside `fn emit_sys_*` (tools/check_abi.py:102-108, verified) -- name the new emitters `emit_sys_scan` etc. so their words are allowlisted automatically, or extend the regex; A7 for `scan` over handles (before A7, `s` is a raw pointer: `scan` works on either -- the address computation is the only difference; land `scan` before A7 on raw pointers if the measurement is done early, then re-derive its 2 address words in A7); A8 for `cmp_mask` on `[u32]`.

## 3. Design

**Measure first (scan).** Hand-patch ONE parser loop -- `skip_ws` (bebop.bp:1674, verified) -- with a 16-byte NEON step written as em() words in a scratch copy of bebop.bp, compile with the promoted compiler, and time the self-compile (K5, 3 runs median, `tools/perf.py` method) against the unpatched: if the delta < 3 % (skip_ws alone; the full set skip_ws/read_ident/skip_string/skip_line_comment/collect_fns is ~4x that), STOP and journal `VERDICT:refuted` for the scan builtin; else proceed.

**scan(s, pos, class) -> new pos** (operands: s in x0, pos in x1, class is a compile-time constant selecting one of four word blocks; A7 handle: `lsr x2,x0,#32 ; add x2,x17,x2 ; and x3,x0,#0xffffffff` = base, len; pre-A7: x2 = x0, len via the caller's pos[1] = slen). Loop: `ld1 {v0.16b},[x4]` at base+pos; class predicate per byte with `cmeq`/`cmhi`/`cmhs` against `dup`'d constants (whitespace: 4 cmeq + orr; ident: `sub v1.16b, v0.16b, #'0'` then `cmhi` ranges for 0-9, A-Z (with `orr #0x20` for case folding), `_`), invert to "stop bytes", `shrn v1.8b, v1.8h, #4` (the 64-bit mask trick: 4 bits per byte) -> `fmov x5, d1` -> `rbit x5, x5 ; clz x5, x5 ; lsr x5, x5, #2` = index of the first stop byte; if 16 (no stop) advance 16 and loop (bound by len: the tail < 16 bytes uses a scalar loop or an over-read guard: over-reading up to 15 bytes past len inside the reserve is safe only if the reserve has slack -- the byte allocation rounds to 8; guarantee 16 bytes of slack after every byte region (A7: allocate len + 16) and document it). Result: `str x6,[x1]` (pos[0] = pos + index), return in x0. ~35-45 words per class block; four blocks = one emitter with a class switch at compile time (each class its own `emit_sys_scan_<class>` body or a table of constants).

**cmp_mask(t, n, c, op) -> bitmap** (`[u32]` or `[i64]` decided by the base symbol's type tag from A8; op and c compile-time: c via `dup v1.4s/2d` from a register): loop over 4 (u32) / 2 (i64) elements: `ld1`, `cmgt/cmge/cmeq v2, v0, v1`, `shrn`/`xtn` + `fmov` + `and #mask` + shift into the current bitmap word, `str` every 64 bits; result cells allocated by the caller (`zeros(ceil(n/64))`, passed as the 5th operand -> deliver 5 operands: x0..x4 -- vs_deliver supports nops up to 8). The zone-map skip (block min/max) stays in generated kernel code (B phase), not in the builtin.

**sum64(t, n) -> i64**: `ld1 {v0.2d, v1.2d}` x2 per step (32 B), `add v0.2d, v0.2d, v1.2d` accumulate, `addp d0, v0.2d`, `fmov x0, d0`; tail scalar. ~18 words. **fill(t, n, v)**: `dup v0.2d, x2` + `st1 {v0.2d, v0.2d}` per 32 B, tail; ~12 words. **umulh(a, b)**: register-parameterised single word `umulh xd,xa,xb` (like `clz`: operands via vs_reg, no delivery) -- 1 word, result REG d.

**bpref stubs** (tools/bpref.py:493 builtin_or_call, verified): scan = python loop over the class predicate; cmp_mask = list comprehension into an int bitmap stored as cells; sum64 = sum with wrap; fill = slice assign; umulh = `(a*b) >> 64` on unsigned 64-bit views (mask both to 2^64).

**Invariants.** Every builtin is call-like (park + deliver) except `umulh` (register-parameterised); no builtin reads past `len + 16` bytes / `n` cells; results identical to the stubs on every construct and on the std_tests that get rewritten to use them (none in this task: the compiler's own parser loops adopt `scan` in a separate step after the K5 measurement).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp: new emit_sys_scan / emit_sys_cmp_mask / emit_sys_sum64 / emit_sys_fill / emit_umulh + dispatch hashes in emit_call_or_ctor (read_ident hash h*131+ch; add next to is_clz bebop.bp:1527-1560 verified) + compile_fn_at's reserved-name list (T122) | builtins | 1527, 1345 (emit_clz pattern), 878 (hvham NEON pattern) |
| bebop.bp: skip_ws / read_ident / skip_string / skip_line_comment / collect_fns | adopt `scan` (step 3) | 1674, 112, 1657, skip_line_comment, collect_fns |
| tools/bpref.py | RESERVED + stubs | 47, 493 |
| tools/check_abi.py | allowlist via emit_sys_* names | 102 |
| bench/parity_constructs c78-c82, construct_parity.sh; bench/vs_rust/kernels/k6 twin (nn.bp scan with cmp_mask+sum64) + rust_once/k6scan.rs; honest.sh row `k6s` | -- | honest.sh:17 |

## 5. Steps

1. Measurement (scan on skip_ws by hand): journal line; decision.
2. `umulh` + `sum64` + `fill` (small, independent): chain `--codegen`; c80/c81/c82.
3. `scan` builtin (four classes) + c78; chain; then the parser adoption commit (skip_ws/read_ident/skip_string/skip_line_comment/collect_fns) with the K5 gate.
4. `cmp_mask` (after A8) + c79; K6 twin rows (bebop nn.bp rewritten with cmp_mask+sum64 vs rust_once/k6scan.rs) in honest.sh.
Leave each uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c78_scan | a 100-byte literal with runs of spaces/idents/quotes/comments; call scan for each class from several positions; sum of returned positions | bpref stub | all four classes, tails < 16, over-read slack |
| c79_cmpmask | `zeros32(100)` filled with i*7 mod 50; cmp_mask lt 25 -> popcount of the bitmap (via hvham on the bitmap cells or a loop) | bpref | u32 lanes, bit packing |
| c80_sum64 | 1000 cells i*i, sum; n = 0, 1, 2, 3, 33 (tails) | bpref | lanes + tail |
| c81_fill | fill 100 cells with -7, sum | bpref | st1 pattern |
| c82_umulh | umulh(2^40+3, 2^40+5), umulh(-1, -1) (= 2^64-2) | bpref | unsigned high half |

Twins: k6s (scan-shaped): bebop nn.bp with cmp_mask+sum64 vs rust_once/k6scan.rs (Rust: iterator filter+sum over the same SoA, auto-vectorised) -- gate ns/row <= 4 and the Rust ratio reported.

## 7. Gates

- Step 1: K5 delta from one hand-patched skip_ws (journal), threshold 3 % (extrapolated 10 % for the full adoption).
- Steps 2-4: chain `--codegen` GREEN; constructs; WORD_DELTA 0 on existing constructs (no adoption inside them); after adoption: K5 <= 0.90 x the pre-A9 row; std_golden 99/99 (the parser changed: every test is a parser test).
- honest.sh `k6s` row: bebop ns/row <= 4; ratio vs Rust reported.
- RED: c78 mismatch = a class predicate or mask extraction bug (probe: single-run inputs of length 1, 15, 16, 17); over-read SIGSEGV = slack missing (A7's +16).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| NEON words clobber v8-v15 (callee-saved low halves) | use v0-v7 only | corruption in callers using NEON (hvham) |
| over-read past the reserve end | slack rule; c78 with a literal at the end of the literal region | SIGSEGV |
| `shrn` mask trick encoding | as+objdump + c78 lengths 1..17 | wrong stop index |
| K5 gain below threshold | step 1 | do not land scan; keep umulh/sum64/fill |
| cmp_mask on i64 vs u32 chosen wrongly | A8 type tag; c79 both widths | wrong bitmap |

## 9. VERDICT format

```
VERDICT: GREEN|RED
scan measurement: K5 <before> -> <after> with hand-patched skip_ws (threshold 3 %): land|refute
builtins landed: <list>; fixpoint(s) <md5>; constructs c78-c82 EXPECT
parser adoption: K5 <b> -> <a> (gate -10 %); std_golden 99/99
k6s: bebop <ns/row> (gate 4) vs rust <ns/row> = <x>x
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, emit_hvham as the NEON precedent, vs_deliver contract, check_abi allowlist rule, harness commands, A7/A8 facts if landed. </context>
<constraints> measurement first; v0-v7 only; words via as+objdump into $OUT/words.objdump; one chain commit per builtin group; bpref stub with every builtin; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A9 steps 1-4; report. </task>
