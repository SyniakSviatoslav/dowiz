Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on A1 for (a) (any time, codeless), A7 (str values) for (b); RESEARCH-DEPS §7 is the evidence base

# A11 Harness dogfooding: measure the spawns, then a bebop-native std-runner lane, then (maybe) chain.bp

## 0. Goal

(a) Codeless: count the fork/execs of one chain and put `chain_spawns` / `chain_spawn_ms` rows into docs/PERF.md; decision gate: spawn + text overhead < 20 % of battery wall closes the question (RESEARCH-DEPS §7c: estimated 15-25 %). (b) If above the gate: a bebop-native std-runner as ONE battery lane (the std_golden lane: 99 tests, 109 `seed` calls, ~700 spawns) using new process builtins, verified byte-for-byte against std_golden.sh's summary lines for three consecutive chains with the old lane kept as the oracle. (c) chain.bp only after (b) and an explicit golden-runner decision by the operator (a frozen runner bin so the compiler never gates itself with its own broken runner). just/Nushell/osh are rejected (dependencies that remove no spawn).

## 1. Scope

(a): tools/perf.py rows; one instrumented chain (`strace -f -e trace=execve -c -o $OUT/execs.txt bash tools/chain.sh ...` if strace exists under this proot, else `bash -x` with `BASH_XTRACEFD` counting `+ ` lines that start an external command; both are estimates -- report which). (b): builtins `sys_execve(path, argv, envp)`, `sys_wait4(pid, status)`, `sys_pipe2(fds)`, `sys_dup3(a, b)`, `sys_kill(pid, sig)`, `sys_getdents64(fd, buf, n)`, `sys_fstat(fd, buf)` (each an emit_sys_* block; `sys_clone(17, 0)` = fork exists -- RESEARCH-DEPS §7d), an in-process `run(bin)` = mmap PROT_EXEC of a .bin + `blr` into its entry with the seed's calling contract (argc/argv cells) under fork isolation (a crashing test must not kill the runner: fork first, run in the child, wait4 for the status), `selfhost/tools/stdrun.bp` (the runner: for each std_test compile with the candidate compiler in-process via `cli_compile`? -- the runner is linked with the compiler by `use`? No: keep the runner a separate program that fork+execs `seed <candidate> compile` for compiles (compile crashes are isolated too) and `run()`s the test bins in forked children; the summary lines printed exactly as std_golden.sh prints them (the battery's `line()` regex contract, tools/battery.sh)). (c): not in this blueprint beyond the decision gate. Out: rewriting the other 38 shell scripts; python lanes (perf/census/check_abi/check_words stay python); a bebop `chain.bp` (needs the golden-runner decision).

## 2. Preconditions

(a): a free box (no worker chain running: strace under proot is ptrace-on-ptrace; test on `tools/chain.sh` of a tiny source first). (b): A7 (`str` values for paths/argv assembly; before A7, argv construction for execve needs NUL-terminated byte arrays built from cells -- possible with str_to_cells but ugly: wait for A7); the seed's calling contract for `run()` (seed/seed.S: last 8 bytes = entry byte offset; argv copied into the arena cells; x9/x24/x25 locals -- seed.S:2, 50-100 verified in the session); check_abi allowlist for the new builtins' words (emit_sys_* naming).

## 3. Design

**(a) Census.** `chain_spawns` = number of execve per chain run (strace -c total for execve, or the `bash -x` count); `chain_spawn_ms` = spawns x measured spawn cost (RESEARCH-DEPS §7b: bash->/bin/true 8.5 ms, seed 4.3 ms; re-measure both on the day); `battery_wall` already recorded (chain_wall minus gen and perf phases -- take it from the chain log timestamps). Decision line in the journal: overhead % = chain_spawn_ms / battery_wall.

**(b) Builtins (asm text; the worker derives words).** Linux AArch64 syscall numbers: execve 221, wait4 260, pipe2 59, dup3 24, kill 129, getdents64 61, fstat 80 (RESEARCH-DEPS §7d); each block: deliver operands to x0..x2 (vs_deliver), `mov x8,#nr ; svc #0`, result in x0 (negative errno kept as is), push REG 0. `sys_execve` needs C strings and NULL-terminated pointer arrays: the runner builds them in a byte region (A7 handles -> `x17 + off` addresses inside the builtin: a `str` handle is converted to an address for the syscall; the argv array = cells holding handles -> the builtin cannot convert an array of handles in place cheaply: define `sys_execve(path: str, argv: [str], envp: [str])` and let the BUILTIN materialise a temporary C-array on the stack: for i in 0..n: `ldr` handle, convert, `str` into a `sub sp` area (n <= 16), NUL-terminate; ~40 words). `run(bin)`: `sys_mapb` the .bin, `mprotect`-less: mmap with PROT_READ|PROT_EXEC directly (a second mapping of the file), read the entry offset (last 8 bytes), set up argc/argv cells per the seed contract in the child's arena, `blr`; exit status via `sys_exit` of the child; the parent wait4s. Fork isolation: `sys_clone(17, 0)` then in the child `run()`; the child's arena = the parent's (copy-on-write pages: fine).

**Runner** `selfhost/tools/stdrun.bp`: reads the test list (getdents64 over bench/vs_rust/std_tests or a static list generated by python into a `.bp` include -- ponytail: static list, regenerated by the same python that maintains std_golden.sh), for each test: fork+exec `seed <candidate> compile <test> <out>` (compile isolation), then fork + `run(out)` capturing stdout via pipe2/dup3, compare the last line with the golden (goldens in bench/vs_rust/std_tests/*.gold or wherever std_golden.sh keeps EXPECT -- the worker reads std_golden.sh:1-60 first), print `PASS/FAIL name` lines and the final `std_golden: N pass, M fail` line byte-identical to std_golden.sh's; J = 3 parallel children pinned to A78 cores via sys_setaffinity (like std_par.sh). tools/battery.sh gets an env switch `STD_LANE=bebop|sh` (default sh until the three-chain verification passes; then the operator flips the default).

**(c) Decision gate.** After (b) is green for three chains: measure battery wall with STD_LANE=bebop vs sh; if the gain is < 10 % of battery wall, stop here (the std lane was the biggest spawn source); else the operator decides on the golden runner (a frozen `stdrun-<md5>.bin` under bench/vs_rust/, rebuilt only by explicit decision) before any chain.bp.

**Invariants.** The old lane stays the oracle (both run, summaries diffed) until the flip; a runner bug can never make the battery greener than std_golden.sh (the diff is the gate); the process cap counts the runner's children (J = 3 + compile children: <= 8 at once).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| tools/perf.py | rows chain_spawns, chain_spawn_ms | perf.py:122 record |
| tools/chain.sh | optional `CHAIN_STRACE=1` instrumentation | chain.sh |
| bebop.bp: emit_sys_execve / wait4 / pipe2 / dup3 / kill / getdents64 / fstat / emit_run + dispatch + reserved names | builtins | emit_call_or_ctor 1527; emit_sys_open 848 as the syscall pattern |
| tools/bpref.py | stubs (subprocess-based or `raise Unsupported` -- the runner is not run under bpref; stubs must at least parse: RESERVED entries) | 47 |
| tools/check_abi.py | new syscall numbers in the sys allowlist (x8 = nr words) | 102 |
| selfhost/tools/stdrun.bp + tools/gen_stdlist.py | the runner | new |
| tools/battery.sh | STD_LANE switch + summary diff | battery.sh |
| docs/DEV-LOOP.md, AGENTS.md L21 (reap) | the lane; children accounting | -- |

## 5. Steps

(a) 1. Instrumented chain on a free box; perf rows; journal decision line. STOP if < 20 %.
(b) 2. Builtins + c83_procs construct (fork+exec `/bin/true`, wait4 status 0; pipe2/dup3 round-trip of one line; fstat size of a known file; kill of a forked child) -- chain `--codegen`.
    3. stdrun.bp + gen_stdlist.py + STD_LANE=bebop side-by-side: three chains with both lanes, summaries diffed byte-for-byte; perf row battery_wall for both.
(c) 4. Decision (operator).
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c83_procs | see step 2 | bpref stub returns fixed values (documented: `sys_execve` etc. are host-only builtins; construct_parity compares bebop's runtime value to EXPECT derived by hand: status 0, line length, file size) | every new builtin |
| c84_run | `run()` of a tiny compiled .bin (c01_lit.bin) in a forked child; parent reads the child's exit status | 0 + the child's printed value via the pipe | in-process run under fork |

Oracle for the lane: std_golden.sh's own output (diff).

## 7. Gates

- (a): rows present; decision line; threshold 20 %.
- (b): chain GREEN; c83/c84; three chains where `diff <(STD_LANE=sh ...) <(STD_LANE=bebop ...)` of the summary lines is empty; battery_wall(bebop) reported.
- RED: any summary difference = the runner's fault by definition (the sh lane is the oracle).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| strace unavailable / ptrace-in-ptrace fails under proot | fall back to bash -x counting; say so | estimate only |
| `run()` in the child corrupts the parent's arena view | fork = CoW; never run in the parent | parent state garbage |
| orphaned children on runner crash | wait4 every child; reap.sh after the lane; kill by pid on timeout | proc cap hit |
| the runner is compiled by the candidate it tests (circular) | compile the runner with the PROMOTED bebop.bin, not the candidate (battery.sh passes both) | a broken candidate breaks its own gate |
| execve argv assembly limits | n <= 16, trap 89 | exit 89 |

## 9. VERDICT format

```
VERDICT: GREEN|RED|STOP-AT-A
(a) chain_spawns <n> chain_spawn_ms <ms> battery_wall <ms> overhead <%> (gate 20 %) method strace|bash-x
(b) builtins landed <list>; fixpoint <md5>; c83/c84 EXPECT; lane diff x3: empty|<diff>; battery_wall sh <ms> vs bebop <ms>
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, RESEARCH-DEPS §7, std_golden.sh + battery.sh read first, seed contract, harness commands and traps (children count against the cap; reap). </context>
<constraints> (a) before (b); the sh lane stays the oracle; runner compiled by the promoted compiler; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A11 (a), then (b) if the gate says so; report. </task>
