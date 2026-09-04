Status: 2026-09-05 CURRENT (T120, decision D11-M; the single table of exit codes — a code that is not here is a bug)

# Exit codes

| code | who | meaning | where |
|---|---|---|---|
| 0..63 | program | `main`'s value modulo 256 is NOT the exit code: the seed prints the value and exits 0; a program exits non-zero only through `sys_exit` or a trap | seed.S |
| 64 | bebop.bin | unknown CLI command (`compile`, `size`, `version`, `run`) | bebop.bp main |
| 80 | program | arena exhausted: a `zeros` crossed x28 (T118) | emit_zeros |
| 81 | program | frame heap exhausted: an array literal / enum ctor crossed the 16 KiB frame (T118) | emit_array_lit, emit_enum_ctor |
| 82 | program | stack overflow guard (planned, T118 b) | — |
| 90 | seed | open failed (source, output or the .bin to run) | seed.S |
| 91 | seed | read failed | seed.S |
| 92 | seed | mmap failed | seed.S |
| 94 | seed | generic failure | seed.S |
| 95 | bebop.bin | expected `)` (emit_paren) or `in` (let-expression) | bebop.bp |
| 96 | bebop.bin | `++` in an expression: string concatenation is not in the surface (T42 d) | emit_expr |
| 97 | bebop.bin | fn body without a tail expression (T42 c) | compile_fn_at |
| 98 | bebop.bin | more than 17 `return` / 18 pending `break` in one fn (T99) | emit_return_stmt / emit_break_stmt |
| 99 | bebop.bin | reserved word used as a function name (T122, planned) | compile_fn_at |
| 128+n | kernel | signal n: 11 = SIGSEGV (unchecked index, deep recursion), 7 = SIGBUS (misaligned sp — a compiler bug) | — |

Gates: compile-time traps are `bench/parity_constructs/neg/*.bp` with `EXPECT=COMPILEFAIL:<code>`,
run-time traps with `EXPECT=RUNFAIL:<code>` (bench/vs_rust/construct_parity.sh); the fuzzer
(bench/fuzz/fuzz.sh) classifies a trap the oracle predicted as TRAP-OK.
