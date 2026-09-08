Status: 2026-09-08, owner main session, grounded at 1ed3719 / bebop.bin 7939ad7e. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §2 (proposal 1, verdict ALREADY + NARROW-ADOPT). This is a SAFETY row, not a performance row. PROPOSAL pending operator decision.

# C1 checked kernel dialect -- the compiler is the verifier

## 0. Goal

A `kernel fn` compiles under a restricted dialect whose emitted code cannot read or write outside
the data it was handed. The dialect replaces the WASM/eBPF sandbox the operator proposed: the
verifier is the compiler that already fixpoint-tests and fuzzes itself, at native speed, with no
new dependency.

Gate: a corpus of 10^4 generated hostile kernels (out-of-range indices, forged `ref` values,
`sys_*` attempts, unbounded loops) produces **0 SIGSEGV / TRAP-82 and 100 % loud traps**; K6
ns/row with checks on is **<= 1.2x** checks off; the sgraph2 frontier BFS fold is unchanged.

## 1. Why this row exists at all

The proposal's premise -- "the DB only verifies memory safety and runs the code" -- is the status
quo minus the verification. `gb_run.bp:379-401` compiles a generated kernel in-process,
`:241-271` maps its image PROT_READ|EXEC and the store PROT_READ and `blr`s into it. What is
missing is not a bytecode. It is that **nothing checks anything**:

- a bebop kernel has no bounds checks at all (docs/LANGUAGE.md:68 "no bounds check: reading past
  the end is UNDEFINED"; :129 lists them under "What is NOT");
- the dispatch is `sys_clone(17, ...)` (`gb_run.bp:246`) -- `SIGCHLD` alone, no `CLONE_VM`, so a
  real fork, and **a fork keeps `MAP_SHARED` mappings shared**;
- `st_open` maps the store `prot=3, flags=1` (`selfhost/prelude/store.bp:146`).

Therefore a kernel dispatched by a process that holds a writer's mapping can **write the store
file**, and the B3 gate is safe only because it happens to map read-only for the dispatch
(`bench/vs_rust/std_tests/gb_pool.bp:229`, `st_map_ro`). The main session confirmed all four
facts from the flags on 2026-09-08 (see the verification section of the research document).

"Physical isolation" today is conditional on the parent's mapping mode at fork. The `run`-inside-
fork rule was chosen for the parent's REGISTER state (`gb_run.bp:241-251`), never as a security
boundary, and it has been read as one.

B3's open TRAP-82 defect is the first test case: a `.gbpool` written by one compiler and read by
another has its kernels dispatched anyway and the children die with `trap 82`. A kernel running
with no fence is exactly what that is.

## 2. Scope

**In.** (i) a `kernel fn` marker and a dialect flag threaded through `compile_fn_at`; (ii) bounds
checks on `[T]` and `ref T` access inside such a fn; (iii) a compile-time reject of every `sys_*`
builtin in the dialect; (iv) a step budget; (v) the dispatching parent dropping RW mappings
before `sys_clone`.

**Out.** Any sandbox for code the local app did not author -- a kernel arriving from a PEER is a
signature problem, not a sandbox problem (MANIFESTO §3.4: ML-DSA-signed code blobs against a
pinned root; docs/LANG-DB-DESIGN.md:225 already rules that a store file cannot inject code,
because code bytes come only from the local `.bcas/<sha256>.bin`). Also out: WASM and eBPF
themselves -- see §6.

## 3. Design

**(i) The check.** `docs/RESEARCH-NOPOINTERS-SQL-2026-09-06.md:84-97` already prices it:
`cmp xidx, xlen ; b.hs <trap>` = 2 words, or `tbnz` = 1 word when the table length is a power of
two. The length must come from somewhere trustworthy, which is why this row **depends on A8**:
a `[T]`/`ref T` typed table carries its length, an untyped `[i64]` does not.

**(ii) The trap.** A new `brk #<code>` in the entry stub's handler family (docs/TRAPS.md), not a
silent clamp. The gate below counts loud traps, so a check that clamps instead of trapping fails
it.

**(iii) `sys_*` rejection.** Same mechanism `scan` already uses to be a reserved word
(`compile_fn_at`'s rsv3 guard, ROADMAP A9). Inside a `kernel fn`, any `sys_` name is a
compile-time `diag_exit` with its own code, and the diagnostic is a construct.

**(iv) Step budget.** `selfhost/std/swpmu.bp:4-7`'s software step counter, decremented on the
loop back edge, trapping at zero. This is the dialect's answer to "unbounded loops over 10M
edges", which is the one thing the eBPF verifier genuinely buys.

**(v) The mapping fence.** The parent `mprotect`s its store mapping to `PROT_READ` before
`sys_clone` and restores it after `sys_wait4`, OR the child does it first thing. Decide by
measurement: the parent form costs two syscalls per dispatch against a 1.04 ms fork floor
(ROADMAP B3), i.e. nothing; the child form is racier to reason about. **Prefer the parent form.**

## 4. Files and functions touched

| file:fn | change |
|---|---|
| `bebop.bp` `compile_fn_at` | parse `kernel fn`; set a dialect bit in fntab; reject `sys_*` names under it |
| `bebop.bp` `emit_array_index` / the `ref` access path | emit the 2-word check (1 word on power-of-two) when the dialect bit is set |
| `bebop.bp` `emit_while_stmt` | decrement + test the step budget on the back edge under the dialect bit |
| `selfhost/std/gb_run.bp:241-271` | `mprotect` the store mapping read-only around the dispatch |
| `docs/TRAPS.md` | the new trap code row |
| `bench/parity_constructs/` | 2 constructs: a checked access that traps, a reject of `sys_*` |
| `tools/bpref.py` | mirror the trap and the reject |

## 5. Steps

1. The mapping fence alone (`gb_run.bp` + `mprotect`), no compiler change. It is independently
   valuable, it closes the confirmed hazard, and it is one gate run.
2. `kernel fn` + `sys_*` rejection (diagnostic only, no emitted-word change). Construct + bpref.
3. Bounds checks under A8's typed tables. Construct + bpref + the K6 measurement.
4. Step budget.

Steps 1 and 2 do not depend on A8 and can land first.

## 6. Why not WASM or eBPF -- the numbers that decided it

| | WASM | eBPF | this dialect |
|---|---|---|---|
| size of the safety artifact | wasm3, the smallest serious interpreter, is 64 KB of flash (its README) = **40 % of bebop.bin's 159,196 bytes** -- and it INTERPRETS | `kernel/bpf/verifier.c` is ~30,000 lines = **4.3x the whole of bebop.bp** (6909 lines) | ~300 lines in `bebop.bp` |
| speed | interpreters run roughly an order below JIT; K6's 18 ns/row would become ~100+, i.e. back at sqlite's 158 ms | native once JIT'd, but the program shape is restricted | native; +15 % on codegen-bound scans, ~0 at the DRAM ceiling |
| reachable here | a WASM interpreter in bebop is T94, OPEN and parked | kernel eBPF needs CAP_BPF; this proot is `untrusted_app_27` uid 10546 | already here |
| trust root | the sandbox's correctness becomes a second root of equal weight to `seed.S`'s 159 lines | same | unchanged |

Prior art points the same way: SingleStore's Code Engine and ScyllaDB both adopted Wasm **as the
UDF language inside a query language they kept**; XRP (OSDI'22) runs eBPF storage functions from
the NVMe hook to remove a kernel storage-stack cost this mmap store does not pay. **No shipped
system replaced its query language with BYOK.** The ones that adopted a sandbox adopted it for
UNTRUSTED code inside a TRUSTED engine; here the app and the engine are one process and one trust
domain.

**Reopening trigger:** a mesh use case that must run UNSIGNED peer kernels. D0's crypto and
post-quantum invariants say that use case must not exist -- so if it ever appears, the invariant
is what is being changed, and that is an operator decision, not a blueprint one.

## 7. Cheapest refutation

Hand-instrument the sgraph2 frontier BFS inner loop with the 2-word check and measure ns/edge-slot
on the promoted binary. If it exceeds 1.2x (45 -> 54 ns), the dialect is **off by default for
kernels and on for store code**, which `RESEARCH-NOPOINTERS-SQL §1.2(c)` already anticipated. That
is a ~20-line experiment and it should be run before step 3.
