# TELEMETRY AS A LANGUAGE FEATURE — blueprint

Research pass, 2026-09-12. Static analysis only; nothing was compiled, nothing was run,
no git command was issued, and no file inside `/root/dowiz/bebop-lang` was written.
Every number below carries its `file:line` or shows its arithmetic. Numbers that are
neither are labelled **HYPOTHESIS**.

---

## 0. CITATION ERRATA FOUND WHILE READING THE MANDATED CONTEXT

The brief asked me to say so when a cited file does not exist. Four of the citations in
my own task were off, and four more stale facts turned up in the tree. All eight are
listed here because this project's most expensive class is a confident number that moved.

| # | Claim | Ground truth |
|---|---|---|
| E1 | "read `CLAUDE.md` (the three laws)" | **There is no `CLAUDE.md` in `bebop-lang/`.** The three laws live in the PARENT repo's file, `/root/dowiz/CLAUDE.md`, in the section "bebop-lang: three laws that override convenience". `/root/dowiz/.claude/CLAUDE.md` is a different document (agent operating discipline). |
| E2 | "`selfhost/bebop.bp`" | **Does not exist.** The compiler is `/root/dowiz/bebop-lang/bebop.bp`. |
| E3 | "a single 7700-line bebop.bp" | `wc -l bebop.bp` = **7708**. (`seed/seed.S` = **159**, as stated — that one is right.) |
| E4 | "116 gates with committed oracles" | Confirmed: `grep -c '^\s*gate ' bench/vs_rust/std_golden.sh` = **116**. The last two journal entries quote the battery as "113 pass/3 fail" (`docs/exp.journal:1789233978`, `:1789233153`), so 113+3 = 116 and the two agree. |
| E5 | fn-table cap | `docs/TRAPS.md` row 89 says the fn/enum-ctor table cap is **512**. `bebop.bp:4410` and `bebop.bp:6198` both test `>= 768`; `bebop.bp:5893` tests `< 768`; and `diag_exit`'s own message bytes at `bebop.bp:100` spell `"too many fns (cap 768)"` (`55,54,56` = `7,6,8`). **The row is stale again** — it already went 256 → 512 once via the F1 census. |
| E6 | free exit codes | `docs/TRAPS.md` (F1, 2026-09-09) says "**65-79, 84, 85, 86, 93** — nineteen free codes below 99". Later the SAME FILE assigns **85** to `st_seal` and **86** to `st_open` (2026-09-12). Free today is `65..79` (15) + `84` + `93` = **17**, not 19. |
| E7 | the arena constant in the entry stub | `bebop.bp:6908` comment says the stub builds `mov x11,#0x10000000` = 268 435 456. The words it actually emits are `bebop.bp:6929` `st[2] = 3531735051` = `0xd282000b` = `movz x11,#4096` and `bebop.bp:6930` `st[3] = 4070703115` = `0xf2a2000b` = `movk x11,#4096,lsl#16`, i.e. **268 439 552** (`0x10001000`). One page off. This matters: see §3. |
| E8 | an orphan ratchet | `tools/arch_ratchet.txt:43` carries `max_unbounded_waits = 0`. **No check in `tools/arch_check.py` reads it** (`grep unbounded_waits tools/arch_check.py` → no hit). A ratchet line with no check behind it is exactly the failure mode `law_manifest.txt` exists to prevent, one layer down. §4 adopts it. |

One more, not an erratum but a live hazard I could not find a guard for:

> **E9.** The IO scratch zone is at `x28 - 8192` (`bebop.bp:1048`, `:7514`) and `clock_ms`'s
> timespec at `x28 - 8208` (`bebop.bp:1201`). `emit_zeros`' capacity trap compares
> `cmp x27,x28` (`bebop.bp:6524-6527`, word at `bebop.bp:6527` = `3944481663` = `0xEB1C037F` = `subs xzr,x27,x28`)
> and traps only when the cursor passes **x28**. Nothing reserves the top 8 KiB. A large
> enough `zeros()` therefore walks into the scratch zone and the timespec **silently** —
> no trap 80, no diagnostic, the next `sys_open`/`sys_write`/`clock_ms` reads clobbered
> bytes. I did not find an incident for this, so it is a latent hazard, not a known defect.
> Step 1 below closes it as a side effect.

---

## 1. THE HARD CONSTRAINTS, STATED BACK

I accept all seven and each one shaped the design:

1. **Self-hosting.** Anything new must be compilable by the CURRENT `bebop.bin` and must
   survive `gen3 == gen4`. Consequence, and it is the sharpest one: **the compiler is the
   LAST thing that can be instrumented, not the first.** A `tel(...)` call written into
   `bebop.bp` cannot compile until a binary that knows `tel` has already been promoted.
   So step 2 lands the emitter, and only a later generation may use it.
2. **Zero dependencies, syscalls only.** Rules out every conventional sink. The one channel
   that already exists and is already loud is fd 2 (`bebop.bp:106` `sys_write(2, buf, ...)`
   in `diag_exit`; the entry stub's SIGTRAP handler writes `trap NN:` the same way).
3. **No raw pointers; `[i64]` values are cell indices; objects carry a +2 header skip.**
   The store's object header is `h0` at `off`, crc at `off+1`, payload from `off+2`
   (`selfhost/prelude/store.bp:302-326`), and only `st_alloc`/`st_seal` may write cells 0/1
   (`tools/arch_check.py:165-198`). A telemetry record therefore cannot be a naked write
   into the store arena without becoming a sealed object — see §3 for why it must not be.
4. **≤ 8 KEPT symbols across a `sys_clone`, failing SILENTLY.** `docs/TRAPS.md` row 103:
   8 → both children correct, 9 → one lost, 11 → both lost; constants do NOT count. This is
   the single constraint that decides the whole design, because **a fixed immediate costs
   zero kept symbols and a passed-in base costs one of eight.**
5. **Android/proot, phantom-process cap, SLOTS=1.** No helper process, no daemon, no
   collector. `tools/platform.txt:6` `tracer_pid=18519` → every syscall is ptrace-stopped.
6. **116 gates with committed oracles.** Anything that changes a gate's **stdout** breaks
   them. §3 shows stdout is not the channel we need.
7. `tools/platform.txt:7` `fsync_mode=nobarrier` → durability is not real on this box, so
   "durable telemetry" is not a property this box can demonstrate even if we built it.

---

## 2. THE DECISION

### 2.1 Argued against the defect table first

The brief requires the design to be argued against the whole-record table
(`AGENTS.md`, "DEFECT TAXONOMY, PART 2"). Here it is, honestly, before the design.

Sum of the ten journal-hit counts: 69+39+31+25+24+24+21+11+10+10 = **264**.

| Class | Hits | Would in-language telemetry have made an instance LOUD at the moment it happened? |
|---|---|---|
| codegen / miscompile | 69 | **NO.** A miscompiled expression returns a wrong number and raises no event. `sys_arena_end() + sys_arena_base()` evaluating to `base + base` produces a plausible integer; there is nothing for a record to say. The defence is and stays differential testing (`tools/bpref.py`, `bench/parity_constructs`, the fuzzer). Worse: `tel` is itself new emitted words, so it JOINS this class until it has a `bench/parity_constructs` entry and a `bpref` mirror. |
| layout / off-by-N | 39 | **PARTIAL.** The PartTab header off-by-two was caught by trap 85 several functions away. A record at every object seal naming `(off, len, digest)` would have named the writer at the moment. But the record has to be placed by `store.bp`, i.e. it is a library discipline; the language only supplies the primitive. Counts as partial, not a win. |
| concurrency / race | 31 | **YES — the strongest case, and the reason to build this at all.** "past 8 kept symbols the children are lost with NO trap and NO diagnostic; `sys_clone` returns valid TIDs and nothing ever writes" (`AGENTS.md`, LAW: FAILURES ARE LOUD). A child that writes one record into a shared ring at a FIXED immediate index turns that into `spawned=2 arrived=0`. The immediate is why it works: it costs none of the eight slots. |
| memory / bounds | 25 | **PARTIAL.** Trap 80 already fires. Telemetry adds the last N allocation sizes and tags before the trap, so "arena exhausted" becomes "arena exhausted after k allocations of size s from site t". Improves the diagnosis, does not create it. |
| silent failure | 24 | **YES.** `AGENTS.md` says the fix for every one of these was to ADD A TRAP (T130, 80, 87). Telemetry is the generalisation of that fix to the cases where you must not die: a record instead of a `brk`. "a call to an undefined function compiled with rc=0" is a compile-time instance and stays out of reach, but the runtime half of the class is exactly this. |
| stale constant or doc | 24 | **NO.** Source-text class. `arch_check` **magic-constant** is the safeguard. A runtime record cannot read a comment. Errata E5-E7 above are fresh instances and telemetry would not have produced one of them. |
| stale artifact | 21 | **MARGINAL.** Traps 86/87 already name a mismatched store. A record at `st_open` naming `(path-digest, version, size, gen)` makes the artifact identity visible without a trap, which helps when the mismatch does NOT trip a trap. Small. |
| no-op that reports success | 11 | **YES.** `st_commit_2pc` "committed nothing at all and reported success". A record counting cells actually written is 0 and visible. This is the same shape as LOUD rule 5 (a guard that annihilates a value must say which factor was false). |
| hang / deadlock | 10 | **YES.** LOUD rule 4 already says a wait must be bounded and must name what it waited for. A ring read on expiry is the mechanism that lets it name it. |
| fitted or unverified claim | 10 | **NO.** Process class. A number nobody ran is not a runtime event. |

Arithmetic, so the claim is falsifiable:

- decisive (concurrency + silent + no-op + hang) = 31+24+11+10 = **76 / 264 = 28.8 %**
- partial (layout + memory + stale-artifact)     = 39+25+21 = **85 / 264 = 32.2 %**
- no help (codegen + stale-constant + fitted)    = 69+24+10 = **103 / 264 = 39.0 %**

**The largest single class in the record gets nothing from this.** That has to be the
headline, because a blueprint that implies otherwise is the over-claim the brief warns
about. What in-language telemetry buys is the SILENT third of the record — and that third
is the one whose instances cost days rather than hours, because a silent failure has no
starting point for the ladder at all.

### 2.2 The axis, costed

| Option | bebop.bp lines | Chain risk | Gate blast radius | Verdict |
|---|---|---|---|---|
| **(a) library only, a prelude `.bp`** | **0** | **none** — no compiler change, no fixpoint | **0** (unless a gate calls it) | **REFUTED on a mechanism, not a preference.** See the proof below. |
| **(b) compiler-emitted at construct boundaries** (every fn entry/exit, every `while` back-edge) | ~150-300 | **high** — every emitted word stream moves, so `tools/census.py --check` branch census, `check_abi` register zones, and `bench/parity_constructs/frozen/*.bin` all move together | every gate's WORDS move; the frozen .bins and `census.txt` must be re-frozen in the same commit | **REFUSED on the A3 precedent.** `ROADMAP.md:90` refused A3 at **+2858 words, +7.27 % of the self-compiler, permanently** for a feature that fired on zero of its own 144 loops. Auto-instrumenting every fn entry is strictly more words than that, permanently, in every program including the compiler, most of it on paths nobody is debugging. |
| **(c) genuine syntax, own keyword + typing rules** | ~80-150 | medium — one new parse path + fixpoint | 0 on stdout, **but** a new reserved word (the T122 table at `bebop.bp:5753-5756` carries 45 names) makes any existing program using that identifier fail with exit 99, and needs a `bench/parity_constructs/neg/` fixture | **REJECTED as over-build.** A keyword buys typing rules the language has no use for: everything is `i64` or `[i64]`, so `trace <expr>` and `tel(a,b,c)` have identical type content. The keyword's only real gain over a builtin is that a keyword cannot be shadowed — and `docs/TRAPS.md` already documents that the fix for shadowable builtins is "adding seven hashes", i.e. put the builtin in the reserved table and it is unshadowable too. |
| **(d) hybrid: ONE builtin + ONE stub word + a prelude library** | **~40** (one `emit_tel` + one dispatch line + one reserved hash + one stub word) | medium — two fixpoint runs, one per compiler-touching step | **0 on stdout by construction** (§3); words move only where `tel(` is written | **PICKED.** |

**Why (a) is refuted on a mechanism.** A library needs the ring at an address that is the
same in the parent and in the child. There are exactly two builtins that can yield an
address-like value, and **both are re-homed by `emit_sys_clone`**:

- `sys_arena_base()` returns `(x27 - x17)/8` (`bebop.bp:1247`), and the clone child sets
  `x27 = sp + 4 MiB` (`bebop.bp:1307`, word `2437940219` = `add x27,sp,#1024,lsl#12`).
- `sys_arena_end()` returns `(x28 - x17)/8` (`bebop.bp:1262`), and the child sets
  `x28 = sp + 12 MiB` (`bebop.bp:1312`, word `2440037372` = `add x28,sp,#3072,lsl#12`).

So a library can only get a stable ring base by having the PARENT compute it and the child
KEEP it — which spends **one of the eight kept slots at the exact call site where the
telemetry matters most**, and over the line the children are lost silently, which is the
bug telemetry is being built to see. A library also gives a *convention*, not an
*invariant*: each program picks its own ring offset, there is no single format, and there
is nothing for `arch_check` to police — which violates the operator's standing rule that
every prose rule needs a concrete mechanical safeguard.

**What (d) is, concretely.**

1. **One new builtin, `tel(tag, a, b)`** — dispatched through the existing ladder
   (`bebop.bp:1845-1873`), registered in the T122 reserved table so it cannot be shadowed.
   It emits an inline sequence of roughly **15-18 words** (HYPOTHESIS; by analogy with
   `emit_sys_write`'s 16 `em()` calls at `bebop.bp:1136-1153` and `emit_sys_atomic_add` at
   `bebop.bp:1414-1438`) that: materialises the ring cursor's **immediate cell index**
   (movz+movk, 2 words — this is the whole trick, an immediate is rematerialised and
   therefore costs **zero** kept symbols across a clone); converts to an address with
   `add x9,x17,x9,lsl #3`; does one `sys_atomic_add`-style `ldaddal` of 4 on the cursor;
   masks the returned slot; and stores 4 cells. **No `svc`. No allocation. No branch.**
2. **One word changed in the entry stub** to reserve the ring's memory (§3.2). Not one word
   ADDED — the stub is 172 words and that length "is a literal in five places"
   (`bebop.bp:6913-6915`); its own comment names `st[7]`, `mov x10,x6`, as "kept at word 7
   purely as the filler". That filler is the slot.
3. **Everything else in `selfhost/prelude/tel.bp`** (new, ≤ 250 lines, well inside the
   800-line cap): `tel_reset`, `tel_count`, `tel_dropped`, `tel_drain(fd)`. Drains are off
   the hot path and have no reason to be in the compiler.

The decisive property of (d), and the reason it beats (b) on this project's own terms:
**a program that does not call `tel` pays exactly zero words.** `bebop.bin` does not grow.
No frozen `.bin` moves. No census line moves. The overhead is placed by hand, at named
sites, and it is visible in the source as a call — which is also what makes it
`grep`-able and therefore `arch_check`-able (§4).

---

## 3. THE OVERHEAD BUDGET, AND THE EMISSION SINK

### 3.1 The budget

**The one hard rule this box imposes: no syscall on the hot path.** That is measured, twice,
from one journal entry, and the two derivations disagree — both are quoted, per law 2.

> `docs/exp.journal:1789232380`: "every commit pays two syscalls to wake a waiter that
> cannot exist, and **under proot each is about 27 us** against an 85.6 us transaction.
> Removed both and re-measured, best of three per point … **85.6 -> 47.3 us per commit**".

- Stated per-syscall cost: **27 µs**.
- Cost derived from the delta: (85.6 − 47.3) / 2 = **19.15 µs**.
- These differ by 41 %. Take the range: **19.2–27 µs per syscall under ptrace**, MEASURED.

Consequences, by arithmetic:

- A telemetry record that costs one `sys_write` costs 19.2–27 µs — i.e. **41–57 % of a
  47.3 µs store commit** (`docs/exp.journal:1789232380` gives 47.3 / 53.3 / 28.6 / 42.6 µs
  per commit for the post-fix P=1/P=1/P=2/P=3 points). Disqualifying.
- `clock_ms()` is a raw `svc` (`bebop.bp:1201-1220`, `movz x8,#113` + `svc`), **not** a vDSO
  call. So a timestamp costs the same 19.2–27 µs. **`clock_ms()` must not appear on the
  telemetry hot path**, which is why the step-1 record format carries the atomic's returned
  sequence number and NOT a wall-clock time.
- The `tel` sequence has zero `svc` words, so ptrace never stops it. Its cost is
  **12-15 retired instructions, of which exactly one (`ldaddal`) can contend**
  (`tools/platform.txt:2` `lse_atomics=8`, so LSE is present on all 8 cores).
  At ~2 GHz and uncontended, ~10-30 ns. **This last figure is a HYPOTHESIS.** It is not
  measured, I did not run anything, and the brief is right that a design whose cost is
  unknown is not acceptable — so step 4 exists solely to replace it, and the design is
  **not** to be called "free on the hot path" until it does.

Against the numbers we do have: 10-30 ns against a 47 300 ns commit is **0.02-0.06 %**,
three orders of magnitude below noise. Against one iteration of a tight arithmetic loop it
could be 100 % or more. So the honest statement is:

> **Telemetry at transaction, spawn, commit and failure boundaries is free to three
> significant figures. Telemetry inside an arithmetic inner loop has an unknown cost and
> must not be placed there until step 4 has measured it.**

**How to measure it on a box where ptrace inflates everything.** The inflation is on
syscalls, not on user-mode instructions — so the measurement is designed to divide the
ptrace tax to nothing rather than to subtract it:

- Two variants of one program, differing in ONE thing (defect class 8: vary the KIND, not
  just the count): variant A runs `N = 10^7` iterations of a loop whose body evaluates three
  `i64` expressions and calls `tel(t,a,b)`; variant B runs the identical loop with the three
  expressions evaluated and consumed by an `if`, and no `tel`.
- Exactly **two** `clock_ms()` calls per variant, outside the loop. Their ptrace tax is
  2 × 27 µs = 54 µs spread over 10^7 records = **5.4 ps per record** — five orders of
  magnitude under the figure being measured. The ptrace problem is arithmetically
  eliminated, not argued away.
- `(A_ms − B_ms) × 10^6 / N` = nanoseconds per record. Best of three, and the whole thing
  run **twice** (`AGENTS.md` defect class 7: every threaded measurement waits and runs at
  least twice). Then repeat with 2 and 3 clone children to price the contended `ldaddal`,
  which is the number that actually matters and the one a single-threaded probe cannot see.
- **State the kill threshold before running** (law L10): the design claims "free at
  boundaries". If a record costs > 100 ns uncontended, the claim narrows to "free at store
  and syscall boundaries only" and the loop-level use is withdrawn.
- L8 is satisfied: `tel` allocates nothing, so there is no `zeros()` in the loop body.

### 3.2 The sink

**Primary sink: a fixed ring in the arena, at an immediate x17-relative cell index.**

The arithmetic, derived and not remembered:

- `seed/seed.S:53-55`: the arena mmap length is `mov x1,#4096` then `movk x1,#4096,lsl #16`
  = 4096 + 4096·65536 = **268 439 552 bytes**; `x27 = x0` (base), `x28 = x0 + x1`.
- `bebop.bp:6929-6931`: the stub builds `x11 = 4096 | (4096<<16)` = 268 439 552 and then
  `st[4] = 3406496652` = `0xcb0b038c` = `sub x12,x28,x11`; `bebop.bp:6933`
  `st[6] = 2852914161` = `0xaa0c03f1` = `mov x17,x12`.
- Therefore **x17 == the mmap base exactly**, and the x17-relative window is
  268 439 552 / 8 = **33 554 944 cells**, with cell 33 554 944 == x28.
  (This corrects erratum E7: the comment's `0x10000000` would have made x17 = base + 4096.)
- `bebop.bp:6905-6906`: the stub advances x27 by 65 600 for the sigaltstack, so the ring
  cannot live at the bottom — the bottom moves. The top does not.
- `bebop.bp:1275-1320`: `sys_clone` re-homes x27/x28/x15 in the child but
  **never touches x17**, and `bebop.bp:6903` makes the stub idempotent on `cbnz x17`
  precisely so a re-entry does not move it. **x17 is the one register that is the same in
  the parent and in every CLONE_VM child.**
- The clone flags in the corpus are `68864` (`bench/vs_rust/std_tests/smw.bp:162`,
  `sconc.bp:99`, `b6core.bp:56`, `bench/fuzz/repros/D4-clone-*.bp`) =
  0x10000 CLONE_THREAD + 0x800 CLONE_SIGHAND + 0x400 CLONE_FILES + **0x100 CLONE_VM**.
  Arithmetic: 65536 + 2048 + 1024 + 256 = 68 864. **CLONE_VM is set, so the arena is shared
  and a ring write by a child is visible to the parent.** This is what makes the whole idea
  work, and it is the reason the sink is memory rather than a file.

Layout (all written in the source as the arithmetic, per law 3 — never as `33546752`):

```
reserved slab R  = 65536 bytes = 8192 cells          ("64 * 1024 / 8")
window top       = 268439552 / 8            = 33554944 cells
slab base cell   = 268439552 / 8 - 65536 / 8 = 33546752
  [base + 0 .. base + 8)      header: magic, version, cursor, dropped,
                                      rec_cells(=4), capacity(=1024), 2 reserved
  [base + 8 .. base + 4104)   1024 record slots x 4 cells = [seq, tag, a, b]
  [base + 4104 .. base + 8192) slack, reserved for a later format
```

`cursor` is bumped with the existing `sys_atomic_add` primitive (`bebop.bp:1414`, `ldaddal`);
the slot is `(seq & 1023) * 4`. Lock-free, wait-free, no futex, no syscall, and safe across
the 8 cores (`tools/platform.txt:8` `core_count=8`).

**How the slab is protected — and why this costs zero words per allocation site.**
`emit_zeros` traps when the bump cursor passes **x28** (`cmp x27,x28`, `bebop.bp:6527` = `3944481663`).
So lowering x28 by the slab size makes trap 80 protect the ring automatically, at every
allocation site in every program, for **zero additional emitted words**. The change is one
word in the entry stub, replacing the declared filler at `bebop.bp:6934` (`st[7] = 2852520938`
= `0xaa0603ea` = `mov x10,x6`) with `sub x28,x28,#16,lsl #12`. The stub length stays **172**,
so none of its five external literals and none of its internal `adr`/`b` offsets move.
The word itself must be produced by `as` + `objdump` (law L1) and never typed from memory —
the encoding is deliberately not quoted here for that reason.

Side effect, free: this also closes hazard **E9**. The IO scratch zone at `x28-8192` and the
`clock_ms` timespec at `x28-8208` become unreachable by `zeros()` for the first time.

**Drain sink: fd 2, and the blast radius is zero. Verified, not assumed.**

- `bench/vs_rust/std_golden.sh:50` — `run()` is `timeout "$t" ./seed/build/seed "$bin" "$@" > "$BEBOP_TMP/memo.$k"`. **Only stdout is redirected.** The run's stderr is never captured and never compared.
- `bench/vs_rust/std_golden.sh:110` and every gate line after it — `r=$(… compile … >/dev/null 2>&1 && run 30 …bin | tail -1)`. The `2>&1` is on the **compile**, not on the run; `$r` is stdout only.
- `bench/vs_rust/std_golden.sh:81-104` — `gate()` compares `$result` against `$golden`.
- `seed/seed.S:96-125` — the seed prints `main`'s value to **fd 1** and exits 0. That value,
  via `| tail -1`, is what every one of the 116 goldens is.
- `tools/arch_check.py:143-163` (CHECK 6, loud-failure) — `2>/dev/null` on a run is a FAIL.
  So the stderr channel is one the project has already made unsilenceable by law.

Conclusion: **a telemetry drain on fd 2 cannot change a single one of the 116 goldens**,
and it lands on the channel the traps already use. Anything on fd 1 changes goldens and is
forbidden.

**Why the records must NOT go into the store.** Four reasons, each with a line:

1. `st_verify_range` (`selfhost/prelude/store.bp:132`, called by `st_open` via
   `st_reopen_verify:173`) crc-walks **every object** from cell 1024 to the cursor on every
   open. N telemetry objects = N more crc walks, on every open, for ever.
2. `st_copy_obj`/`st_forward` (`store.bp:1014`, `:1032`) copy every reachable object during
   `st_compact` (`store.bp:1065`). Rooted telemetry is copied for ever; unrooted telemetry
   is silently dropped at the next compaction — so it is neither durable nor bounded.
3. A telemetry root needs a superblock cell. `st_sb_write_m` (`store.bp:73-91`) writes 16
   cells; only cell 9 ("reserved (B1 `anc`)") and 12..14 are free. Changing the superblock
   layout is **precisely** the B5-step-1 defect (`AGENTS.md` taxonomy row 10): "five gates,
   two oracles and one harness model, found one at a time over a day".
4. Every allocation moves `used` and `live`, and four gate goldens are literally
   `live == <derivation>` comparisons — `tools/arch_check.py:199-205` names schain, sevolve
   and scompact with the bare values `4028`, `10060`, `5008` they used to hide.
   **Telemetry in the store changes gate goldens. That alone disqualifies it.**

Also, on this box: `tools/platform.txt:7` `fsync_mode=nobarrier` — even the store is not
durable here, so "put it in the store for durability" buys a property this box cannot
demonstrate.

**When the sink is full.** It is a ring: it overwrites, and `dropped = max(0, seq - 1024)`
is a derived field the drain always prints — *including when it is zero*, per LOUD rule 3
("never report an empty result as empty"). The drain's first line is always
`tel: n=<count> dropped=<d> cap=1024`, so "no records" and "the ring is absent" can never
print identically, which is the exact failure `got=` empty had three times in one day.

**When the sink is absent.** It cannot be, in the CLONE_VM case: the ring is at an immediate
index inside a mapping `seed/seed.S` always creates. The one genuine absence case is named
and must be printed: a `sys_clone(17, …)` **fork** child (`bench/parity_constructs/c84_run.bp:62`,
`bench/vs_rust/std_tests/sgraph2.bp:1076`, `gb_bfs_gen_addr_gate.bp:124`,
`selfhost/std/gb_run.bp`) has flags 17 = SIGCHLD only — **no CLONE_VM** — so its arena is
copy-on-write and its records die with it, invisibly. The drain must say so in its header
line rather than let a silent zero mean two different things.

---

## 4. MECHANICAL ENFORCEMENT

A new invariant in `tools/arch_check.py`, in the house style (numbered **CHECK 22** —
CHECK 19 was never written and **CHECK 21 is now taken by `prereq-guard`/L24**, which
landed in `tools/law_manifest.txt` and `tools/arch_ratchet.txt` while this pass was reading).

```
# --- CHECK 22: telemetry at the silent-loss sites -----------------------------------
# The two classes in-language telemetry decisively fixes are concurrency (31 journal
# entries) and hang/deadlock (10), and they are the same shape: a child that never
# arrives, with no trap and no diagnostic. A spawn with no arrival record and a spin
# on a shared cell with no record are the two places where that shape is manufactured.
# This check also adopts `max_unbounded_waits`, which has sat in tools/arch_ratchet.txt
# since 2026-09-12 with NO check reading it -- a ratchet with nothing behind it is the
# law_manifest failure mode one layer down.
def check_telemetry_at_loss_sites(r):
```

Three rules, each stated precisely enough to implement:

**(22a) SPAWN RECORD.** For every `.bp` in `bp_sources()`: a function containing a real
spawn — `re.search(r"(?<!emit_)\bsys_clone\s*\(", line.split("//")[0])`, the identical
exclusion CHECK 20 already uses at `tools/arch_check.py:527` so the compiler's own
dispatch line is not a spawn — must contain at least one `\btel\s*\(` on a line **after**
the spawn line and before the function's closing line. Otherwise:
`"<file>:<n> spawn with no arrival record -- over 8 kept symbols the children are lost SILENTLY"`.
Ratchet `max_unrecorded_spawns`, **seeded at the measured current count and never at 0**, so
the check lands green and ratchets down. The count today is 35 `sys_clone(` call sites
across 20 files (`bench/fuzz/repros` 5, `bench/parity_constructs/c84_run.bp` 2,
`bench/tq_sqlite/nn4.bp` 1, `bench/vs_rust/std_tests` 6, `bench/wip` 5,
`selfhost/std` 14, `selfhost/attic` 1 + 1 in `selfhost/std/sconc.bp`) — but
`bp_sources()` may exclude `attic`/`wip`, so **the seed must be measured by running the
check, not taken from this paragraph** (this is exactly the stale-number class).

**(22b) BOUNDED WAIT.** A `while <cond> {` whose condition contains a shared-cell read
(`\w+\s*\[`) and whose body, to the matching brace, contains **neither** a `\btel\s*\(`
**nor** an increment of a local counter that also appears in the condition, is an unbounded
wait: `"<file>:<n> unbounded wait with no record -- on expiry nothing can name which worker never arrived"`.
This is LOUD rule 4 made mechanical. Ratchet: the existing `max_unbounded_waits`
(`tools/arch_ratchet.txt:43`), re-seeded to the measured count in the landing commit.

**(22c) NO SYSCALL IN `tel`.** Within `bebop.bp`, the body of `fn emit_tel(` must not
`em()` the `svc #0` word. The word is already in the file 28 times — every `emit_sys_*`
ends with it (e.g. `bebop.bp:1152`, `:1165`, `:1215`) — so the check reads it out of the
file rather than hardcoding it: take the last `em(insns, n, W)` of `emit_sys_close` (`bebop.bp:1165`) as the
reference `W`, then fail if `W` appears inside `emit_tel`'s body.
`"emit_tel emits a syscall: telemetry on the hot path costs 19.2-27 us per record on this box (docs/exp.journal:1789232380)"`.
This is the safeguard that turns §3.1's central claim from prose into a fact the tree
cannot lose.

**(22d) The law itself becomes self-policing for free.** CHECK 14
(`check_laws_have_enforcement`, `tools/arch_check.py:352-387`) already fails if a law in
`AGENTS.md` has no row in `tools/law_manifest.txt`. So adding the prose law plus one row —

```
L25 | arch_check:telemetry-at-loss-sites | a spawn and an unbounded wait must each leave a record, and tel must emit no syscall
```

— is all that is needed for the manifest to keep the new prose honest. No extra machinery.

---

## 5. STEPS, IN DEPENDENCY ORDER

Each carries: files, effort, **the ONE NUMBER that kills it**, and whether it needs a
chain/fixpoint run. Run anything long through `bash tools/slot.sh <label> <cmd>`.

### Step 0 — falsify the arena arithmetic before writing anything
- **Files:** none in the repo. One ≤ 6-line `.bp` in the session scratchpad.
- **Does:** prints `sys_arena_end()`. §3.2's entire cell layout rests on x17 being the mmap
  base and the window being 33 554 944 cells; that was derived from `seed.S` plus two
  decoded stub words, and a derivation is not a measurement.
- **KILL NUMBER:** `sys_arena_end() != 33554944`. If it differs, every index in §3.2 is
  wrong and the slab must be re-derived before step 1 exists.
- **Chain:** no. Cost: minutes. **Do this first; it is cheaper than any other step by two
  orders of magnitude and it can invalidate all of them.**

### Step 1 — reserve the slab (one word)
- **Files:** `bebop.bp` (replace `st[7]` at `:6934`, ~15 lines of comment carrying the
  derivation and the `as`+`objdump` provenance per L1); `docs/TRAPS.md` (trap 80 now fires
  65 536 bytes early — that is a behaviour change to a documented trap and row 80 must say
  so); `tools/platform.txt` is **not** touched (no platform assumption moves).
- **Effort:** ~half a day, almost all of it the chain.
- **KILL NUMBER:** **the battery must stay at 113 pass / 3 fail**
  (`docs/exp.journal:1789233978`, `:1789233153`). Any gate that moves means a program was
  living in the top 64 KiB. Secondary: `tools/census.py --check` must report no change (the
  replacement word is an ALU op, not a branch), and `check_abi` must still pass.
- **Chain:** **YES** — `tools/chain.sh`, fixpoint `gen3 == gen4`, then promote. Nothing may
  be `cp`'d onto `bebop.bin` except from a fixpoint-green rebuild (law L13).
- **Note:** I found no corpus program that hardcodes the arena END. The `268435456` hits in
  `bench/vs_rust/std_tests/{sbench,sgraph}.bp` are store FILE sizes and
  `selfhost/prelude/store.bp:318` is the 2^28-cell object-length guard (trap 85) — none of
  the three is the arena bound. Risk assessed as low, but the kill number above is what
  decides it, not this note.

### Step 2 — the `tel` builtin
- **Files:** `bebop.bp` (`fn emit_tel` ~35 lines **including its L2 register table**, which
  CHECK 15 enforces and whose ratchet `max_syscall_no_regtable = 8` may not rise; one
  dispatch line near `:1870`; one hash in the T122 reserved table at `:5753-5756`);
  `bench/parity_constructs/c85_tel.bp` (**mandatory**: CHECK 10 `builtin-coverage` fails at
  `max_uncovered_builtins = 1` and `sys_mapb` already holds that single slot, so a second
  uncovered builtin fails `arch_check` immediately); `tools/bpref.py` (the reference
  interpreter must mirror `tel` or construct parity breaks — law L5, both engines);
  `docs/TRAPS.md`.
- **Effort:** ~1 day.
- **KILL NUMBER:** **`bebop.bin` grows by more than ~200 bytes.** At this step `bebop.bp`
  calls `tel` zero times, so the only growth is `emit_tel`'s own compiled body. If the
  binary grows by thousands, something is auto-instrumenting and the design has slipped
  from (d) to (b), which `ROADMAP.md:90` already refused at +2858 words / +7.27 %.
  Measured as `ls -l bebop.bin` before and after, which is the same identity evidence
  CHECK 3 `artifact-identity` uses.
- **Chain:** **YES** — fixpoint, then promote. **This is the step after which the compiler
  may begin to instrument itself, and not before** (the self-hosting tax, §1.1).

### Step 3 — the prelude library
- **Files:** `selfhost/prelude/tel.bp` (NEW, ≤ 250 lines; the 800-line cap and the
  `max_new_bp_lines` ratchet apply and there is no reason to go near them):
  `tel_slab()`, `tel_seq()`, `tel_dropped()`, `tel_reset()`, `tel_drain(fd)`, one fixed
  text format, one record per line, the header line always printed. Also
  `tools/gen_selfsrc.sh` if the prelude is expanded into `bench/vs_rust/std_tests`
  (invariants.sh rung (v) gates that expansion).
- **Effort:** ~half a day.
- **KILL NUMBER:** **any one of the 116 goldens moves.** If one does, the drain is on the
  wrong fd and must go back to fd 2 (§3.2).
- **Chain:** no. Battery re-run: yes.

### Step 4 — measure the overhead (this is what makes the design acceptable)
- **Files:** `bench/vs_rust/std_tests/tel_cost.bp` + `bench/oracles/tel_cost.py`
  (**the oracle is mandatory and must land in the SAME commit** — law L17, enforced by
  CHECK 13 `gate-oracle`); one `gate` line in `bench/vs_rust/std_golden.sh`;
  one `docs/exp.journal` line in the `H:/DID:/GOT:/VERDICT:` + `COST:` format (CHECK 16,
  and `journal_cost_since = 1789229455` means the COST field is now required).
- **Method:** exactly as §3.1 — two variants, 10^7 iterations, two `clock_ms()` calls per
  variant, best of three, whole thing run twice, then repeated at 2 and 3 clone children to
  price the contended `ldaddal`.
- **KILL NUMBER:** **> 100 ns per record uncontended.** Declared here, before the run, per
  law L10. Above it, the claim "free on the hot path" is withdrawn and narrowed to "free at
  store, spawn and syscall boundaries", and §2.1's PARTIAL rows for memory/bounds go to NO.
- **Chain:** no.
- **Also show the gate can fail** (`AGENTS.md`, "HOW TO WRITE THE NEXT FILE" #7): break `tel`
  on purpose, watch the number move, put it back, quote all three values.

### Step 5 — the enforcement, the law, the manifest
- **Files:** `tools/arch_check.py` (+~60 lines, CHECK 22); `tools/arch_ratchet.txt`
  (`max_unrecorded_spawns = <measured>`, re-seed `max_unbounded_waits` from its orphaned 0
  to its measured value); `AGENTS.md` (the prose law); `tools/law_manifest.txt` (row L25).
- **Effort:** ~half a day.
- **KILL NUMBER:** **`arch_check` must still print "all invariants hold"** on the unchanged
  tree. If seeding `max_unrecorded_spawns` needs a value above ~35 (the measured `sys_clone(`
  site count), the rule is matching things that are not spawns and is too broad to land.
- **Chain:** no.

### Step 6 — the trap handler drains the ring  ← highest value, highest risk, LAST
- **Does:** the entry stub's SIGTRAP/SIGSEGV handler currently writes `trap NN: <text>` and
  exits (`bebop.bp:6890-6903`). Extending it to also drain the ring makes **every existing
  trap — 80, 82, 87 — carry the last 1024 events**, which is the single largest jump in
  visibility available anywhere in this design, and it applies retroactively to every trap
  already in the tree.
- **Files:** `bebop.bp` `entry_stub`, and — if the stub grows — `cli_compile` plus
  `selfhost/std/gb_compile1.bp` (×3) and `selfhost/std/gb_run.bp`, which `bebop.bp:6913-6915`
  names as the five places the length 172 is a literal.
- **KILL NUMBER:** **the stub length must stay 172.** If the drain does not fit in the
  words available, this step does not land in this form; the fallback is an explicit
  `tel_drain(2)` before every `sys_exit` on a failure path, which CHECK 22 can then police
  as a fourth rule. Changing 172 without moving all five literals **in the same commit** is
  the `AGENTS.md` taxonomy row 10 defect, which cost a day the last time.
- **Chain:** **YES** — fixpoint, promote.

Dependency order is strict: 0 → 1 → 2 → {3, 5} → 4 → 6. Steps 3 and 5 are independent of
each other and can be lanes, provided they are file-disjoint (they are: `selfhost/prelude/`
vs `tools/` + `AGENTS.md`).

---

## 6. WHAT THIS DOES NOT SOLVE

Stated specifically, because an over-claimed blueprint is worse than none.

1. **The largest defect class gets nothing.** codegen/miscompile is 69 of 264 journal hits
   (26.1 %) and 38 % of HISTORY's defects. A wrong number raises no event. Nothing in this
   design touches it, and nothing in this design should be sold as touching it.
2. **`tel` is itself a new member of that class.** New emitted words are exactly what
   miscompiles. Until `bench/parity_constructs/c85_tel.bp` exists and `tools/bpref.py`
   mirrors it, `tel` is an untested construct by the project's own definition.
3. **stale constant / doc (24 hits) and fitted-or-unverified claim (10 hits): no help at
   all.** These are source-text and process classes. Errata E5, E6 and E7 in §0 are three
   fresh instances found during this very pass, and no runtime record would have produced
   one of them.
4. **Fork children are invisible.** `sys_clone(17, …)` has no CLONE_VM, so its arena is
   copy-on-write and its records die with it. That covers `c84_run.bp`, `sgraph2.bp`,
   `gb_bfs_gen_addr_gate.bp` and `selfhost/std/gb_run.bp` — i.e. the whole `sys_run` /
   `gb_pool` dispatch path. Only CLONE_VM threads (flags 68 864) share the ring.
5. **The compiler is instrumented last, not first.** Self-hosting forbids `bebop.bp` from
   calling `tel` until a binary that knows `tel` has been promoted (step 2's fixpoint). So
   compiler-internal visibility — arguably where it is most wanted, given `bebop.bp` is 7708
   lines — arrives one full generation after everything else.
6. **Nothing here is durable.** The ring is anonymous memory. A SIGKILL from the box
   watchdog (`AGENTS.md` L19: kills the largest worker below 450 MB MemAvailable, kills a
   shell spinning at 100 % for 30 s) loses every record. And `tools/platform.txt:7`
   `fsync_mode=nobarrier` means even the store is not durable on this box, so there is no
   durable sink here to fall back to.
7. **There is no source position at runtime.** The compiler has `pos`; the program does not.
   Tags are hand-assigned integers, which is a stale-constant hazard of the kind §0 keeps
   finding. Mitigation is law 3 — derive the tag in the source (`TEL_SPAWN = 1 * 1000 + 1`)
   — and it is a mitigation, not a fix.
8. **1024 records.** A run that produces more wraps. The drain reports how many were
   dropped; the dropped ones are gone. Growing the ring costs arena and nothing else, but
   the slab size is baked into an immediate in `emit_tel`, so changing it is a compiler
   change plus a fixpoint, not a runtime knob.
9. **64 KiB of every program's arena is spent whether or not it uses telemetry.**
   65 536 / 268 439 552 = **0.0244 %**. Cheap, but not free, and it is charged to programs
   that never call `tel`.
10. **The 10-30 ns figure is a hypothesis and the contended cost is completely unknown.**
    One `ldaddal` on a line shared by up to 8 cores is the entire uncertainty in this
    design, and step 4 exists for it. Until step 4 runs, the correct statement is that the
    cost is unmeasured — and by the brief's own standard, an unmeasured cost is not
    acceptable, which is why step 4 is a gate with an oracle and a declared kill threshold
    rather than a footnote.
11. **This is not a tracing system.** No spans, no causality, no correlation IDs, no
    ordering across threads beyond the atomic sequence number. What it gives is: a
    monotonic sequence, a tag, and two `i64`s, per event, shared across CLONE_VM threads,
    drained to fd 2. That is enough to answer "which worker never arrived" and "what did
    this thread do before it died", which is what the record says cost the days. It is not
    enough to answer "why is this slow", and it should not be asked to.

---

## 7. THE OPERATOR'S REQUIREMENT — can it be met as stated?

"Telemetry must be in the language itself." **Yes, but only as one builtin, not as a
subsystem**, and the reason is the reason the requirement is right in the first place.

The thing that must be in the language is not a logging API — a prelude `.bp` could give
that, at zero compiler cost, tomorrow. The thing that must be in the language is **a
storage location a clone child can reach without spending one of its eight kept symbols**,
and there is no way to produce that from the library level, because both address-yielding
builtins are re-homed by `emit_sys_clone` and the only register that survives — x17 — has
no surface syntax. That single missing primitive is the language-level gap. Everything
else follows from it and belongs outside the compiler.

So the honest shape of the answer is: **telemetry is in the language to the extent of ~40
lines of `bebop.bp`, and in the library for everything else** — and the 40 lines are the
ones that are impossible anywhere else.

If the ambition were larger — automatic instrumentation at every construct boundary, the
(b) option — I would say plainly that it should not be built here, and the reason is
`ROADMAP.md:90`: this project has already refused a feature at +2858 words and +7.27 % of
the self-compiler for firing on zero of its own 144 loops. Automatic instrumentation costs
more than that, permanently, in every program, including in the 39 % of the defect record
it cannot help with at all.
