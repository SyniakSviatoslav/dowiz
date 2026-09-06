Status: 2026-09-06 (session 17) -- Opus analyst blueprint for the operand-tag-stack IR rung (ROADMAP critical path item 2, D12-B amended by D14 item 2), copied from the session scratchpad by the main session. Three deviations from docs/DECISIONS-RESEARCH-2026-09-06.md §3.3 await operator ratification (see the VERDICT block at the end): (1) spill target = machine stack instead of [x15,#s*8]; (2) a sixth tag kind MULC r,c replacing the five word-pattern fusions; (3) k3h_loopwords <= 10 as a gated PERF row at R4. Main-session recommendation: accept (1) and (2) -- both keep R1 byte-identical and delete more matcher code; accept (3) as a gate only once R4's row proves reachable, reported before that. R1 (byte-identical scaffold) depends on none of the three.

# The operand-tag-stack rung — executable blueprint (B3 / D14 item 2)

Grounded at HEAD 67855f5 (B1 = 308f2db landed; PERF row `dcaccb6/1a3b2cc2`, `bin_words` 74222,
`k2h_loopwords` 51, `k1h/k3h/k4_loopwords` 11/25/15). All bebop.bp line numbers below are the
**current** ones; the design report's §3 numbers predate B1 and are stale
(`emit_cond` is 2496 not 2451, `emit_while_stmt` 3154 not 3113, `pop2` 1825 not 1780,
`left_single_begin` 1858 not 1813, `emit_bl` 531, `sym_bind` 181, `compile_fn_at` 3760).
The T96/T101-T104 history lives in `HISTORY.md:2684,2711-2715` (repo root, **not** `docs/`).

Every instruction word below is given as **asm text only**. The coding agent derives each number by
`as` → `objdump` → python int → scripted insert → disassembly diff (L1); `tools/check_words.py` in
the battery is the enforcement.

---

## 1. Model

### 1.1 Tag kinds

The value stack is a compile-time stack of `dep` entries (`dep` = `fntab[3700]`, unchanged). The top
`w` entries are **window entries** carrying a tag; the bottom `dep - w` entries are **STACK** entries —
real values at `[sp]`, in LIFO order, exactly today's push/pop discipline.

| kind | name | payload0 | payload1 | emits on push | materialises as |
|---|---|---|---|---|---|
| 1 | `CONST c` | `c` (i64) | — | nothing | `movz x<d>,#imm16` + `movk x<d>,#imm16,lsl #16/32/48` per non-zero half |
| 2 | `REG r` | `r` in 1..7 | — | nothing (r was just written) | `mov x<d>,x<r>` (nothing if `d == r`) |
| 3 | `SYM r` | `r` in 19..26, or `100+slot` | — | nothing | `mov x<d>,x<r>` / `ldr x<d>,[x15,#slot*8]` |
| 5 | `MULC r,c` | `r` in 1..7 or 19..26 | `c` > 0 | nothing | see 1.4 |
| 4 | `STACK` | — | — | `sub sp,sp,#16` ; `str x<s>,[sp]` | `ldr x<d>,[sp]` ; `add sp,sp,#16` |

`STACK` is never stored in the window array — it is implied by position (`i < dep - w`). Kind 4 keeps
its number so the report's four-kind vocabulary still reads.

Kind 5 exists because it is what replaces the five word-pattern fusions (§2 R2/R3). `MULC r,c` is
produced by `vs_binop(mul, X, CONST c)` and `vs_binop(shl, X, CONST k)` and is the *only* provenance
the model carries. Register×register `mul` is emitted immediately (today's `madd_try` only ever matched
a `movz`-constant multiplier, `bebop.bp:1962 is_movc`, so nothing is lost).

### 1.2 Stack layout in fntab

```
fntab[3700]        value-stack depth                       (unchanged)
fntab[3701 + d]    legacy per-slot bookkeeping, d < 96      (unchanged — L16: bookkeeping only)
fntab[3797]        w = number of window entries, 0..8
fntab[3798]        free mask over x1..x7 (bit r-1 set = x<r> free)
fntab[3799 + 3i]   kind of window entry i        (i = 0 deepest .. w-1 top)
fntab[3800 + 3i]   payload0
fntab[3801 + 3i]   payload1
```

`i` ranges 0..7, so the block is `3797..3822`. `fntab = zeros(4096)` (bebop.bp:2859, 3640, 3667, 4042,
4208) already covers it and cells 3797-3889 are unused today (verified: the only `fntab[36xx/37xx/38xx]`
bases in the source are 3655-3663, 3680-3681, 3700-3701, 3890-3892, 3899).

`tools/check_abi.py:166` gains one tuple, inserted after the `slots` entry:

```python
         (3655, 3661, "fold"), (3662, 3699, "jumps"), (3700, 3796, "slots"),
         (3797, 3822, "vstack"),
```

and a comment block in the style of the `b1_facts` one at `check_abi.py:169-174`. At R2 the `fold`
tuple narrows to `(3660, 3661, "fold")` because 3655-3659 die with `fold_clear`.

### 1.3 The five primitives

- `vs_push_const(c)`, `vs_push_sym(r)`, `vs_push_reg(r)`, `vs_push_mulc(r,c)` — record a window tag,
  emit nothing. If `w == 8`, first `vs_spill_deepest()`.
- `vs_alloc()` — lowest set bit of `fntab[3798]`; clears it and returns `r`. Empty mask →
  `vs_spill_deepest()` first, which is guaranteed to free one because every window entry that owns a
  register is above the STACK prefix.
- `vs_spill_deepest()` — take window entry 0 (the deepest), materialise it into x0, emit the canonical
  push (`sub sp,sp,#16` ; `str x0,[sp]`), return its register to the mask, shift entries 1..w-1 down,
  `w -= 1`. Because it always takes the *deepest*, the spilled entries always form a contiguous bottom
  prefix in stack order, so the machine stack is a correct LIFO for them — this is why kind 4 is `[sp]`
  and not `[x15,#s*8]`: unbounded (array literals reach depth ~99, `bebop.bp:1636`), byte-identical to
  today's spill words, and it needs no slot allocator that could collide with the symbol slots.
- `vs_materialise(i, want)` — the per-kind table of §1.1; frees the source register when the tag was
  `REG`/`MULC`.
- `vs_operand(i)` — returns a register already holding the value with **no** word emitted when the tag
  is `REG r` or `SYM r` with `r < 100`; otherwise `vs_alloc()` + `vs_materialise`.
- `vs_sync()` — for `i = 0..w-1` in order: materialise into x0 and push (i.e. repeated
  `vs_spill_deepest`), then `w = 0`, mask = all-free. Byte-for-byte the stream today's emitter already
  produces at those points.

### 1.4 `MULC r,c` materialisation into `x<d>` (the fusion table, on tags)

Identical decisions to `shl_try` (1985) / `mulc_try` (2006) / `addshift_try` (2040), with no retraction
and no `fntab[3660]` guard:

| `c` | words |
|---|---|
| `2^k` | `lsl x<d>,x<r>,#k` |
| `2^k + 1` | `add x<d>,x<r>,x<r>,lsl #k` |
| `2^k - 1`, c ≥ 3 | `lsl x<d>,x<r>,#k` ; `sub x<d>,x<d>,x<r>` |
| `2^j + 2^k`, 0<j<k | `add x<d>,x<r>,x<r>,lsl #(k-j)` ; `lsl x<d>,x<d>,#j` |
| otherwise | `movz x<d>,#c` (+`movk`) ; `mul x<d>,x<r>,x<d>` |

Consumed by `vs_binop(add, X, MULC r c)` **without** materialising:

- `c == 2^k` → `add x<d>,x<X>,x<r>,lsl #k` (this is `addshift_try`)
- otherwise → `vs_alloc()` a register for `c`, `movz`/`movk` it, then `madd x<d>,x<r>,x<c>,x<X>`
  (this is `madd_try`)

`lsl x<d>,x<a>,#k` is the `ubfm x<d>,x<a>,#(64-k),#(63-k)` alias — the agent derives it from `as`, not
from the existing literal.

### 1.5 Labels, barriers, calls, while marks, spilled symbols

- **Any label or branch target is a hard barrier**: `vs_sync()` immediately before the position is
  recorded, and after the sync `w = 0` and the mask is all-free. The sites are exactly where
  `fntab[3660]` is written today: `emit_cond` 2496 (`else_pos`, `end_pos`), `emit_while_stmt` 3154
  (`loop_start`, `endl`), `cond_branch_word` 2475, `compile_fn_at` 3760 (the epilogue join), and the
  rd=0 retraction bar in `pop` 1662 (which disappears at R3).
- **Calls**: `vs_sync()` before the argument sequence is *materialised*, i.e. at the top of
  `emit_bl_call` 706 after the arguments are parsed; then argument `i` is materialised into `x<i>`
  (R5). x1-x7 are caller-saved and are the window, so nothing may be live in them across a `bl`.
  `emit_bl` 531 and its B1 `stp/ldp x15,x14` conditional are untouched by every rung.
- **Builtins and aggregates** write x0-x8, x14, x2: `vs_sync()` is the first statement of
  `emit_array_lit` 2635, `emit_array_get` 2683, `emit_array_set` 2693, `emit_struct_lit` 389,
  `emit_struct_field` 371, `emit_field_access` 429, `emit_enum_ctor` 488, `emit_enum_ctor_nullary` 512,
  `emit_str` 2614, `emit_zeros` 4277 and each of the 39 `emit_sys_*` / `emit_clz` / `emit_crc32*` /
  `emit_hvham*` / `emit_clock_ms` / `emit_str_len_fn` / `emit_char_fn` builtins. Mechanical rule for
  review: *any function that calls `em()` with a hand-derived word touching a register other than x0
  syncs first*.
- **T43 while marks** (`fntab[3661]`, the `pmark` nop at 3163 patched to `str x14,[sp,#80+8*depth]`)
  are unaffected: they sit at the loop head, which is already a sync point. The `count_word(insns, ...,
  "mov x0,x14")` alloc scan at 3196 keeps working because allocation sites still emit that exact word.
- **Symbols 9+** are `SYM 100+slot` tags: `ldr x<d>,[x15,#slot*8]` on materialisation, `str x<d>,
  [x15,#slot*8]` on bind. `sym_bind` 181 is not touched by any rung. Operand spills never use x15
  slots, so no collision with symbol slots is possible — that is the second reason for kind 4 = `[sp]`.
- **Reset**: `vs_reset()` (`w=0`, mask all-free) is called from `compile_fn_at` 3760 next to the
  `fntab[3660] = 0 - 1` reset at 3770, for the same reason: per-fn state leaking across the planning
  and emission passes made them disagree by 3 words once already (T96 step 1, HISTORY.md:2684).

**Planning/emission agreement invariant.** Every rung must preserve: *the emitted word count of any
expression depends only on the tag kinds, never on which register was allocated.* All allocated forms
are one word regardless of register field, and `str/ldr [x15,#slot*8]` is one word for any slot ≤ 4095.
This is what lets the planning pass (`is_emission == 0`) and the emission pass agree on `n[0]`.

---

## 2. Rung table

One row = one commit = one variable = one gate (L14). `CH=${BEBOP_TMP:-/tmp/opencode}`.

| # | name | functions touched | words | gate | deletes |
|---|---|---|---|---|---|
| **R1** | tag scaffold, byte-identical | new `vs_reset/vs_push_*/vs_alloc/vs_spill_deepest/vs_materialise/vs_operand/vs_sync`; `push` 1643, `pop` 1662, `emit_lit` 1741, `emit_var` 2098, `bind_reg` 2120 become wrappers; `compile_fn_at` 3760 calls `vs_reset`; `check_abi.py:166` | **0 on every construct and kernel; md5(gen4) == md5(bebop.bin) at HEAD** | `PROC_CAP=30 tools/chain.sh bebop.bp $CH/r1` — must print `gen2 == gen3 == gen4` **and** `battery: GREEN` with FREEZE unset (a `WORD_MISMATCH` line is RED); then `md5sum bebop.bin $CH/r1/gen2.bin` equal | — |
| **R2** | `CONST` becomes lazy | `emit_lit` 1741, `emit_num` 2169, `emit_negnum` 2727, `fold_try` 1791→`vs_binop` const-const, `fold_clear` 1778, `shl_try` 1985, `mulc_try` 2006, `cmp_try` 2229 (imm half), `emit_binop*` 1937/1944/2067/2075, `emit_cmp_regs` 2253, `emit_while_stmt` 3154 (`dead` retraction), `check_abi.py` fold zone | no construct may **grow**; expect ≤ 0 everywhere, K4/K3H/K1H loop words unchanged (their constants already fused) | `PROC_CAP=30 tools/chain.sh bebop.bp $CH/r2 --codegen` (FREEZE=1 implied, chain.sh:22); every `WORD_DELTA` line recorded in the commit message; any positive delta needs a `word_budget.txt` line in the same commit (D11-F) | `fold_try` (33 l), `fold_clear` + the 5 fold cells `fntab[3655..3659]`, `shl_try` (21 l), `mulc_try` (34 l), `emit_lit`'s fold bookkeeping, the `dead` trailing-literal retraction in `emit_while_stmt` |
| **R3** | `SYM` becomes lazy; binops are 3-address (dest x0, scratch x1) | `emit_var` 2098, `emit_var_or_ctor` 1491, `emit_binop_plain` 1944, `emit_binop_regs` 2067, `emit_binop_regs_plain` 2075, `emit_cmp_op` 2215, `emit_cmp_regs` 2253, `emit_apply_op/mul/mod/bits/cmp` 2176/2192/2204/2264/2355, `pop` 1662, `cond_branch_word` 2475 | **k4_loopwords 15 → 11, k3h 25 → 10, k1h 11 → 8, k2h 51 → ~44** (each −1 more if B5 landed) | `--codegen` chain as R2, **plus** `k4_loopwords <= 13` and `k4_ms <= 3.0` (D12-B) read from `docs/PERF.md`; `census.py --freeze-check` — a `b.cond` increase needs a `census_allow.txt` line in the same commit | `pop2` (33 l), `left_single_begin` (15 l), `left_single_finish` (32 l), `writes_producer` (19 l), `count_masked` (13 l), `madd_try` (30 l), `addshift_try` (27 l), `cmp_try` (24 l), the push-retraction and rd=0 barrier in `pop`, the expression half of `fntab[3660]` |
| **R4** | destination allocation over x1-x7 + bind-into-symbol | `vs_alloc`/`vs_spill_deepest` go live, `emit_let_stmt` 3230, `emit_let_plain` 2566, `emit_let_in` 2532, `emit_compound_stmt` 3312, `bind_reg` 2120, plus the `vs_sync()` insertions of §1.5 | **k4 11 → 9, k3h 10 → 8, k1h 8 → 6, k2h ~44 → ~40**; new construct `c55_vswindow` frozen | `--codegen` chain; `k3h_loopwords <= 10`, `k4_loopwords <= 13`, `k4_ms <= 3.0`; `c55_vswindow` in `bench/parity_constructs/` with `c55_vswindow) EXPECT=312;;` added to `construct_parity.sh` and `c22/c23/c24/c53` unchanged (L15) | — |
| **R5** | call/return materialisation | `emit_bl_call` 706, `emit_self_call` 759, `emit_return_stmt` 3104, `flush_on_bl` 1684 + `str_reg`/`pop_back`/`ldr_reg` 1691-1717 | **k2h_loopwords ~40 → ~34**, `bin_words` −1 to −2 % | `--codegen` chain; `k2h_ms` not worse; `c08_call`, `c09_recursion`, `c21_param13`, `c23_spillcall`, `c26_selfrec`, `c27_zeroarg`, `c53_param9` re-frozen at recorded deltas | `flush_on_bl`/`str_reg`/`pop_back`/`ldr_reg` (34 l, dead since 4e6a1d6: `rep` is always 0) and the `fntab[3890]` bank cell |

Cumulative deletion: ~300 lines, matching the report's §2 B3 estimate. Kernel numbers are the
expected values **at issue time** (L10); the gate is the inequality, not the point estimate.

### 2.1 Word-pattern matching that dies, and when

| function | line | word patterns it decodes | dead at |
|---|---|---|---|
| `emit_lit` fold cells | 1741 | records `fntab[3655..3659]` | R2 |
| `fold_clear` | 1778 | — (the model it clears is gone) | R2 |
| `fold_try` | 1791 | retracts `cnt`/`cTop` literal chunks | R2 |
| `shl_try` | 1985 | `movz x1,#c` | R2 |
| `mulc_try` | 2006 | `movz x1,#c` | R2 |
| `cmp_try` imm half | 2229 | `movz x1,#imm` | R2 (function survives to R3) |
| `emit_while_stmt` `dead` | 3154 | `movz x0,#imm ; sub sp ; str x0,[sp]` | R2 |
| `pop2` | 1825 | `push a ; P ; push b`, P ∈ {`mov x0,xR`,`ldr x0,[x15,#k]`,`movz x0,#imm`} | R3 |
| `left_single_begin` | 1858 | the same three P forms under a push | R3 |
| `left_single_finish` | 1905 | `bl` imm26 bump, `adr` refusal, end-push check | R3 |
| `writes_producer` | 1873 | `mov xR,x0` / `str x0,[x15,#k]` hazard scan | R3 |
| `count_masked` | 1892 | generic masked count (only caller is `left_single_finish`) | R3 |
| `madd_try` | 1955 | `[mov x0,xA][movz x1,#c][mul][mov x1,x0][mov x0,xB]` | R3 |
| `addshift_try` | 2040 | `[mov x0,xA][lsl][mov x1,x0][mov x0,xB]` | R3 |
| `cmp_try` | 2229 | `mov x0,xR` / `mov x1,xS` before the cmp | R3 |
| `pop` retraction + rd=0 bar | 1662 | `sub sp,#16 ; str x0,[sp]` | R3 |
| `cond_branch_word` | 2475 | `cmp`+`cset` — **kept**, it decodes the *condition* not an operand; only its `fntab[3660]` fix-up at 2490 goes | R3 (partial) |
| `count_word` alloc scan | 3087/3196 | `mov x0,x14` — **kept**, B4 depends on it | — |

---

## 3. Rung 1 in detail — the byte-identical step

**What it is.** A pure indirection layer. Every producer and consumer call site is routed through the
new primitives; the primitives' bodies are today's code verbatim.

- `vs_reset(fntab)` sets `fntab[3797] = 0`, `fntab[3798] = 127`; called from `compile_fn_at` 3760
  beside the `fntab[3660] = 0 - 1` reset (3770).
- `vs_push_const(c)` = today's `emit_lit` body (`movz` + up to three `movk` + `push`) and then records
  **kind 4** — i.e. it materialises into x0 and pushes, `w` stays 0.
- `vs_push_sym(r)` = today's `emit_var` body (`mov x0,x<r>` or `ldr x0,[x15,#slot*8]`, plus the two
  `-1`/`-2` diagnostics at 2107-2113) + `push`; records kind 4.
- `vs_push_reg(r)` = `push` for `r == 0`; no other caller exists at R1.
- `vs_materialise(top, d)` = `pop(insns, n, d, fntab)` — the existing retraction and its `fntab[3660]`
  barrier are untouched.
- `vs_sync()` = no-op (`w` is always 0).
- `vs_alloc` / `vs_spill_deepest` exist and are **unreachable** at R1 (mask never consulted).

**Why the byte stream is bit-for-bit today's.** Three facts, each checkable by reading the diff:

1. `w == 0` is an invariant of R1: the only writers of `fntab[3797]` are `vs_reset` (to 0) and
   `vs_push_*`, and every `vs_push_*` at R1 sets kind 4, which by §1.1 is not a window entry.
   Therefore no branch in `vs_alloc`/`vs_spill_deepest`/`vs_sync` is ever taken.
2. Every `vs_*` body is a *textual move* of an existing body. The word sequence emitted per call is
   unchanged, so `n[0]` after each call is unchanged, so every position-dependent decision downstream
   (the `pop`/`pop2`/`*_try` retraction windows, `fntab[3660]` comparisons, the `p1`/`p2` branch
   patches, `fntab[3663+k]` return positions, `left_single_finish`'s slide) sees identical inputs.
3. `fntab[3797..3822]` are cells nothing else reads, and `fntab[3700]`/`fntab[3701+d]` keep their
   existing writes verbatim (L16: bookkeeping only, no extra words, guarded `0 <= d < 96`).

**Gate, exactly.**

```sh
PROC_CAP=30 tools/chain.sh bebop.bp ${BEBOP_TMP:-/tmp/opencode}/r1
md5sum ${BEBOP_TMP:-/tmp/opencode}/r1/gen2.bin ${BEBOP_TMP:-/tmp/opencode}/r1/gen3.bin   # equal = byte-identical codegen (main-session correction: bebop.bp itself changes at R1, so gen2 != HEAD's bebop.bin; the identity claim is gen2 == gen3 == gen4 plus 0 WORD_MISMATCH on the frozen constructs)
```

`chain.sh` without `--codegen` already fails with `gen3 == gen4 ... but gen2 differs -- codegen
changed: rerun with --codegen` (chain.sh:34) the moment one word moves, so the fixpoint line *is* the
byte-identity test; the explicit `md5sum` is the belt. `FREEZE` must stay unset so
`construct_parity.sh` reports `WORD_MISMATCH` rather than silently re-freezing. **If R1 is not
byte-identical in one commit, stop and re-read §3 rather than patching forward** (report §3.5).

---

## 4. The spill-path construct (D14 item 3)

The window fallback never fires on any committed program (the Sethi-Ullman survey: 0 trees above SU 4).
It must therefore be forced by one synthetic construct, landed **in the R4 commit**.

`bench/parity_constructs/c55_vswindow.bp`:

```
// D14 item 3: force the x1-x7 operand window to spill. Eight call arguments,
// each a live REG tag, are on the value stack at once: the eighth vs_alloc
// finds fntab[3798] empty and vs_spill_deepest pushes argument 1 to [sp].
fn s8(a: i64, b: i64, c: i64, d: i64, e: i64, f: i64, g: i64, h: i64) -> i64 {
  a + b * 2 + c * 3 + d * 4 + e * 5 + f * 6 + g * 7 + h * 8
}
fn main() -> i64 {
  let k = 3;
  s8(k + 1, k + 2, k + 3, k + 4, k + 5, k + 6, k + 7, k + 8)
}
```

**Expected fold: 312.** (`k = 3` → args 4..11; `4 + 10 + 18 + 28 + 40 + 54 + 70 + 88 = 312`.)

Why it spills: `emit_bl_call` 706 evaluates all eight arguments before popping them into x0..x7
(the `while i >= 0 { pop(insns, n, i, fntab) }` loop at 733-737). Each `k + i` is
`vs_binop(add, SYM x19, CONST i)` → one `add x<d>,x19,#i` into a freshly allocated `d`. Seven
allocations exhaust `fntab[3798]`; the eighth calls `vs_spill_deepest`, which emits
`sub sp,sp,#16` ; `str x1,[sp]` for the first argument. `vs_sync()` at the head of the argument
materialisation then pushes the remaining six as well, and the arguments are `ldr`'d back into
x0..x7 in `emit_bl_call`'s existing order.

**Expected fold of the body**: `a + b*2 + ...` is left-nested (`emit_expr` 2397 / `emit_term` 2422),
never more than three live tags — it exists to give the construct a checkable value, not to spill.

Registration in the same commit: add `c55_vswindow) EXPECT=312;;` to the case list in
`bench/vs_rust/construct_parity.sh` (the block starting at its line 47), then freeze with
`FREEZE=1 BEBOP_BIN=<candidate> bash bench/vs_rust/construct_parity.sh` and record the
`WORD_DELTA c55_vswindow 0 -> N` line (`0 = new construct`, no `word_budget.txt` entry needed).

---

## 5. Risks, each with its probe

| # | risk | probe construct | what it looks like when it fires |
|---|---|---|---|
| 1 | **R1 is not byte-identical** — a `vs_*` body drifts by one word or one `fntab` write | the chain itself: `chain.sh` non-codegen on `bebop.bp` | `gen3 == gen4 ... but gen2 differs` on a commit that claims zero codegen change |
| 2 | **Planning/emission disagree on `n[0]`** — a rung makes word count depend on the allocated register (the T96 step-1 bug class: ordfsm, every `bl` 3 words late, HISTORY.md:2684) | `bench/vs_rust/std_tests/ordfsm.bp` via `std_par.sh`, and `c26_selfrec` | SIGBUS / wrong values in a *large* program while every small construct passes |
| 3 | **A window register survives a barrier** — a tag in x1-x7 is live across a `bl`, a builtin, or a label | `c08_call`, `c23_spillcall`, `c46_andor` (nested if inside a call arg), plus `fn p(x: i64) -> i64 { (x + 1) * (x + 2) + clz(x + 3) }` — a builtin between two live tags; `p(4) == 30 + clz(7) == 30 + 61 == 91` | a value silently replaced by a syscall return or a callee's x1 |
| 4 | **The spill fallback is never exercised**, so its first firing is in production | `c55_vswindow` (§4), plus the deep-nesting twin: `fn nest(p: i64) -> i64 { let a = p+1; let b = p+2; let c = p+3; let d = p+4; let e = p+5; let f = p+6; let g = p+7; let h = p+8; (a*2+1) + ((b*2+1) + ((c*2+1) + ((d*2+1) + ((e*2+1) + ((f*2+1) + ((g*2+1) + (h*2+1))))))) }` — `nest(10) == 240`, and its 9th symbol `h` also exercises the `SYM 100+slot` path | wrong value, or a `WORD_DELTA` that grows because the spill emits more than push/pop |
| 5 | **`MULC` provenance outlives its register** — the register in `MULC r,c` is reallocated before the tag is consumed | `c19_multi`, `c20_deep`, `c31_nested_lit`, and K4 itself. Rule to enforce: `vs_alloc` may never return a register named by a live `MULC`/`REG` payload; the mask is the single source of truth and `vs_push_mulc(r,c)` with `r < 19` must **not** free `r` | a value that is one multiply stale |
| 6 | **Census regression** — the tag predicates add `if` chains to the emitter, so `bebop.bin`'s own `b.cond` count grows (this happened at T104b: 1542 → 1559, and at P3: 1588 → 1604) | `invariants.sh --freeze` lane in the battery | `census.txt NOT frozen (D11-F: add the allow lines to census_allow.txt)` — add them in the same commit, it is a recorded increase, not a failure |
| 7 | **The `left_single_finish` slide is deleted while something still depends on its `bl` imm26 bump** | `c08_call`, `c09_recursion`, `c47_usenest`, `c44_use24` | every `bl` in a slid expression off by 3 words — the b4326b5 / DIVERGE-42122 neighbourhood. R3 removes the slide entirely rather than adjusting it, which is why it is safer than it looks |
| 8 | **Symbol-slot overflow becomes visible** — x15's spill region is `[sp+256 .. sp+768]` = 64 slots but `stab` allows 128 symbols (`sym_bind` 188, `compile_fn_at` 3778 `zeros(385)`) | pre-existing, not introduced here; noted so the agent does not "fix" it inside a rung | a 65th spilled symbol writes into the frame heap |

---

## 6. What B2 and B5 change

**B2 (if-expression value in x0 at the join).**

- *If B2 lands first* (assumed): `emit_cond` 2496 ends with the arms' values in x0 and one
  `vs_push_reg(0)` after `end_pos`. R1 wraps that single push; R3 turns it into a `REG 0` tag; R4 must
  **reset the free mask at the join** (`fntab[3798] = 127`, `w = 1` with the single `REG 0` entry),
  because the two arms allocate independently and only x0 is guaranteed live on both paths. That reset
  is the whole B2 interaction — one line in `emit_cond`.
- *If B2 lands after*: nothing in R1-R5 changes. `emit_cond`'s two arm-ending pushes and the consumer's
  pop are already sync points by §1.5, so B2 becomes a strictly smaller diff on top of R3 than it is
  today (no `fntab[3660]` reasoning, just "push a `REG 0` tag instead of syncing").
- **Independent of B2: R1, R2, R3, R5.** Only R4 carries the join-reset line.

**B5 (loop rotation, bottom test).**

- *If B5 lands first* (assumed): `emit_while_stmt` 3154 emits `[b .test][body][.test: cond][b.cond
  .body]`. There are then **two** barrier positions per loop instead of two (`.body` and `.test`), so
  §1.5's rule is unchanged in kind — the agent syncs at both, and at R4 resets the mask at both.
  The `pcond` re-parse B5 introduces re-runs `emit_cmp` over the same source with an empty tag stack,
  which is exactly the state a sync leaves behind, so the two emissions of the condition are identical
  word-for-word — that is what keeps B5's own fixpoint.
- *If B5 lands after*: R1-R4 are unchanged; B5's diff gains one `vs_sync()` before the back edge.
- **Independent of B5: R1, R2, R3, R4, R5.** B5's only coupling is the number of sync sites, and the
  −1 word per loop it buys is additive with every rung's kernel numbers above (K4 11→10 at R3, 9→8 at
  R4; K3H 10→9 / 8→7; K1H 8→7 / 6→5).

**Ordering.** B1 (landed), B2, B5, then R1..R5 in order. R1's md5 claim is relative to whatever
`bebop.bin` is at the moment R1 is committed, so if B2 or B5 land between the writing of this
blueprint and R1, re-derive the baseline md5 — do not carry a stale one into the commit message.

---

VERDICT: blueprint written, rungs: 5, first-rung md5 claim: byte-identical, open questions for the
operator: (1) the report's §3.3 spill tag is `[x15,#s*8]`; this blueprint uses the machine stack
(`sub sp,#16 ; str`) instead — unbounded, byte-identical to today's spill words, and free of any
collision with `sym_bind`'s x15 slots — please ratify or override before R4; (2) `MULC` is a sixth tag
kind not named in D14 item 2, and it is what lets `madd_try`/`shl_try`/`mulc_try`/`addshift_try` be
deleted rather than kept as word peepholes — ratify or say to keep the word-level peepholes;
(3) `k3h_loopwords <= 10` is proposed as a new gated PERF row at R4 (D12-B gates only K4 today) —
confirm it should be a gate and not just a reported number.
