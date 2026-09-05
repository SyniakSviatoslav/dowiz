Status: 2026-09-05 CURRENT (T119, decision D11-M; describes the surface bebop.bin accepts at fixpoint 4c454e21+; tools/bpref.py is the executable grammar and semantics reference; keep the two in step)

# The Bebop language surface

Bebop is a small, integer-only, self-hosting language compiled straight to AArch64
machine words by `bebop.bin` (itself written in Bebop, `bebop.bp`) and loaded by the
frozen `seed/seed.S`. There is no runtime library, no garbage collector, no strings
beyond literals, and no types at run time: every value is a 64-bit integer (`i64`),
and an array is the address of a run of i64 cells. Types are parsed and, today,
discarded (T48 will check them).

## Program

```
program := (use | enum | struct | fn | module)*  -- top level, any order
use     := 'use' '"' PATH '"'                     -- line-initial; textual inclusion (T47)
fn      := 'fn' NAME '(' (NAME ':' TYPE (',' NAME ':' TYPE)*)? ')' '->' TYPE '{' body '}'
enum    := 'enum' NAME '{' CTOR ('(' TYPE ')')? (',' CTOR ('(' TYPE ')')?)* '}'
struct  := 'struct' NAME '{' NAME ':' TYPE (',' NAME ':' TYPE)* '}'   -- literals disabled (T43 rest)
module  := 'module' NAME '{' '}'                 -- inert
TYPE    := 'i64' | 'str' | '[' 'i64' ']' | NAME
```

`main` is the entry: `fn main() -> i64` or `fn main(argc: i64, argv: i64) -> i64`.
The program's result is `main`'s value, printed by the seed as a decimal line.
A function body is a sequence of statements followed by ONE tail expression (a body
without a tail expression is a compile-time error, exit 97). Functions may have up to
14 parameters (args in x0..x13). Recursion is ordinary; there is no inlining.

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
| `zeros(n)` | allocate n zeroed i64 cells from the arena (never freed; exit 80 when the arena is exhausted, T118) |
| `str_len(s)`, `char(s, i)` | length / byte of a string literal (`"..."` is only valid as an argument) |
| `clock_ms()` | CLOCK_MONOTONIC in ms |
| `sys_open(cells, len, flags)`, `sys_read(fd, buf, n)`, `sys_write(fd, buf, n)`, `sys_close(fd)`, `sys_readbuf(fd, len)`, `sys_slurp(fd, len)`, `sys_mmap(addr, len, prot, flags, fd, off)`, `sys_munmap(a, len)`, `sys_ftruncate(fd, len)`, `sys_rename(a, la, b, lb)`, `sys_export(cells, n, path, len)`, `sys_exit(code)` | raw Linux syscalls |
| `sys_arena_base()`, `sys_arena_end()`, `sys_clone(flags, stack_top)`, `sys_cond_set(c, arr, i, v)`, `sys_futex_wait_guard(c, arr, i, v)`, `sys_futex_wake(arr, i, n)`, `sys_atomic_add(arr, i, v)`, `sys_exit_thread_guard(c, code)` | threads over the shared arena (T45; see selfhost/std/pool.bp) |
| `hvham(a, b, n)`, `hvham2(...)` | NEON popcount of a^b over n words |
| `clz(x)` | count leading zeros of the 64-bit word, clz(0) = 64 (T105; seeds the Newton isqrt) |
| `crc32(cells, n)` | zlib crc32 of n bytes held one per cell (CRC32B loop, T109) |
| `crc32x(cells, off, n)` | zlib crc32 of the raw little-endian bytes of n cells from cells[off] (CRC32X, 8 B per step, T109b; the store's integrity crc) |
| `sys_msync(addr, len, flags)` | msync (227), the store's durable-commit call (T110) |
| `sys_setaffinity(arr, idx)` | sched_setaffinity(0, 8, &arr[idx]) — pin the calling thread to the mask in arr[idx] (T72) |
| `sys_exit_thread_guard(cond, code)` | exit the calling THREAD (svc 93) iff cond != 0 (T127: it was exit_group before) |

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
  stack (SIGSEGV today; exit 82 planned, T118 b).
- Arrays carry no length; indices are not checked (T48 will add `[T]` with length).

## Exit codes of a compiled program and of the compiler

See docs/TRAPS.md.

## What is NOT in the language

Strings as values, string concatenation (`++` is rejected, exit 96), struct literals
(disabled), closures, generics, floats (Q32 fixed point lives in selfhost/prelude/fp.bp),
modules with contents (only `use` inclusion), bounds checks, garbage collection.
