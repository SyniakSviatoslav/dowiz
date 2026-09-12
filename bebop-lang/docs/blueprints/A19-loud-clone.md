Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row A19 (new). Delivers
language item 6 (loud failures) for the one silent class the tree has MEASURED and not yet closed: kept
symbols across `sys_clone`. It supersedes the withdrawn exit-103 check (TRAPS.md row 103).

# A19 -- the clone-spanning frame: loud first, then correct

## 0. Three corrections this blueprint rests on

**(0a) The mechanism is a FRESH frame, not a SHARED one.** The brief says "spill slots live in a frame
shared between parent and child, so post-clone locals stomp each other". The tree says the opposite.
`selfhost/std/pool.bp:6-7`: "spilled slots are per-thread stack memory and are NOT shared".
`emit_sys_clone` (`bebop.bp:1275-1321`): after the `svc`, the parent branches over the child's re-home
block (`cbnz x0`, `:1305`), and the child executes `:1306-1307` -- x15 (the spill base) is set to
`sp+256` on the NEW 12 MiB stack ("that is a FRESH 12 MiB stack, not this fn's frame", `:1300-1301`),
x27/x28 (arena) to `[sp+4 MiB, sp+12 MiB)` (`:1312-1317`). Symbols 1-8 live in x19..x26 and are
DUPLICATED by the clone; symbol 9+ lives at `[x15,#k*8]` (`sym_bind:242-247`, `vs_bind:2937`) and the child
reads that slot from a stack nothing has written: **zero**. A handle of 0 is cell 0; a marker written to
cell 0 is "the child never wrote". That is journal 1789219189 exactly: 8 kept -> correct, 9 -> one child
lost, 11 -> both, TIDs valid, no trap.

**(0b) The withdrawn check was right in intent and wrong in what it counted.** TRAPS.md row 103 and
journal 1789216976/1789219189: the 292b8953 binary counted in-scope symbols including constants, refused
programs that ran fine (a `let x = 5` is a CONST window entry that never touches a slot until it is
bound -- `vs_bind_reg:2955` materialises it INTO the slot, so a bound constant is kept too; what the
probe measured as "constants are free" is that the probe's constants were consumed before the spawn or
re-materialised from the window). What matters is: **a symbol read AFTER the spawn whose location is a
spill slot**. `arch_check.py` CHECK 20 (`check_clone_kept_symbols`) approximates this with a regex and cap 8, ratchet 0,
and it is conservative by design (its 'conservative in the right direction' comment).

**(0c) The 120 s b6core hang WAS real, and it was measured after this blueprint was drafted.**
CORRECTED by the main session: the research pass looked for it in the journal and did not find it, which
was true at that moment -- it had not been written yet. Two independent measurements: the battery of
2026-09-12 21:54 recorded `b6core_test 120199 miss rc=124` in `bench/perf_fn/gates.txt` (rc=124 = the
gate's own 120 s `run` cap), and a direct `timeout 45 ./seed/build/seed <b6core.bin> 0 1` returned rc=143.
Cause, and it is NOT this row's mechanism: `b6_par`'s join waited for `sys_atomic_add(cells, W+w, 0) == 1`
and nothing in the file ever set that flag. Fixed in `132063b`; b6core now terminates at every (shape, W).
The separate 60 s hang the tree records is journal 1789233153 (`sys_exit` emitted EXIT not exit_group;
fixed, rc=80 in 516 ms) -- a different silent class, already closed by `arch_check` unbounded-wait
(ratchet 0, L25). **This row's own mechanism stands on its own evidence and needs neither hang**: the 8/9/11
symbol measurements of journal 1789219189, and `pool.bp:6-7` against the fresh-frame reading of
`bebop.bp:1296-1307`.

---

## 1. Scope

### A19 IS

1. **A compile-time refusal (step 1)** at every `sys_clone` site whose enclosing fn keeps a spilled
   symbol live past the spawn: `error[E<code>]: <fn> keeps <n> symbols across sys_clone; only 8 survive in
   registers -- pack the rest into one env: [i64]`. Zero program words. The published per-fn fact `vc`
   (`fw_vc`, `compile_fn_at_facts:5961`) is the count; `> 8` is the refusal, conservative exactly as
   CHECK 20 is.
2. **The child's frame made correct (step 2)**: before re-homing x15 the child copies its parent's
   `S + tsp` slots (`frame_size:4519`, both published facts) from `[x15_parent]` to the fresh frame, so
   symbol 9+ reads its true value in the child. After step 2 the refusal is unnecessary and is deleted;
   the positive construct with 12 kept symbols is the proof.
3. **Three constructs**: `neg/c126_clone9` (refused, step 1), `c127_clone8` (8 kept, both children
   write, the boundary), `c128_clone12` (12 kept, step 2 only; RUNFAIL or wrong value under step 1's
   binary).
4. **CHECK 20 kept as the static mirror** at ratchet 0 until step 2 lands, then retired (its reason
   disappears) -- an `arch_check` that guards a fixed defect is a ratchet nobody reads (L25).
5. **B6's blueprint §4 rule** (three params + one local across a spawn) stays the recommended shape for
   readability even after step 2, but stops being a correctness requirement.

### A19 IS NOT

1. **Not a general "live symbols" analysis.** `vc` over-counts (a symbol bound before the spawn and dead
   after it still counts). Refusing more than needed is the direction CHECK 20 chose and B6 accepted.
2. **Not a fix for `zeros` inside a clone child** (8 MiB window, trap 80, the sconc class) -- that is
   loud already (`trap 80`, `:1308-1313`) and B6 §2.3's `noalloc` witness covers it.
3. **Not the release/acquire handshake** (B6 step 2, landed in `05fcc0f`).
4. **Not a change to `sys_clone`'s flags, stack pitch, or the arena window.**
5. **Not a runtime check.** A runtime "did the child see its symbols" cannot exist: the child cannot know
   what the parent's slot held. Compile time or by construction; nothing in between.

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/neg/c126_clone9.bp` | 9 kept symbols (array handles from `zeros`, read after the spawn) across a `sys_clone(68864, ...)`; `EXPECT=COMPILEFAIL:<code>` under step 1 |
| `bench/parity_constructs/c127_clone8.bp` | 8 kept, two children each write `w + 100` into a shared cell, parent futex-waits (the journal-926 probe shape, synchronised), EXPECT = 201 (100 + 101) |
| `bench/parity_constructs/c128_clone12.bp` | 12 kept, same protocol; EXPECT 201 under step 2; under step 1 it is `neg/` and moves to positive when step 2 lands |
| `tools/arch_check.py` CHECK 20 | unchanged until step 2; then deleted with its ratchet key (ratchet-orphan would otherwise fail, L25) |

### 2.2 The number

```
construct parity: pass=<n> fail=0   with c126 COMPILEFAIL:<code>, c127 201, c128 201 (step 2)
```

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c126` refused with no `.bin` | the check counts constants only, or reads the planning pass's `vc` (0 in the planning pass: `fw_vc` is published AFTER the fn is planned -- the check must run on the emission pass, `is_emission == 1`, or it never fires) |
| B | `c127` = 201 | the check refuses 8 (too eager), or the synchronisation is missing (a race reads 100 or 0 -- journal 923's mistake) |
| C | `c128` = 201 under step 2 | the copy loop copies `S` but not `tsp` slots, or copies BEFORE the parent's last pre-spawn spill was written (it must run in the child, after the `svc`, before the re-home) |
| D | a mutant that removes the refusal makes `c126` produce a `.bin` and RUN to a wrong value (< 201) | the construct's 9th symbol is not actually read after the spawn (then the program is fine and the check is the eager one) |
| E | `smw` 303000, `sconc` 40000, `pool_parity` 5/0, b6core's `ok` unchanged | an existing clone-spanning fn is refused (check the six sites in §3 BEFORE landing) |

---

## 3. Which existing work A19 sits on

| existing | verdict | reason |
|---|---|---|
| `emit_sys_clone` (`bebop.bp:1275-1321`) | **KEPT; step 1 adds a check before `:1284`; step 2 inserts the copy loop between the `svc` result branch `:1305` and the re-home `:1306`** | the parent's skip length `cbnz x0,.+16` at `:1305` is COUPLED to the block length (`:1297-1304`, bisected once at a day's cost); every inserted word changes it -- derive with `as`+`objdump` |
| `compile_fn_at_facts` (`:5961`), `fw_vc/fw_tsp` (`:5955-5958`) | **READ** | the counts are already published per fn |
| `check_reg_collision` (`:2262`) | pattern for "a compile-time refusal with a position" | |
| `tools/arch_check.py` CHECK 20 (`check_clone_kept_symbols`) | KEPT until step 2 | the static mirror; its cap key `max_clone_kept_symbols` is read by it (ratchet-orphan clean) |
| journal-926 probe (described, not committed) | **BECOMES `c127`/`c128`** | a measurement with no committed script is what B6's §0b refused to accept |
| `smw.bp:4-12`, `b6core.bp`, `nn4.bp`, `sconc.bp`, `pool.bp`, `gb_run.bp:359,534` | the six spawn sites in the tree; each must pass step 1's check on day one | CHECK 20 at ratchet 0 says they do |
| TRAPS.md row 103 (reserved) | **RETIRED**: the new code is from the free set, 103 stays reserved as documented | |
| `docs/WORKER-CARD.md` clone trap paragraph | corrected to the fresh-frame mechanism | it still describes the limit as a rule of thumb |

---

## 4. Constraints as design input

1. **The check must run on the emission pass only.** `is_emission == 0` (planning) has no facts yet
   (`compile_fn_at:6054`); `diag_exit` from the planning pass would fire on every fn that the emission
   pass would accept.
2. **Zero program words in step 1** -- it is a `diag_exit` in the emitter; WORD_DELTA 0 on 104/104.
3. **Step 2's copy is a fixed shape**: `S + tsp` is a per-fn constant at emission, so the loop is
   `mov x16, x15 ; add x15, sp, #256 ; <k times: ldr x9,[x16,#8i] ; str x9,[x15,#8i]>` unrolled (k <= 64
   by the slot cap `S + tsp > 64 -> exit 89`, TRAPS.md row 89), or a 5-word loop when k > 8. x9 is a
   caller-saved scratch the child owns at that point; x16 is the model's parallel-move scratch, dead at
   every builtin site (A5 row). Word count = 2 + 2k (unrolled) -- both passes agree because k is a fact.
4. **The parent's skip** (`cbnz x0, .+16`) becomes `.+(16 + 8k)` -- computed, not literal, and
   assembler-verified for k = 0, 1, 8, 64.
5. **The fork case (`sys_clone(17, ...)`, `gb_run.bp`)**: a forked child has COW memory -- the parent's
   frame IS readable at the old x15 -- so the copy is correct there too and merely redundant.
6. **Bootstrap:** step 1 single-stage (a check). Step 2 changes clone words -> gen2 != gen3, fixpoint at
   gen3 == gen4; the compiler itself never clones, so no two-stage issue.

---

## 5. Steps, in order, each with a number that can kill it

### Step 0 -- confirm the mechanism with one probe (30 min, a worker's slot)

Compile the journal-926 shape with 9 kept symbols; in the child, write `sys_arena_base()` and the 9th
symbol's value into two shared cells before anything else. **Expected:** the 9th reads **0** in the child
and its true handle in the parent. **If it reads the true value, this blueprint's mechanism is wrong and
step 2 must not be built** -- report and stop.

### Step 1 -- refuse (one codegen commit, diag words only)

**Do:** in `emit_sys_clone`, when `is_emission == 1` and `fw_vc(fact) > 8`, `diag_exit(s, srcpos,
<code>)` with A17's text. Constructs `c126`, `c127`. Run the six existing spawn sites.
**Expected:** fixpoint; WORD_DELTA 0 on 104/104; `construct parity: pass=106 fail=0`; bcond +1 with an
allow line; `smw`/`sconc`/`pool_parity`/`b6core` unchanged.
**Kills the step:** any of the six sites refused (then B6 §4's env rule is applied to that site FIRST,
in its own commit, before this lands -- the refusal is right and the site is wrong).
**Effort:** 1 day. **Gate-days:** 1.

### Step 2 -- copy the frame (one codegen commit, clone words)

**Do:** the copy block; the skip arithmetic; `c128` moves from `neg/` to positive; delete the step-1 check
and CHECK 20 with its ratchet key; TRAPS.md and WORKER-CARD updated.
**Expected:** `c128` = 201; `c127` = 201; every clone-site gate unchanged; 3 constructs' WORD_DELTA =
+2+2k for their k; `smw` 303000 with its k = 0 (four kept, all in registers: 0 extra words).
**Kills the step:** `c128` < 201 (the copy ran before a late spill, or copied the wrong region -- check
`frame_x15off(marks)`, `:4518`: the region starts at `sp+88+8*marks`, NOT at sp+256, in the PARENT; in
the child it is the fresh `sp+256` -- the two bases differ and the copy must use each side's own).
**Effort:** 3-5 days. **Gate-days:** 1.

---

## 6. The honest ceiling

- **Step 1 refuses programs that would run.** A fn with 9 symbols whose 9th is dead after the spawn is
  refused. The cost is one `env` array; the tree already pays it at 22 sites; B6 mandates it. This is
  the conservative direction, and it is the only one available without liveness.
- **Step 2 does not make spilled symbols SHARED** -- they are copies, exactly as x19..x26 are copies.
  A child that writes a spilled symbol writes its own frame; the parent does not see it. That is
  thread-local semantics, the same as registers, and it is the only consistent choice. A program that
  wants to share must share a cell, which is what `sys_cond_set`/`sys_atomic_add` exist for.
- **What stays silent after this row:** a child that dies by `sys_exit` (thread-only, `svc 93` from
  `sys_exit_thread_guard`) before setting its flag -- the parent's wait is bounded by `arch_check`
  unbounded-wait's rule, not by the language. A per-child status cell written by the stub's SIGTRAP
  handler would make it loud: costed at ~20 stub words + a convention, not scheduled, named for B6.

---

## 7. VERDICT format for an A19 worker

```
VERDICT: GREEN|RED
step: <0-2>
probe (step 0): 9th symbol in child = <value>   (0 confirms the mechanism)
fixpoint: gen3 == gen4 <md5>
constructs: pass=<n> fail=<n>; c126 COMPILEFAIL:<code>; c127 <value>; c128 <value>
clone sites: smw <v> sconc <v> pool_parity <p>/<f> b6core ok=<0|1> nn4 ok=<0|1> gb_pool <v>
census: bcond <before> -> <after> (allow line yes|no)
words (step 2): +2+2k on c127/c128 with k = <S+tsp>; skip word verified by objdump <yes|no>
mutation: refusal removed -> c126 runs and yields <value> (< 201 required)
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations, each with the line it deviates from>
```
