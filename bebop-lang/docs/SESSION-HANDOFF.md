# SESSION HANDOFF — 2026-09-06 (session 9; resume in ONE read)

Status: 2026-09-06 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint chain = three generations (gen3 == gen4). Landed this session: T72
  sys_setaffinity, T105 clz + Newton isqrt + sdiv fp_div, T108 .becache (exact bytes,
  warm 113 ms vs cold 346 ms per gate; floor 106 ms), T109/T109b crc32 (bytes) + crc32x
  (raw words), T110 sys_msync, T126 (x27/x28 were saved/restored by >= 9-symbol fns:
  every arena allocation they made was rolled back — root cause of the `use` crashes),
  T127 (thread exit was exit_group), T129 (unwritable output -> exit 90), T47b nested
  `use`, T130 (unresolved call -> runtime exit 87), T118b (entry stub: sigaltstack +
  SIGSEGV/SIGBUS handler -> exit 82; the code image is PROT_READ|EXEC so the stub's
  structs live in the arena). Pending at close: see "In flight".
- Battery: std_golden 99/99 (92 + slayout, sround x2, scompact, scrash, sevolve, sconc),
  constructs 47 (+ neg c48 stack overflow, c52 undefined call), run_all 99, pool 5/5,
  invariants GREEN incl. the T48b negative sample; mutation tool (T123) now mutates 3
  operator sites of the EXPANDED program.
- Store (T111, selfhost/prelude/store.bp): superblock pair, append arena, object-relative
  refs, crc32x integrity, commit = superblock toggle, Cheney compaction to .tmp + rename,
  migration table, reader snapshots, msync'd commits. Gates G1-G7 green with numbers;
  G8 stages 1+2 + frontier BFS measured; hub-skew variant wired.
- Numbers (bench/vs_rust/RESULT-sbench.md, RESULT-sgraph.md, ROADMAP Measured): store vs
  sqlite 3.46.1 C API: insert 17x, PK lookup 630 ns vs 7.8 us, window scan 1.8 vs 63.6 us,
  update 6.4x, reopen 6x, size 2.5x LOSS, compaction 0.7x of VACUUM, durable commit in the
  fsync-per-commit class; graph BFS 187 ns/edge vs sqlite 10.8 us (57x); frontier BFS 45
  vs queue 192 ns/slot; 100/100 SIGKILL trials consistent.

## In flight / next
1. T130 + T118b chain (scratch fx/v2..v4) -> promote, battery (c48/c52 neg), commit.
2. T80 `use "cas://sha256:<hex>"` (scratch t80.py, .bcas/, hold/c50_cas + c51_casbad
   COMPILEFAIL:88): bebop.bp gains `use "selfhost/prelude/sha256.bp"`; chain + battery.
3. Full `bench/vs_rust/sgraph2.sh` run (frontier + skew rows) on a quiet core 4.
4. T123: fix the folds of the gates the rewritten sweep still calls insensitive.
5. T104b wider peephole (x*c1*c2, mul-by-const -> shift, LICM of movz); honest.sh R=11 on
   a quiet box (self-compile today 293 s vs the 108.7 s row of 2026-09-04 — not reproducible).
6. Profile the 45-90 s CSR build in the store (50M library calls) — sgraph phase b.

## Pitfalls that cost hours this session (also in the memory file)
- Derive every instruction word with python int(hex,16) and objdump the emitted .bin: a
  hand-typed clz became `rev`; the first entry stub wrote into the RX code image.
- A clone-spanning fn must keep <= 8 live symbols; workers exit with
  sys_exit_thread_guard (svc 93 since T127).
- Never prefix a runner with `S=...` in this shell ($S is the scratchpad variable); the
  runners use NSRC. `&&`-lists followed by `&` background the whole list.
- Nested `if` as a call argument is the AGENTS.md #6 trap: bind it to a let first.
- st_open ftruncates to the preallocated size: report logical size = arena_used*8.
- The store's crc is crc32x over raw words; crc32(cells,n) is byte-per-cell.
