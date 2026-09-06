Status: 2026-09-06 (session 18) -- written by the main session (Fable) after reading the whole emitter (bebop.bp 181-560, 706-765, 1416-1560, 1630-3000, 3120-3550, 3620-3995 at HEAD 69e0eb5 + the B5 working tree). Operator decisions of 2026-09-06 (AskUserQuestion, session 18): (1) B5 lands first; (2) values that outlive the register window go to callee-saved registers, then to x15 frame slots sized by the planning pass -- never to `[sp]`; (3) the new emitter lands as ONE commit (big bang), not as the R1-R5 rungs of docs/IR-RUNG-BLUEPRINT.md. That document is SUPERSEDED by this one; its §1.4 MULC table, §1.5 barrier inventory and §5 risk table are reused below where still true.

# The register model -- executable blueprint (one commit)

Goal: **the stack machine is gone.** No `sub sp,sp,#16 ; str x0,[sp]` / `ldr xN,[sp] ; add sp,sp,#16`
anywhere in emitted code. Every expression value lives in a register or, when it cannot, in a
callee-saved register or an x15 frame slot chosen at compile time. The compile-time operand list
("window", §1) is allocator bookkeeping and emits nothing.

Baseline (bebop.bin at B5 fixpoint f7a25d38, 68229 words): 6790 `sub sp,sp,#16`, 6790 `str x0,[sp]`,
1953 `ldr x0,[sp]`, 1978 `ldr x1,[sp]`, 6801 `add sp,sp,#16` = ~24 k of 68 k words are stack traffic.

Every instruction word in this document is **asm text**. The worker derives each number by `as` ->
`objdump -d` -> python int, records the listing in `$OUT/words.objdump` BEFORE editing bebop.bp
(L1; `tools/check_words.py` in the battery enforces it), and never types a literal from memory.
Register-parameterised forms are derived once per FORM with the field offsets read from the
listing (rd = bits 0-4, rn = bits 5-9, rm = bits 16-20, imm12 << 10, imm16 << 5, imm19 << 5,
imm26 << 0).

---

## 0. Scope of the one commit

In: everything in §1-§7. Out (separate commits after this one lands, in this order): T52 `csel`
for pure `if` arms (K8), B4 frame *shrink* (this commit publishes the facts B4 needs: symbol
spills, temp slots, while depth, alloc), the `docs/IR-RUNG-BLUEPRINT.md` deletion.

Fixed points that do NOT change: the grammar and bpref.py; `sym_bind` numbering (symbols 1..8 =
x19..x26, 9+ = x15 slot `100+k`); the 16 KiB frame and `x15 = sp+256`, `x14 = sp+1024`; `emit_bl`
and its B1 `stp/ldp x15,x14` conditional; the T43 while mark (`pmark` nop / `str x14,[sp,#80+8*d]`)
and `loop_alloc_safe`; the T118 heap trap words; `emit_prologue*`/`emit_epilogue*` word forms
(their *conditions* change, §5); B5's loop shape; the return/break jump lists (3662+, 3680+); the
literal data section (3899-3903); the struct guard (3891/3892); every `emit_sys_*` hand-word block
(only its operand delivery changes, §3.9).

---

## 1. Model

### 1.1 Registers

| class | registers | who allocates | lifetime |
|---|---|---|---|
| **window** (caller-saved temps) | x0..x7, allocation order x0 first (lowest free bit) | `vs_alloc` | within one expression; dies at any call-like site (§3.8) |
| **cs temps** (callee-saved) | x(19+stab[0]) .. x26, lowest first | `vs_park` | across calls/builtins/branches, within one expression |
| **temp slots** | `[x15, #(S + k)*8]`, S = number of spilled symbols (`vc-8` if `vc > 8` else 0), k = 0.. | `vs_park` when no cs temp is free | same as cs temps |
| symbols | x19..x26 / `[x15,#k*8]` | `sym_bind` (unchanged) | fn |
| arg registers | x0..x13 (params 9-14 arrive in x8..x13 exactly as today) | call placement §3.8 | one `bl` |
| scratch | x16 | parallel move only (§3.8) | one word |

x0 is a window register like any other; a call/builtin result is the tag `REG x0`. A call site that
needs x0 for arg 0 while another tag owns x0 *evicts* it (moves it to another free window register
with `mov`, else parks it, §1.4).

**cs temps are the bottom-up run above the symbols.** At park time the lowest free register in
x(19+stab[0])..x26 is taken. If `sym_bind` later needs a register that a live cs temp owns (a
`let … in` inside the rest of the same expression), the compiler exits **89** (new row in the
exit-code table at `emit_paren` and in docs/TRAPS.md: "let binding while a call temp is live --
bind the call result with a let first"). Verified free: 87 (unresolved callee), 88 (cas_verify),
90-94 (seed loader), 95-99 (parser). bpref.py raises the same shape (the fuzzer must predict it,
TG-DONE 8).

### 1.2 Tags

The window is a compile-time list of `w` entries (`fntab[3797]`), entry `i` = 0 (deepest) ..
`w-1` (top). Every expression emitter leaves **exactly one new entry** on top; statement
emitters leave none.

| kind | name | payload0 | payload1 | emits on push | materialise into x<d> |
|---|---|---|---|---|---|
| 1 | `CONST c` | c | -- | nothing | `movz x<d>,#lo16` + `movk … lsl #16/32/48` per non-zero half (emit_half logic, dest parameterised); negative c as today (emit_half normalises) |
| 2 | `REG r,p` | r in 0..7 | p = index of the producing word, or -1 | nothing (r was just written) | `mov x<d>,x<r>` -- or **retarget** (§1.5) |
| 3 | `SYM r` | r in 19..26, or 100+k | -- | nothing | `mov x<d>,x<r>` / `ldr x<d>,[x15,#k*8]` |
| 4 | `CS r` | r in 19..26 | -- | nothing | `mov x<d>,x<r>` |
| 5 | `SLOT k` | k | -- | nothing | `ldr x<d>,[x15,#(S+k)*8]` |
| 6 | `MULC r,c` | r = any register of kinds 2/3/4 (its own tag was consumed) | c in 1..65535 | nothing | IR-RUNG-BLUEPRINT §1.4 table (lsl / add-shifted / lsl+sub / add-shifted+lsl / movz+mul); the multiplier register when needed is `vs_alloc`'d and freed |
| 7 | `FLAGS cond` | AArch64 cond code 0..15 | -- | nothing | `cset x<d>,<cond>` |

Rules: a `FLAGS` entry is only ever the **top** entry -- `vs_push_*` of anything while the top is
`FLAGS` first materialises it into a fresh window register (so no flag-setting word can slip
between the `cmp` and its consumer). A `MULC` whose `r` is a window register keeps that register
owned until the `MULC` is consumed or materialised. A `REG` register is owned by exactly one tag.

### 1.3 State cells (all per-fn, reset by `vs_reset` in `compile_fn_at` beside the 3661/3662/3680 resets)

Correction 2026-09-06 (session 18, after the worker hit it): the window is a compile-time LIST --
only REG / MULC-on-window / FLAGS entries need a register, and a call has up to 14 pending
arguments (emit_body takes 10) while an array literal may have ~99 pending elements. The entry
array is therefore NOT tied to the 8 registers: it lives in the free zone fntab[2000..2383]
(nothing is written between 1803 and 3654: fn zones end below ~1100 with 256 fns, facts are
1500+i and 1800-1802) with capacity 128 entries.

```
fntab[3797]        w                       window entry count, 0..128 (a push at 128 = compile-time exit 89)
fntab[3798]        free mask over x0..x7   bit r set = free; reset = 255
fntab[2000 + 3i]   kind of entry i         i = 0..127
fntab[2001 + 3i]   payload0
fntab[2002 + 3i]   payload1
fntab[3823]        cs mask                 bit (r-19) set = cs temp r LIVE (owned); reset = 0
fntab[3824]        temp slot cursor k      next free temp slot; reset = 0
fntab[3825]        cs_hi                   highest cs register index used (r-18, 0 = none), max over the fn
fntab[3826]        tsp                     max temp slots used, max over the fn
fntab[3827]        S                       spilled-symbol count the emission pass uses for slot addresses (planning pass: 0)
```

`tools/check_abi.py:167` zones: `(3655, 3661, "fold")` becomes `(3661, 3661, "whiledepth")`,
`(3700, 3796, "slots")` becomes `(3797, 3798, "window_hdr")`, plus `(2000, 2383, "window")` and
`(3823, 3827, "window_cs")`; 3655-3660, 3700-3796 and 3890 are retired (§6).

**Entry count cap.** `w == 128` and another push: compile-time exit 89 ("pending operands >
128"). `vs_park_deepest` is only the fallback of `vs_alloc` when the free MASK is empty (it
frees a register by parking the deepest register-owning entry IN PLACE; it never changes `w`).

### 1.4 Primitives (new functions, replace push/pop/fold/pop2/left_single/*_try)

- `vs_reset(fntab)` -- cells 3797, 3798=255, 3823=0, 3824=0, 3825=0, 3826=0.
- `vs_push(fntab, kind, p0, p1)` -- FLAGS-on-top rule, cap rule (exit 89 at 128), then append; a REG push clears the register's mask bit (a `bl`/builtin result arrives in x0 without `vs_alloc`).
- `vs_pop(fntab)` -- drop the top entry; if it owned a window register (REG, or MULC with r < 8)
  free the bit; if it was CS free the cs bit; SLOT frees nothing (slots are bump-allocated per
  expression statement and the cursor resets to 0 whenever `w` returns to 0 -- statement boundary).
- `vs_alloc(fntab)` -- lowest set bit of 3798, clear it, return r. Empty mask: `vs_park_deepest`
  then retry (it always frees one because every window register is owned by a window entry).
- `vs_reg(i)` -- the register that holds entry i **without emitting a word** when the kind is
  REG / SYM (r < 100) / CS; otherwise `vs_alloc` + `vs_mat(i, d)` and the entry becomes `REG d,p`.
  Used for operands of register-parameterised forms.
- `vs_mat(i, d)` -- materialise entry i into x<d> per §1.2; the entry becomes `REG d,p'` (p' = the
  index of the last word emitted, or -1 for movk-terminated constants and for `MULC` forms whose
  last word reads its own rd -- none of the §1.4 table forms do, so only movk sets -1); frees the
  old register when the entry was REG (r != d) or CS.
- `vs_park(fntab, keep)` -- for every entry i in 0..w-1-keep whose kind is REG or MULC-on-window
  or FLAGS: materialise MULC/FLAGS into its own fresh window register first, then move the
  register to a free cs temp (`mov x<cs>,x<r>`, entry becomes `CS cs`) or, when
  `19 + stab[0] + popcount(cs mask)` exceeds 26, to a temp slot (`str x<r>,[x15,#(S+k)*8]`, entry
  becomes `SLOT k`, k = cursor++, tsp = max). The window register is freed. Word count: exactly one
  word per parked entry either way (this is what keeps planning and emission in agreement, §4).
- `vs_park_deepest(fntab)` -- `vs_park` restricted to the deepest entry that owns a window register (the empty-mask rule only); in place, `w` unchanged.
- `vs_evict(fntab, r)` -- if a window entry owns x<r>: `mov x<r'>,x<r>` into another free window
  register if any (entry becomes `REG r',-1`), else park that one entry. Then x<r> is free.
- `vs_bind(fntab, insns, n, reg)` -- consume the top entry into symbol register/slot `reg`:
  `reg < 100`: REG with `p == n[0]-1` -> retarget (§1.5); REG otherwise / CS / SYM -> `mov`; CONST
  -> movz/movk into x<reg>; FLAGS -> `cset x<reg>,<cond>`; MULC -> materialise into x<reg>;
  SLOT -> `ldr x<reg>,[x15,#…]`. `reg >= 100`: `vs_reg` then `str x<r>,[x15,#(reg-100)*8]`.
  Trap words for reg = -1/-2 stay as `bind_reg` has them today.
- `vs_to(fntab, insns, n, d)` -- materialise the top into a fixed window register d (fn tail and
  `return` use d = 0; builtin operand delivery uses 0..2): `vs_evict(d)` unless the top owns it,
  then `vs_mat(top, d)` (retarget when `p == n[0]-1`).

### 1.5 Retarget (the only stream edit left)

A `REG r,p` entry with `p == n[0] - 1` is the value of the last word emitted and that word's
rd field is r. Binding it to x<S> (or delivering it to x<d>) rewrites bits 0-4 of `insns[p]` to
S/d instead of emitting `mov`. It is always semantics-preserving on AArch64 because every integer
instruction reads its sources by number (`add x1,x1,x20` retargeted to `add x19,x1,x20` still reads
the old x1); the single exception, `movk`, never carries a valid p. One positional rule (correction, session
18): retarget is only valid when NO label sits at n[0] (right after the producing word) -- at the
`if` join the else-arm's last word writes d and the then-arm jumps to `end_pos` with the value in d,
so renaming that word would leave the then-path's value in the wrong register. `emit_cond` therefore
pushes `REG d,-1` (§3.5); no other label follows a producing word (B5's `endl` follows the backward
branch, `else_pos` follows the `b`). A label AT p (the word itself is a target) is harmless. No other function
may read or rewrite emitted words: `fold_try`, `pop2`, `left_single_*`, `madd_try`, `shl_try`,
`mulc_try`, `addshift_try`, `cmp_try`, `cond_branch_word`, the `pop` retraction and its rd=0 bar,
and the `emit_while_stmt` `dead` retraction all go (§6). `fntab[3660]` is retired with them.

---

## 2. Binary operators on tags (`vs_binop(op, fntab, insns, n)`; consumes the top two entries a, b; pushes one)

Order of tests; the first match emits. `d` = a's register when a is REG (reuse), else b's when b is
REG, else `vs_alloc`. Ops: 2 add 3 sub 4 mul 5 div 11 mod 12 and 13 orr 14 eor 15 lsl 16 lsr 17 asr.

| a | b | op | words | result |
|---|---|---|---|---|
| CONST | CONST | any | nothing | `CONST` folded (div/mod by 0: fall through to registers so the runtime traps as today) |
| any | CONST c, 0 <= c < 4096 | add/sub | `add/sub x<d>,x<a>,#c` | REG d |
| any | CONST c, -4096 < c < 0 | add/sub | `sub/add x<d>,x<a>,#-c` | REG d |
| any | CONST 2^k | mul | nothing | `MULC a,2^k` |
| any | CONST c, 2 <= c < 65536 | mul | nothing | `MULC a,c` |
| any | CONST k, 0 <= k < 64 | lsl/lsr/asr | `lsl/lsr/asr x<d>,x<a>,#k` (ubfm/ubfm/sbfm aliases from `as`) | REG d |
| any | MULC r,2^k | add | `add x<d>,x<a>,x<r>,lsl #k` | REG d |
| MULC r,2^k | any | add | `add x<d>,x<b>,x<r>,lsl #k` | REG d |
| any | MULC r,c | add | `movz x<m>,#c` ; `madd x<d>,x<r>,x<m>,x<a>` (m = vs_alloc, freed) | REG d |
| MULC r,c | any | add | same with the roles swapped | REG d |
| any | MULC r,2^k | sub | `sub x<d>,x<a>,x<r>,lsl #k` | REG d |
| MULC / FLAGS / CONST / SLOT / SYM(slot) | -- | other | materialise via `vs_reg` first, then the row below | |
| reg | reg | add sub mul div and orr eor lslv lsrv asrv | `op x<d>,x<a>,x<b>` (mul = `mul`, div = `sdiv`) | REG d |
| reg | reg | mod | `sdiv x<t>,x<a>,x<b>` ; `msub x<d>,x<t>,x<b>,x<a>` (t = vs_alloc, freed) | REG d |

`MULC` materialisation (when a MULC is used as a plain register operand) is IR-RUNG-BLUEPRINT
§1.4 verbatim: `2^k` -> `lsl x<d>,x<r>,#k`; `2^k+1` -> `add x<d>,x<r>,x<r>,lsl #k`; `2^k-1` (c >= 3)
-> `lsl x<d>,x<r>,#k ; sub x<d>,x<d>,x<r>`; `2^j+2^k` -> `add x<d>,x<r>,x<r>,lsl #(k-j) ; lsl
x<d>,x<d>,#j`; otherwise `movz x<m>,#c ; mul x<d>,x<r>,x<m>`. Logical-immediate encodings are NOT
used (and/orr/eor with a constant materialise it: one movz + one op).

**Comparisons** (`vs_cmp(cond)`, consumes a, b, pushes `FLAGS cond`): b `CONST c` in 0..4095 ->
`cmp x<a>,#c`; a `CONST` and b a register -> `cmp x<b>,#c`/`cmp x<b>,x<a>` with the mirrored cond
(eq/ne unchanged, lt<->gt, le<->ge); both CONST -> `CONST (0/1)`; else `cmp x<a>,x<b>`. A `FLAGS`
tag stores the REAL AArch64 condition number (eq 0, ne 1, ge 10, lt 11, gt 12, le 13). `b.<cond>`
encodes that number; `cset x<d>,<cond>` encodes its INVERSE in its cond field (today's
`emit_cmp_op` constants -- "eq=1 ne=0 lt=10 gt=13" -- are those inverted field values, and
`cond_branch_word` relied on it). Both encodings come from the listing, never from this sentence.

**Unary**: `-e`: CONST -> CONST(-c); else `neg x<d>,x<a>` -> REG. `!e`: CONST -> CONST(c==0);
FLAGS c -> FLAGS (c ^ 1), no word; else `cmp x<a>,#0` -> FLAGS eq.

---

## 3. Every emitter, what it becomes

Parsing is untouched: the same functions, the same `pos` arithmetic. Only the codegen half of each
changes. "Push X" below means `vs_push`.

3.1 **Literals** `emit_lit`/`emit_num`/`emit_negnum`: push `CONST c`. `emit_half`'s per-half
normalisation moves into `vs_mat` for CONST (dest-parameterised). `emit_str`: `adr x<d>,<lit>`
with d = vs_alloc -> `REG d,p`.

3.2 **Variables** `emit_var`: push `SYM reg`. The two diagnostics stay: reg = -2 (table full)
emits the trap word as today; reg = -1 (unbound name) emits today's `mov x0,x0` word after
`vs_evict(0)` and pushes `REG 0,-1` (§7 risk 9).

3.3 **Binops / cmp / unary**: `emit_apply_op/mul/mod/bits/cmp` call `emit_term/factor/...` for the
right operand exactly as today, then `vs_binop` / `vs_cmp`. `left_single_begin/finish` calls go.
`emit_unary` per §2.

3.4 **Parens**: unchanged (the inner expression leaves one entry).

3.5 **`if c then a else b`** (`emit_cond`):
```
emit_cmp (cond)                          -- top: FLAGS / REG / SYM / CONST
vs_park(1)                               -- deeper entries become path-independent
branch word:  FLAGS c -> b.<inv c> ->else ; register r -> cbz x<r>,->else ; CONST -> materialise, cbz
vs_pop                                   -- the cond entry (frees r if it was a temp)
d = vs_alloc()                           -- the join register, allocated BEFORE the arms
save = (w, mask, cs mask, slot cursor)
then-arm: emit_cmp ; vs_to(d) ; vs_pop ; restore save (d stays owned)
b ->end (placeholder, patched as today)
else_pos: else-arm: emit_cmp ; vs_to(d) ; vs_pop ; restore save
end_pos:  push REG d,-1
```
`b.<cond>` / `cbz` placeholders are patched by adding `(target - p) * 32` as today. The old
`cmp x0,#0 ; b.eq` pair and `cond_branch_word` are gone. bpref/interpreter semantics unchanged.

3.6 **`while`** (B5 shape, kept): at entry `vs_park(0)` (statement context; the window is
normally empty here, the park makes it exact for the inline-call path). Body via `emit_body`; a
final expression value is dropped with `vs_pop` (no word). Then the bottom test: `emit_cmp` ->
FLAGS c -> `b.<c> ->body_start` (the NON-inverted cond: the branch is taken to continue); register
-> `cbnz x<r>,->body_start`; CONST -> materialise, `cbnz`. The T43 mark, `do_reset`, `patch_jumps`
for `break`, the `x14` reset words and `fntab[3661]` are untouched. `fntab[3660]` writes go.

3.7 **Statements** (`emit_let_stmt`, `emit_let_plain/chain`, `emit_compound_stmt`, `emit_body`,
`emit_return_stmt`, fn tail in `compile_fn_at`/`compile_fn`/`compile`): RHS via `emit_cmp` then
`vs_bind(reg)`. `emit_compound_stmt` (`x += e`): push `SYM x`, RHS, `vs_binop`, `vs_bind(x)` -- the
retarget makes `let i = i - 1` / `i -= 1` one word. **`let _ = e;` and `let _ = e in …`** (name
hash 95, the discard convention: `_` is never read; bpref treats it the same) do `vs_pop` instead
of `vs_bind` -- no word; `sym_bind("_")` is still called so `stab` and the facts stay as today.
`emit_body` drops non-final expression values
with `vs_pop` (it decides "left a value" by `w` after vs. before the item, not by `n[0]`: the
`grew`/`nb_item` logic goes). Fn tail: `vs_to(0)` then `vs_pop`; `return e;`: `vs_to(0)`, `vs_pop`,
`b` placeholder as today. `emit_let_in`: bind, then the body expression leaves its entry.

3.8 **Calls** (`emit_bl_call`, `emit_self_call`; `emit_call` inline path keeps its
evaluate-then-bind loop using `vs_bind(param)` and `emit_body` unchanged):
```
args: emit_cmp per argument (each leaves one entry; nargs entries on top)
vs_park(nargs)                     -- every deeper window temp survives the call in cs/slot
placement (parallel move into x0..x(nargs-1), x8.. for 9+):
  repeat until no change: for i in 0..nargs-1 with entry_i not already REG x<i>:
     if x<i> is free (no window entry owns it): vs_mat(entry_i, i)
  if entries remain (a cycle among REG entries): mov x16,x<r_j> for one of them, free r_j, mark it "in x16",
     continue the loop; at the end mov x<j>,x16
then vs_pop x nargs; rep/flush_on_bl/pop_back are deleted (rep is always 0 since 4e6a1d6)
emit_bl(target, 1 - fntab[1801]) exactly as today
push REG 0,-1 (x0 owned; mask bit 0 cleared)
```
x1..x7 are all free after the call (nothing lived there: parked). The `brk #87` unresolved-callee
path pushes `REG 0,-1` after the brk. tools/check_abi.py's `ARGPASS` set (`ldr/mov x9..x13`) is
generalised to "any word whose rd is x8..x13 and that sits within the nargs words before a `bl`
(or before `stp x15,x14` + `bl`)"; c21_param13 and c53_param9 are the probes.

3.9 **Builtins** (all 39 `emit_sys_*`, `emit_clz`, `emit_crc32*`, `emit_hvham*`, `emit_clock_ms`,
`emit_str_len_fn`, `emit_char_fn`, `emit_zeros`, `emit_sys_slurp`, `emit_sys_readbuf`, …): the
hand-word block is untouched. Operand delivery replaces the `pop(insns,n,k,fntab)` sequence:
`vs_park(nops)` then the §3.8 placement into x0..x(nops-1) (the old `pop 2 ; pop 1 ; pop 0` order
is "top -> x2", i.e. argument k -> x<k> -- identical), then `vs_pop` x nops. Result: the trailing
`push` becomes `push REG 0,-1`; builtins that return nothing push `CONST 0` if today's code pushed
a 0. The hand blocks may clobber any of x0..x13 and the flags: the park guarantees no window temp
and no FLAGS entry is live across them.

3.10 **Register-parameterised builtins** (these emit forms with the operand registers read from
the tags, no delivery moves): `emit_array_get`: base = `vs_reg`, index CONST c < 4096 ->
`ldr x<d>,[x<base>,#c*8]`, else `ldr x<d>,[x<base>,x<idx>,lsl #3]` -> REG d. `emit_array_set`:
`str x<v>,[x<base>,#c*8]` / `str x<v>,[x<base>,x<idx>,lsl #3]`, pops the three, pushes `CONST 0`.
`emit_field_access`: `ldr x<d>,[x<base>,#idx*8]`. `emit_clz`: `clz x<d>,x<a>`.

3.11 **Allocations** (`emit_array_lit`, `emit_struct_lit`, `emit_enum_ctor`,
`emit_enum_ctor_nullary`): allocate FIRST, then fill -- no value stack, constant pressure:
```
vs_park(0) ; vs_evict(0)
mov x0,x14 ; add x14,x14,#8*n ; T118 trap words (unchanged, they use x2 and the flags -- nothing live)
push REG 0,-1                                 (the base; `mov x0,x14` stays the literal alloc word count_word scans for)
for each element/field/payload: emit_cmp ; r = vs_reg(top) ; str x<r>,[x<base>,#i*8] ; vs_pop
```
`x<base>` is `vs_reg` of the base entry at store time (it may have been parked by a call inside an
element: then it is `CS r` or reloaded from its slot). The T42 store-after-bump order is preserved
by construction and the nested-ctor case (`Some(Some(1))`: today both write their tag at `[x14]`
before either bumps) is fixed as a side effect -- construct c60_nestctor, §7. Enum ctor tag word:
`movz x<t>,#tag ; str x<t>,[x<base>]` with t = vs_alloc. Struct field order via `slots[]` as today.

3.12 **`match`** (`emit_match`, `emit_match_arm`): compile-time dispatch, no branches: the
payload's `emit_cmp` is followed by `vs_bind(var)` (the hand `ldr x0,[sp] ; add sp,#16` goes); the
arm body leaves its entry. `fold_clear` call goes.

3.13 **Prologue/epilogue/params** (`compile_fn_at`, `compile_fn`): unchanged words; §5 conditions.

---

## 4. Planning/emission agreement (the invariant that makes two passes agree on n[0])

Both passes run the same code on the same text. The emitted word count of any expression must be
independent of (a) which window register was allocated, (b) whether a parked value went to a cs
temp or a slot, (c) which cs temp. (a): every form is one word for any register field. (b): park
= 1 word either way, reload = 1 word either way, placement = 1 word either way. (c): same.

What differs between the passes and is already corrected arithmetically by B1 (`total_saved`
in `compile_fn_at`): prologue/epilogue pairs, the x15/x14 setup words, and the `stp/ldp x15,x14`
around each `bl`. This commit extends that correction (§5) and adds nothing else that differs.
Slot addresses differ (planning S = 0) -- address fields never change word counts.

`vs_reset` runs at the top of `compile_fn_at` (and `compile_fn`, `compile`) so no state leaks
across fns or passes (T96 step 1 lesson: a leaked cell moved every `bl` by 3 words).

---

## 5. Facts, frame and the prologue (B1 mechanism extended)

The planning pass publishes per fn (fntab[1802] -> copied by `compile_program_offs` into
`fntab[1500+i]`, read back by `fntab_fact_lookup` on the emission pass):

```
fw = real_vc + 256*real_alloc + 512*cs_hi + 8192*tsp        (real_vc <= 128, cs_hi <= 8, tsp < 64)
```

Emission-pass decisions (`emit_prologue_sized` / `emit_epilogue_sized` / `fntab[1801]`):
- callee-saved pairs: pair k (k = 0..3) kept iff `max(real_vc, cs_hi) > 2k` (cs_hi counts
  registers 19.. as 1.., so a cs temp in x20 needs pair 0 only: fib pays nothing extra).
- `needs15` = `real_vc > 8` or `real_alloc` or `tsp > 0` (temp slots need x15).
- `fntab[1801]` skip-save = NOT (`real_vc > 8` or `real_alloc` or `tsp > 0`).
- `S` (fntab[3827]) = `real_vc - 8` if `real_vc > 8` else 0; on the planning pass 0.
- **overflow trap**: `S + tsp > 64` -> compile-time exit 89 (same code as §1.1; both are "register
  pressure this compiler does not spill further"). The 64-slot region is `[sp+256, sp+768)`; the
  frame stays 16 KiB in this commit (B4 shrinks it later using these facts).

Planning-pass arithmetic correction: `saved_pairs01` is computed from `max(real_vc, cs_hi)`;
`real_needs_save` includes `tsp > 0`; everything else as B1 left it. The unsized
`emit_prologue`/`emit_epilogue` used by the planning pass keep today's exact word count.

---

## 6. Deletions (same commit)

Functions: `push`, `pop`, `flush_on_bl`, `str_reg`, `pop_back`, `ldr_reg`, `fold_clear`,
`fold_try`, `pop2`, `left_single_begin`, `writes_producer`, `count_masked`, `left_single_finish`,
`madd_try`, `shl_try`, `mulc_try`, `addshift_try`, `emit_binop`, `emit_binop_plain`,
`emit_binop_regs`, `emit_binop_regs_plain`, `cmp_try`, `emit_cmp_op`, `emit_cmp_regs`,
`cond_branch_word`, `bind_reg` (folded into `vs_bind`; keep its trap words), `emit_lit`'s fold
bookkeeping. `count_word` stays (T43 + B1 use it).

Cells: 3655-3660 (fold + barrier), 3700-3796 (legacy depth), 3890 (bank). `check_abi.py` zones
per §1.3. `self_check`'s `compile("…") == <checksum>` constants (bebop.bp:4232-4240) are
re-derived from the new emitter (they are word checksums, not values) and the diag lane that
runs them must stay green.

Expected net: about -450 lines, +300 lines.

---

## 7. Gates, constructs, expectations

**The gate is the chain**: `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen`
must print `chain: fixpoint gen3 == gen4` and `battery: GREEN` (FREEZE=1 implied: every construct
is re-frozen with its `WORD_DELTA` line; any construct that GROWS needs a
`bench/parity_constructs/word_budget.txt` line with the reason; the census will move -- add the
printed `census_allow.txt` line and `invariants.sh --freeze`; the words lane needs
`$OUT/words.objdump` from before the edit).

**New invariant** (same commit): tools/perf.py `EXACT` gains `push_words` = count of
`str x0,[sp]` + `ldr x0,[sp]` + `ldr x1,[sp]` + `sub sp,sp,#16` + `add sp,sp,#16` words in the code
region of the .bin (after `stub_words`); `bench/vs_rust/invariants.sh` fails when `push_words != 0`
for `./bebop.bin`. docs/PERF.md shows the row.

**Kernel expectations** (loop words = today's `k*_loopwords` metric, B5 fixpoint in brackets):

| kernel | today | expected | gate added in this commit |
|---|---|---|---|
| K4 `let v = (v + i*7)*3 - 11; let i = i - 1` | 14 (B5) | 7: `movz;madd` (v + i*7) ; `add t,t,t,lsl #1` ; `sub x19,t,#11` ; `sub x20,x20,#1` ; `cmp x20,#0 ; b.gt` | `k4_loopwords <= 13` (D12-B, stays); `k4_ms <= 3.0` OR `k4_ms <= 1.15 x` the Rust honest twin measured in the same honest.sh run (the twin is 3.25 ms on this box, so the absolute 3.0 ms figure of D12-B predates the honest twins and cannot be met without beating Rust; the ratio form is the gate) |
| K1H `let s = s*3 + i` | 10 | 5 | `k1h_loopwords <= 8` |
| K3H inner `a*3 + x*2 + y*3` | 24 | 7 | `k3h_loopwords <= 10` |
| K2H fib (fn words) | 51 | ~21: prologue 4, `mov x19,x0`, `cmp x19,#2 ; b.ge`, `mov x0,x19 ; b`, `sub x0,x19,#1 ; bl ; mov x20,x0 ; sub x0,x19,#2 ; bl ; add x0,x20,x0`, epilogue 4 | `k2h_loopwords <= 30` |
| bin_words | 68229 | < 55000 (report; not a gate) | -- |

A gate is added in the commit that first meets it, never before (L10). `honest.sh` row for the
new bin in `bench/vs_rust/REPORT-honest.md` (K1H/K2H/K3H/K4 ms and x vs Rust).

**New constructs** (`bench/parity_constructs/*.bp` + `EXPECT` lines in
`bench/vs_rust/construct_parity.sh`, frozen in the same commit):

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c55_vswindow | IR-RUNG-BLUEPRINT §4 (`s8(k+1, …, k+8)`) | 312 | 8 live REG args: window cap -> park -> placement reloads |
| c56_nest | IR-RUNG-BLUEPRINT §5 row 4 (`nest(10)`) | 240 | right-nesting depth 8 + the 9th symbol in an x15 slot |
| c57_flags | `fn main() -> i64 { let i = 3; let n = 5; let b = i < n; let c = !(i == n); b + c * 2 + (if b then 10 else 20) }` | 13 | FLAGS materialise (cset), `!FLAGS` inversion, cbz on a register cond |
| c58_callmix | `fn g(a: i64) -> i64 { a * 2 } fn h(a: i64) -> i64 { a + 1 } fn f(a: i64, b: i64, c: i64) -> i64 { a * 100 + b * 10 + c } fn main() -> i64 { f(g(1), h(2), g(3) + h(4)) }` | 241 | args with inner calls: park to cs temps, parallel move |
| c59_evict | `fn g(a: i64) -> i64 { a * 2 } fn h(a: i64) -> i64 { a + 1 } fn main() -> i64 { let k = 4; g(k) + h(k) + k * 3 }` | 25 | `REG x0` live across a call (cs park), MULC after |
| c60_nestctor | an enum with a payload ctor applied to itself, e.g. `enum E { Leaf, Node(x) }` … `match Node(Node(Leaf))` -- take the syntax from c11_enum/c12_match; if the surface cannot express it, skip and say so in the VERDICT | (derive) | alloc-first ordering (§3.11) |
| c61_arrcall | `fn g(a: i64) -> i64 { a * 2 } fn h(a: i64) -> i64 { a + 1 } fn main() -> i64 { let v = [g(1), h(2), 3]; v[1] }` | 3 | array base parked across element calls |

**Risks and where each shows** (IR-RUNG-BLUEPRINT §5 rows 2, 3, 6, 7, 8 still apply):

| # | risk | probe | symptom |
|---|---|---|---|
| 1 | planning/emission n[0] disagreement (a form whose word count depends on a register or on cs-vs-slot) | `std_tests/ordfsm.bp`, c26_selfrec, the self-compile itself | SIGBUS / wrong value in a large program while every construct passes -- diff `objdump` of gen2 vs gen3 |
| 2 | a window temp live across a call-like site that did not park | c58, c59, c61, `fn p(x: i64) -> i64 { (x + 1) * (x + 2) + clz(x + 3) }` = 91 | value replaced by a callee's x1 |
| 3 | FLAGS clobbered before its consumer | c57; `(a < b) == (c < d)`; a `while` whose body ends with an array literal (T118 `cmp`) | wrong branch |
| 4 | cs temp vs `sym_bind` collision | exit 89 fires in std_golden or fuzz | COMPILEFAIL 89 -- if it fires on the corpus, rewrite the offending expression with a `let` and record it; if it fires in fuzz, teach gen.py/bpref the rule |
| 5 | retarget of a movk-terminated CONST (p must be -1) | c18_bigconst, c01_lit | a constant with a wrong high half |
| 6 | `mov x0,x14` no longer literal (an alloc emitted into another register) | c33_loopalloc, c34_loopescape, c40_struct, B1's `real_alloc` | T43 reset never fires / prologue omits x14 setup |
| 7 | census: `b.cond`/`cbz` counts move a lot (cbz is new at `if`) | invariants lane | add the allow line, it is a recorded change |
| 8 | ARGPASS allowlist too loose or too tight after §3.8 | c21_param13, c53_param9, check_abi on every frozen construct | `ABI` lane RED |
| 9 | the `-1` unbound-symbol `mov x0,x0` path | c-constructs with unbound names (neg/), diag lane | a diag test's expected words |

**Reversion rule**: this is one commit. If the chain cannot reach GREEN, the working tree is
left with the diff and the VERDICT says RED with the failing lane, the gen2/gen3 objdump diff
location, and the smallest reproducing program (miscompile recipe: top-down cuts on the failing
source, then a <= 15-line hypothesis probe; the T77 shrinker does not work) -- the main session
decides; nothing is patched forward with a peephole.

---

VERDICT: blueprint owned by the main session; one commit; end state push_words == 0; gates: chain --codegen GREEN,
push_words == 0, k4 <= 13, k3h <= 10, k1h <= 8, k2h <= 30, k4_ms <= 3.0; constructs c55-c61; open for the operator: none.
