Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A7 (str values), A5/A6 (indices); enables A9's cmp_mask on u32 and A12's alias facts

# A8 Typed tables: T48 checked types inside bebop.bp, `[u32]` cells for CSR and the store

## 0. Goal

The compiler itself rejects (compile-time exit 84) what tools/typecheck.py's census rejects today (T48a/T48b, invariants gate vii: `ref T` misuse, `[i64]`/`str`/scalar confusion -- tools/typecheck.py:12-20, verified), at zero runtime cost, with typecheck.py as the oracle; and a second cell width `[u32]` (4-byte elements, zero-extended loads, truncating stores, wraparound in bpref) so CSR `ci`/`rp` and store columns halve their bytes. Gates: the typecheck oracle and bebop.bin agree on the whole corpus (0 findings) and on bench/typecheck_neg/ref_misuse.bp (COMPILEFAIL:84); G7 file-size row <= 1.2 x sqlite (today 2.5x loss, HISTORY G7); K6 nn.bp ns/row halves once its columns are `[u32]` (report; the DRAM ceiling argument of RESEARCH-TENSOR §2); c77_u32 construct.

## 1. Scope

In: a per-symbol type in `stab` (today triples name/reg/srcpos, bebop.bp:182 sym_bind verified -> a 4th cell: type tag), types parsed from signatures (parse_params bebop.bp:222 skips them today via skip_to_delim; collect_fns records nothing about returns -> a return-type cell per fn in the fn zone), inferred for `let` from the RHS tag kind + builtin table (literal -> i64; `zeros` -> `[i64]`; `zeros32` -> `[u32]`; `"..."` -> str; `[e..]` -> `[i64]`; call -> the callee's declared return; `st_alloc/st_ref/...` -> `ref *` per typecheck.py's producer list; array get on `[u32]` -> i64), checks at use sites exactly as typecheck.py (arithmetic on ref, ref as index, scalar where ref declared and vice versa, str where cells expected and vice versa, `[u32]` value where `[i64]` declared) -> `diag_exit(s, pos, 84)`; the `[u32]` forms; bpref: `zeros32`, u32 semantics; the store: `arr u32` object kind (digest tag), sgraph/csr over u32 ci; sbench size row. Out: generics, inference across calls, user-declared struct field types beyond today's textual declarations, enums with typed payloads. Fixed points: every accepted program compiles to the same words (types cost 0 words); bpref's own untyped evaluation.

## 2. Preconditions

A5-A7 landed (x17 indices: a `[u32]` table index is in 4-byte units: address = x17 + idx*4); typecheck.py rules (the oracle) frozen at 0 findings on the corpus; A2 step 0's fntab relayout (room for a return-type zone: put it at `fntab[2712 + i]`, i < 512 -- verify the zone is free after A2 step 0's layout; check_abi tuple).

## 3. Design

**Type tags (i64 in stab[4*i+3] and the fn return zone):** 0 unknown/i64, 1 `[i64]`, 2 `str`, 3 `ref T` (T's hash in the high bits, `ref *` = 3 with 0), 4 `[u32]`, 5 `[str]` (argv), 6 fp (a tagged i64; the store's layout digest already distinguishes `fp`: LANG-DB "[T] and fp tagged in the layout digest"). Params: from the signature text (`: [i64]`, `: str`, `: ref RP`, `: [u32]`); returns: `-> T` recorded by collect_fns (bebop.bp:collect_fns) into the return zone; `let`: from the RHS -- the RHS's final tag kind plus a "type" side channel: extend the window entry payload? No (3 cells per entry are fixed) -- the type of an expression is computed by a tiny parallel walk: emit_factor knows what it produced (literal / call / builtin / var / index) -> write `fntab[3836] = type of the last factor` (a scratch cell; verify 3836 free after A2/A3 -- A3 uses 3836 for a debug count in step 1 only; pick 3837) and binops set it to 0 (i64) unless both sides are the same non-scalar type (then error 84: arithmetic on refs/arrays -- exactly typecheck.py's rule) -- ponytail: the check that matters most is refs and str, so the walk only tracks those.
**Checks (mirror typecheck.py):** (1) arithmetic/comparison with a ref or `[..]` or str operand -> 84; (2) index expression of ref type -> 84; (3) call argument type != declared param type (with `ref *` compatible with any `ref T`, literal 0 allowed for ref) -> 84; (4) return expression type != declared -> 84; (5) `[u32]` used where `[i64]` declared or vice versa -> 84. The oracle equality gate: `python3 tools/typecheck.py <file>` findings == bebop.bin's exit 84 position (file:line:col printed by diag_exit) for every corpus file and every bench/typecheck_neg/*.bp.

**`[u32]` forms (asm text).** `zeros32(n)`: `zeros(ceil(n/2))` then the returned CELL index i becomes the u32 index `2*i` (`lsl x0,x0,#1`, 1 word). Get: `add xt,x<base>,x<idx> ; ldr wd,[x17,xt,lsl #2]` (zero-extends); const index: `add xt,x<base>,#c ; ldr wd,[x17,xt,lsl #2]`. Set: `str w<v>,[x17,xt,lsl #2]` (truncates). Which form to emit is decided by the base symbol's type tag (4) -- the first place the type changes codegen, still 0 words for the i64 path. Store: `arr u32` objects (object kind in h0, digest), `st_get/st_put` over u32 via the same forms; sgraph.bp `CI` as `[u32]` (n <= 2^32), `RP` stays i64 (nnz up to 10M fits u32 too: make both u32 and report the size row).

**bpref.** Type tags are irrelevant to evaluation; `zeros32` = list of ints masked to 32 bits on store; `[u32]` params annotate nothing. typecheck.py stays the oracle for types (it already parses signatures).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:sym_bind / sym_lookup / stab layout (`zeros(385)` = 1 + 3*128 -> 1 + 4*128 = 513) | type cell | 182, every `zeros(385)` |
| bebop.bp:parse_params / collect_fns / fntab return zone | param types, return types | 222, collect_fns, new zone 2712+ |
| bebop.bp:emit_factor / emit_ident / emit_call_or_ctor / emit_bl_call / emit_let_stmt / emit_return_stmt / vs_binop / vs_cmp / emit_array_index | type walk + checks 84 | 3128, 249, 1527, 727, 3672, 3489, 2397, 2414, 3051 |
| bebop.bp:emit_zeros (+ zeros32 dispatch), emit_array_get/set | u32 forms | emit_zeros, 3010, 3029 |
| tools/bpref.py | zeros32, RESERVED | 47, 498 |
| tools/typecheck.py | `[u32]` type, zeros32 producer; unchanged rules otherwise (it is the oracle) | 19+ |
| selfhost/prelude/store.bp, selfhost/std/csr.bp, sgraph.bp, bench/vs_rust/sbench.sh | `arr u32`, CI/RP u32, size row | store.bp:84-176, csr.bp:20, sgraph.bp:15 |
| docs/TRAPS.md, emit_paren table | exit 84 row | 2765 |

## 5. Steps

1. Types + checks (no codegen change; exit 84 only): chain WITHOUT `--codegen` must be byte-identical (gen3 == gen4, 0 WORD_MISMATCH -- types cost nothing); oracle-equality gate script `tools/typecheck_gate.sh` added to the battery (invariants lane vii extended).
2. `[u32]` forms + zeros32 + bpref + c77 -- chain `--codegen`.
3. Store/CSR/sgraph on u32 + sbench size row + K6 columns u32 (nn.bp's u,v,cell) -- chain (no codegen change; std_golden re-frozen where folds change? they must NOT change: same values) + rows.
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c77_u32 | `zeros32(10)`, store 2^32+5 (reads back 5), store -1 (reads 4294967295), sum; pass the table to a fn declared `(t: [u32])` | bpref | forms, wrap, param type |
| bench/typecheck_neg/*.bp (existing ref_misuse + new u32_misuse, str_misuse) | must COMPILEFAIL:84 at the oracle's position | oracle | checks |
| tools/typecheck_gate.sh | corpus: 0 findings both sides | -- | equality |

Twins: sbench.sh size row (G7), K6 run.sh (nn.bp u32 columns), sgraph.sh BFS row.

## 7. Gates

- Step 1: chain without codegen byte-identical; typecheck_gate.sh: bebop.bin == typecheck.py on corpus + neg.
- Step 2: chain `--codegen` GREEN; c77; WORD_DELTA 0 on every existing construct (no u32 in them).
- Step 3: G7 logical size <= 1.2 x sqlite's 34.1 MB (today 85.2/72.4 MB); K6 ns/row reported (expect ~2x better once codegen-bound loops are gone; DRAM floor 1 ns/row); BFS row.
- RED: a corpus file that typecheck.py accepts and bebop.bin rejects (or vice versa) -> the walk diverges: fix the walk, never the oracle, unless the oracle is proven wrong by LANGUAGE.md.

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| stab layout change breaks the register model's symbol numbering | c06_let, c53_param9 (stab count semantics `stab[0]`) | wrong register |
| generic store helpers use `ref *` everywhere (typecheck.py allows) | typecheck_gate on store.bp | false 84 |
| u32 index arithmetic (`2*i`) confused with cell indices when a u32 table is passed as `[i64]` | check 5; c77 | garbage |
| bpref wrap on negative stores | c77 -1 case | mismatch |
| fn count: +6-8 fns (cap 512 after A2 step 0) | grep -c '^fn ' | exit 89 at build |

## 9. VERDICT format

```
VERDICT: GREEN|RED
step1: byte-identical fixpoint <md5>; typecheck_gate: corpus 0/0, neg <n>/<n> agree
step2: fixpoint <md5>; c77 EXPECT; deltas 0 elsewhere
step3: G7 size <MB> vs sqlite 34.1 (gate 1.2x); K6 ns/row <b> -> <a>; BFS <b> -> <a>
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, typecheck.py as the oracle (read it fully first), A5-A7 facts, harness commands. </context>
<constraints> step 1 must be byte-identical codegen; the oracle's rules are not changed; u32 forms via as+objdump; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A8 steps 1-3; report. </task>
