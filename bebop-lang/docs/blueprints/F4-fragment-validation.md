Status: 2026-09-13, written for the ROADMAP row that cites it -- **row F5, ROADMAP.md:199**, whose blueprint column reads `docs/blueprints/F4-fragment-validation.md` (row letter and file name differ by one; F_PHASE_STATUS.md:277 lists the file as missing). Base commit `ddb534f`. Every `file:line` below was re-derived with `sed -n` / `grep -n` against that tree on 2026-09-13. Binary measured: `bebop.bin` digest `072c01d1` (8 hex = md5 prefix per `tools/battery.sh:56`; not a commit hash). This blueprint depends on `docs/blueprints/F3-lean-semantics.md` for the language rules and on `docs/blueprints/F2-bounds-by-type.md` for the checks that change the fragments.

# F4 Fragment validation: translation validation from the one-pass emitter trace

## 0. Goal

Per compiled binary, a machine-checked statement that every construct's emitted words implement that construct's semantics relative to the register model the emitter believed at the time -- "this build is right" (`docs/RESEARCH-VERIFICATION-2026-09-09.md:248`), without proving the compiler. The row's gates: `tv_fragments: v/t` over the constructs then over the self-compile; `trace_zone_words: 0` in code; fixpoint gen3 == gen4 unchanged; the trace-derived `bounds_census` agreeing with F2's static one. The number today is **0/0, and the one script that prints it prints PASS** (§2). Making the gate able to fail comes first.

## 1. What a fragment is HERE (from the source, not from the literature)

The compiler has no IR. Every machine word reaches the output through ONE function, `em` (`bebop.bp:2276`: bounds check, `insns[n[0]] = w`, `n[0] += 1`). Construct emitters (`grep -c "^fn emit_" bebop.bp` = 105 functions) call `em` directly or through `vs_*` helpers, and back-patch branch words through `patch_b` (`bebop.bp:2011`). The emitter's belief about machine state is the register-model window: the entry list at `fntab[3700..4335]` (512 slots, `bebop.bp:2813`), its top `fntab[5387]` (read as `let w = fntab[5387]` at `bebop.bp:4137`), the cs mask `fntab[5389]` (`bebop.bp:2292`; note `docs/REGISTER-MODEL-BLUEPRINT.md:99` still says `fntab[3823]` -- the code is the authority), and `stab[0]` = S, the symbol count. Window entries have a KIND and a payload: kind 1 is a compile-time CONSTANT (`emit_array_get`, `bebop.bp:4138-4140`, tests `idxk == 1` and takes the literal from `vs_entry_p0`), so a construct can legitimately emit ZERO words and change only the window (the OPT-C fast path, `bebop.bp:6727-6729`: "when both operands ... are known the operation is computed at compile time and NO instruction is emitted").

So a **fragment** is: the source position of one construct; the half-open word range `[first, last)` that its emitter appended to `insns` between entry and exit; the window state at entry and at exit (entries with kind/register/slot/constant, cs mask, S); and the list of `(word index, value)` patches the emitter applied inside that range after the fact. `emit_array_get` (`bebop.bp:4136-4151`) is the smallest instance: two window entries in, one `ldr` or `add`+`ldr` out (`vs_array_get_imm` :4153, `vs_array_get_reg` :4158), one entry of kind 2 pushed. `emit_while_stmt` (`bebop.bp:5292`) is the largest common one: a diamond whose entry asserts the cs mask is 0 (`bebop.bp:5170`, `:5188`: "its entry assertion, cs mask must be 0"), which is the loop invariant the row names.

Where a fragment lives in the file: a `.bin` is code words, then the string-literal data section (`bebop.bp:6522`: "append the string-literal data section after the code words"), then the 172-word entry stub (`bebop.bp:7118`, `:7139`), then an 8-byte LE64 entry offset (`seed/seed.S:2`, `:50`). `tools/check_abi.py:62 load_bin` defines `code_end` as the word after the LAST `ret`; `tools/census.py:22-25` counts `words = code_end`, "data cells excluded". Measured on `072c01d1`: 44,970 words in the file, `code_end` 44,020, entry at word 44,798 -- so 778 data words and the stub sit past the last `ret`. Any bytes appended after the stub and before the footer are inert to the seed and invisible to `census.py`.

## 2. What exists today, and why it must not be wired as it is

`tools/tv_fragments.py` (299 lines, committed in `4286ec1` on 2026-09-11 alongside `formal/harness.lean`; `git cat-file -t 4286ec1` = commit) is the only F5 artifact on disk. `F_PHASE_STATUS.md:136` says "Evidence: None on disk"; that is wrong, but the file's existence is worse than none:

1. **Nothing wires it.** `grep -rn tv_fragments tools/battery.sh tools/chain.sh bench/vs_rust/*.sh tools/*.py` finds only its own file and the ROADMAP row.
2. **It looks for a zone the compiler never writes.** It scans past `code_end` for a marker word `0xF5F5F5F5` (`tv_fragments.py:121`); `grep -n "F5F5F5F5\|4126537205\|trace" bebop.bp` returns nothing. No `.bin` in the tree has ever carried a trace zone.
3. **It passes when it finds nothing.** `tv_fragments.py:271`: `PASS: no trace zone found ... (0/0)`, rc=0; `:278`: `PASS: trace zone empty`. Wired today it would print a green line on every binary -- the vacuous-PASS class the F8 row measured on 2026-09-12 (ROADMAP.md:202) and spent a step removing.
4. **Its "validation" cannot fail on a wrong translation.** `validate_trace_zone` (`:207-252`) recomputes an FNV digest over the fragment's own words (`compute_fragment_digest`, `:188-204`) and compares it with the recorded `window_after` (`:236-242`). The digest is of the words, not of any semantics; it can detect a corrupted file and nothing else. The docstring at `:184` defers "full symbolic execution" to "the Lean layer", which does not exist (F3 §1).
5. **Its decoder is wrong on documented words.** Run against five words whose meaning `bebop.bp` states in comments: `0xF8607A20` (`ldr xd,[x17,xt,lsl #3]`, `bebop.bp:4149`) -> `eor_reg`; `0xD65F03C0` (`ret`) -> `b`; `0xD2800001` (`movz`) -> `add_imm`; `3944481663` (`cmp x27,x28`, `bebop.bp:6754`) -> `str_imm`; `3558869504` (`brk #80`, `:6756`) -> `b`. Five of five misclassified: the `op0` table at `:18-97` does not match the AArch64 top-level encoding classes. Applied to the 308 distinct base words the compiler emits, it reports 96 `str_imm` and 11 `unknown` -- not an inventory of anything.

Decision: keep the path `tools/tv_fragments.py` (the row and F_PHASE_STATUS name it) and replace the contents; step 0 makes it REFUSE a binary without a trace (`NO TRACE` rc=1) and holds it out of the battery until step 3 gives it a number that can be red.

## 3. Measurements the design rests on

| what | number | how |
|---|---|---|
| `em(insns, n` call sites in `bebop.bp` | 553 lines (555 occurrences) | `grep -c` / `grep -o \| wc -l`; the row and research say 532 |
| sites whose first argument is an integer literal (the fixed part of the word) | 538; **308 distinct** base words | regex `em\(insns, n, (\d+)` |
| `fn emit_*` functions | 105; 58 contain `em(` calls directly | `grep -c "^fn emit_"`; per-fn count by awk |
| emit_* with <= 20 static `em(` sites | 53 of 58 | the five over: `emit_sys_scan` 47, `emit_sys_rename` 33, `emit_sys_export` 29, `emit_sys_readbuf` 26, `emit_hvham2` 25 -- all builtin bodies, straight-line svc/NEON sequences |
| entry stub | 172 words | `bebop.bp:7139` |
| positive constructs / frozen binaries | 100 / 100; 83,724 words total, largest 10,750 | `ls`; `stat -c %s frozen/*.bin` |
| negative constructs | 20 | `ls bench/parity_constructs/neg` |
| `bebop.bin` `072c01d1` | 44,970 words in file; code_end 44,020; entry word 44,798 | `tools/check_abi.py load_bin` |
| constructs calling any `sys_*` | 8 of 100 (2 use clone/run/wait4) | `grep -l` |

A static count of `em(` sites per function is an upper bound on the words one call emits only where the function has no loop around `em`; it is a proxy for the row's "<= ~20 words", not a measurement of fragment lengths. Step 2 measures them from the trace.

## 4. Design

### 4.1 The trace is a side FILE, not a zone inside the `.bin` (deviation from the row's wording, with the reason)

The row says "into a side zone of the `.bin` (A10's reloc zone)". Three facts argue for `<out>.trace` next to `<out>.bin` instead, the way `use_expand` already writes `<out>.use` (`bebop.bp:7097`, written with `sys_export` at `:7110`):

- **A10 was REFUTED** (ROADMAP.md:97) and its reloc zone was never built: `grep -n reloc bebop.bp` hits only register-window relocation comments (`:224`-`:3907`). The research's "design together" (`RESEARCH-VERIFICATION-2026-09-09.md:297`) has nothing to design with.
- **The row's own gate forbids changing the bytes.** "fixpoint md5 unchanged by the zone" and `trace_zone_words: 0` are literal about the `.bin`. A zone inside the file changes every md5: the 100 frozen construct binaries compared byte-for-byte by `construct_parity.sh:41` (`cmp -s`) would all need re-freezing, `word_budget.txt` lines would be demanded for growth the census does not even count, and gen3/gen4 digests would move. A side file leaves every existing artifact and gate untouched.
- **The seed must not learn a format.** `seed/seed.S` is the trust root (`docs/TRUST-CHAIN.md`); it reads the footer and jumps. A zone it skips is a zone it must know about.

Cost of the choice: one more file per compile, and the validator takes two paths. Format (little-endian i64 cells, written like `.use` through the byte-per-cell IO path): a header `(magic, version, md5 prefix of the `.bin` as 8 hex chars so a trace cannot be replayed against a different binary -- the 15.1 binding rule of ROADMAP.md:200 applied here, count)`, then per fragment `(pos, first, last, S, csmask, window depth d, d x (kind, reg_or_slot, const), same four after, patch count p, p x (index, value))`.

### 4.2 Coverage is a partition, and it is the first number

Fragments must tile every code word of every function exactly once: `[first, last)` ranges contiguous, non-overlapping, and together equal to `[fn start, fn end)`. Words emitted by prologue/epilogue (`emit_prologue` `bebop.bp:4622`, `emit_epilogue` :4637 and the `_sized` pair :4677/:4704) are fragments of their own. The gate `trace_cover: 100 %` is checked by the validator before anything semantic, and a construct at 99 % is RED. Without this, the validator cannot know that the words it did not see are harmless.

### 4.3 The trace carries the window STATE, not only a digest (deviation from the row, with the reason)

The row records "window-before digest, window-after digest". A digest supports only equality. Translation validation needs, for each fragment, the mapping from window entries to registers/slots/constants at entry (to interpret the words) and at exit (to know what the emitter claims the words achieved). Recording the state itself lets the validator check LOCAL correctness -- given W0, these words yield W1 and implement the construct -- without replicating the allocator's decisions, which would otherwise be a second compiler. The digest is kept as a compact identity only: the validator recomputes it from the state it reads, so a trace writer and a trace reader that disagree on the window encoding fail loudly on the first fragment.

### 4.4 The decoder: a table derived from the emitter, verified by the L1 recipe

The row's "inventory = check_words.py's allowlist" does not exist: `tools/check_words.py:1-6` verifies NEW `em()` literals in a diff against an `objdump` listing the author produced first; it holds no allowlist (`:20` is its only regex). The inventory is instead derived: the 308 distinct base words at the 538 literal `em(` sites, each classified once by hand against `as`/`objdump -d` exactly as `check_words.py` demands of every new word (its L1 recipe), and committed as `tools/tv_inventory.txt` (`<base word hex> <mnemonic> <which bit fields are parametrised: rd/rn/rm/imm>`). `decode_cover: 308/308 unknown=0` is step 0's gate, and `check_words.py`'s existing objdump discipline means every future `em` literal arrives with the listing the table needs. The `vs_*` helpers that compose a word from a base plus register fields (`vs_array_get_imm`, `bebop.bp:4154`: `2432696320 + idxc * 1024 + base * 32 + d`) are the parametrisation the table records.

### 4.5 The validator: per-fragment symbolic execution over the register model

`tools/tv_fragments.py` (rewritten) reads `<out>.trace` and `<out>.bin`, checks §4.2, then for each fragment: decodes the words (§4.4), applies the patches, builds a symbolic machine state from W0 (registers x0-x28 as terms over the window's symbolic values, x17 the cell base, x27/x28 the arena cursor/end, x14 the frame, x15 the stack), executes the words symbolically, and checks that the resulting state agrees with W1 and with the construct's rule. The construct's rule comes from the same AST F3's reader uses (`tools/bpref.py --dump-ast`, F3 §4.1), so `arrGet` means what `Semantics.lean` says it means: cell index `base + idx`, load `x17 + 8 * (base + idx)`. Loops (`emit_while_stmt`) are validated as diamonds: entry test, body as a sequence of already-validated fragments, back-edge, with the cs mask asserted 0 at entry as the compiler itself asserts. Calls (`emit_bl` :651, `emit_bl_call` :830) validate the argument placement in x0..x13 and the `bl` target against the layout table, not the callee.

Fragments that carry an `svc` (the `emit_sys_*` bodies, the five longest emitters in §3) are validated for register plumbing only -- the syscall's effect is F3's axiom -- and reported as `plumbing-only`, named, in the gate line.

### 4.6 The residual: peepholes to QF_BV, certificates through F6

Four sites replace a canonical sequence with a shorter one whose equivalence is a bit-vector fact, not a syntactic one: `vs_try_ubfx` (`bebop.bp:3147`), `vs_try_and_imm` (:3188), `vs_try_madd` (:3276), `emit_cond_csel` (:3801). For these the validator emits an obligation `canonical(W0) == emitted(W0)` over 64-bit terms in the obligation DAG form `tools/certgen.py` consumes, obtains a certificate in the format of `docs/CERTIFICATE-FORMAT.md:1-14` (text normative, `x <oid>` binding), and checks it with `tools/certcheck.py` -- and, when `tcheck.bp` covers the operators, with the Bebop checker (ROADMAP.md:200 measured `shl(x,1) == x + x` at 709 hinted steps). Gate `tv_residual: r/r`. The residual is per-SITE, not per-fragment: one certificate per (site, operand shape), cached by the obligation's sha256, so the count is small and stable.

### 4.7 Python first, Lean as the cross-check

Phase F's header (ROADMAP.md:191) demotes Lean to a cross-check oracle. The validator is therefore Python (untrusted, like `certgen.py`) and gives the row its number first; the "Lean semantics of the emitted AArch64 subset" is step 6, a second implementation of §4.4-4.5 that must agree with the first on every verdict, and it cannot start before F3 step 0 makes `formal/` build. What stays trusted regardless: `seed/seed.S`, the trace WRITER inside `bebop.bp` (it is the VC generator of this row; a writer that lies about W1 is undetectable by construction -- the same standing as F6's VC generator, ROADMAP.md:200 "what stays trusted REGARDLESS"), and the inventory table.

### 4.8 A chain gate, not a compile-time one

The trace is written on every compile (cost gated by `K5`, §5 step 1); validation runs in `tools/chain.sh` after the battery, over the constructs (100 traces, 83,724 words) and, at step 5, over gen4's own trace (44,020 code words). Never inside `bebop.bin`.

## 5. Gates

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| 0 | honest gate + inventory | `tv_fragments.py` refuses a binary with no trace (`NO TRACE` rc=1); `tools/tv_inventory.txt` from the 308 base words, objdump-verified | `decode_cover: 308/308 unknown=0` (denominator re-derived from `bebop.bp` on every run) | standalone; NOT wired (F0/F1 precedent, ROADMAP.md:194-195) |
| 1 | trace writer | `bebop.bp`: `tr_open(pos)`/`tr_close` around ~30 construct-level emitters; `<out>.trace` written like `.use` | `trace_cover: 100 %` on all 100 constructs; `.bin` byte-identical (all 100 frozen `cmp -s` pass, no `word_budget` line); `trace_zone_words: 0`; fixpoint gen3 == gen4 unchanged; `K5 <= +3 %` self-compile | all five; bebop.bp lane, after F2 (ROADMAP.md:196 changes the fragments) |
| 2 | fragment statistics | `tv_fragments.py --stats` | `frag_max_words`, `frag_p99`, `frag_zero_word` printed per construct and for the self-compile | numbers, no threshold; the row's "<= ~20" becomes measured |
| 3 | validator | §4.5 | `tv_fragments: v/t plumbing_only=k` over the 120 constructs, `v` ratcheted in the script (`tools/f8_dt.py:24` precedent) | end v + k = t; wired into `battery.sh` at the first v > 0 |
| 4 | residual | §4.6 | `tv_residual: r/r` certificates checked by `certcheck.py` (and `tcheck.bp` where covered); F6's `checker_neg` unchanged | r/r |
| 5 | self-compile | validator over gen4's trace | `tv_fragments: v/t` on `bebop.bin`; `bounds_census` re-derived from the trace equals F2's static census (ROADMAP.md:197: "static census first, F4's trace later") | equal counts, both printed |
| 6 | Lean cross-check | `formal/Bebop/AArch64.lean` + fragment rules | `tv_lean_agree: v/v` on step 3's verdicts | depends on F3 step 0 |

## 6. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| `bebop.bp:em` | unchanged; it is the choke point the trace measures around | `bebop.bp:2276` |
| `bebop.bp` new `tr_open`, `tr_close`, `tr_write` | record `(pos, n[0], window, csmask, S)` at entry/exit; write `<out>.trace` after the data section | window top `:4137` (`fntab[5387]`), cs mask `:2292` (`fntab[5389]`), entries `:2813` (`fntab[3700..4335]`); writer shape `use_expand` `:7097-7110` |
| `bebop.bp:patch_b` | also append `(p, target)` to the open fragment's patch list | `bebop.bp:2011` |
| `bebop.bp` construct emitters (`emit_array_get` :4136, `emit_array_set` :4166, `emit_cond_csel` :3801, `emit_while_stmt` :5292, `emit_let_stmt` :5466, `emit_bl_call` :830, `emit_enum_ctor` :586, `emit_match_rt` :2086, `emit_zeros` :6733, prologue/epilogue :4622/:4637/:4677/:4704, the `emit_sys_*` bodies) | bracket with `tr_open`/`tr_close` | each `fn` line verified by `grep -n "^fn emit_"` |
| `bebop.bp:compile_program_offs` | write the trace after the data section is placed | `bebop.bp:6420`, data section `:6522` |
| `tools/tv_fragments.py` | rewritten: refuse-no-trace, trace reader, inventory decoder, coverage, symbolic executor, residual emitter; `--stats` | today's `:121` marker, `:184` deferral, `:207-252` digest check, `:271/:278` vacuous PASS are all deleted |
| `tools/tv_inventory.txt` (new) | 308 base words, mnemonic, parametrised fields, objdump-verified | derived from `em(insns, n, <lit>` sites |
| `tools/check_words.py` | unchanged; its objdump recipe is what keeps the inventory current | `check_words.py:1-6, :20` |
| `tools/certgen.py`, `tools/certcheck.py` | consumers of the residual; unchanged unless an operator is missing | exist (`ls`); format `docs/CERTIFICATE-FORMAT.md:1-14` |
| `tools/chain.sh` | one validator step after the battery, behind the same slot | `chain.sh:39-47` (battery + fixpoint) |
| `tools/battery.sh` | one `line` for `tv_fragments` at step 3 | `battery.sh:57-70` |
| `formal/Bebop/AArch64.lean` (new, step 6) | subset semantics for the inventory's mnemonics; fragment rules | none yet; blocked on F3 §5 step 0 |
| `docs/REGISTER-MODEL-BLUEPRINT.md` | correct the cs-mask slot | `:99` says `fntab[3823]`; code says `fntab[5389]` |
| `docs/TRUST-CHAIN.md` | name the trace writer as trusted and the validator/inventory as producers | precedent `:158-172` |

## 7. Cost and schedule (the row says 6-10 weeks; this says 7-10 without the Lean half, 9-13 with it)

| step | estimate | reasoning |
|---|---|---|
| 0 honest gate + inventory | 2-3 days | 308 words to classify against objdump listings; most are already commented in `bebop.bp` (e.g. `:4149`, `:6743-6757`) |
| 1 trace writer | 1-2 weeks, bebop.bp lane, serial after F2 | ~30 bracket pairs plus a ~100-150-line writer (the row's 50-100 lines did not include dumping the window entries, §4.3); every change re-frozen by `cmp -s` at zero word delta |
| 2 statistics | 2 days | reading the trace |
| 3 validator | 3-4 weeks | symbolic executor over ~15 mnemonics and the register model; the rules are F3's AST; the diamond for `while`; 120 constructs as the corpus |
| 4 residual | 1 week | four sites; `certgen.py`/`certcheck.py` exist and are measured (ROADMAP.md:200) |
| 5 self-compile | 1 week + unmeasured runtime | 44,020 words; the fragment count is unknown until step 2 (§8) |
| 6 Lean cross-check | 2-3 weeks, parallel, after F3 step 0 | a second implementation of steps 0 and 3 in Lean, compared on verdicts |
| **total** | **7-10 weeks (steps 0-5), 9-13 with step 6** | |

Calibration (`docs/RESEARCH-VERIFICATION-2026-09-09.md:271`): the tree's lane-day velocity applies to step 1 (a `bebop.bp` codegen-adjacent slice) and not to steps 3 and 6.

**What must land first:** step 0, then step 1. Nothing semantic can be measured until a trace exists, and a validator written before the trace format is fixed would be written twice. Step 1 waits for F2 because F2's bounds checks add words to exactly the fragments (`emit_array_get`/`emit_array_set`) the validator learns first.

**What is NOT proposed:** proving the emitter or any pass of it (there is no pass); whole-function or inter-fragment validation (callee behaviour is the callee's fragments); validating the seed or the entry stub semantically (the stub is 172 fixed words, checked by identity against the frozen bytes); a semantics of syscalls (F3's axioms; `svc` fragments are plumbing-only); the 7 threading builtins; a zone inside the `.bin`; running the validator at compile time; wiring `tv_fragments` while it can only print PASS.

## 8. Speculative, labelled

- **Fragment count and trace size.** If the mean fragment is 4-6 words, `bebop.bin` has ~7-11k fragments and a trace of ~20 cells each, ~1-2 MB. SPECULATIVE until step 2.
- **`K5` cost of an always-on trace.** The writer runs on every compile; if it costs more than 3 % of the self-compile it becomes opt-in (`compile --trace`), which the compiler's CLI does not have today. Unmeasured.
- **Adequacy of the diamond model for `while`.** A2b (`bebop.bp:5188-5192`) releases and re-takes an enclosing loop's hoisted cs registers around a nested loop; whether "cs mask 0 at entry" plus per-fragment validation is enough to validate that re-take, or whether the hoist needs its own fragment kind, is not known.
- **The number of mnemonics.** "~15" is a guess from the 308 base words' comments; the inventory of step 0 is the measurement.
- **Symbolic-execution cost at 44k words.** Unmeasured; the constructs (max 10,750 words) are the first datum.
- **Whether `tcheck.bp` covers the residual's operators** (ubfx/and-imm/madd/csel over 64 bits) today: ROADMAP.md:200 lists add/sub/logic/shifts/compares/mux/mul; `madd` and `ubfx` are compositions of those, unverified here.

## 9. Stale or contradicted in the citing row (ROADMAP.md:199), for the operator

- "A10's reloc zone": A10 is REFUTED (ROADMAP.md:97) and no reloc zone exists in `bebop.bp` (§4.1).
- "inventory = check_words.py's allowlist": `check_words.py` has no allowlist (§4.4).
- "532 emission sites": 553 `em(` lines / 538 with a literal / 308 distinct base words at `ddb534f`.
- "over the 86 constructs": 100 positive + 20 negative today.
- "window-before digest, window-after digest": a digest cannot be symbolically executed against; the state is needed (§4.3).
- "~50-100 lines" for the writer: 100-150 once the window entries are dumped (§7).
- Research §9 anchors (`RESEARCH-VERIFICATION-2026-09-09.md:246`) are all stale: `emit_array_get` 3937-3963 -> `:4136`; `emit_bl` 627-646 -> `:651`; `emit_enum_ctor` 562-599 -> `:586`; `emit_match_rt` 1976-2025 -> `:2086`.
- `docs/REGISTER-MODEL-BLUEPRINT.md:99`: cs mask at `fntab[3823]`; the code says `fntab[5389]` (`bebop.bp:2292`).
- `F_PHASE_STATUS.md:136` "Evidence: None on disk": `tools/tv_fragments.py` exists and is vacuous (§2).
- Found in passing, not this row: the F8 row (ROADMAP.md:202) cites `scan_inert` at `bebop.bp:4503`; it is at `:4554` (`:4503` is an `em(` line).

## 10. Attribution and evidence

- Citing row ROADMAP.md:199 (F5); read with :97 (A10), :191 (Phase F header), :194-197 (F0-F3), :200 (F6), :202 (F8).
- Research: `docs/RESEARCH-VERIFICATION-2026-09-09.md` §9 (:244-250), §10 F4 rung (:262), :297 (A10 note), :271 (calibration).
- Existing artifact: `tools/tv_fragments.py` (`4286ec1`), read in full; its decoder run on five documented words (§2 item 5).
- Emitter anchors: `grep -n "^fn <name>" bebop.bp` for every function named in §1, §4, §6; `sed -n` for :2276, :2292, :2813, :4137-4158, :5170, :5188, :6522, :7118, :7139.
- Layout: `seed/seed.S:2,50`; `tools/check_abi.py:62`; `tools/census.py:8,22-25`; `python3` over `load_bin('bebop.bin')`.
- Counts: `grep -c`, `grep -o`, awk over `bebop.bp`; `stat` over `bench/parity_constructs/frozen/*.bin`; `ls` over constructs.
- House rules applied: hold a gate out rather than wire a vacuous PASS (ROADMAP.md:194-195, :202); ratchet in the script (`tools/f8_dt.py:24`); L1 objdump recipe (`tools/check_words.py:1-6`); labelled speculation (`docs/blueprints/A25-textual-rewriter.md` §3.2 precedent); certificate binding by obligation hash (ROADMAP.md:200, `docs/CERTIFICATE-FORMAT.md`).
- Not verified here: `selfhost/tcheck.bp`'s operator coverage; any runtime of the validator; the Lean side (blocked on F3 §2).
