# Optimization playbook: what actually worked

Status: 2026-09-04 CURRENT (practices playbook; the 340/340 parity and self_check counts are from the expr_compile.bp era — the live gates are bench/vs_rust/std_golden.sh + bench/oracles/run_all.sh)

Practices distilled from getting 4 kernels 1.3x-5.4x faster than equivalent
Rust, ordered by leverage. Each survived bit-exact differential parity
(340/340) and self-hosted checksums (self_check).

## 1. Verify every encoding on real execution, never trust tables
Every AArch64 constant enters the compiler only after being (a) dumped from
objdump for reference inputs and (b) executed through exec_words computing a
known result. Two hand-derived constants were wrong this arc alone
(movrr base, UBFM decimal). Derive word(s,rn,rd) = base + fields
programmatically; paste decimals into .bp via script, not by hand.

## 2. Compile-once artifact cache (cold start abolished)
Key artifacts on crc32(compiler source) + crc32(kernel source); replay on hit.
Warm replay is ~12x faster than recompilation and byte-identical. Cache lives
in `.becache/`; mutating any input yields a new key naturally. Rule: repeated
work must be a cache lookup, not a recomputation - compilation is an event,
artifacts are the product.

## 3. Pre-tokenize once, walk many (fast path)
The biggest win was architectural: tokenize the whole function into a flat
[kind,value] buffer once, then let expression walkers consume tokens by index
(no rescanning, no string slicing per peek). Layout st[1088]: kinds at 16+i,
values at 528+i, cursor at 1040.

## 4. Lazy constant folding with materialization points
Constants ride in cells (flag cx[40+s], value cx[50+s]) and emit ZERO words
until a consumer materializes them. Fully-constant subtrees cost nothing.
Discipline: every consumption site (binop sides, cmp sides, if arms, call
args, while conds, tail position) MUST have a materialization branch - the
three worst bugs of the arc were one missing site each.

## 5. Dead-hardware elimination classes
Systematically NOP or delete: (A/B) let-binding windows that move a register
then immediately op it in place; (G1/G2) x15/x14 setup words when body scan
proves no reference; (F3) trailing pure-constant statements whose value is
never read; redundant `[mov x0,xR][cmp x0,#imm]` before compares. Scan the
emitted window, rewrite fields, drop words. k4 lost 15% of its stream here.

## 6. Peepholes that respect semantics, not patterns that hope
Sound transformations only: pow2-multiply -> single LSL/UBFM (rhs-const case,
variable already in place); `==0`/`!=0` conditions -> cbz/cbnz (both if-core
and loop headers). Unsound-but-tempting: replacing `while x>0` with cbz -
negative values break equivalence. If you cannot argue ranges, don't.

## 7. Branch-mode codegen instead of stack juggling
If/while emit direct b.cond/cbz with patch slots instead of pushing booleans.
Arms evaluate at their own depth; patches close after arms complete. Removes
push/pop traffic from every conditional.

## 8. Fast calls with minimal spill
First two call args go directly in x0/x1 (const materialized into scratch),
deeper args spill caller accums BEFORE parsing args; bl; result-mov happens
BEFORE pops (pops clobber x0). Guard against unresolved callees to avoid
infinite re-parse recursion.

## 9. Differential parity as the only trusted oracle
Interpreter vs compiled-native on generated corpora, out-of-process, results
compared as strings. Every optimization above landed behind 40+300 case runs.
Checksums (self_check, fuzz_selfhost consts) pin exact streams; regenerate
after ANY emitter change - 2 of 34 changed for the LSL round, proving
selectivity.

## 10. Measure min-of-N, never single runs
Host noise under proot swamps small kernels; medians lie, minimum converges.
Interleave A/B when comparing toolchains. Never chase sub-10% deltas between
runs.

