Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Every number is
QUOTED from a committed file with its line or DERIVED by arithmetic from such a number and marked
derived. Grounded at HEAD `d75172c`. Roadmap row A17 (new; ROADMAP-PATCH.md). Delivers language item 5
(real diagnostics) and the agent-framing item 11 (a machine-readable diagnostic protocol) -- one
mechanism, one row.

# A17 -- diagnostics as data: one table, one line shape, grep-able in source and binary

## 0. Three corrections this blueprint rests on

**(0a) "Neither grep nor strings finds a message" is half true, and the half that is false matters.**
`strings -n 8 bebop.bin` finds the four RUNTIME trap texts (`trap 80`, `trap 81`, `trap 82`, `trap
87`) because they live as packed bytes in the entry stub (`bebop.bp:6904` `entry_stub`, T90 2c). It finds
**none** of the eleven COMPILE-TIME texts, because `diag_exit` (`bebop.bp:75-107`) builds each message
from an i64 array literal -- `[101, 120, 112, 101, 99, 116, 101, 100, ...]` at `:92-102` -- one cell per
character, and sums their lengths by hand at `:104`. The binary carries the STORE instructions that build
the message, never the message. So the compile-time path is the defect, and the runtime path is the
model to copy -- except that the stub also still ships `trap 81: frame heap exhausted`, a code A6
RETIRED (`docs/TRAPS.md` row 81).

**(0b) The exit-code space was partitioned on 2026-09-09 and the documentation did not follow.**
`bebop.bp:3607-3640` states the partition: user-facing diagnostics below 99 (plus grandfathered 99-104),
compiler self-checks at one code, 201, with `kind.site` on stderr (`selfcheck_exit:2205-2216`); free
codes 65-79, 84, 85, 86. Since then 65 was taken by F3's static bounds check (`bebop.bp:4232`, text
`:101`). `docs/TRAPS.md` row 89 still says the fn cap is **512** (the compiler says 768: `bebop.bp:102`,
`:4410`, `:5893`), and rows 85/86/87 document `store.bp` PROGRAM exit codes that numerically overlap the
compiler's free range -- different process, same integer. An agent reading `rc=86` cannot tell which
program spoke. The line shape must say.

**(0c) The current output IS machine-readable, and the harness already parses it.** `diag_exit` writes
`<line>:<col>: <text>\n` to stderr and exits with the code; `bench/vs_rust/diag_check.sh:11-15` parses
it with `cut -d: -f1,2` against a hand-counted `// EXPECT line:col code` header in each of the ten
`bench/diag_neg/*.bp`. What is missing is the FILE (a `.use`-expanded program has positions in the
expansion, `bebop.bp:69-71`), the CODE as a token, the PROCESS, and the repair. JSON is not missing;
JSON is costed in §6 and not chosen.

---

## 1. Scope

### A17 IS

1. **One table of diagnostics as string literals** inside `bebop.bp`: `diag_text(buf, at, "expected
   `)` or `in`")` instead of an i64 array and a hand-summed length, for every compile-time code, plus
   `cap_exit` and `selfcheck_exit`. `str` literals are legal as call arguments today (`docs/LANGUAGE.md:91`
   "only valid as an argument"; `char(s, i)` reads bytes: `emit_char_fn:1776`).
2. **One line shape for every non-zero compiler exit**, the gcc/rustc shape every agent tool already
   parses: `<file>:<line>:<col>: error[E<code>]: <text>` on stderr, exactly one line, exit = code.
   `<file>` is `argv[3]` (`cli_compile:7119`, `cli_check:7246`). Self-checks: `<file>:0:0:
   error[E201]: compiler self-check <kind>.<site>`.
3. **The table is the documentation**: `tools/trap_census.py` (already derives the code census from
   `bebop.bp`, `docs/TRAPS.md` and `neg/`) regenerates the compile-time rows of `docs/TRAPS.md` from the
   literals and FAILS on a code with no text, a text with no code, or a documented code the compiler
   never emits. TRAPS.md's compile-time rows stop being hand-edited.
4. **Runtime traps in the stub get the same token** (`trap 80: ...` -> `error[E80]: arena exhausted
   ...` with the PROGRAM's name absent -- the stub does not know it -- but the word `trap` kept so the
   two processes are distinguishable), and the retired 81 text is deleted. This is a separate commit
   (step 3) because the stub is copied into EVERY binary: 104 `word_budget` lines, the A5 precedent (+41).
5. **A repair hint where one exists**, as a second sentence in the same line: exit 89's text becomes
   `let binding while a call temp is live -- bind the call result with a let first`, which is what
   `docs/TRAPS.md` row 89 already says and the compiler does not.

### A17 IS NOT

1. **Not a new diagnostic.** No new check lands here; F2's sixteen zero-word traps stay F2's. This row
   changes how every existing code is stored and printed, and nothing about what is rejected.
2. **Not JSON.** Costed in §6: escaping arbitrary source text in a language with no string operations
   is ~60 lines of byte loops for a shape no agent needs -- the gcc line is already parsed by every
   editor, CI and LLM tool. If a JSON consumer appears, it is one `python3 -c` over the line.
3. **Not a change to any exit code number.** The grandfathered 99-104 stay; the partition stays; the
   fuzzer classifies by exit code (`bench/fuzz/fuzz.sh:7`) and every `EXPECT=COMPILEFAIL:<code>` in
   `construct_parity.sh:206-238` stays valid.
4. **Not the runtime message path for `sys_exit`.** A program's own `sys_exit(n)` prints nothing; that
   is the program's business (LANGUAGE.md "Exit codes").
5. **Not the store's exit codes 85-87** (`selfhost/prelude/store.bp`, `st_seal`/`st_open`). They are a
   library's choice; this row only makes the compiler's line say `error[E..]` so the two are
   distinguishable in a log.

---

## 2. The gate

One number, in a committed script, that a tautology cannot satisfy because it counts two independent
things that must agree.

### 2.1 Files

| file | what |
|---|---|
| `bench/vs_rust/diag_check.sh` | already the diag lane (`battery.sh:43`); its parser changes from `cut -d: -f1,2` to `-f2,3` and it additionally requires the `error[E<code>]` token to equal the exit code |
| `tools/trap_census.py --texts` | new flag: prints `diag_texts: <n> codes, <m> in source, <k> in binary, <d> documented` and exits 1 unless `n == m == k == d` |
| `bench/diag_neg/d11_selfcheck_shape.bp` | (new) a program that trips ONE self-check deliberately is not writable from the outside -- instead the gate greps `bebop.bin` for the literal `compiler self-check` (k counts it) |
| `bench/vs_rust/std_golden.sh` | no new `gate` line (this is the diag lane, not a std gate); `ok=117` unchanged |

### 2.2 The number

```
diag: <pass> pass 0 fail            (diag_check.sh, today 16 pass)
diag_texts: N codes, N in source, N in binary, N documented
```

`N` today would be 11 (diag_exit) + 1 (cap 83) + 1 (self-check 201) + F3's 65 = **14** compile-time
texts; after step 3, plus the 3 live runtime traps (80, 82, 87) = 17, and the retired 81 must be
**absent** from the binary (a negative assertion in the same check).

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | every `diag_neg` program exits with its header's code AND its stderr's `error[E<code>]` token equals that code AND the `file:line:col` prefix equals the header | a site prints the old shape, the wrong code token, or no line |
| B | `strings bebop.bin` contains every text in the source table | a message is still an i64 array (0 hits today for all 11) |
| C | `docs/TRAPS.md` compile-time rows == the table | a code is added without documentation, or documented without a site (F1's `trap_census` already finds 105/106/107 emitted and documented nowhere) |
| D | `trap 81` absent from `bebop.bin` after step 3 | the retired text survives |
| E | `bin_words` DECREASES in step 1 (a `bebop <newwords>` `word_budget` line is NOT needed; the perf row must show a negative delta) | the replacement emitted more words than the arrays, which means the literal path is not being used |
| F | every `EXPECT=COMPILEFAIL:<code>` construct (15) still matches | a code number moved |

RED on day one: A (no token), B (0 of 11), C (TRAPS.md says 512), D (81 present). Not a record.

---

## 3. Which existing work A17 sits on

| existing | verdict | reason |
|---|---|---|
| `diag_exit` (`bebop.bp:75-107`): line/col computation `:76-85`, buffer `:86-92`, the eleven `if code == K then diag_text(...)` arms `:92-102`, `mlen` `:104`, write+exit `:104-106` | **KEPT as the site, REWRITTEN in the middle** | the position arithmetic and the write are right; only the storage of the text and the line prefix change |
| `diag_text(buf, at, m: [i64], len)` (`:66-73`) | **REPLACED** by `diag_str(buf, at, m: str)` using `str_len`/`char` | the length argument is the off-by-one A13's blueprint warns about (`A13-A15-diagnostics.md` §4) |
| `put_num` (`:46-64`) | KEPT | decimal formatting of line/col/code |
| `cap_exit` (`:2172-2183`), `selfcheck_exit` (`:2205-2216`) | **REWRITTEN onto the same helper** | two more hand arrays |
| the entry stub's trap texts (`entry_stub:6904+`, packed bytes) | KEPT in step 1-2, **EDITED in step 3** | every binary changes; own commit with 104 budget lines |
| `bench/vs_rust/diag_check.sh` + `bench/diag_neg/*.bp` (10) | KEPT, parser 1-line change | the lane exists and is in the battery |
| `tools/trap_census.py` | **EXTENDED** (`--texts`) | it is already the derivation tool F1 built |
| `docs/TRAPS.md` compile-time rows | **GENERATED** from step 2 on | hand-edited rows drift (512 vs 768 today) |
| `docs/blueprints/A13-A15-diagnostics.md` | reference for the "derive the ASCII mechanically" rule | superseded by literals: no ASCII arrays to derive |
| exit-code partition comment `bebop.bp:3607-3640` | KEPT, corrected (65 taken) | it is the authority on numbers |

---

## 4. Constraints as design input

1. **One pass, and the message is compiler code, not program code.** A compile-time text costs words in
   `bebop.bin` only. Replacing an N-element array literal (each element: a constant materialisation + a
   store, `emit_array_lit:4002`, ~2 words per element plus the allocation) by a string literal (`adr` + `ceil(N/4)` data words, `write_lit_cells:6327`) shrinks the compiler by roughly `11 * 30 * 2 - 11 * 8 = ~570` words (derived; the gate reads the real delta).
2. **`str` literals are arguments only** (`LANGUAGE.md:91`), and `diag_text`'s replacement takes one:
   `fn diag_str(buf: [i64], at: i64, m: str) -> i64 { let n = str_len(m); let i = 0; while i < n { let
   _ = buf[at + i] = char(m, i); let i = i + 1; 0 }; at + n }` -- entirely inside today's grammar, so the
   bootstrap is single-stage and the compiler uses it in the landing commit.
3. **Literal count.** `bebop.bp` has 88 `"..."` literals today (`grep -o`); the literal table is
   `fntab[6000..6999]` (`check_abi.py:212`), so +20 is safe. (A7 step 2's reverted table at 7000/7016
   collided at 16; this row must not re-introduce that layout.)
4. **The `sys_write` buffer is cells, one byte per cell** (`emit_sys_write:1138-1153` packs cells to
   bytes); `buf = zeros(128)` at `:86` grows to 256 for a file name -- `cli_compile` has the path as
   cells already (`str_to_cells`, `:6651`).
5. **The self-check must not gain `diag_exit`'s position loop** (`selfcheck_exit` deliberately prints
   `kind.site`, `:2199-2203`); it prints `0:0`.
6. **The fuzzer's classifier is exit-code based** (`fuzz.sh:7`): unaffected. `bench/fuzz/gen.py`
   unaffected.

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- literals and the line shape (one codegen commit, compiler words only)

**Do:** add `diag_str`; rewrite the eleven arms of `diag_exit`, `cap_exit`, `selfcheck_exit` to
literals; prefix `<file>:` (pass the path cells into `diag_exit` via a fntab cell set by `cli_compile`/
`cli_check` -- `fntab[5601]` is free per `check_abi.py:205-212`; verify with `check_abi.py --fntab`);
insert `error[E<code>]: ` after the position; append the repair sentence to 89, 97, 100, 101.
**Chain:** `tools/chain.sh bebop.bp $OUT --codegen`. Expected: gen3 == gen4; **WORD_DELTA 0 on all 104
constructs** (no program word changes); `bebop` words DOWN by 400-800.
**Kills the step:** any construct WORD_DELTA != 0 (the change leaked into program emission -- most
likely a `diag_str` call placed on an emitted path); or `bebop` words UP (the literal path is not what
runs); or `diag: N pass` with a FAIL naming a position that moved (the `.use` line arithmetic at `:69-71`
was touched by accident).
**Effort:** 1 day. **Gate-days:** 1.

### Step 2 -- the census asserts the table (no compiler change)

**Do:** `tools/trap_census.py --texts` as in §2; regenerate TRAPS.md's compile-time rows; correct row
89's cap to 768 and mark 85-87 as `store.bp` program codes in a separate table with the process named.
Wire `--texts` into `bench/vs_rust/invariants.sh` next to rung (vii).
**Expected:** `diag_texts: 14 codes, 14 in source, 14 in binary, 14 documented`.
**Kills the step:** a code the census finds that has no text -- that is a real finding (F1 found three
last time) and is fixed by adding the literal, not by relaxing the check.
**Effort:** 0.5 day.

### Step 3 -- the stub (one codegen commit, EVERY binary changes)

**Do:** edit the three live trap texts in `entry_stub` to carry `error[E<code>]`, delete `trap 81`.
The stub's byte table is packed by hand (`entry_stub` words); derive the new words with `as` +
`objdump` per L1, record in `$OUT/words.objdump` first.
**Expected:** every construct's WORD_DELTA = the same constant (the stub delta), 104 `word_budget`
lines with one reason, `stub <newwords>` line in `word_budget.txt` (`tools/perf.py` gates it,
`word_budget.txt:3-4`); `strings bebop.bin | grep -c 'trap 81'` = 0.
**Kills the step:** two constructs with different deltas (the stub length is a literal in FIVE places
per the A5 row -- `gb_compile1.bp` x3, `gb_run.bp:415`, `entry_stub` -- and one was missed; that
defect cost A5 a day: `ROADMAP.md:92`).
**Effort:** 1 day. **Gate-days:** 1.

### Step 4 -- the diag lane reads the shape (harness)

**Do:** `diag_check.sh` parses `<file>:<line>:<col>: error[E<code>]:` and asserts token == rc.
**Expected:** `diag: 16 pass 0 fail` (same count), and a deliberately broken token in a scratch copy of
`bebop.bp` makes it FAIL (the mutation the honesty floor asks for).

---

## 6. The honest ceiling

- **What this buys an agent, measured today:** `exit 89` alone has three meanings (TRAPS.md row 89);
  after step 1 the line names which one and the repair. The `EMPTY(rc=...)` naming in `std_golden.sh`'s
  `gate()` (`:94-107`) already showed the value of this one level up.
- **What it cannot buy:** a message for a SIGSEGV that the stub's handler catches (82) can only say
  "wild access or stack overflow" -- the pc is available in the handler (`entry_stub` reads it from
  ucontext, `:6900`) and printing it as a word offset is +~15 stub words; worth it, and it is step 3's
  optional extra, costed not scheduled.
- **JSON, costed:** a `{"file":..,"line":n,"col":n,"code":n,"msg":".."}` line needs `"`/`\` escaping of
  the text (the texts are ours, so none contains them -- a rule, not a check) and of the FILE name
  (arbitrary; a byte loop, ~30 lines). ~60 lines for zero additional information over the gcc shape.
  Not scheduled; the gcc shape is what `diag_check.sh`, editors, and every agent's regex already read.
- **The one thing that stays hard:** positions are counted over the `.use` expansion (`bebop.bp:69-71`).
  Mapping back to the original file and line needs the `use` splice table (`use_scan:6818` knows the
  offsets); +~40 lines, and it changes the `diag_neg` headers. Costed at 1 day, not scheduled here --
  named because an agent editing `selfhost/prelude/store.bp` gets a position in `<out>.use` today.

---

## 7. VERDICT format for an A17 worker

```
VERDICT: GREEN|RED
step: <1-4>
fixpoint: gen3 == gen4 <md5>     bebop words <before> -> <after> (must fall in step 1)
constructs: WORD_DELTA 0 on 104/104 (step 1)  |  uniform +<k> on 104/104 with 104 budget lines (step 3)
diag: <pass> pass <fail> fail      diag_texts: <n> codes, <m> source, <k> binary, <d> documented
strings: 'trap 81' hits in bebop.bin = <0|1>
mutation: broken token in scratch copy -> diag lane FAIL <yes|no>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations, each with the line it deviates from>
```
