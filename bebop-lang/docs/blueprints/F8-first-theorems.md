Status: 2026-09-13, written by lane bp78 at base `ddb534f` for ROADMAP row **F9** (`ROADMAP.md:203`), which cites this file. Measured against `bebop.bin` `072c01d1` and `tkernel.bin` `f48daac5`. This row is about theorems nobody has proved; §2 separates what is DESIGNED (a statement exists, the discharge route is named in a document) from what is HOPED (no statement exists in any checkable language yet). Numbering: row F9's "after F5-F7" and "depends on F7" use blueprint letters (see `F7-dependent-types.md` §1); in row letters they read F6 (certificate checker), F7 (kernel), F8 (dependent types).

# F8 First theorems: machine-checked and certificate-checked

## 0. Goal

`theorems: >= 3`, then growing, each with (i) a term the Bebop kernel (`selfhost/tcheck_kernel.bp`) accepts and (ii) a certificate the Bebop certificate checker (`selfhost/tcheck.bp`) accepts, bound to the obligation by sha256 (`docs/CERTIFICATE-FORMAT.md` §3.5 `x` line; 15.1). Candidates named by the row: `fp_mul(a, b) = sign * floor(|a| * |b| / 2^32)`, `isqrt(s)^2 <= s < (isqrt(s)+1)^2`, the money laws of `money.bp` against the Rust oracle, the store invariants (`st_len`, cursor monotone, crc). And `samples/theorem-sample.bp` / `theorem-false.bp` retired or re-homed.

The row's gate says "an LRAT certificate". The Phase F header (`ROADMAP.md`, "CERTIFICATE FORMAT DECIDED 2026-09-09: our own, end to end. 11.3 a custom minimal format, NOT LRAT") and row F6 (`:200`, "the LRAT-over-DRAT argument survives the format being ours") say the certificate is the hinted text format of `docs/CERTIFICATE-FORMAT.md`, produced FROM cadical's LRAT by `tools/certgen.py`. This file reads the gate as "a certificate in the tree's format"; the word LRAT in the row is stale.

## 1. What exists today, measured

### 1.1 Statements: seven Lean axioms, zero theorems

`formal/Bebop/Theorems.lean` states the row's four families as `axiom`s -- `fp_mul_correct` (`:105`), `isqrt_correct` (`:169`), `addov_correct` (`:220`), `mulov_correct` (`:231`), `st_len_invariant` (`:287`), `cursor_monotone` (`:295`), `crc_consistent` (`:305`): `grep -c '^axiom'` = 7, `grep -c '^theorem'` = 0. Its header (`:4`) says the file "states and proves"; it proves nothing, and `docs/VERIFIED-STATE-2026-09-12.md:58` already records "7 items as Lean axioms, 0 proved theorems". Lean is not in the trust root (row F7, 13.3) and does not run on this box (row F4), so even a proved version would be a cross-check, not a theorem in the row's sense. **Designed: the four statements exist in a precise language. Hoped: every proof.**

### 1.2 The kernel: sound on 18 terms, but no theorem can be STATED in it

`kernel_parity: 18/18`, `kernel_neg: 0 accepted of 14`, `kernel_internal: 0 of 18` (`tools/kcheck.py --corpus bench/kernel_neg` with `TKERNEL_BIN` set; `F7-dependent-types.md` §2). The calculus has six node kinds -- `sort`, `var`, `pi`, `lam`, `app`, `const` (`kcheck.py:34-41`; tags 1-6 at `tcheck_kernel.bp:32`) -- and three declaration forms plus `check` (`kcheck.py:99`). There is **no equality type, no `I64`, no literal, no primitive operator, no recursor**. `fp_mul(a, b) = ...` is not a `.core` term today. The design study plans `I64 : Type 0` with the machine operators as primitive constants (6-10 fns, `docs/RESEARCH-PROOF-DESIGN-2026-09-09.md` §4 "number type") and `Eq` as an inductive family (§4 "inductive families"), i.e. it needs K-ι of `F7-dependent-types.md` §5. And the kernel's conversion is structural (`tcheck_kernel.bp:94-97`): a `def` cannot be unfolded, so a theorem about `fp_mul` could not be checked against `fp_mul`'s body even once it can be stated (probe g1 there: twin ACCEPTED, kernel 26).

### 1.3 The certificate layer: real, hand-tested, gated nowhere

`selfhost/tcheck.bp` (28 fns; contract at `:1-18`; caps at `:22-28`: 8192 vars, 16384 clauses) and `tools/certcheck.py` agree 8/8 on the hand-written corpus of `docs/F6-COMPLETENESS.md`, including the replay rejection (OID MISMATCH). Two limits from that document's own "Limits" section still hold: CaDiCaL is absent on this box, so the producer path `certgen -> cadical -> certcheck` is untested here; and `tcheck.bp` holds the certificate layer ONLY. Measured additions:

- **No gate.** `grep -n "tcheck\|certcheck\|checker_neg\|cert_checked" tools/battery.sh tools/chain.sh` returns nothing. Row F6's gate names (`cert_checked`, `checker_neg`) are not printed by any script in the tree.
- **The producer's operator set is 10 ops**: `var`, `const`, `and`, `or`, `xor`, `not`, `add`, `shl`, `lshr`, `eq` (`tools/certgen.py:166-193`). No `sub`, `mul`, `sdiv`, `srem`, no ordered compare, no `mux`. Row F6 (`:200`) says "bit-blaster for 64-bit add/sub/logic/shifts/compares/mux/mul"; the producer in the tree has add, logic, shifts and `eq`. `fp_mul`, `mulov`, `isqrt` and the `op_taxx`/`op_taxi`/`op_eur` divisions cannot be bit-blasted by anything in the tree today.
- **No committed obligations**: `find . -name '*.obl' -o -name '*.cert'` (non-attic) is empty; the obligation grammar is `obligation <name>` / `prove <id>` / numbered nodes (`certgen.py:100-112`).
- **No joint step.** `tkernel.bin` and `tcheck.bin` are separate programs with no shared cell; the `sat <n>` oracle step by which a term closes a goal through a certificate (`RESEARCH-PROOF-DESIGN-2026-09-09.md` §8.2) exists in neither. "Kernel-checked AND certificate-checked" is today two independent verdicts on two files with no binding between them.

### 1.4 The surface: `theorem` is lexically validated and semantically inert

F8 step 0 (`ddae413`, `1e51e48`) made `collect_fns` recognise a top-level `theorem ` line and validate it to end of line with `scan_inert` (`bebop.bp:6068-6100`; validator at `:4554`; exit 110, text at `:105`, row `docs/TRAPS.md:34`). Measured on the two SUPERSEDED samples with the promoted compiler, output to the scratchpad:

| file | line | result |
|---|---|---|
| `samples/theorem-sample.bp` | `:9` `theorem plus_one : (\x:i64. x + 1)(4) = 5 := refl` | **rc 110 at 9:21** (`\` refused) |
| `samples/theorem-false.bp` | `:7` `theorem bad : 1 + 1 = 3 := refl` | **rc 0** -- a false theorem compiles |

Both files carry `Status: 2026-09-04 SUPERSEDED` on line 1. `bench/VERIFICATION.md` (itself marked SUPERSEDED at `:3`) documents the C-era kernel they targeted: three `refl` identities on closed `i64` terms, an induction rule that "accepts any input". Nothing in the tree proves or refutes either file today; the compiler accepts the false one.

### 1.5 The evidence the row leans on, located

| claim | where | what it is |
|---|---|---|
| `isqrt` "tested on 1.2M values" | `selfhost/prelude/fp.bp:31-34` (comment; `isqrt` at `:35`) | bit-exact against `math.isqrt` on 1.2M values incl. every `2^k+-1` and `r^2+-1`, scratch script not in tree |
| `fp_mul` spec | `fp.bp:4-6` (comment; fn at `:6`) | signed 64x64 -> Q32 via 32/16-bit limbs |
| money laws vs Rust oracle | `selfhost/std/money.bp:3-6`; `bench/oracles/rust/src/bin/money.rs` (exists) | production Rust on the same case table; `addov` `:28`, `subov` `:29`, `mulov` `:30` |
| `eqc_gen` discipline | `/root/dowiz/crates/dowiz-core/src/eqc_gen.rs` (88 lines), `/root/dowiz/kernel/src/eqc_gen.rs` (7-line re-export) | existence only, as the 2026-09-09 study also said (`RESEARCH-VERIFICATION-2026-09-09.md:298`) |
| store invariants | `selfhost/prelude/store.bp`: `st_crc:66`, `st_alloc:328`, `st_len:340`, `st_alloc_p:570`, `st_copy_obj:1066`, `st_compact:1117` | **no gate today**: `bench/vs_rust/invariants.sh` checks register zones, branch census, fntab map, footer identity, expansion identity (`:2-7`) -- nothing about `st_len` or the cursor; only `selfhost/std/sevolve.bp` references `st_crc`/`st_len` outside the prelude |

## 2. The theorems, one by one: designed vs hoped

Discharge routes and risks are the design study's (`RESEARCH-PROOF-DESIGN-2026-09-09.md` §10 table), re-read against §1.3's operator set.

| # | theorem | statement shape | needs from the kernel | needs from the certificate layer | status |
|---|---|---|---|---|---|
| T1 | `addov(a,b) = 1 <-> a+b overflows` (`money.bp:28`) and `subov` (`:29`) | straight-line over `^ & + <`; `x < 0` is bit 63, expressible with `lshr 63` + `eq` | `I64`, `Eq`, `sat` step | **the 10 ops suffice** (`add`, `xor`, `and`, `lshr`, `eq`) | DESIGNED; the cheapest first theorem in the tree |
| T2 | `op_add`/`op_sub`/`op_neg`/`op_nonneg` (`money.bp:43-105`) vs the oracle's semantics | `st` as update chains, adders, comparators | as T1 | needs `sub`, ordered compare, `mux` in `certgen` | DESIGNED; producer ops missing |
| T3 | `op_taxx`/`op_taxi`/`op_eur` (`:75-95`) | `sdiv` by 10^6 / 10^9 constants | as T1 | needs `sdiv` (constant divider ~4k gates each, study §10) | DESIGNED, unmeasured |
| T4 | `fp_mul` = `sign * floor(|a||b| / 2^32)` (`fp.bp:6`) | limb identity vs a 128-bit product | `ring` (8-12 fns, study §5.1) + nested-floor lemma | `mul` in `certgen`; the multiplier miter is the hard SAT case (row F6) | HOPED: neither route exists |
| T5 | `mulov` (`money.bp:30`) | 64x64 mul AND 64-bit div vs a 128-bit product | `ring` + two floor lemmas | `mul`, `sdiv` | HOPED |
| T6 | `isqrt` bracket for `0 <= s < 2^62` (`fp.bp:35`) | Newton loop, <= 6 iterations, invariant `x >= floor(sqrt s)` | `induct` over the loop = recursors (K-ι) + `ring` + division lemmas | `sdiv` | HOPED; needs eliminators |
| T7 | `st_len` bounds, cursor monotone, crc-on-copy | `Cells` as `I64 -> I64`, interval arithmetic, syscall footprints as axioms | K-ι, the fixed axiom table (`deps`), `induct` | per-iteration `sat` | HOPED; not even a test today (§1.5) |

**So `theorems: >= 3` is reachable from T1 and T2 alone -- `addov`, `subov`, `op_add`** -- before any `ring`, any multiplier and any eliminator, provided the three joints of §3 exist. The flagship theorems in the row's first sentence (`fp_mul`, `isqrt`) are the LAST to land, not the first, and the row should not be read as promising them in 3-5 weeks.

## 3. The three joints that do not exist, in the order they are needed

1. **Statement language in the kernel** (K-ι + `I64` + `Eq`): `theorem` bodies must elaborate to a `.core` `check` line whose type is `Eq I64 (app (const addov) ...) (lit 1)`-shaped. Until `Eq` and `I64` exist the gate's numerator is 0 by construction. Owner: the kernel lane (`F7-dependent-types.md` §5 K-ι, then the study's K6).
2. **The `sat` oracle step**: a `.core` declaration form `sat <term> : <cert-path>` (or an `oracle` node) under which the kernel accepts a `Prop` iff `tcheck.bp`'s check of the named certificate, bound by `x` line to the canonical text of the obligation the kernel prints for that term, succeeds. Design choice, open: one binary (`tkernel.bp` `use`s `tcheck.bp`; both are under their own 511 cap and the study's total is 137-206 fns, `:5.1`) or two binaries with the kernel recomputing the obligation sha256 and reading `tcheck`'s verdict file. The single-binary form is what "the certificate checker and the dependent type checker are the SAME trusted program" (`RESEARCH-VERIFICATION-2026-09-09.md` §8.1) prescribes. SPECULATIVE until written.
3. **A gate that runs both**: `theorems: k/N` printed by a script that, per theorem directory `bench/theorems/<name>/` holding `stmt.core`, `stmt.obl`, `proof.cert`, runs `tkernel` and `tcheck`, requires verdict 0 from both AND the `x` line to match, and counts. `theorem_neg`: a corrupted certificate or a false statement under the same script must count 0. Neither `battery.sh` nor `chain.sh` runs `tcheck` today (§1.3), so this gate is also the first gate the certificate layer has.

## 4. Retiring `theorem-sample.bp` / `theorem-false.bp`

Both are already SUPERSEDED on their first line and reference "ROADMAP T85", a row that no longer exists in `ROADMAP.md` by that name. Plan: (i) when surface step S-a lands (`F7-dependent-types.md` §5), `theorem-false.bp` becomes construct `neg/c145_theorem_false.bp` expecting `COMPILEFAIL:<new code>` with `line:col`, next to `c143_contract_garbage.bp` / `c144_contract_ok.bp`; (ii) `theorem-sample.bp`'s three `refl` identities are re-stated as `bench/theorems/` entries T0a-T0c whose proofs are `reduce`-shaped (study §8.2) -- they are the smallest end-to-end exercise of joint 1-3 and should be the FIRST numbers on the gate, before T1; (iii) `samples/` is compiled by no gate (row F8's control arm), so nothing is lost by deleting the two files after (i)-(ii). Until S-a lands, `theorem-false.bp` compiling to rc 0 stays the measured statement that a false theorem is accepted -- record it, do not hide it by deleting the file first.

## 5. Gates

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| 0 | joint 3 script + T0a-T0c (`refl` identities re-homed) | `tools/theorems.py`, `bench/theorems/` | `theorems: 3/3` closed identities; `theorem_neg: 0 accepted of 2` (false statement; corrupted certificate) | success |
| 1 | joint 1 (`I64`, `Eq`, literals) + joint 2 (`sat` step) | kernel + `tcheck.bp` | `theorems: 3/3` re-run through the oracle path; `kernel_parity: N/N` unchanged | success |
| 2 | T1 `addov`, `subov` | `bench/theorems/addov`, `subov` | `theorems: 5/5`; certificates check under `certcheck.py` AND `tcheck.bp`; size and `tcheck_ms` committed | success |
| 3 | `certgen` gains `sub`, compares, `mux`; T2 `op_add`/`op_sub`/`op_neg`/`op_nonneg` | `tools/certgen.py`, `bench/theorems/` | `theorems: 9/9`; `checker_neg: 0/N` on corrupted certificates of each | success |
| 4 | `certgen` gains `sdiv` by constant; T3 | | `theorems: 12/12` or a measured timeout per obligation (the study's pre-measurement: bit-blast the miter, commit the number) | success or a number |
| 5 | `ring` + floor lemmas; T4 `fp_mul`; T5 `mulov` | kernel | `theorems: 14/14` | HOPED |
| 6 | K-ι + `induct`; T6 `isqrt`; T7 store | kernel | growing | HOPED |

The row's `theorems: >= 3` is met at step 0 by the re-homed identities and at step 2 by theorems about `money.bp`; the row's named flagships are steps 5-6. Every entry of `bench/theorems/` carries `obl-sha256`, `rules-sha256` and `deps` (study §8.2) so that a theorem's axioms are listed, never discovered.

## 6. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| `selfhost/tcheck_kernel.bp` | `I64`, literals, primitive operators as constants; `Eq` after K-ι; a `sat`/oracle rule | new tags after 6 (`:32`); constant table `k_def_add:170` |
| `selfhost/tkernel.bp:kp_decl` | parse `theorem <name> : <id> := <id>` and `sat <id> <cert>` declaration forms | `:165-212` |
| `selfhost/tcheck.bp` | callable as a function of `(cert, obl)` from the kernel driver, or a verdict file the driver reads | `:1-18` (usage), `check_chain:219`, driver call at `:305` |
| `tools/certgen.py` | `sub`, `slt`/`ult`, `mux`, `sdiv`-by-constant, later `mul` | `:166-193` |
| `tools/certcheck.py` | unchanged; stays the differential twin | header |
| `tools/theorems.py` (new) | joint 3: run both checkers per `bench/theorems/*`, print `theorems: k/N`, `theorem_neg`, `checker_neg` | -- |
| `tools/battery.sh` | assert `^theorems:` and `^theorem_neg:` | next to `:64-65` |
| `formal/Bebop/Theorems.lean` | header `:4` "states and proves" -> "states"; each axiom gains the `bench/theorems/` name it corresponds to | `:105,:169,:220,:231,:287,:295,:305` |
| `samples/theorem-sample.bp`, `theorem-false.bp` | retired per §4 | `:1` status lines; `theorem-false.bp:7` |
| `bench/parity_constructs/neg/c145_theorem_false.bp` (new) | the false theorem as a COMPILEFAIL construct once S-a lands | beside `c143_contract_garbage.bp` |

## 7. Cost and schedule

The row says 3-5 weeks "after" the kernel, certificate checker and dependent types. Re-costed by joint: joint 3 is a ~150-line Python script and a directory convention (days); joint 1 is the study's K6 (1 step) plus `Eq` from K-ι (part of 6 steps); joint 2 is the study's `sat` oracle inside the trusted program (K7, 10 steps, of which the bit-blaster and hinted resolution are ALREADY in `tcheck.bp` -- the remaining cost is the binding between kernel and checker, unmeasured); `certgen` ops are Python, days each; `ring` is 8-12 fns (K8). Steps 0-2 of §5 are reachable in the row's 3-5 weeks ONCE K-δ and K-ι of `F7-dependent-types.md` exist; steps 5-6 are not costed here because the study's own pre-measurement (bit-blast the `fp_mul`/`mulov` miters through `bv_decide` off-box and commit the size or the timeout) has not been done, and without it the choice between the SAT route and `ring` is a guess.

## 8. Speculative, marked

- Joint 2's shape (one binary or two) is undecided; the `sat` rule's soundness depends on the obligation text the kernel prints being canonical in exactly `certcheck.py`'s sense (`docs/CERTIFICATE-FORMAT.md` §2.2) -- a second implementation of `canonical_text` in Bebop already exists in `tcheck.bp` (F6-COMPLETENESS.md, "cross-validates ... to the hex digit"), which is the one to reuse.
- T1's encoding of overflow as a bit-63 test through `lshr`/`eq` is designed on paper here and not run through `certgen`.
- T3's "~4k gates per constant divider" is the study's number, unmeasured in this tree.
- The claim that `isqrt` needs recursors assumes the loop is stated by induction on the iteration count; a bounded-unrolling statement (<= 6 iterations, `fp.bp:32`) might avoid `induct` at the cost of a larger certificate -- a route worth one measurement before K-ι is on the critical path for T6.
- Everything in T7 assumes the fixed axiom table of syscall footprints (study §4 "axioms"), which does not exist.

## 9. Stale or contradicted claims in the citing row, for the record

- "an LRAT certificate": the format is the tree's own hinted text (§0).
- "`theorem-sample.bp`/`theorem-false.bp` ... their C-era kernel is a stub per bench/VERIFICATION.md": true, and additionally `theorem-false.bp` now COMPILES at rc 0 under `bebop.bin` `072c01d1` (§1.4).
- "3-5 weeks after F5-F7": the prerequisites are named by blueprint letter; in row letters F6-F8, of which the kernel lacks unfolding conversion and any statement language (§1.2) and the dependent-types row has not started.
- `F_PHASE_STATUS.md:230` places this row "at line 193"; it is `ROADMAP.md:203` today.
- The certificate-layer gate names of row F6 (`cert_checked`, `checker_neg`) are printed by nothing in the tree (§1.3).

## 10. Attribution

- Rows: `ROADMAP.md:203` (F9, this file), `:200` (F6), `:201` (F7), `:202` (F8); Phase F header (format decision).
- Statements: `formal/Bebop/Theorems.lean:105,169,220,231,287,295,305`; `docs/VERIFIED-STATE-2026-09-12.md:56,58`.
- Kernel: `selfhost/tcheck_kernel.bp:32,94-97,170`; `selfhost/tkernel.bp:165-212`; `tools/kcheck.py:34-41,99`; measurements in `F7-dependent-types.md` §2-4.
- Certificate layer: `selfhost/tcheck.bp:1-28,219,305`; `tools/certgen.py:100-112,166-193`; `tools/certcheck.py` header; `docs/CERTIFICATE-FORMAT.md` §2-3; `docs/F6-COMPLETENESS.md` (8/8 table, Limits).
- Surface: `bebop.bp:105,4554,6068-6100`; `docs/TRAPS.md:34`; `samples/theorem-sample.bp:1,9`; `samples/theorem-false.bp:1,7`; `bench/VERIFICATION.md:3`; constructs `c143`/`c144`.
- Targets: `selfhost/prelude/fp.bp:4-6,31-35`; `selfhost/std/money.bp:3-6,28-30,43-105`; `selfhost/prelude/store.bp:66,328,340,570,1066,1117`; `bench/oracles/rust/src/bin/money.rs`; `bench/vs_rust/invariants.sh:2-7`.
- Design: `docs/RESEARCH-PROOF-DESIGN-2026-09-09.md` §4, §5.1, §8.2, §10, §12; `docs/RESEARCH-VERIFICATION-2026-09-09.md` §8.1, `:298`.
- Commits verified with `git cat-file -t`: `ddb534f`, `3a61acd`, `ddae413`, `1e51e48`.
