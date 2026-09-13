Status: 2026-09-05 CURRENT (T119, decision D11-M; describes the surface bebop.bin accepts at fixpoint 4c454e21+; tools/bpref.py is a DIFFERENTIAL CHECKER over the subset its docstring declares, not the semantics reference -- `bebop.bp` defines the language, and A23 is the row that makes the division explicit; corrected 2026-09-12 after bpref's `set` node was found copying arrays on write, disagreeing with the compiler for four days with nothing in the battery running its evaluator)

# The Bebop language surface

Bebop is a small, integer-only, self-hosting language compiled straight to AArch64
machine words by `bebop.bin` (itself written in Bebop, `bebop.bp`) and loaded by the
frozen `seed/seed.S`. There is no runtime library, no garbage collector, no strings
beyond literals, and no types at run time: every value is a 64-bit integer (`i64`),
and an array is the address of a run of i64 cells. Types are parsed and discarded by
the compiler; they are checked by the T48 census outside it (tools/typecheck.py over
bpref's AST, invariants.sh rung (vii)) — D12-H (2026-09-06) moves that check into bebop.bp.

## Program

```
program := (use | enum | struct | fn | module)*  -- top level, any order
use     := 'use' '"' PATH '"'                     -- line-initial; textual inclusion (T47)
fn      := 'fn' NAME '(' (NAME ':' TYPE (',' NAME ':' TYPE)*)? ')' '->' TYPE '{' body '}'
enum    := 'enum' NAME '{' CTOR ('(' TYPE ')')? (',' CTOR ('(' TYPE ')')?)* '}'
struct  := 'struct' NAME '{' NAME ':' TYPE (',' NAME ':' TYPE)* '}'   -- literals enabled for the FIRST declared struct ONLY: find_struct returns the first match and emit_field_access resolves every `.f` against it, so a second struct is silently mis-resolved (A22)
module  := 'module' NAME '{' '}'                 -- inert
TYPE    := 'i64' | 'str' | '[' 'i64' ']' | NAME
```

`main` is the entry: `fn main() -> i64` or `fn main(argc: i64, argv: i64) -> i64`.
The program's result is `main`'s value, printed by the seed as a decimal line.
A function body is a sequence of statements followed by ONE tail expression (a body
without a tail expression is a compile-time error, exit 97). Functions may have up to
14 parameters (args in x0..x13); a 15th parameter is NOT a compile error today: the fn gets a `brk #8` prologue and the program exits 8 silently when it is called (docs/TRAPS.md). Recursion is ordinary; there is no inlining.

`use "path"` (T47, nested since T47b) includes the file once, dependencies first, with
the same content-hash dedup for every path; the compiler writes the expanded program to
`<out>.use`. `use "cas://sha256:<64 hex>"` (T80) resolves to `.bcas/<hex>.bp` and the
file's SHA-256 must equal the name, else the compile exits 88 — a module is named by
what it is. `bebop.bin cas add <file>` stores a file under its digest and prints the
address line to paste into a `use`.

## Statements (inside `{ ... }`, separated by `;`)

```
let NAME = expr ;             -- bind or REBIND (symbols are function-scoped: a
                              --   later `let x` updates the same register; there is
                              --   no block scoping and no shadowing)
let _ = expr ;                -- evaluate for effect
let _ = ARR[expr] = expr ;    -- store into an array cell
NAME += expr ;   -= *= /= %=  -- compound rebind
while expr { body } ;         -- loop; the body's tail expression is discarded
return expr ;                 -- leave the function with expr (T99)
break ;                       -- leave the innermost while (T99)
expr ;                        -- an expression statement (value dropped)
```

Reading a symbol that no `let` has executed yet (e.g. bound only inside a loop that
ran zero times, or after `break`) is UNDEFINED (whatever the register holds); the
fuzzer avoids that shape and so should you.

## Expressions, by precedence (loosest first; C precedence since T42(a))

```
comparison   == != < > <= >=      -- yields 0 or 1, chains left to right
bit-or       |
bit-xor      ^
bit-and      &
shifts       << >> >>>            -- `>>` is LOGICAL (lsrv), `>>>` is ARITHMETIC (asrv)
additive     + -
multiplicative * / %              -- `/` and `%` are signed (sdiv, truncating); x/0 = 0, x%0 = x (hardware)
unary        -e  !e               -- neg; !e = (e == 0)
postfix      ARR[expr]            -- load (no bounds check: reading past the end is UNDEFINED)
             f(args)              -- call (user fn, enum ctor, builtin)
primary      literal | NAME | '(' expr ')' | '[' e0, e1, ... ']' | if | match | let-in
```

All arithmetic is 64-bit wrapping. Literals: decimal, `0x` hex, negative literals.
Shift counts are taken mod 64.

```
if c then a else b               -- an EXPRESSION; only the taken arm runs
(let NAME = e in expr)           -- expression-level binding; `;` is a synonym for `in`
[e0, e1, ...]                    -- array literal on the frame heap (<= 511 elements)
match CTOR(payload) { CTOR => expr, CTOR(x) => expr, ... }   -- COMPILE-TIME: the
                                 -- scrutinee must be a literal constructor; `x` binds the payload
```

## Builtins (calls that the compiler emits inline)

| call | meaning |
|---|---|
| `zeros(n)` | allocate n zeroed i64 cells from the arena (never freed; exit 80 with `trap 80: arena exhausted (zeros crossed x28)` on stderr when the arena is exhausted, T118/T90) |
| `str_len(s)`, `char(s, i)` | length / byte of a string literal (`"..."` is only valid as an argument) |
| `clock_ms()` | CLOCK_MONOTONIC in ms |
| `sys_open(cells, len, flags)`, `sys_read(fd, buf, n)`, `sys_write(fd, buf, n)`, `sys_close(fd)`, `sys_readbuf(fd, len)`, `sys_slurp(fd, len)`, `sys_mmap(addr, len, prot, flags, fd, off)`, `sys_munmap(a, len)`, `sys_ftruncate(fd, len)`, `sys_rename(a, la, b, lb)`, `sys_export(cells, n, path, len)`, `sys_exit(code)` | raw Linux syscalls |
| `sys_arena_base()`, `sys_arena_end()`, `sys_clone(flags, stack_top)`, `sys_cond_set(c, arr, i, v)`, `sys_futex_wait_guard(c, arr, i, v)`, `sys_futex_wake(arr, i, n)`, `sys_atomic_add(arr, i, v)`, `sys_exit_thread_guard(c, code)` | threads over the shared arena (T45; see selfhost/std/pool.bp). `sys_exit_thread_guard` exits the calling THREAD (svc 93) iff `c != 0` (T127: it was exit_group before) |
| `hvham(a, b, n)`, `hvham2(...)` | NEON popcount of a^b over n words |
| `clz(x)` | count leading zeros of the 64-bit word, clz(0) = 64 (T105; seeds the Newton isqrt) |
| `crc32(cells, n)` | zlib crc32 of n bytes held one per cell (CRC32B loop, T109) |
| `crc32x(cells, off, n)` | zlib crc32 of the raw little-endian bytes of n cells from cells[off] (CRC32X, 8 B per step, T109b; the store's integrity crc) |
| `crc32b(s)` | zlib crc32 of the bytes of the NUL-terminated string `s` (A7 step 1; repr-independent contract: crc over exactly `str_len(s)` bytes, valid for raw pointers today and handles after the migration) |
| `sys_msync(addr, len, flags)` | msync (227), the store's durable-commit call (T110) |
| `sys_setaffinity(arr, idx)` | sched_setaffinity(0, 8, &arr[idx]) — pin the calling thread to the mask in arr[idx] (T72) |
| `scan(s, pos, class)` | advance `pos[0]` over bytes of one class and return the new pos: 0 = whitespace, 1 = ident `[0-9A-Za-z_]`, 2 = not-`"`-not-`\`, anything else = not-newline. Stops at the `pos[1]` length bound, so it never reads past it (A9 step 3, scalar form) |
| `sys_fsync(fd)` | fsync (74), the store's durable-commit partner to `sys_msync` |
| `sys_mprotect(addr, len, prot)` | mprotect (226) |
| `sys_run(addr, size, argc, argv)` | execute an already-mapped RX image in-process (issued from a `sys_clone` child; see selfhost/std/gb_run.bp) |
| `sys_wait4(pid, status, opts, rusage)` | wait4 (260) on a `sys_clone` child |

## Memory model

- Arena: one 256 MB anonymous mapping (x27 cursor, x28 end). `zeros` bumps it; nothing
  is freed, and an allocation survives the return of the fn that made it (T126, 2026-09-05:
  fns with >= 9 symbols used to save/restore x27/x28 and silently roll their allocations
  back — construct c43_arena_persist guards this). Crossing the end exits 80.
- Frame heap: array literals and enum constructors live in a 16 KiB per-call frame (x14);
  overflowing it exits 81. A `while` body's frame allocations are released at the back-edge
  and at loop exit (T43), so an array literal bound INSIDE a loop body is per-iteration:
  reading it after the loop is a use-after-release (the next literal overwrites it) — bind
  such arrays before the loop. The reset is skipped (the body leaks instead) when a `let`
  in the body rebinds an outer name to a bare literal or stores one (construct c34).
- Frame heap: 16 KiB per function activation (x14). Array literals and enum
  constructors live there and die at return. `while` bodies that allocate are reset
  per iteration when the compiler can prove no pointer escapes (T43); otherwise they
  leak within the frame. Overflow exits 81.
- Stack: eval values and spills (x15). Deep recursion overflows the 64 MiB process
  stack (exit 82 with `trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)` on stderr, T118b/T90; the codes are in docs/TRAPS.md).
- Arrays carry no length; indices are not checked (T48 will add `[T]` with length).

## Exit codes of a compiled program and of the compiler

See docs/TRAPS.md.

## What is NOT in the language

**THREE OF THE ENTRIES BELOW ARE NOW REQUIRED FEATURES, NOT PERMANENT EXCLUSIONS (operator,
2026-09-13, binding): FRACTIONS, STRINGS AS VALUES, and MODULE CONTENTS.** The operator's
reasoning, recorded because it decides the DESIGN and not just the priority: the problem was
never the concepts, it was the industrial implementations of them -- heaps, rounding
indeterminacy and dynamic expansion. Constrained instead to bit-exactness, zero allocation and
compile-time folding, all three fit the substrate:

- **Fractions: Q32 fixed point, NOT IEEE-754 and NOT posit.** IEEE's five basic ops are
  deterministic on one ISA, but the verification bill is Lean's opaque `Float`, an unverifiable
  hardware axiom and a second register file; posit costs ~75-80 words per software multiply
  against 8-10 for a hardware-backed Q32 form, and its `clz` and variable shifts close F6's
  `ring` proof route. Q32 already has the clean bit structure, the first theorem and 25 oracles.
  Lands as ROADMAP A8's already-reserved type tag 6 `fp`, not as a new row.
- **Strings: a `str` is a VIEW and a `[u8; N]` is the BUFFER**, both on the same linear arena,
  with no allocation the program did not write. There is no dynamic concatenation and no
  collector. This is already how the tree writes strings by hand -- `diag_str(buf, at, m: str)`
  writes into a caller-owned destination, and `qdsl_explain`/`qdsl_int_to_str` are hand-rolled
  `buf`/`bp[0]` writers. Lands as A7 step 2 (the producer) + A8 tag 7 `[u8]` + F3's declared
  length at cell width 1, not as a new row.
- **Module contents: pure namespace flattening at elaboration**, fully expanded before emission,
  zero words in the binary. No dynamic import and no runtime linking, ever. Reference syntax is
  `::` (`m::f`), which is lexically free today -- `m::f()` exits 101 with a proper diagnostic --
  while `m.f` would collide with field access. Lands on ROADMAP A25's textual rewriter.

What Bebop refuses is the LUXURY these features are usually chosen for: strings that live as long
as the program, and fractions whose behaviour depends on the machine. See
`docs/RESEARCH-LANG-EXPANSION-2026-09-13.md` for the full costing.

**Until those rows land, two forms are SILENTLY ACCEPTED and both are defects (MEASURED
2026-09-13, ROADMAP WAVE 0):** `let x = 1.5; x` compiles rc=0 and prints garbage that is not
stable across compilers (516661588010 on `89f889a4`, 495807598623 on `0780f16f`), and
`module m { fn f() -> i64 { 7 } }` compiles and prints 7 with the braces invisible and `f` leaked
to global scope. `tools/bpref.py` refuses both, so the oracle is right and the compiler is wrong.

Still genuinely not in the language: string concatenation (`++` is rejected, exit 96), struct
literals (disabled), bounds checks, garbage collection.
**Closure emission, generic monomorphisation, and dependent-type checking are NOT
yet implemented, and NEITHER IS THEIR SURFACE SYNTAX.** This paragraph used to claim the
syntax "IS parsed and erased by the compiler as of A16 Phase 1". Measured 2026-09-12 and
false: `bebop.bp` has no code for `requires` / `ensures` / `theorem` at all. The parser
DISCARDS everything between a signature's `)` and its `{`, and discards unrecognised
top-level text, so
`fn add(x: i64, y: i64) -> i64 requires @@@ %%% not_a_thing ensures 1 2 3 ][ { x + y }`
compiles and runs, as does a top-level `theorem <nonsense> ][ @@@`. That is not erasure of
parsed syntax, it is silent acceptance of invalid text -- and `tools/f8_dt.py` passes on it
because it only asks whether such a program compiles. Tracked on ROADMAP row F8; the first
deliverable there is a NEGATIVE construct that must COMPILEFAIL.
