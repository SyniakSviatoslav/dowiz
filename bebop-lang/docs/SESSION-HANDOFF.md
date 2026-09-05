# SESSION HANDOFF — 2026-09-06 (session 10; resume in ONE read)

Status: 2026-09-06 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint chain = three generations (gen3 == gen4). Landed this session: T130
  (unresolved call -> exit 87), T118b (39-word entry stub: sigaltstack + SIGSEGV/SIGBUS -> exit
  82; every program +39 words, budgeted), T48b (`ref T` census in tools/typecheck.py + the
  negative sample), check_abi (iv) accepts the stub, T80 cas:// imports (`use
  "cas://sha256:<hex>"` -> .bcas/<hex>.bp verified by digest, exit 88; `bebop.bin cas add
  <file>`; bebop.bp's first line is `use "selfhost/prelude/sha256.bp"`; fixpoint 0b0f07f0).
- Battery: std_golden 99/99, constructs 51 (+ c50_cas, neg c51_casbad/c48/c52), run_all 99,
  parity 12/12+1, pool 5/5, invariants GREEN; T123 DONE: every gate mutation-sensitive
  (5 folds extended: base64 hex lsm dispatcher mvcc; goldens from oracle == bebop == bpref).
- ONE COMMAND now: `tools/chain.sh <src.bp> <out-dir> [--codegen]` = gen2, then gen3->gen4
  in parallel with `tools/battery.sh` (all gate scripts on the 3 cores, one summary block).
  invariants.sh still runs after promotion (hardcodes ./bebop.bin; `--freeze` for census).
- Language rule made explicit (DIVERGE-20056): an array literal bound INSIDE a while body is
  released at the back-edge / loop exit (T43 frame heap) — LANGUAGE.md memory model; bpref
  raises "use after loop release"; gen.py never emits the shape. Not a miscompile.
- Compiler speed: str_len was a NUL scan called ~10x per token on the whole source; the length
  now travels in pos[1] (slen). Self-compile 187 s -> 15 s; a std gate 20 s -> 0.6 s. The
  third compile pass (emit_offsets) is gone. tools/pcprof.sh's gdb samples are NOT reliable
  under proot (they clustered at one call site) — use differential timing (scratch mb/).
- Fuzz: gen.py named loop vars by len(loopvars) and big loops were not in it -> reused
  names + assignable big-loop vars = non-terminating programs in BOTH engines; those were
  the 49 "BPREF-TIMEOUT"s. Fixed: 20 seeds in 12.8 s, 0 timeouts (1.56/s vs 0.05/s).
- T104b step 1: mul by 2^k -> lsl, `xB + xA*2^k` -> one shifted-register add (constants
  as+objdump-verified); the chain ran through tools/chain.sh --codegen.
- Store: unchanged this session (G1-G8 numbers in ROADMAP Measured / RESULT-*.md).

## In flight / next
1. Full `bench/vs_rust/sgraph2.sh` run (frontier + hub-skew rows) on a quiet core 4.
2. T104b wider peephole (x*c1*c2, mul-by-const -> shift, LICM of movz) via tools/chain.sh
   --codegen; honest.sh R=11 on a quiet box.
3. Profile the 45-90 s CSR build in the store (50M library calls) — sgraph phase b.
4. Fuzz at scale (TG-DONE 8): 10^5 programs now ~6 h at J=3 (1.5/s) — run it in batches
   of 5000 (`bench/fuzz/fuzz.sh 5000 <start>`), journal each batch's summary line.
5. T124 fold specs for the 8 gates named in TG-DONE row 3; T47c (prelude-headered gates to
   `use`) is done for all 99 already — verify and close the note in HISTORY.

## Pitfalls that cost hours this session (also in the memory file)
- An 8-deep parenthesised chain of calls `((((f(a)*16+f(b))*16+...` hits exit 95 in the
  compiler — write a loop. A `zeros` inside a while body is L8 (the T80 draft had one).
- `pgrep -f <pattern>` matches the shell that runs it; never pipe it to kill.
- Timing-flag gates (lcjit) miss under three parallel batteries; rerun standalone first.
- AGENTS.md L18: never a wait-only turn while a shell runs — do the next independent item.
- Derive every instruction word with python int(hex,16) and objdump the emitted .bin: a
  hand-typed clz became `rev`; the first entry stub wrote into the RX code image.
- A clone-spanning fn must keep <= 8 live symbols; workers exit with
  sys_exit_thread_guard (svc 93 since T127).
- Never prefix a runner with `S=...` in this shell ($S is the scratchpad variable); the
  runners use NSRC. `&&`-lists followed by `&` background the whole list.
- Nested `if` as a call argument is the AGENTS.md #6 trap: bind it to a let first.
- st_open ftruncates to the preallocated size: report logical size = arena_used*8.
- The store's crc is crc32x over raw words; crc32(cells,n) is byte-per-cell.
