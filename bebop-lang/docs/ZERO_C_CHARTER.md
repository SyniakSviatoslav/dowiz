# CHARTER: Zero-C Bebop (realistic goal, fixed)

Decision: complete migration of ALL C functionality to Bebop. C leaves
the project entirely. This is recorded as the project's terminal goal —
realistic, because every mechanism it needs already has a working seed
in the tree today.

## Target architecture (end state)

    seed.bin        frozen hand-written AArch64 loader (~64 instrs,
                    no libc): open/read the flat binary, mprotect RWX,
                    arm x27/x28 arena, jump. NEVER modified again.
    bebop.bin       the compiler itself, compiled to AArch64 BY ITSELF
                    (self-bootstrap through the current toolchain once).
                    Reads .bp text via syscalls, emits .bin, executes.
    *.bp            all sources: compiler, stdlib (std/, 73 seeds),
                    kernels, tools, tests.
    *.bin           all artifacts (flat, 4 bytes/word, already shipped).

C files disappear group by group as coverage lands. The interpreter
(tree-walk) is NOT ported — post-C, differential testing becomes
artifact-vs-artifact (two compilers/two flag sets), which the exec_words
pattern already does.

## Milestones (acceptance criteria)

M1 seed loader          seed.bin runs k1.bin on bare syscalls, prints
                        result, exits. NO C executed at runtime. [gate:
                        output == exec_words output]
M2 syscall I/O builtins open/read/write/close/exit + clock available to
                        .bp programs (emitter words + interp mirror);
                        std/file_io.bp wrappers.
M3 self-bootstrap       beboSelf.bin = expr_compile.bp compiled BY
                        itself; beboSelf.bin compiles k-kernels to
                        byte-identical streams vs current toolchain.
M4 CLI-in-.bp           bebop.bin subcommands: compile / run-via-exec /
                        size / version. Args parsed from stack block the
                        loader passes.
M5 module ports         std/ grows .bp twins for toolchain-adjacent
                        algorithms first (sort, rng, checksum, base64,
                        sha256...), golden-vector tested vs C results
                        BEFORE C removal of each.
M6 parallelism          clone/futex via svc; pool.c design reimplemented
                        as .bp work-splitting over the shared arena;
                        compilemany and k7 queries go multi-core.
M7 C deletion           native/src removed except docs; repo = seed +
                        .bp + .bin + docs.

## Non-goals / notes
- Interpreter is retired, not ported: compile-and-run replaces it;
  differential testing continues via dual-artifact execution.
- Wasm/GPU dormant backends stay archived until needed.
- The 106-mode C CLI shrinks to the .bp modes that matter; legacy mode
  names die with their C implementations.

## Why realistic (evidence already in tree)
- Self-hosted compiler emitting verified AArch64: expr_compile.bp ✔
- Flat-binary artifact format + proven runtime contract (x27/x28) ✔
- NEON emission from the compiler (hvham/hvham2) ✔
- Syscall primitive in the language (TERM_SYSCALL) ✔
- std/ library seeds (73 files) ✔
- Compile-once caching, parity corpus, fuzz discipline ✔
