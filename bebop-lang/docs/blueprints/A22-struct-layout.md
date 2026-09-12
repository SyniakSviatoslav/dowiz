Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row A22 (new), adjacent to A8
(typed tables) and F3 (`[T]` with length): step 2 of this row IS A8's step 1a and is written so A8
inherits it. Delivers language item 2 (structs with named fields plus a derived layout).

# A22 -- struct layouts as names: `T.f`, `T.size`, every struct, typed symbols

## 0. Three corrections this blueprint rests on

**(0a) Structs exist, for exactly ONE struct per program, and the docs say the opposite twice.**
`docs/LANGUAGE.md:16` says "literals disabled (T43 rest)"; they are enabled under a guard
(`emit_ident:315-330`, T43 rest 2026-09-05). But every path resolves against the FIRST declaration only:
`find_struct` (`bebop.bp:354-386`) returns the first `struct ` at a line start, `struct_name_hash` (`:418`)
is cached once per program in `fntab[5541]`, `field_index` (`:388-416`) searches that declaration, and
`emit_field_access` (`:500-516`) computes `idx` from it for ANY `.f`. A program with two structs gets
the second's literal parsed as a block (`is_sname` false at `:326`) and its `.f` resolved against the
first -- silently wrong. `c40_struct` has one struct; `c10_struct` has none (`c10_struct.bp:1-4` is call
composition). The compiler declares no struct at all.

**(0b) The layout defect the row is named after was an OFFSET, and it bit twice in one day.** The PartTab
episode of 2026-09-12: `st_parttab_write` wrote its 19 cells from `base[pt_off + 0]` over the 2-cell
object header that `st_alloc` had just written (journal 1789217115); `tools/stdump`'s `storelib.parttab`
read from `parttab_off` instead of `parttab_off + 2` (journal 1789234991) -- "the SAME off-by-two the
store itself suffered this morning, now in the tool built to diagnose it". Then four gates went red on
a +21 CONSTANT (slayout 925, schain 930, sevolve 934, scompact 935 -- `scrash_torn` was a model
divergence, 943/944), and the +21s were reverted when the PartTab moved into the superblock page (939).
The offsets are hand-numbered in `store.bp:269-283` ("cell 17 for P=1"), `st_get` is `base[obj + 2 + i]`
(`:862`), and 34 `base[... + 1x]` sites carry the same arithmetic.

**(0c) A per-symbol side channel already exists and is the template.** F3's static bounds check
publishes a per-symbol STATIC LENGTH on every `let` -- `fntab[5410 + lk] = rhs_len_p1(...)` at
`emit_let_stmt:5324-5326`, zone `arrlen` 5410..5537 (`check_abi.py:210`), read at `emit_array_index:4232`
to exit 65. A per-symbol STRUCT TAG is the same cell shape written from the same RHS scan. That is how
step 2 gets `p.f` for any struct without A8's full type walk -- and it is precisely A8 §3's "type tag in
stab[4*i+3]" restricted to struct names, so it is A8's first step, not a parallel mechanism.

---

## 1. Scope

### A22 IS

1. **Step 1 -- layout constants, zero inference, zero words.** For every declared `struct T { f0: ..,
   f1: .., ... }`: `T.f1` is the compile-time constant 1 (the field's 0-based index), `T.size` the field
   count. Both are `CONST` window pushes (`vs_push kind 1`, `vs_push:2752`) that materialise only when
   consumed and fold into `add #imm` / `ldr [.., #imm]` by the existing peepholes
   (`vs_try_addsub_imm:2997`, `vs_array_get_imm:4089`). `a[i * Node.size + Node.child]` replaces
   `a[node + 3 + ci]`; `base[pt + 2 + PartTab.used]` replaces "cell 17". Works on `[i64]`-backed
   objects, store objects and CSR rows -- the object model the thesis names.
2. **A struct table**: `collect_structs`, the `collect_ctors` shape (`:725-755`), records (name hash,
   declaration position, field count) for EVERY `struct` at a line start into a free fntab zone;
   `field_index` takes a declaration position instead of calling `find_struct`.
3. **Step 2 -- every struct, typed symbols (= A8 step 1a).** `NAME {` is a literal for any declared
   struct (the `fntab[5541]` cache becomes a table lookup; the `while` guard `fntab[5542]` stays); a
   per-symbol struct tag `fntab[<zone> + lk]` is written on `let` from (i) a struct-literal RHS (its
   name), (ii) a `: T` parameter type (read in `parse_params`, today `skip_to_delim`'d, `:289`), (iii) a
   `-> T` return type (recorded by `collect_fns`, the A21 arity zone's neighbour). `p.f` resolves through
   the tag; `.f` on a symbol with no tag is a `diag_exit` ("field access on a symbol of unknown struct")
   -- never "field of the first struct".
4. **A store consequence as acceptance**: `st_parttab_cells/_used/_gen/_root` rewritten over
   `PartTab.*` and the object header as `Obj.h0/Obj.h1/Obj.size` with the six store gates BYTE-IDENTICAL
   in output (`slayout 749213972`, `schain 71563930701023`, `sevolve`, `scompact`, `scrash_torn 0/50`,
   `smw 303000`). If a fold moves, the rewrite is wrong, not the gate.
5. **Constructs**: `c131_structs2` (two structs, both literals, both `.f`, `T.f`/`T.size` used as
   indices into a `zeros` table), `neg/c132_untagged_field` (step 2), and `c40_struct` re-frozen
   byte-identical.

### A22 IS NOT

1. **Not struct values in registers** (T49 "records = register images", HISTORY.md:1744, OPEN). A
   struct value stays a cell index into an arena block (`emit_struct_lit:475-490`, A6); that IS the
   store's layout and this row does not touch it.
2. **Not field types beyond names.** `f: i64` / `f: [i64]` / `f: ref T` are read for the tag only; type
   CHECKING of field use is A8's walk. Step 2 tags symbols by struct NAME; nothing else.
3. **Not `sizeof` in bytes.** Cells are 8 bytes everywhere until A8's `[u32]`; `T.size` is in cells and
   says so. When `[u32]` lands, a `T.bytes` is a second constant, A8's to add.
4. **Not nested struct literals or struct-typed fields resolving transitively** (`p.q.f` where `q: Q`):
   the tag of `p.q` would need the field's declared type -- A8 step 1b, costed at ~40 lines, not here.
5. **Not a change to enums** (`emit_enum_ctor:563`, `emit_match_rt:2050`).

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/c131_structs2.bp` | step 1+2 positive |
| `bench/parity_constructs/neg/c132_untagged_field.bp` | step 2; `.f` on an `i64` symbol; `EXPECT=COMPILEFAIL:<code>` |
| `bench/parity_constructs/c40_struct.bp` | re-frozen, WORD_DELTA 0 |
| `selfhost/prelude/store.bp` | PartTab/Obj accessors over constants (step 3); six store gates unchanged |
| `bench/vs_rust/std_golden.sh` | no new gate; `ok=117` unchanged |

### 2.2 The number

`c131` declares `struct P { x, y }` and `struct Node { kind, nch, attr, c0, c1 }` (in that order), builds
a 4-node tree in `zeros(4 * Node.size)` using `Node.*` offsets, builds a `P` literal and a `Node` literal
(the SECOND struct's literal), reads `.attr`/`.c1` from the Node literal and `.y` from the P literal, and
folds. EXPECT hand-derived from the tree's contents in the header (the fold is a weighted sum with
weights chosen so that a wrong offset changes it: `attr * 1000 + c1 * 100 + y * 10 + kind`).

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c131` MATCH at the hand value | `Node.c1` resolves against `P` (the first struct) -> index out of P's range -> `field_index` returns -1 -> today's `slots[fcnt] = fcnt` fallback (`emit_struct_field:434`) silently uses declaration order; or the Node literal is parsed as a block (`is_sname` false) |
| B | `c40_struct` WORD_DELTA 0 | the one-struct path changed words (it must not: same offsets, same literal words) |
| C | `c132` COMPILEFAIL:<code> (step 2) | an untagged `.f` still falls back to the first struct |
| D | six store gates identical after step 3 | a rewritten accessor changed a cell offset (the gates are the layout's fingerprint; `slayout`'s oracle re-derives the bytes independently, `bench/oracles/slayout.py`) |
| E | a mutant that swaps two field names in `Node`'s declaration changes `c131`'s value | the construct reads a field whose offset equals its declaration-order index by coincidence (choose the weights so no permutation of `c0`/`c1`/`attr` is invariant) |
| F | `c131` with `Node` declared BEFORE `P` gives the same value | the table is order-dependent (it must not be) |

---

## 3. Which existing work A22 sits on

| existing | verdict | reason |
|---|---|---|
| `find_struct` (`:354`), `struct_name_hash` (`:418`), `fntab[5541]` cache | **REPLACED** by `collect_structs` + a table; `find_struct` kept as a wrapper for one release then deleted (dead-function ratchet, `arch_check` CHECK 19) | first-struct-only |
| `field_index(s, spos, fname)` (`:388`) | **KEPT** (it takes a declaration position already) | |
| `emit_struct_field` (`:429`), `emit_struct_lit` (`:447`) | **KEPT**; the `idx < 0 -> fcnt` fallback at `:435` becomes a `diag_exit` (unknown field) | the fallback is a silent guess |
| `emit_field_access` (`:500`) | **CHANGED**: idx from the symbol's tag (step 2); step 1 leaves it on the FIRST struct so `c40` is byte-identical until step 2 | |
| `emit_ident` (`:302-335`) struct-literal guard | **CHANGED**: `is_sname` = table lookup; also the new `NAME.` case -> CONST push | |
| `emit_factor` postfix `.` loop (`:4356-4368`) | KEPT | `T.f` is handled in `emit_ident` before `.` is seen by the postfix loop -- `read_ident` stops at `.`, then `emit_ident` checks "is a struct name AND next char is `.`" |
| `emit_let_stmt` F3 length channel (`:5324-5326`), `rhs_len_p1` (`:4190`) | **TEMPLATE** for the tag channel | |
| `parse_params` (`:272-300`), `skip_to_delim` (`:254`) | **EXTENDED** (step 2): read the type ident before skipping | |
| `collect_fns` (`:5870`) | **EXTENDED** (step 2): `-> T` recorded (shares A21's header scan) | |
| `tools/bpref.py` structs (`:222-233`, keyed by name, `first_struct`) | **EXTENDED**: `T.f`/`T.size` as constants; field access by the VALUE's struct -- bpref's struct value is a Python object? It is a `Cells` block (`cells_alloc`); bpref needs the same tag or a per-object header -- simplest: bpref tags at `let` like the compiler | |
| `tools/typecheck.py` | **EXTENDED**: struct names as types | |
| `store.bp` PartTab accessors (`:231-283`), `st_get/st_put` (`:855-856`) | **REWRITTEN** over constants (step 3) | the acceptance |
| `docs/blueprints/A8-typed-tables-u32.md` §3 (stab 4th cell) | **AMENDED**: "step 1a = A22 step 2" | one mechanism |

---

## 4. Constraints as design input

1. **Zero words for constants.** A CONST entry emits nothing until consumed (`REGISTER-MODEL-BLUEPRINT`
   §1.2); consumed as an index it folds into the immediate forms already used for literal indices
   (`vs_array_get_imm`, `:4089`). So `a[i * T.size + T.f]` costs what `a[i * 5 + 3]` costs.
2. **The struct table zone**: 3 cells per struct; cap 64 structs = 192 cells; a free zone per
   `check_abi.py:205-212` (e.g. 5601..5792); `check_abi.py` ZONES updated in the same commit (rung (iii)).
3. **The tag channel**: 128 symbols x 1 cell = 128 cells, next to `arrlen` (5410..5537) -- 5538/5539 are
   free but too few; take 5800..5927 and register the zone.
4. **`while NAME {` ambiguity** (`emit_ident:321-327`): `fntab[5542]` (inside a while condition) stays the
   guard; with several struct names the guard applies to each.
5. **Planning/emission agreement**: constants are the same in both passes; tags are text-derived in
   both passes identically (like F3's lengths).
6. **Bootstrap**: two-stage. The compiler declares no struct; `fntab`'s 43 bases as a `struct Fntab`
   would be generation N+1 work and its own decision.

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- `collect_structs`, `T.f`, `T.size` (one codegen commit, compiler words only)

**Expected:** fixpoint; WORD_DELTA 0 on 104/104 (no construct uses `T.f`; `c40` unchanged because
`emit_field_access` still uses the first struct); `c131` (step-1 subset: `T.*` indices + first-struct
literal only) MATCH; `bebop` +80..+140 words with a budget line; `check_abi` ZONES updated, `ABI ok`.
**Kills:** `c40` WORD_DELTA != 0; `ABI` FAIL on the new zone (the zone overlaps `lit_table` 6000+ or
`budget` 5600 -- read the map, do not guess).
**Effort:** 2-3 days. **Gate-days:** 1.

### Step 2 -- every struct, tags, `p.f` by tag (one codegen commit)

**Expected:** `c131` full MATCH; `c132` COMPILEFAIL; `c40` WORD_DELTA 0 (its `p.x` resolves through the
tag to the same index); `f8_dt`'s test 4 (`a: [i64; 10]`) still compiles -- `parse_params` now READS the
type ident and must tolerate `[i64; 10]` / `ref T` / `str` (typecheck's grammar, `bpref.py:180-186`).
**Kills:** a `std_tests` program with a param type the new reader rejects (run the full `std_golden.sh`;
the store's `ref RP`/`ref CI` params are the ones to check first).
**Effort:** 5-7 days. **Gate-days:** 1.

### Step 3 -- the store over constants (no codegen; typecheck 0, oracles 0, std_golden full)

**Expected:** six store gates identical; `st_parttab_cells(p) = PartTab.size - 3 + 3 * p` (16 + 3P
today, `store.bp:231`) reads as arithmetic over a name; `storelib.parttab` in `tools/stdump.py` reads its
`+2` from a shared constant emitted by... nothing: the python tool must carry `OBJ_HDR = 2` with a
comment naming `Obj.size`. (The tool cannot import a `.bp` constant; that gap is named, not closed.)
**Kills:** any store gate value moves.
**Effort:** 1-2 days.

---

## 6. The honest ceiling

- **Step 1 is verbose by design and it is a human-legibility trade**: `a[i * Node.size + Node.child]`
  is what the store already writes with numbers; the name replaces the number, the arithmetic stays
  visible until step 2 gives typed symbols. An agent writes it without complaint; a human wants `p.child`.
  Step 2 closes that for symbols with a tag; array-backed objects (`a[node]`) never get `p.f` because
  `node` is an i64 index, and that is the honest limit of a language whose objects are cell runs.
- **The tag is syntactic, not inferred.** `let p = f(x)` where `f -> P` tags `p` from `f`'s recorded
  return type; `let p = if c then P {..} else q` does not (no walk) -> untagged -> `.f` refused. A8's
  full walk lifts that; until then the refusal is loud and the fix is a `: P`-typed helper.
- **What this row cannot do about the +2 header**: nothing forces `st_get`'s `+2` and `stdump`'s `+2`
  to be the same constant across two languages. F4's Lean semantics or an exported constants file
  (`bebop.bin` printing `Obj.size` on request) is costed at ~30 lines and not scheduled; named for A23.

---

## 7. VERDICT format for an A22 worker

```
VERDICT: GREEN|RED
step: <1-3>
fixpoint: gen3 == gen4 <md5>   bebop words <before> -> <after> (budget line yes|no)   ABI ok <yes|no> (zones <list>)
constructs: pass=<n> fail=<n>; c40 WORD_DELTA 0; c131 <value> == hand <value>; c132 COMPILEFAIL:<code>
order test: Node before P -> c131 <value> (must equal)
mutation: swapped field names -> c131 <value> (must differ)
store (step 3): slayout <v> schain <v> sevolve <v> scompact <v> scrash_torn <k>/50 smw <v> (all unchanged)
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations>
```
