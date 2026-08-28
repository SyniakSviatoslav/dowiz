# Bebop — THE Roadmap (single source of truth)

This file supersedes and replaces PLAN_B.md, MASTER-FINISH-PLAN.md,
ROADMAP_SELFHOST.md, docs/ZERO_C_CHARTER.md and SWEEP-B3-3.md — all removed.
BUGFIXES.md stays (bug journal), AGENTS.md stays (process laws), bench/
reports stay (evidence).

## Terminal goal

1. **Zero C**: every mechanism of the toolchain exists in Bebop; `native/src`
   (C) is deleted. The repo is `seed.bin` (frozen AArch64 loader, no libc) +
   `bebop.bin` (the compiler compiled by itself) + `*.bp` sources + `*.bin`
   artifacts. The C interpreter is retired, not ported.
2. **Math validation**: a Lean4-like proof/verification layer for the Bebop
   language (QTT kernel, theorem checker, machine-checked proofs).
3. **Full language**: the complete glyphic+VSA+QTT surface compiled by the
   self-hosted pipeline (lexer/parser/typecheck/codegen + aarch64/wasm/NEON/
   GPU-FPGA backends).

## Fixed execution order (binding)

1. **BLOCKERS** — close the known correctness gaps (see below).
2. **M4 → M7** — the Zero-C milestones.
3. **THEOREM WORKSTREAM** — Lean4-like math validation & verification.
4. **PART B** — full language front-end + backends, LAST.

Rationale: the compiler must be trustworthy before it replaces C (blockers
first); C can only leave after every C-provided capability has a verified .bp
twin (M4-M7); the proof layer is built on the stable core language; the full
language surface is the final expansion once everything beneath it is solid.

## Step 1 — Blockers (active)

**B1. Execution parity for call-in-loop and deep-expression programs.**
Word-level emitter parity is green (20/20 constructs byte-identical), but
execution diverges: `c19_multi` (helper call inside a while + call in the
accumulator) gives interp=115 vs native=31; `c20_deep` (call + array +
str_len inside a deep expression) stalls natively. Same class as the old
"helper call in while loop gets garbage arg" (commit db48711). Reproduce →
minimize → fix in `selfhost/expr_compile.bp` → re-run the full gate.

**Gate for Step 1 (all must be green):**
- `bebopc run selfhost/expr_compile.bp self_check 0` → `0`
- full self_bootstrap: `exec_words` count == compilewords count, word-for-word
- `bench/vs_rust/parity_driver.sh` → 0 fail / 0 crash (main-less = skip)
- `bench/vs_rust/construct_parity.sh` → 20/20 MATCH (word bytes)
- construct corpus VALUE parity (interp run == native exec) for all 20

## Step 2 — Zero-C milestones (M4 → M7)

| M | Goal | Acceptance |
|---|---|---|
| **M4** | CLI in .bp: `bebop.bin` subcommands `compile / run-via-exec / size / version`. Args passed from the seed loader's stack block into the arena. | `seed bebop.bin compile k1.bp` emits byte-identical words vs `compilewords`; `run` executes a kernel to the known result; `size`/`version` print correct values. |
| **M5** | std/ .bp twins for toolchain-adjacent algorithms: sort, rng, checksum, base64, sha256 (then whatever else the compiler/tooling consumes). | Each twin golden-vector tested against the C result BEFORE the C twin is removed. |
| **M6** | Parallelism: clone/futex via svc; pool.c reimplemented as .bp work-splitting over the shared arena. | compilemany and k7 queries run multi-core with identical outputs. |
| **M7** | Delete `native/src` (keep docs). | Repo = seed + .bp + .bin + docs; full gate green without the C compiler. |

Non-goals (unchanged from the charter): the interpreter is NOT ported;
wasm/GPU backends stay archived until Step 4; the C CLI's legacy modes die
with their implementations.

## Step 3 — Theorem workstream (Lean4-like math validation)

Built on the stable core after M7:

- **T1** Wire the QTT kernel (`qtt_kernel.bp`, `nat_peano.bp`, `proof.bp`,
  `universes.bp`) into an end-to-end proof checker: parse statement → kernel
  type-check → proof term validates.
- **T2** Port the C-side theorem machinery (3 machine-checked theorems) to
  .bp; grow the theorem corpus (nat induction, list, arithmetic identities).
- **T3** Refl-style automation (conv/norm/subst already exist as slices) +
  a tactic layer in Bebop; golden-vector proofs re-verified.
- **T4** Verification reports: fraction of the stdlib proven; all proofs
  machine-checked by the kernel with zero trust in the checker's own claims.

Acceptance: `make theorem-check` green; N machine-checked theorems with the
kernel as sole authority; docs report the proven coverage honestly.

## Step 4 — Part B: full language (LAST)

Only after M7 + the theorem workstream. Ports the C compiler surface to the
self-hosted pipeline, keeping the Zero-C state:

- **B1 Front-end**: full lexer.bp/parser.bp/typecheck.bp parity (modules, ADTs,
  dependent Pi types, generics, match, let-in, lambdas, arrays, field access;
  QTT quantities 0/1/ω, universes, conv/norm kernel).
- **B2 Backends**: aarch64.bp full native parity (entire construct set incl.
  closures/floats/syscalls/alloc); wasm.bp valid+executable modules; NEON
  vector-first backend; GPU/FPGA VIR slice + emit contract.
- **B3 Closure**: self-compile the FULL pipeline end-to-end (checksum-stable);
  port all native self-tests; fuzz to 1M inputs; honest bench vs Rust; docs.

DoD: the full glyphic+VSA+QTT surface compiles itself; backends verified;
fuzz/bench/docs green; every milestone committed+pushed.

## Current verified state (2026-08-28)

- **M1** seed loader: DONE — k1..k7 run through seed.bin, outputs identical
  to the interpreter; zero C at runtime.
- **M2** syscall I/O builtins: DONE (open/read/write/close/exit + clock;
  interp mirror + emitter words).
- **M3** self-bootstrap: DONE — full selfsource compiled by itself is
  byte-identical to the interpreter's output (67816/67816 words); selfcompile
  fingerprint 236065248692568 == word-sum; self_check = 0.
- **Hardening** (this session): verified sym table ((name,reg,srcpos)
  triples, byte-compare, capacity trap), unsigned-normalized 64-bit literal
  halves, capacity guards on fn/ctor tables, single-level guard discipline,
  fp/fpC subsystem deleted; parity driver skips main-less programs.
- **B2-8 corpus**: 20/20 constructs byte-identical (word level) via
  `bench/vs_rust/construct_parity.sh`; VALUE-level execution parity for
  c19/c20 still open → Step 1 blocker.
- Point B slices (dormant until Step 4): lexer/parser/expr_parser parity +
  typecheck green; aarch64 4/4, aarch64_data 4/4, wasm MVP + memory ops,
  vir.bp NEON ADD/SUB 2D + MUL 4S, gpu_fpga.bp WGSL+Verilog skeleton,
  wasm-check 22/22 in V8, fuzz ~630k/1M inputs.

## Design laws (inviolable, every swarm)

Branchless Σ k·(k==N)·expr; no_std; O(n); atomic/lock-free; vector-first
(NEON, scalar fallback); hypervectors where possible; living memory.
Per-fn bind budget ≤ 128 (overflow = trap, never silent); literals ≥ 2^63
must use the normalized-half emit path; nested ifs inside let-statements and
plain-var assignment inside `let _ =` are banned (index cells + single-level
guards only); capacity asserts on every fixed table.

## Coordination

Every milestone: commit + push to origin/main. Full verification gate after
each batch: self_check, self_bootstrap parity, parity driver, construct
corpus, fuzz, bench — evidence in commit messages.
