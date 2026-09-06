Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A5 (x17 reserve, indices) and A6 (arena aggregates; frame layout final)

# A7 Pointer-free step 3: bytes live in the reserve, `str` is a value `(off << 32) | len`

## 0. Goal

End "bytes-in-cells" (1 byte per i64 cell = 8x memory on every IO path, sys_slurp `round16(len*8)`) and make strings first-class values: a `str` is one i64 handle `(byte offset from x17) << 32 | length`; `char(s, i)` is a byte load through x17; literals, argv, `sys_readbuf`, `sys_mmap` and a new `sys_mapb(path, len)` produce handles; `crc32b(s)` runs the hardware CRC over a handle's bytes. Gates: construct c68_strval; the 100 MB ingest twin (bebop raw path <= 1.5 x the best Rust row; maxrss <= 1.5 x the file size; five rows reported); std_golden 99/99 (bebop.bp's own parser runs on handles); K5 within +10 % of the A6 row (the 3-word `char` is compensated by A9's `scan`).

## 1. Scope

In: `emit_char_fn` (bebop.bp:1472, verified), `emit_str_len_fn` (:1455), `emit_str` (:2933: `adr` -> a CONST handle), `emit_sys_readbuf` (:1432), `emit_sys_slurp` (:1490: stays legacy cells; a new `sys_mapb`), `emit_sys_mmap`, argv handling in the stub (`argc/argv` copied into the arena: the seed's M4 contract puts argv cell pointers in the arena today -- the stub converts each argv string into a handle by copying its bytes into the byte region), `str_to_cells(s)` (legacy: reads through the handle), `emit_crc32x`/a `crc32b(s)` variant, the literal data section copied by the stub into the byte region at startup (so literal handles are compile-time constants), tools/bpref.py (`char`/`str_len` over a bytes object indexed by handle -- bpref.py:508 `char`, :506 `str_len`, verified), docs/LANGUAGE.md ("What is NOT: strings as values" becomes "strings are values"). Out: string concatenation / slicing builtins (a handle makes `substr` a 3-word expression `((off+i) << 32) | n` -- document it, do not add a builtin); UTF-8; `sys_read` into cells (legacy, kept for existing tests). Fixed points: `[i64]` semantics from A5/A6; the store format; every construct value.

## 2. Preconditions

A5 + A6 landed; x17 reserve with the stub's cells 0..15 and the literal copy region; A2 step 0's literal table cap (5000 + i, up to 1000 literals); the register model's CONST materialisation (movz/movk chains: a handle is a 64-bit constant -> up to 4 words, but as a CONST TAG it is materialised lazily and only when used as an operand -- `char(lit, i)` delivers it into x0 anyway).

## 3. Design

**Byte region.** Bytes are just memory inside the reserve; a handle's `off` is a BYTE offset from x17 (2^32 bytes = the whole 4 GiB reserve, matching A5's reserve; with the 1 GiB fallback the top 2 bits are spare). Allocation of bytes = `zeros(ceil(len/8))` cells (index i -> off = i*8) -- no second allocator. Literals: the stub copies the .bin's literal data section (cells `fntab[3899]` total, `write_lit_cells` layout bebop.bp:4398-4447 verified: today 4 bytes per cell? -- the worker reads write_lit_cells and keeps whatever packing it finds, but the COPY must produce contiguous bytes: if literal cells are byte-per-cell, the stub packs them (loop: ldr, strb) once at startup; ~40 KB of literals in bebop.bin -> < 1 ms) to a fixed byte offset LIT0 = 128 (after the stub cells) and reserves `x27 += round8(total bytes)`; `emit_str` pushes `CONST ((LIT0 + literal_byte_offset) << 32 | len)` -- the offsets come from the same per-literal table (fntab[5000+i]) expressed in bytes.

**Forms (asm text; derive with as+objdump).** `char(s,i)`: `lsr xt,xs,#32 ; add xt,xt,xi ; ldrb wd,[x17,xt]` (3 words; d = vs_alloc; s, i via vs_reg). `str_len(s)`: `and xd,xs,#0xffffffff` (1 word; a valid logical immediate). `substr` idiom (documented, no builtin): `((s >> 32) + i) << 32 | n`. `crc32b(s)`: deliver s in x0; `lsr x1,x0,#32 ; add x1,x17,x1 ; and x2,x0,#0xffffffff` then the existing crc32 byte loop over (x1, x2) (emit_crc32 bebop.bp:1361 verified: today pops cells and n; reuse its loop body with `ldrb` steps -- or keep the 8-byte `crc32x` loop for the aligned prefix + a byte tail: ponytail says byte loop first, measure). `sys_readbuf(fd, len)`: allocate `zeros(ceil(len/8))`, `read(fd, x17+off, len)`, return `(off << 32) | nread`. `sys_mapb(path, len)`: `sys_mmap` of the file into the reserve slot (A5), return `((addr - x17) << 32) | len`. `sys_mmap` itself keeps returning a CELL index (cells view of the same bytes: off/8 -- document that a mapped file is both). argv: the stub copies each argv C-string into bytes after the literal region and stores handles into the argv cells (`argv: [str]` reads handles).

**bpref.** One global `bytearray` for the byte region; `char(s,i)` = `mem[(s >> 32) + i]`; `str_len` = `s & 0xffffffff`; literals get handles at load; `sys_readbuf`/`sys_mapb` read files into the bytearray; argv likewise. Parity on every std_test that touches strings (c14_string, base64, hex, morph...).

**Compiler's own parser.** bebop.bp reads source through `char(s, pos[0])` and `slen(s, pos)` (str_len cached in pos[1]) everywhere: unchanged source, new codegen: 3 words per char (was 1) -- K5 cost bounded by the gate; A9's `scan` builtin is the compensation. `read_ident` hash loop, skip_ws, skip_string, skip_line_comment, collect_fns are the hot sites.

**Ingest twin (measured-first for the claims of RESEARCH-DEPS §6d-8/9).** Data: 100 MB text, lines `id,u,v,cell,label` (label 8-16 ASCII chars), generated by python from LCG seed 12345. Rows: (1) bebop raw: `sys_mapb` + parse via `char` (then `scan` after A9) building CSR by counting sort into the store; (2) bebop cells: `sys_slurp` + `str_to_cells` legacy path; (3) sqlite `.import`-equivalent (python executemany in one transaction, native rate = minus the ctypes floor per LANG-DB §8 rule); (4) Rust memmap2 + winnow (rust_once/ingest_best.rs); (5) Rust serde-owned lines (rust_once/ingest_common.rs). Report ms/MB, maxrss, minor faults (python `resource`), fold of the CSR (all five must agree on the fold).

**Invariants.** A handle never leaves the reserve (off < 2^32); `str_len` of a literal equals the source length (bpref checks); the stub's byte layout (stub cells, literals, argv) is fixed before any `zeros`; both passes emit the same counts (forms are unconditional).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:entry_stub | literal copy + argv handles + byte layout | fn entry_stub |
| bebop.bp:emit_char_fn / emit_str_len_fn / emit_str / emit_crc32 (+crc32b dispatch in emit_call_or_ctor :1527) / emit_sys_readbuf / emit_sys_mmap / new emit_sys_mapb / str_to_cells | handle forms | 1472, 1455, 2933, 1361, 1527, 1432, ~5502 |
| bebop.bp:write_lit_cells / scan_one_lit | byte offsets in the literal table | 4398-4447 |
| tools/bpref.py | bytearray model, RESERVED += sys_mapb, crc32b | bpref.py:47, 498-510 |
| tools/check_abi.py | new words allowlisted (sys_allow reads em() literals per emit_sys_*: check_abi.py:102-108 verified) | -- |
| docs/LANGUAGE.md | strings as values; substr idiom; sys_mapb | -- |
| bench/vs_rust/ingest.sh + rust_once/ingest_*.rs + tools/gen_ingest.py | twin | new |

## 5. Steps

0. Read write_lit_cells / entry_stub / the seed's argv contract (M4) and write the byte layout down in the journal before editing.
1. Handles: stub + char/str_len/literals/argv + bpref -- one chain commit (every string test re-frozen; WORD_DELTA on c14_string, c50_cas...).
2. IO builtins: sys_readbuf/sys_mapb/crc32b/str_to_cells -- second chain commit; c68_strval.
3. Ingest twin (parallel-safe after step 2): generator, five rows, REPORT row in bench/vs_rust/REPORT-honest.md (new section "ingest").
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c68_strval | strings passed through a call, stored in an array, `str_len` of literals, `char` at the ends, a substr idiom, `crc32b` of a literal vs the python zlib.crc32 value | bpref | handles as values |
| c14_string, c50_cas, c51_casbad, base64/hex std_tests | re-frozen | same values | parser paths |
| ingest twin | §3 | folds equal across five rows | the claim |

## 7. Gates

- chain `--codegen` GREEN (steps 1, 2); K5 <= 1.10 x the A6 row; bin_words growth budgeted (+2 words per `char` site in bebop.bp: expect +3-5 %).
- `bash bench/vs_rust/ingest.sh`: bebop raw <= 1.5 x Rust best; maxrss(raw) <= 1.5 x file size; cells row reported (expect 4-8 x worse); sqlite row (expect 10-30 x worse than raw).
- RED: a string std_test value change = a handle mis-split (off/len swapped) or the literal copy offset wrong (probe: c68's `str_len` of literals first).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| literal packing in the .bin is not byte-per-cell as assumed | read write_lit_cells first (step 0) | garbage literals |
| argv strings longer than the stub's byte budget | cap 64 KiB total, trap 89 if exceeded | exit 89 at start |
| `and #0xffffffff` encoding | as+objdump | wrong length |
| bpref parity on sys_readbuf (partial reads) | c68 reads a small file | mismatch |
| K5 regression > 10 % | measure; A9 scan is the fix, not a rollback | gate RED -> ship A9 before promoting |

## 9. VERDICT format

```
VERDICT: GREEN|RED
layout: LIT0=<off> lit bytes <n> argv bytes <n>
step1 fixpoint <md5>; step2 fixpoint <md5>; bin_words <b> -> <a>; K5 <b> -> <a> (gate +10 %)
constructs: c68 EXPECT + deltas; string tests re-frozen: <list>
ingest: raw <ms/MB, maxrss> | cells | sqlite | rust-best | rust-common; folds equal: yes
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, A5/A6 facts, the seed M4 argv contract, harness commands and traps. </context>
<constraints> two chain commits + the twin; `str` handle layout exactly `(off<<32)|len`; legacy cells IO untouched; words via as+objdump; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A7 steps 0-3; report. </task>
