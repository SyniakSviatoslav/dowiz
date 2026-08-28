# Bebop Self-Hosting Status Report

Status of `selfhost/expr_compile.bp` — a compiler written in Bebop that parses a
small language and emits AArch64 machine-code words. This report documents what
is **verified** against what is still **unverified**, the architecture, the ABI,
design-law compliance, and the verification methodology. No code is changed by
this document.

> **Bottom line:** the loop is *partially* closed. `./build/bebopc selfcompile
> ../selfhost/expr_compile.bp` deterministically prints **`123745140208`** — the
> machine-code checksum for the compiler's own source, produced by the compiler
> running inside its own runtime. The emitted AArch64 *executes correctly* for
> the tested subset (`fact(5)==120`, while-sum `==10`, cross-call `add(3,4)==7`,
> `helper(41)==42`), verified through the `exec` builtin (mmap + run). What is
> **not** yet verified: that the self-compiled image is bit-identical to the
> native reference emitter (`native/src/native.c`) across the *full* feature set
> — specifically arrays, strings, `char`, and `str_len` have no native-reference
> cross-check today. That gap is the current work item.

---

> **Stage B update (2026-08-23):** this report describes expr_compile.bp only.
> Since it was written: the corpus grew to 144 files, all strict+check green
> (ROADMAP.md "Current verified state"); backends gained exec-verified slices (aarch64, wasm, NEON,
> GPU/FPGA emit contract); parity vs live C goldens is 20/20 constructs with
> real failure accounting; `make wasm-check` executes emitted modules in
> node/V8; two evaluator root causes were fixed (255-char string cap + chr()
> aliasing -> string arena; cross-invocation env aliasing under in_while ->
> boundary markers). Current honest gaps: ROADMAP.md
> Stage B update.

## 0. Method

All facts below were re-verified against the checked-out tree on 2026-08-20:

- `cd bebop-lang/native && ./build/bebopc selfcompile ../selfhost/expr_compile.bp` → `123745140208`
- `./build/bebopc run ../selfhost/expr_compile.bp self_check` → `0` (all 41 tests pass)
- `./build/bebopc strict ../selfhost/expr_compile.bp` → `strict: PASS (no branchless violations)`, `parsed 0 struct declarations (0 types in registry), 62 functions`
- `grep -c '^fn ' selfhost/expr_compile.bp` → `62`; `wc -l` → `1222` (83,054 bytes)

---

## 1. What "self-hosting" means here (and what it does not yet)

`expr_compile.bp` is a **compiler written in Bebop**: 62 `fn` declarations, ~1200
lines, that lex/parse a tiny source language and emit AArch64 instructions as an
array of decimal `i64` words (Bebop has no hex literals, so every instruction is
an integer constant). It supports, in order of appearance in the file:

| Feature | Emitter / notes |
|---|---|
| Arithmetic (`+ - * /`, parens, precedence) | `emit_term`/`emit_factor`/`emit_binop` |
| Comparison (`< <= == >= >`) | `emit_cmp_op` |
| `if … then … else` (real branches) | `emit_cond` — `B.cond` + `B`, not `csel` |
| Functions (`fn name(a,b,…) { … }`) | `compile_fn` / `compile_fn_at` |
| `let` (mutable via register reuse) | `sym_bind`, `emit_let_stmt` |
| Calls — inline, `bl` (cross-fn), two-pass layout | `emit_call`, `emit_bl_call`, `emit_bl` |
| Identifiers / variables | `emit_ident`, `emit_var` |
| Structs (literal + field access) | `emit_struct_lit`, `emit_field_access` |
| Enums (nullary + payload ctors) | `emit_enum_ctor`, `emit_enum_ctor_nullary` |
| `match` (compile-time, literal scrutinee) | `emit_match`, `emit_match_arm` |
| Recursion | `emit_self_call` (self `bl`) |
| `while` loops | `emit_while_stmt` |
| Multi-function programs | `compile_program` / `compile_program_to` |
| Self-compile bridge | `exec` builtin + `selfcompile` CLI command |

"Self-hosting" here is the *second* stage: the Bebop compiler compiles its **own
source text** to a word array, and the runtime (the C bootstrap in
`native/src/`) parses that `.bp` file, loads the 62 functions, evaluates
`compile_program` with the file's own text as its argument, and prints the
resulting checksum. That is a genuine "compiler compiles itself" loop, but it
runs **inside the C bootstrap's interpreter** (`qtt_eval_binds`), not as a
standalone self-built binary. A fully closed loop (the `.bp` compiler emitting a
freestanding binary that re-compiles itself with no C runtime) is **not** the
claim being made.

**Verified vs. unverified:**

| Claim | Status |
|---|---|
| `selfcompile` prints `123745140208`, deterministic across runs | **Verified** (run above) |
| `self_check()` returns `0` (41/41 regression tests) | **Verified** |
| Emitted code *executes*: `fact(5)==120`, while-sum `==10`, `add(3,4)==7`, `helper(41)==42`, `42` | **Verified** via `exec` (mmap + run) |
| `strict` branchless scan passes; 62/62 fns typecheck | **Verified** |
| Emitted image is bit-correct vs `native.c` for **arith + conditional** subset | Claimed as "faithful port" in the file header; not independently re-derived here |
| Emitted image is bit-correct vs `native.c` for **arrays / strings / `char` / `str_len`** | **Unverified** — the current gap |
| Self-check golden checksums (c1–c36) trace to `native.c` rather than being self-frozen | **Unverified** for features beyond the arith subset |

---

## 2. Architecture

```
   .bp source  (selfhost/expr_compile.bp, 62 fns)
        │
        │  read as a `str` by `selfcompile` / `run` (native/src/main.c)
        ▼
   ┌───────────────────────────────────────────────┐
   │  Bebop compiler (expr_compile.bp)             │
   │  parse → collect_fns → compile_program(_to)   │
   │  two-pass: measure sizes, then emit           │
   └───────────────────────────────────────────────┘
        │  emits AArch64 machine-code words (i64 array, decimal)
        ▼
   AArch64 word array  [ insn0, insn1, …, insnN ]   (one 32-bit word per slot)
        │
        │  exec(words, count, arg0)  — TERM_EXEC, native/src/qtt.c
        ▼
   mmap(RW) → memcpy → __builtin___clear_cache → mprotect(RX) → fn(arg0)
        │
        ▼
   result (i64)  ──►  compared against golden checksums / expected values
```

**Checksum source of truth.** The native reference emitter is
`native/src/native.c` (`emit_expr`, `em_code[512]`, `SAVED_REGS 80`, `N_LOCAL_REGS 10`).
The `.bp` header (lines 13–14) states it is "a faithful port of
`native/src/native.c` emit_expr for the LIT + BIN arith + conditional subset."
For that subset the native emitter is the reference the `.bp` golden checksums
must match. For the features added since (structs, enums, match, `bl`, two-pass
layout, `while`, recursion), the `self_check` golden values are currently the
`.bp` compiler's own frozen output; bit-equality against `native.c` across that
fuller set has not been re-established.

The `exec` builtin (`native/src/qtt.c`, `TERM_EXEC`) is the bridge that makes the
word array *run*: it copies the `i64` array to 32-bit words, `mmap`s anonymous
RW memory, copies, clears the instruction cache, flips to `PROT_READ|PROT_EXEC`,
and calls it as `long (*)(long)` with `arg0` in `x0`. The `selfcompile` command
(`native/src/main.c`, `cmd_selfcompile`) wraps this: parse the file, load the
functions, evaluate `compile_program(source_text)`, print the returned checksum.

---

## 3. ABI

**Registers.** Parameters map `a→x19`, `b→x20`, … — the prologue moves them from
the incoming `x0, x1, …`. All `let`-bound locals and parameters live in the ten
callee-saved registers `x19–x28`, which is why recursion and cross-function `bl`
do not clobber the caller's bindings. This mirrors `native.c`'s
`N_LOCAL_REGS 10`.

**Prologue** (`emit_prologue`, 7 words):
`stp x29,x30,[sp,#-16]!` (`0xa9bf7bfd`) · `mov x29,sp` (`0x910003fd`) ·
five `stp` pairs saving `x19–x28`.

**Epilogue** (`emit_epilogue`, 7 words): five `ldp` pairs restoring `x19–x28` ·
`ldp x29,x30,[sp],#16` · `ret` (`0xd65f03c0`).

**Branch-based `if/else`, not `csel`.** `emit_cond` emits `cmp x0,#0`
(`0xf100001f`) then a **real** `B.cond` (`0x54000000`, patched with an imm19
offset) and a `B` (`0x14000000`, imm26), so only the taken branch executes. The
file comment at lines 685–687 still describes the earlier `csel` design
("evaluate cond + BOTH branches, then csel picks the result"); the code now
branches because `csel` eagerly evaluates *both* branches, which cannot
terminate for recursion (`fact`) and is wrong for non-pure branches. (That
comment is stale — a documentation nit, not a code defect.)

**Two-pass layout for multi-`fn` `bl`.** `collect_fns` records each `fn`'s source
offset; `compile_program_to` measures sizes first, then emits, so cross-function
`bl` targets are known. `emit_bl` encodes the offset as
`0x94000000 + offset + (is_back ? 1<<26 : 0)` — `BL` with an imm26 field,
manually sign-extended for backward calls.

**`sym_bind` register reuse for mutable `let`.** `sym_bind` first consults
`sym_lookup`; if the name is already bound it returns the existing register,
otherwise it allocates `x(19+cnt)`. Re-binding `acc` in a `while` loop therefore
reuses `acc`'s register rather than allocating a new one — this is what makes
loop-carried mutation (`let acc = acc + i`) work without a stack frame.

---

## 4. Design-law compliance

| Design law | How the compiler honors it | Verdict |
|---|---|---|
| **branchless** `Σ k·(k==N)·expr` | Source uses branchless flag arithmetic (`is_match = if nm==name then 1 else 0` → 0/1 flag; `is_id = is_alpha + is_digit + is_us`; `ge48 * le57`) instead of nested conditionals. `strict` scan enforces "no nested if-else in fn bodies" and passes. | **Honored** (source level) |
| **no_std** | Compiler is pure Bebop (`module core {}`); only core builtins (`str`, `str_len`, `char`, `exec`). Native bootstrap is libc-only, zero external deps (Makefile comment). | **Honored** |
| **O(n)** | Single-pass lex/parse; two-pass emit is `O(2n) = O(n)`. No quadratic behavior in the compiler's own code paths. | **Honored** |
| **atomic** | Compiler is a pure, deterministic function — no shared mutable state; every function takes its `insns`/`n`/`stab` buffers explicitly and returns a value. Trivially re-entrant/thread-safe. It does **not** exercise lock-free primitives (single-threaded). | **Honored (weakly)** |
| **vector-first** | Emitted code is **scalar AArch64 only** — no NEON/SIMD, no `vir` lowering. The vector-first machinery exists in `native/src` but is not wired into `expr_compile.bp`. | **Not yet honored** — gap |

The "branchless" law applies to the *compiler's own source*; the code it emits
intentionally uses real branches (see §3) because `csel` cannot express
recursion or side-effecting branches.

---

## 5. Verification methodology

1. **Checksum regression — `self_check()` (41 tests).** `self_check` is a
   zero-arg `fn` that compares `compile`, `compile_fn`, `compile_program`, and
   `run_program` results against 41 hard-coded golden values and returns the sum
   of failures (`0` = pass). Breakdown:
   - c1–c9: `compile(expr)` — arithmetic, precedence, `if/then/else` (9)
   - c10–c11: `compile_fn` single/two-param (2)
   - c12–c16: comparison operators (5)
   - c17–c18: `let` binding (2)
   - c19–c24: cross-function calls, inline + two-pass (6)
   - c25: `let` + arithmetic (1)
   - c26–c28: structs (3) · c29–c30: enums (2) · c31–c32: `match` (2)
   - c33: recursion (`fact`) checksum (1) · c34: `while` checksum (1)
   - c35–c36: `compile_program` multi-fn (2)
   - c37–c41: **execution** via `run_program` — `42`, `fact(5)==120`,
     while-sum `==10`, `helper(41)==42`, `add(3,4)==7` (5)
2. **Execution via `run_program` → `exec`.** The five `run_program` tests close
   the loop end-to-end: source → word array → mmap → execute → observed result.
3. **Strict lint (`strict`).** Enforces the branchless law (no nested `if`) and
   gates on the typechecker; passes.
4. **Typecheck (`check`).** Two-pass elaboration: collect all 62 fn types, then
   typecheck each with earlier fns bound. All 62 pass (`… ok`), types are QTT
   linear types (e.g. `(c :^ω i64 -> i64)`, `Str`, `Vector<0,i64>`).
5. **`make test`.** Runs the full native self-test suite; **does not currently
   invoke `selfcompile` or `self_check`** — the self-host regression is run
   manually today (see §0), which is itself a gap (§6).

---

## 6. Honest gaps & next steps

- **No native cross-check for arrays/strings/`char`/`str_len`.** The self-host
  image is self-consistent (deterministic `123745140208`, executes correctly),
  but it has not been shown bit-identical to `native/src/native.c` for the
  string/array builtins. This is the primary open work item.
- **`self_check` golden checksums are self-frozen.** For features beyond the
  arith/conditional subset, the 41 golden values were frozen from the `.bp`
  compiler's own output; they are not independently re-derived from `native.c`.
- **Self-host not wired into `make test`.** `make test` does not run
  `selfcompile` or `self_check`; a future edit that changes codegen would not be
  caught by CI until run manually.
- **`emit_cond` comment is stale.** It still describes the `csel` design; the
  code emits real branches (see §3).
- **`selfhost/readme.md` is stale.** It lists "fn params in .bp" and "string
  char access" as *not yet*, both of which now work (`char(s,pos[0])`,
  `str_len(s)`, and fn params are used throughout `expr_compile.bp`). The file
  also ends mid-sentence (`bebopc ji…`).
- **Not a freestanding self-built binary.** The loop runs inside the C
  bootstrap's interpreter; producing a `.bp`-emitted standalone binary that
  re-compiles itself is not yet done.
