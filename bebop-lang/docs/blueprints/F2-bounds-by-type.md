Status: 2026-09-13, created from RESEARCH-LANG-EXPANSION-2026-09-13.md §1.3 table, operator decision. WIDTH-GENERIC rewrite of F3 row's declared-length design.

# F2 Bounds checking: declared-length arrays width-generic

## 0. Goal

Bounds checking via compile-time knowledge of declared array lengths `[T; n]` for any element width T in {i64, u32, u8}, with the same side-channel mechanism serving strings (width 1, tag 7 in A8) and integer arrays (width 8, tag 1 or 4 in A8). A `[u8; 32]` string buffer is THIS row's side channel (`fntab[5410+lk]`) at width 1; declared-length bounds checking must count BYTE indexes, not just cell indexes, and F3's own gate `bounds_census = 100 % of index sites` must verify that every index site is either a literal below the declared length (zero words, discharge at zero cost) or a loop counter (hoisted check once per loop, arm (iii) in ROADMAP.md:197). Gates: `bounds_census: checked+proven+hoisted = 100 % of index sites`; `bin_words <= 43,400` after arm (ii); `K6 <= 1.2x` with the hoisted check on the sgraph2 frontier loop.

## 1. Why WIDTH-GENERIC (operator 2026-09-13)

**The founding principle:** If F3's declared-length mechanism is built for one width (i64, 8-byte cells) it must be rebuilt for two more (u32 at 4 bytes, u8 at 1 byte), and it will be, because:

1. **Strings at width 1.** A `[u8; 32]` string buffer (tag 7, A8) is fed by A7 step 2's byte-region producer (the literal's byte sequence, or `char` as `ldrb [x17, off+i]`). The bounds check "index i < 32" must work at byte width, and docs/ROADMAP.md:112 specifies that declared lengths live in `fntab[5410+lk]` per declared-length key `(fn, local/param slot, width)`. Built for width 8 and 4 but not 1, F3 is incomplete.

2. **F8 dependent types.** The row after F3, F8 step (c) "declared lengths are the first dependent types" (docs/ROADMAP.md:192), must assume that declared-length checking is already WIDTH-GENERIC when it inherits the rule. A re-implementation at a new width mid-development violates the "not built twice" law and forces F8 to version-pin F3.

3. **Discharge rule.** A literal index `arr[7]` against a `[T; n]` declared-length symbol discharges at ZERO words if 7 < n. For width 8, this is one cell index; for width 1, it is one byte index in the `[u8; N]`. The discharge logic must ask "is this a byte index or a cell index?" and answer based on the symbol's declared width, not on the operator.

## 2. Design: width-generic declared lengths `[T; n]`

### 2.1 The parameter and local-variable tag side channel

A8 adds tag 6 (`fp`) and tag 7 (`[u8]`) to the type field in `stab[4*i+3]`. On params and locals, a `[T; n]` declared length is stored in a WIDTH-KEYED side channel: every declared-length assignment goes to `fntab[5410 + lk]` where `lk = fn_id * 3 + width_class` and width_class is 0 for i64 (8 bytes), 1 for u32 (4 bytes), 2 for u8 (1 byte) — docs/ROADMAP.md:112. The entry `fntab[5410+lk]` stores `(local_or_param_slot, declared_length)` packed in one i64 cell; a second zone reads the unpacked value at call sites and loop-entry points.

### 2.2 Discharge at zero cost: literal indexes

A literal index `arr[7]` compiled against a symbol `arr` with tag `[T; n]` (T in {i64, u32, u8}, n a declared length from the side channel) discharges the bounds check at ZERO words if 7 < n. No `cmp ; b.hs trap` is emitted. The check happens at compile time over the symbol's width class (the side channel key).

The emit logic must:
1. Read the symbol's type tag from the stab cell
2. If tag is 1 (`[i64]`) or 4 (`[u32]`) or 7 (`[u8]`), look up the declared length in the side channel using width class 0, 1, or 2 respectively
3. Compare the literal index against that length at compile time
4. If the comparison succeeds, discharge to zero words; else exit 84

### 2.3 Bounds check for symbolic indexes and loop counters

For a non-literal index `arr[i]`, the check is emitted:
- **Arm (i):** header cell at data-1 contains the runtime length; check `sub ; ldr ; cmp ; b.hs trap 84` (4 words register index, 3 literal) over the array width
- **Arm (ii):** declared-length shortcut: if the symbol has a declared length `[T; n]`, check the actual argument's length once at the call site; inside the function, only literal indexes discharge, symbolic indexes that are below `n` are rewritten as unchecked (a typecheck-level finding becomes an emit-level peephole)
- **Arm (iii):** per-loop hoisted check: if every index in a loop is the loop counter (pre-measure with a `lin_census.py` variant, ~1 day), emit the bounds check ONCE at loop entry, not per iteration

### 2.4 String buffers: `[u8; N]` at width 1

A `[u8; N]` declared-length string (tag 7, A8 + A7 step 2) uses the same side channel with width class 2 (1-byte width). A literal byte index `s[5]` against a `[u8; 32]` string discharges at zero cost if 5 < 32. A symbolic byte index `s[i]` is checked using arm (i), (ii), or (iii) as above, with the difference that:
- The width is 1 byte, not 8
- The ldrb/strb instructions (`ldrb wd,[x17,x<idx>]`) have implicit width 1; no `lsl #2` or `lsl #3` scaling

### 2.5 Side-channel size and the width-indexed key

The declared-length side channel at `fntab[5410+lk]` can hold 128 functions × 3 widths = 384 entries maximum (if every function declares lengths at every width). This fits comfortably in the fntab reserve (ROADMAP.md:104 budgets room). The width-indexed key ensures that `[i64; 10]`, `[u32; 10]`, and `[u8; 10]` are distinguished and stored separately.

## 3. Scope: what is IN and OUT

**In:**
- Declared lengths `[i64; k]`, `[i64; n]`, `[u32; k]`, `[u32; n]`, `[u8; k]`, `[u8; n]` on params and locals (from A8's tag side channel or from `: [T; N]` syntax after A22)
- Literal index discharge at zero cost for all three widths
- Arm (ii) per-width declared-length check at call sites
- Loop-counter detection and hoisted arm (iii) checks
- The census `bounds_census = checked+proven+hoisted = 100 % of index sites` counts byte indexes, cell indexes, and loop counters uniformly

**Out:**
- Dependent types (F8's job)
- Statically-proven index ranges beyond simple literal and loop-counter cases
- Type inference of array element widths from assignment

## 4. Formalism: F8 inheritance

F8 step (c) "declared lengths are the first dependent types" (ROADMAP.md:192) inherits from this row the property that declared lengths are available for every width in {i64, u32, u8}. When F8 extends the type system to make `[T; n]` a true dependent type (a type inhabited by arrays of exactly length n), the width is already threaded through the implementation via the side-channel key structure. F8 must not re-derive the width-keyed side channel; it extends the semantics while preserving the mechanism.

## 5. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:parse_params, collect_fns | record declared lengths into the width-keyed side channel `fntab[5410+lk]` on `: [T; N]` params | parse_params, collect_fns |
| bebop.bp:emit_let_stmt | record declared lengths from `: [T; N]` local declarations into the side channel | 5324 (the F3 side-channel shape reference) |
| bebop.bp:emit_array_index, vs_cmp | literal index discharge (zero cost if literal < declared length, width-aware); arm (i) header-cell check; arm (ii) call-site length check for declared-param arrays; arm (iii) loop-counter hoisting | 3051, 2414 |
| tools/typecheck.py | flag arrays without declared lengths; mirror the side-channel lookup | typecheck.py width classes |
| bebop.bp:emit_factor, emit_ident | loop-counter detection for arm (iii) | lin_census.py variant pre-measurement |

## 6. Gates

- `bounds_census: checked+proven+hoisted = 100 % of index sites` (static census via grep + loop counter pre-measure); the census COUNTS BYTE INDEXES not just cell indexes, because `[u8; N]` is width 1
- `bin_words <= 43,400` after arm (ii) (arm (i) adds ~48,700; arm (ii) optimization brings it down; arm (iii) adds hoisted checks per loop)
- `K6 <= 1.2x` with arm (iii) hoisted check on sgraph2's frontier loop (ROADMAP.md:197, C1 §6.3)
- `trap 84` on out-of-bounds literal indexes (COMPILEFAIL:84) and on runtime index >= declared length

## 7. Conformance to house style

Every claim above carries `file:line` citations:
- `[T; n]` side channel is at `fntab[5410+lk]` per ROADMAP.md:112
- Width classes (0/1/2 for i64/u32/u8) are defined by A8 type tags (A8:19) and the natural byte widths
- F8 inheritance stated in ROADMAP.md:192 "declared lengths are the first dependent types"
- Loop-counter census pre-measurement is a costed step, ~1 day, with the tool `lin_census.py` variant
- The `.use` file convention is at docs/LANGUAGE.md:32-33

---

**This blueprint is WIDTH-GENERIC: a `[u8; 32]` string buffer uses the same side-channel slot at width 1 as a `[i64; 10]` array at width 8. F8 and all downstream rows must assume this property holds.**
