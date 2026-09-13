Status: 2026-09-13, created from RESEARCH-LANG-EXPANSION-2026-09-13.md §1.2, operator decision (added today). The ONE source rewriter for modules, monomorphisation, and test expansion.

# A25 Textual rewriter: one mechanism for three elaboration steps

## 0. Goal

A single compile-time pass that rewrites source text, producing the `.use` file that becomes the canonical program text consumed by the compiler, bpref.py, typecheck.py, and the Lean conformance harness. The pass is a ONE **rewriter that serves three rows** (A16 step 4, A24 step 2, and module contents) so that the two primitives (`src_copy_range`, `src_rename_idents`) are written once instead of three times, the `.use`-writing path is one instead of three, and position remapping for diagnostics (`diag_exit`) is solved once, not thrice. Gates: `.use` byte-identity on three fixtures; `diag_check.sh` positions correct after expansion; 0 words in any compiled program.

## 1. Why ONE mechanism, not three

### 1.1 A16 step 4: monomorphisation in the `use_expand` shape

A16-closures-generics-hof.md:78-80 and :222-224 specify monomorphisation as "instantiation textual in the `use_expand` shape". When a generic function `f[T]` is called at concrete types whose codegen differs (`[u32]` vs `[i64]` params, or other width-dependent differences), the source text is rewritten to emit `f__u32` and `f__i64` function definitions, and every call to `f[u32](...)` becomes a call to `f__u32(...)`. This is a TEXTUAL rewrite of the source, followed by emission through the same `use_expand` shape.

### 1.2 A24 step 2: test block expansion

ROADMAP.md:110 names the blocker for A24 step 2 as "the missing primitive -- 'append a RANGE of the source' -- is in no file". Test blocks `test { ... }` inside a function expand to a generated `fn test_main() { ... }` that runs the test and exits. The expansion is also a textual rewrite of the source text, inserting the test function before the original function that declared it, and rewriting calls to `sys_run(test_main, ...)`.

### 1.3 Module flattening (M1 in the RESEARCH ladder, §5.3)

A module declaration `module m { fn f() { ... } fn g() { ... } }` flattens to global functions `m__f` and `m__g` with a rewritten `.use` text where:
1. Every definition `fn f` inside the module becomes `fn m__f`
2. Every unqualified call `f(...)` inside the module becomes `m__f(...)`
3. Every qualified call `m::f(...)` anywhere in the program becomes `m__f(...)`

This is also a textual rewrite: copy the module's content, prefix the names, rename the references, write to `.use`.

### 1.4 The shared primitives

All three transformations use exactly two operations:
- **`src_copy_range(dst, s, from, to)`**: copy bytes from source `s` starting at position `from` to position `to` (exclusive) into destination buffer `dst`. This is what A24 step 2 called "in no file".
- **`src_rename_idents(dst, s, from, to, table)`**: copy bytes from `s[from..to]` into `dst`, but rewrite every identifier whose 131-rolling hash (bebop.bp:151-163) is a key in `table` to the corresponding value. Skip strings (anything between `"` ... `"`) and comments (`//` to end of line) exactly as `collect_fns` does (bebop.bp:5978-5999).

With these two primitives and a driver (one `if` statement per transformation type), A16 step 4, A24 step 2, and module flattening all land in ONE row instead of three, with one `.use`-writing path and one position-remapping rule for diagnostics.

## 2. The existing foundation: `use_expand` in bebop.bp

The compiler already writes a `.use` file containing the flattened program text (docs/LANGUAGE.md:32-33: "`use` includes the file once... the compiler writes the expanded program to `<out>.use`"). The function `use_expand` at bebop.bp:7030-7049 implements this today:

```
fn use_expand(srcv: str, tmpc: [i64], tmpl: i64) -> str {
  let seen = zeros(64);       // dedup table for use statements
  let nseen = zeros(1);
  let buf = zeros(1600000);   // output buffer for the rewritten text
  let off = zeros(1);         // write offset in buf
  let nuse = use_scan(srcv, buf, off, seen, nseen);    // scan for `use` statements
  let base = off[0];
  let _ = off[0] = use_append_str(buf, base, srcv);    // append main source
  let _ = use_comment(buf, base, srcv);                // comment out `use` lines
  let total = off[0];
  // [write buf to .use file, read it back]
  let ofd = if nuse > 0 then sys_open(tmpc, tmpl, 578) else 0 - 1;
  let _ = if nuse > 0 then sys_export(ofd, buf, total) else 0;
  let _ = if nuse > 0 then sys_close(ofd) else 0;
  let rfd = if nuse > 0 then sys_open(tmpc, tmpl, 0) else 0 - 1;
  let out = if nuse > 0 then sys_slurp(rfd, 1600000) else srcv;
  let _ = if nuse > 0 then sys_close(rfd) else 0;
  out
}
```

This function today handles `use` statement inclusion and deduplication. A25 extends it with:
1. A driver parameter selecting which transformation (module / monomorphisation / test) to apply
2. Calls to `src_copy_range` to copy ranges efficiently
3. Calls to `src_rename_idents` to rewrite identifiers in those ranges
4. A position map (see §3) to keep diagnostics correct

The primitives fit into this shape because they write to the same `buf` and update the same `off` offset.

## 3. Module syntax and position preservation

### 3.1 The `::` operator and `m__f` flattening

Module reference syntax is `m::f()`, where `::` is a lexically free token (RESEARCH-LANG-EXPANSION-2026-09-13.md, line 489: "m::f() exits 101 `unbound symbol`" today, so `::` is not yet used and the error is loud and correct). The operator 2026-09-13 decision (ROADMAP.md and the RESEARCH document) chooses `::` over `m.f` because:
- `m.f` collides with field access (e.g. `obj.x` is a field; `m.f` would be ambiguous)
- `::` is already a known token (lexical freedom MEASURED), and `m::f()` exits 101 today

The flattened spelling is `m__f` (two underscores), preserving the 4-character length of `m::f` (both are 4 bytes: m, :, :, f vs m, _, _, f). This length preservation means text can be expanded without shifting byte positions, so `diag_exit` reports remain pointing at the original source.

### 3.2 Position remapping: the hardest unsolved part

When `src_expand` rewrites text (e.g. copying a range and renaming identifiers within it), the byte positions in the output `.use` file may shift if:
- Identifiers are renamed to different lengths (e.g. `f` -> `m__f` is 1 -> 4 bytes, a shift of +3)
- Blocks are copied and inserted, changing the offset of later code

The diagnostic system reports errors as `<line>:<col>` (bebop.bp:43-44), extracted from the **original source**. When the `.use` file has expanded text, the compiler runs on that expanded text and produces positions relative to `.use`. A correct error report must map those positions back to the **original source** so the user sees line:col in their input file, not in the expanded `.use` file.

**Three approaches, with their costs:**

1. **No remapping (current state)**: Errors in expanded text point to `.use` byte positions, which are wrong for the user's original source. This is unacceptable but easy.

2. **Position map (ideal)**: A25 writes a position map alongside `.use`, mapping every byte in `.use` back to (original_file, original_byte). The compiler reads this map and rewrites all diagnostic positions. Cost: ~200 lines in bebop.bp for map construction, ~50 lines in the emitter for position translation, and a .posmap file written alongside .use. Complexity: moderate. This approach is **SPECULATIVE** -- the implementation strategy is clear but unmeasured.

3. **Length-preserving rewrites only**: Constrain A25 to emit only rewrites that preserve byte lengths (e.g. `m__f` for `m::f`, padding or fixed-length names). Cost: lower (the table-driven rewrite knows only substitutions of equal length), but loses flexibility for future rows. Viable for the first cut (M1 only), but A16 step 4 and A24 step 2 may need longer spellings.

**The decision here is UNSOLVED**: A25 lands with approach 1 (no remapping; diagnostics in `.use` coordinates are visible but imperfect) and the position map is gated as a follow-up. Alternatively, a hand-rolled position map suffices for three fixtures (module, monomorphisation, test) and the architecture is proven before general-purpose rewrites land.

## 4. The two primitives

### 4.1 `src_copy_range(dst, s, from, to) -> new_off`

Copy bytes `s[from..to]` into `dst` starting at the current write offset, returning the new offset. Bounds-checked: if `to > len(s)` or `from > to`, exit 84. Used by all three drivers to efficiently copy source text ranges without character-by-character loops.

**Anchor**: this primitive implements what ROADMAP.md:110 called "in no file" for A24 step 2. The function `use_append_str` (bebop.bp:6882) is the whole-string special case of it.

### 4.2 `src_rename_idents(dst, s, from, to, table) -> new_off`

Copy bytes `s[from..to]` into `dst`, but rewrite every identifier whose 131-rolling hash (bebop.bp:151-163, the same hash `collect_fns` uses) is a key in `table` to the value in `table`. Skip strings (quoted regions) and comments (`//` ... newline) exactly as `collect_fns` does (bebop.bp:5978-5999). Bounds-checked: if `to > len(s)` or `from > to`, exit 84.

**Usage**: Module flattening passes a table `{hash(f) -> "m__f", hash(g) -> "m__g", ...}` to rename function definitions and calls. Monomorphisation passes a table `{hash(f) -> "f__u32", ...}` to rename instantiated functions. Test expansion does not use renaming (test functions are new).

## 5. The drivers: module, monomorphisation, test

Each driver is a ~40-line function that builds the rewritten source using `src_copy_range` and `src_rename_idents`. All three are embedded in A25 and share the primitives and the `.use`-writing infrastructure. The driver is selected at compile time based on what transformations the program contains (modules, generics, or test blocks).

## 6. Output: the `.use` file becomes the canonical text

The rewritten source is written to `<out>.use` (already done by `use_expand` today). Under A25, **this `.use` text is the ONE program text consumed by**:
- The compiler (bebop.bin) emits instructions to match the `.use` text
- `tools/bpref.py` executes the `.use` text (no rewrite; it reads `.use` directly)
- `tools/typecheck.py` type-checks the `.use` text
- The Lean conformance harness runs the `.use` text through F4's semantics

Before A25, the four implementations (compiler, bpref, typecheck, Lean) each handled modules, test blocks, and generics separately or not at all. After A25, they all read the same flattened `.use` text and need not re-implement elaboration.

## 7. Gates (from RESEARCH-LANG-EXPANSION §5.3)

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| A25 | `src_copy_range` + `src_rename_idents` + `.use` driver skeleton | one rewriter | `.use` byte-identity on 3 fixtures (a module, a generic instantiation, a test block); `diag_check.sh` positions still correct after expansion | success |
| M1 | `mod_expand`: `module m { }` with contents, `m::f`, prefixing | modules | `c_mod2` MATCH; `neg/c_mod_unresolved` COMPILEFAIL:<code> with line:col; `.use` twin byte-identical; census +2-3 b.cond | success |
| (A16 step 4 / A24 step 2) | `mono_expand` / `test_expand` as drivers | generics, test blocks | their own rows' gates, now ~2 days each instead of weeks | success |

## 8. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp: new `src_copy_range`, `src_rename_idents` + driver selection | textual primitives + module/generic/test routing | bebop.bp:7030 (use_expand entry) |
| bebop.bp: `use_expand` extension | call the driver; write `.use`; (future: position map) | use_expand |
| bebop.bp:emit_call_or_ctor, T122 table | route `m::f` calls to the flattened `m__f` | 1527 (emit_call_or_ctor) |
| tools/bpref.py, tools/typecheck.py | read `.use` text, not raw source | unchanged core logic; switched input source |
| formal/Bebop/Conformance.lean | run `.use` text through the semantics | unchanged; uses the flattened program |

## 9. The binding operator decision: modules elaborate to `.use`

A25's output is the one program text consumed by all four implementations. This is a **binding decision** made by the operator 2026-09-13 in the RESEARCH-LANG-EXPANSION document: "that flattened text is the ONE program text `tools/bpref.py`, `tools/typecheck.py` and the Lean harness consume, so the flattener exists once, in Bebop, instead of four times" (§1.2, line 70). This decision locks the architecture: elaboration (module flattening, monomorphisation, test expansion) is resolved in A25, the output is `.use`, and every downstream implementation reads the same flattened text.

## 10. Scope: what this row DOES and does NOT do

**In:**
- `src_copy_range` and `src_rename_idents` primitives
- Module contents: `module m { fn f() { ... } }` becomes global `fn m__f` with `.use` rewrite
- Module references: `m::f()` becomes `m__f()`
- Monomorphisation driver skeleton (landing fully in A16 step 4)
- Test expansion driver skeleton (landing fully in A24 step 2)
- Position map architecture (SPECULATIVE; implementation deferred)

**Out:**
- Position remapping implementation (gated as follow-up)
- Semantic changes to the language (elaboration happens before codegen, not during)
- Separate `-mono` or `-flatten` compiler passes (A25 is the only pass)

## 11. Cost and schedule

**Estimate from the RESEARCH document (§1.2):**
- **250-400 lines of bebop.bp**, 8-12 fns
- **0 words in any compiled program** (compile-time pass, no runtime code)
- **+10-15 b.cond** in the compiler (one `if` arm per driver type; A24 step 1 precedent was +16 b.cond)
- **2 weeks** (250-400 lines at ~100 LOC/week baseline, with two weeks for testing and position-map design)
- **BLOCKS A16 step 4, A24 step 2**, and module contents; should land **before any of them**

## 12. House style: attribution and formalism

- `.use` file output: docs/LANGUAGE.md:32-33
- 131-rolling hash: bebop.bp:151-163 (the same hash `collect_fns` and `src_rename_idents` use)
- Position tracking already implemented: bebop.bp:43-44 (`diag_exit` uses source positions)
- Module syntax decision: ROADMAP.md (operator 2026-09-13)
- Monomorphisation shape: A16-closures-generics-hof.md:78-80, :222-224 (`use_expand` shape)
- Test expansion blocker: ROADMAP.md:110 ("in no file" primitive)
- Drivers and gates: RESEARCH-LANG-EXPANSION-2026-09-13.md §1.2, §5.3
- `m::f()` exits 101 (MEASURED): RESEARCH-LANG-EXPANSION-2026-09-13.md :489, bebop.bp:100 (unbound symbol diagnostic)

---

**Summary**: A25 is one row for three elaboration steps, sharing two textual primitives (`src_copy_range`, `src_rename_idents`) and one `.use`-writing path. The output is the canonical program text consumed by compiler, bpref, typecheck, and Lean. Position remapping for diagnostics is SPECULATIVE and deferred to a follow-up; the initial gate verifies `.use` byte-identity on three fixtures and `diag_check.sh` positions.
