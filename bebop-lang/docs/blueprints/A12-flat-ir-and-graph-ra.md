Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A5-A8 (indices, typed tables = alias facts), A3 (LIN on tags stays; the IR's `lin` pass is the same composition over rows), A10 (memo keys are unaffected: the IR is internal)

# A12 Flat per-fn index IR (rung 1), then graph register allocation (rung 2)

## 0. Goal

Rung 1: the parser builds, per fn, a flat array of rows `{op, a, b, aux}` (blocks as ranges, SSA with phi at while heads / if joins), runs the QBE-sized pass list (fold, lin, cse, sroa, inline, tre, dce), and the register-model emitter consumes rows instead of text; the planning/emission double pass disappears (sizes are known from rows). Gate: **K5 -15 %** measured BY HAND FIRST on one hot loop (SROA of `pos[0]`/`n[0]` cells + inlining `slen`/`is_alpha` in `skip_ws`/`read_ident`), **K2H ~1.0x Rust** via tail-recursion -> loop, chain GREEN, every construct value unchanged, word budget for growth. Rung 2: Hack/Goos SSA colouring + coalescing replaces the window/park/retarget; gate **K5 -3 % more, bin_words -2 %**, pressure-> 8 constructs (c55/c56/c62/c63) green.

## 1. Scope

In (rung 1): a row arena per fn (`zeros`), `ir_*` builders called from the existing parse functions instead of `vs_*` (the parser stays one-pass; it now produces rows), SSA conversion (symbols -> row values; phi rows at loop heads and if joins for every symbol live across the edge), passes as single `while` loops over rows, an emitter that walks rows and calls the register-model primitives (`vs_push/vs_binop/vs_bind` become row-driven: the window model stays in rung 1), layout from row counts (no planning pass; B1's arithmetic correction deleted), the memo (A10) stores final words -- unchanged. In (rung 2): liveness over blocks, dominance order, chordal colouring over x0..x7 + x19..x26 with callee-saved colours for values live across calls, spill pass by pressure (furthest next use), phi resolution = the bounded parallel move, coalescing of copies; deletes the window/park/retarget/evict machinery (~250 lines). Out: LICM/GVN beyond CSE within a block (RESEARCH-DEPS §1: ~0 on this corpus), instruction scheduling (OoO does it), auto-vectorisation, an interpreter of the IR.

## 2. Preconditions

A5-A8 landed (typed tables give the alias rule "different tables never alias"; indices make `[0]` cells SROA-able when the cell's array index never escapes: escape = stored into another cell / passed to a call / returned -- the same text scan `loop_alloc_safe` uses, now over rows); A3 (LIN) as the model for a pass; the register model's primitives as the emission backend; docs/RESEARCH-DEPS §3 (row sketch), RESEARCH-TENSOR §1 (rung 2 model).

## 3. Design

**Measure first (rung 1 gate is conditional).** Hand-rewrite `skip_ws` + `read_ident` in a scratch copy: hoist `pos[0]` into a local (`let p = pos[0]; ... let _ = pos[0] = p` at exit), inline `slen`/`is_alpha`; compile with the promoted compiler; K5 median of 3. If the delta < 5 % for these two (the corpus-wide extrapolation is ~3x), journal `VERDICT:refuted` and STOP A12 (rung 2 alone is then not worth it either).

**Rows.** `ir[4*k..4*k+3] = {op, a, b, aux}`; a, b = row indices or `-1 - c` for constants (the constant pool `cpool[c]`); aux = imm / cond / slot / callee / symbol id. Ops: const, sym (parameter/initial), add sub mul sdiv mod and orr eor lsl lsr asr, cmp(cond) -> flags value, csel, neg, not, ldr/str (table, index; with the A8 width), call(callee, first arg row, n), builtin(id, ...), ret, br(cond row, then-block, else-block), jmp(block), phi(block a -> row, block b -> row), alloc(n), lit(handle), select. Blocks: `blk[2*j] = first row, blk[2*j+1] = terminator row`; a fn = rows 0..R, blocks 0..B, entry block 0. SSA: each `let NAME = e` defines a new value row; `NAME` reads resolve to the current definition per block (a per-block symbol map: the existing `stab` with a version cell); at a loop head every symbol assigned in the body gets a phi (pre-scan the body text for assigned names -- A3's detector already does this); at an if join, symbols assigned in either arm get a phi (arms are expressions today: only `let..in` inside arms assign -- rare; a value-producing if is a select/phi of the arm values).

**Passes** (each one `while` over rows, O(rows)): fold (const op const; const propagation through copies), lin (A3's composition expressed on rows: detect the affine loop from phi + rows; same legality), cse (hash (op,a,b,aux) -> earlier row in the same block, tables/loads excluded unless no intervening store to the same table), sroa (a cell array of length 1 (`[0]`/`[x]` literal) whose index row never escapes: replace ldr/str rows by a versioned value + phis), inline (callee <= 8 rows, no loops, no calls: copy rows with an index shift; arguments = rows), tre (`f(x) = g(f(x-1), c)` with g associative (add/mul) -> accumulator loop: fib's second call -> the loop LLVM emits, RESEARCH-DEPS §5.3), dce (use counts, reverse pass).

**Emission from rows.** A block walk in layout order; each row calls the register-model primitives exactly as the text parser did (`vs_push CONST/SYM`, `vs_binop`, `vs_cmp`, `vs_bind` for symbol definitions, calls via the same placement); phis at block ends = binds of the incoming values (a bounded parallel move when several); the sizes: rows -> words is not known before emission (forms vary by tag kinds) -> keep the two-pass structure for SIZE only if needed: a first emission into a throwaway buffer gives sizes (as today's planning pass, but now cheap: no parsing) -- ponytail: keep two emissions of rows, delete the parse-twice; the B1 arithmetic correction goes away because both emissions are the same code.

**Rung 2.** Liveness per block (bit-vectors over rows: rows <= 4096 per fn -> 64 words), dominance order = block order for structured code (loops nest; no irreducible CFG in this language), colouring in dominance order with χ = max pressure (Hack/Goos: SSA interference graphs are chordal), colours: x0..x7 (caller-saved) for values not live across a call/builtin, x19..x26 for those that are (the callee-saved set the prologue must save = the facts vc-equivalent), spill pass when pressure > available (furthest-next-use to slots), phi/call moves via the bounded parallel move, coalescing copies whose ranges do not interfere (removes most `mov` the window model emits at binds). The emitter then takes registers from the colouring: `vs_*` shrinks to the word forms; window/park/evict/retarget deleted.

**Invariants.** Rows -> words deterministic; every pass is value-preserving in wraparound i64 (fold/lin/cse) and bpref-checked by the constructs and std_golden; sroa never touches a cell whose index escapes; inline never duplicates a loop; tre only for associative g; rung 2's colouring never assigns a callee-saved colour that the prologue does not save (facts from the colouring itself).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp: emit_factor / emit_term / emit_expr / emit_cmp / emit_cond / emit_let_* / emit_while_stmt / emit_body / emit_call_or_ctor / emit_bl_call / builtins | call `ir_*` builders instead of `vs_*`/`em` | 3128, 2720, 2536, 2805, 3672, 3593, 3830, 1527, 727 |
| bebop.bp: new ir_* (builders, ~200 lines), ssa (~150), passes (~60-120 each), ir_emit (~200) | rung 1 | new; fn cap 512 |
| bebop.bp: compile_fn_at / compile_program_offs | rows per fn; two emissions of rows; B1 correction deleted | 4320-4465 |
| bebop.bp: rung 2: ra_liveness / ra_colour / ra_spill / ra_moves; delete vs_park*/vs_evict*/retarget | rung 2 | 1900-2060 |
| tools/check_abi.py, docs/REGISTER-MODEL-BLUEPRINT.md (superseded sections), docs/LANGUAGE.md (no semantic change) | -- | -- |

## 5. Steps

0. Hand measurement (skip_ws/read_ident); journal; decision.
1. Rows + SSA + emission from rows with ZERO passes: chain `--codegen` -- expected byte-identical or near (the window model consumes rows like it consumed text; any WORD_DELTA must be explained); the planning pass replaced by the second row emission.
2. Passes one commit each in the order fold, sroa, inline, cse, dce, tre, lin (each a chain commit with its own WORD_DELTA and K5/K2H rows; lin must reproduce A3's numbers exactly).
3. Rung 2 as one commit (colouring + spill + moves + deletions) -- the big one; c55/c56/c62/c63 (pressure) + a new c85_pressure16 (16 simultaneously live values across a call).
Leave each uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c86_sroa | `let p = [0]; while ... { p[0] = p[0] + i }` with p never escaping vs a twin where p is passed to a call (must NOT sroa) | bpref | escape rule |
| c87_inline | a 3-row helper called in a loop | bpref | inlining |
| c88_tre | fib(20) and a sum-recursion | bpref | tail-recursion -> loop (K2H shape) |
| c85_pressure16 (rung 2) | 16 live values across a call | bpref | spill pass |
| all existing constructs | re-frozen | same values | -- |

Twins: K5 (docs/PERF.md), K2H honest row (gate ~1.0x), K1H/K3H/K4 rows unchanged vs A3 (lin on rows == LIN on tags).

## 7. Gates

- Step 0: K5 delta >= 5 % on the hand rewrite, else STOP.
- Step 1: chain GREEN; WORD_DELTA explained; K5 not worse than +3 % (the row build costs time; two row emissions are cheaper than two parses -- expect K5 -10-20 % already from dropping the second parse).
- Step 2: after all passes K5 <= 0.85 x the A9-adopted row; K2H <= 1.1x Rust; every construct value unchanged.
- Step 3: K5 -3 % more; bin_words -2 %; pressure constructs green; the register-model pitfalls' invariants (mask helpers) replaced by the colouring's own assertion (every value has exactly one colour at every use).
- RED: gen2 != gen3 = a pass that is not deterministic (hash order in cse -> use insertion order); a construct value change = a pass legality bug (probe by disabling passes one at a time: `BEBOP_PASSES=` env read by... no env in bebop.bin: a compile-time constant table `fntab[3838]` bitmask set by cli flag `compile --passes N`).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| SSA for `let..in` inside expressions and match arms | c46_andor, c12_match, c22-c24 | wrong value |
| sroa on a cell that escapes via a builtin (sys_read into it) | c86 twin | garbage |
| inline changes evaluation order of side effects (calls inside args) | c58_callmix | wrong |
| tre on non-associative g | c88 sub-recursion twin (must not transform) | wrong |
| rung 2 colouring gives a callee-saved colour the prologue does not save | c85; facts from ra | caller's register clobbered |
| row arena size per fn (bebop.bp's compile_fn_at ~ 400 rows) | trap 89 at 4096 rows | exit 89 |

## 9. VERDICT format

```
VERDICT: GREEN|RED|STOP-AT-0
step0: K5 hand rewrite <b> -> <a> (threshold 5 %)
step1: fixpoint <md5>; WORD_DELTA summary; K5 <b> -> <a>
step2: per pass: fixpoint, K5, K2H, deltas
step3: fixpoint; K5; bin_words; c85 + pressure constructs
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, RESEARCH-DEPS §3, RESEARCH-TENSOR §1, the register model as the backend, A3's detector as the lin model, harness commands. </context>
<constraints> measurement first; one commit per pass; rows deterministic (no hash-order dependence); constructs unchanged in value; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A12 steps 0-3; report. </task>
