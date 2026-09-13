Status: 2026-09-13, written by lane bp78 at base `ddb534f` for ROADMAP row **F8** (`ROADMAP.md:202`), which cites this file. Every number below was measured in this lane against the promoted binaries `bebop.bin` (digest `072c01d1`) and `tkernel.bin` (digest `f48daac5`); none is inherited from the row. The nine probe fixtures of §3-4 are NOT committed here (they turn `kernel_parity` red by construction, §3.3) -- their full text is in the appendix so the next lane can land them.

# F7 Dependent types directly in Bebop: the kernel's owed list, then the surface

## 0. Goal

Dependent types in Bebop, in the only order with a first theorem under a year (row F8): the kernel in `selfhost/tcheck_kernel.bp` closes its owed list -- conversion that unfolds, the Hurkens witness, eliminators, universe polymorphism -- and only then does the surface land: (a) `bebop.bp` parses `[T; n]`, `{x : T | p}`, `requires`/`ensures`/`invariant`, `theorem` as INERT syntax erased before emission; (b) an UNTRUSTED elaborator produces `.core` terms the kernel re-checks; (c) `let`-renaming and loop invariants over rebound registers; (d) full dependent surface types. Gates: `kernel_neg: 0 accepted` and `kernel_parity: N/N` over a corpus that grows by nine fixtures in this blueprint, `elab_neg: 0 accepted of N`, `dt_fns: n/925`, `K5_typed <= 1.5x K5`, fixpoint unchanged, every rejection with `line:col`.

**Why the kernel half comes first, in one sentence:** the kernel agrees with its twin on 18 of 18 corpus terms and disagrees on 7 of the 9 probes written for this blueprint (§3-4), two of them on the ACCEPT side -- so the parity number is a property of the corpus, not of the kernel, and a surface that elaborates into this kernel today would be checked by a checker that cannot unfold a definition.

## 1. A numbering note, because the citations in the rows depend on it

Rows F3, F4, F5, F8 and F9 in `ROADMAP.md` cite blueprint files whose letter is the row letter minus one: row F3 -> `F2-bounds-by-type.md`, F4 -> `F3-lean-semantics.md`, F5 -> `F4-fragment-validation.md`, F8 -> this file, F9 -> `F8-first-theorems.md` (measured: `grep -n "docs/blueprints/F" ROADMAP.md`; both names entered at `0ceb80b`, 2026-09-09, the commit that opened Phase F). Prose in the rows uses the BLUEPRINT numbering: row F8's "F6's kernel" is row F7's kernel; its "F4's side zone" is the trace zone of row F5 (`ROADMAP.md:199`, "a side zone of the `.bin`"); row F9's "after F5-F7" and "depends on F7" read as dependent types under blueprint numbering. This file names things instead of lettering them: **the kernel** = `selfhost/tcheck_kernel.bp` + driver `selfhost/tkernel.bp` (row F7); **the certificate checker** = `selfhost/tcheck.bp` (row F6); **the trace zone** = row F5's.

## 2. The kernel as it is today, measured

### 2.1 The corpus, both implementations, both level rules

`python3 tools/kcheck.py --corpus bench/kernel_neg` with `TKERNEL_BIN=/root/dowiz/bebop-lang/tkernel.bin`:

```
kernel_neg: 0 accepted of 14
kernel_internal: 0 of 18  (a CRASH is not a rejection and is never scored as one)
kernel_pos: 0 rejected of 4
kernel_parity: 18/18
```

Per fixture, the kernel's printed verdict (`./seed/build/seed tkernel.bin <fixture>`; the seed prints `main`'s return value, `tkernel.bp:6-7`, and the process exit code is 0 for every fixture -- a loop that reads `$?` sees eighteen zeros and learns nothing):

| fixture | pins (KERNEL-REFUTATION.md §4) | twin | kernel, `max` | kernel, `imax` |
|---|---|---|---|---|
| n01_type_in_type | `Sort n : Sort (n+1)` | rejected | 26 | 26 |
| n02_hurkens_shape | `pi` at `max`, the arm discriminator | rejected | 26 | **0** |
| n03_nonpositive_inductive | strict positivity | rejected | 30 | (not run) |
| n04_ill_typed_app | argument conversion | rejected | 24 | (not run) |
| n06_unbound_var | context bounds | rejected | 20 | (not run) |
| n07_forward_reference | node ordering | rejected | 12 | (not run) |
| n08_shared_subterm | no sharing | rejected | 14 | (not run) |
| n09_pi_domain_not_a_type | domain must be a type | rejected | 22 | (not run) |
| n10_def_type_mismatch | def body vs declared type | rejected | 26 | (not run) |
| n11_hurkens_pow | nested pi levels (does not discriminate `imax`) | rejected | 26 | 26 |
| n12_larger_id_reference | strictly-smaller ids | rejected | 12 | (not run) |
| n13_apply_non_function | head must be a pi | rejected | 23 | (not run) |
| n14_ctor_wrong_return | constructor returns its inductive | rejected | 31 | (not run) |
| n15_hurkens_full (landed `ddb534f`) | see §4 | rejected | 24 | 24 |
| p01_identity, p02_apply, p03_positive_inductive, p04_dependent_app | must be accepted | accepted | 0 | (not run) |

The `imax` column was measured only where it matters (n02, n11, n15); the row's claim that the two tables are identical except n02 is consistent with that but not re-measured on the other fifteen. The level rule is selected by a fourth argv whose first byte is `i` (`tkernel.bp:224-227`); **no gate runs the `imax` arm** -- `grep -n imax tools/battery.sh tools/kcheck.py tools/build_tkernel.sh` finds only docstring text, and `measure_kernel_parity` (`kcheck.py:443`) invokes the kernel without it. "The corpus runs twice" (row F7) is true by hand and false as a gate.

### 2.2 What the row F7 text says that the tree no longer supports

| row F7 claim (`ROADMAP.md:201`) | measured |
|---|---|
| "every one of the 16 fixtures", "0 ACCEPTED of 13", "17 fixtures", "13 negatives, 3 positives" | 18 fixtures: 14 negatives, 4 positives (`ls bench/kernel_neg`) |
| n03, n14, p03 "reported as code 71, inductives unimplemented" | 30, 31, 0 -- inductives ARE implemented (`k_check_inductive_ctor`, `tcheck_kernel.bp:270`); no fixture returns 71 |
| `kernel_parity` "hardcoded `0/%d`" | never hardcoded: `measure_kernel_parity` exists at `kcheck.py:443` and was unreachable until `3a61acd` changed the default to `./tkernel.bin` (`kcheck.py:513`); absent now prints `NOT MEASURED` and returns 1 (`kcheck.py:521-522`) |
| `tcheck_kernel.bp` "14 fns", `tkernel.bp` "10 fns" | 19 and 11 (`grep -c '^fn '`) |
| `tools/kcheck.py` "~470 lines" | 536 |
| "13 of 16 agree with the twin" | 18/18 |

One consequence for lanes: `./tkernel.bin` is gitignored (`bccbde4`) and built on demand by `tools/battery.sh:25-28` through `tools/build_tkernel.sh` -> `tools/cc.sh`, so a lane tree without a build prints `kernel_parity: NOT MEASURED` and `kcheck.py --corpus` returns 1; in a lane, point `TKERNEL_BIN` at the promoted binary.

### 2.3 What the kernel does not have, from its own source

| rule | twin | kernel | anchor |
|---|---|---|---|
| delta (unfold a `def` body) in conversion | `whnf`, `kcheck.py:199-201` | **absent**: `k_conv` is structural; the file says so -- "beta and delta are not reachable from the fixtures this step covers" | `tcheck_kernel.bp:94-97` |
| beta in conversion | `kcheck.py:202-207` | **absent** (same) | `tcheck_kernel.bp:97-109` |
| delta on the HEAD of an application | `whnf(infer f)`, `kcheck.py:259` | **absent**: the head's type must literally carry tag 4 or the verdict is 23 | `tcheck_kernel.bp:153-155` |
| constructor names become constants | `env.types[cname] = ctype`, `kcheck.py:351` | **absent**: `kp_ctor` hashes the name and drops it, "TODO: register in def table" | `tkernel.bp:155-161` |
| a constructor's type must itself be a type | `sort_of(env, [], ctype)`, `kcheck.py:338` | **absent**: `k_check_inductive_ctor` infers the ARITY's type, walks the constructor's spine for its return type and positivity, and never infers the constructor's type | `tcheck_kernel.bp:270-285` |
| fuel / depth bound | `MAXFUEL`, `kcheck.py:75,194-197`; `RecursionError` scored INTERNAL, `:402-405` | **absent**: `k_conv`, `k_shift`, `k_subst`, `k_occurs` recurse unboundedly; the only bounds are capacities (uid < 8192 -> 71, `:48-49`; 256 constants -> 71, `:172-173`; ctx depth < 256, `:30`, unchecked) | `tcheck_kernel.bp:22-30,46-56` |
| eliminators / recursors / iota | absent in both (KERNEL-REFUTATION.md §1: "459 lines bought everything except elimination") | absent | -- |
| universe polymorphism | absent in both by design (`kcheck.py:48`) | absent | -- |

## 3. Five probes that split the twin from the kernel today

Written for this blueprint, run in the scratchpad against both implementations (text in appendix A). Each is a fixture the corpus lacks; none needs a feature the calculus lacks.

| probe | what it exercises | twin | kernel `max` | kernel `imax` | class |
|---|---|---|---|---|---|
| g1 `def A : Sort 0 := B`, `check b : A` | delta in `k_conv` | ACCEPTED | 26 | 26 | positive the kernel rejects |
| g2 `check b : ((fun A : Sort 0 => A) B)` | beta in `k_conv` | ACCEPTED | 26 | 26 | positive the kernel rejects |
| g3 `inductive Nat ...` then `check zero : Nat` | constructor as a constant | ACCEPTED | 21 | 21 | positive the kernel rejects |
| g4 `inductive Foo : Sort 0 \| mk : a -> Foo` with `a : A` a VALUE | constructor type well-formed | REJECTED, "expected a type, got something of type A" | **0** | **0** | **negative the kernel ACCEPTS** |
| g5 `inductive Foo : Sort 0 \| mk : (var 7) -> Foo` | unbound var in a constructor | REJECTED, "var 7 is unbound (context depth 0)" | **0** | **0** | **negative the kernel ACCEPTS** |

### 3.1 g4/g5 are a `kernel_neg`-class hole the gate cannot see

`tools/battery.sh:64` asserts `kernel_neg: 0 accepted of 14`, and that line is computed by the TWIN (`kcheck.py:487-493,507`): `run_file` is the Python checker. A kernel that accepts an ill-formed inductive moves `kernel_parity` (`:65`) and nothing else. `3a61acd` recorded exactly this vacuity for the deleted-binary case; g4/g5 are the same defect on the accept side. The fix is one more printed line, `kernel_neg_bin: k accepted of N`, counting `n*` fixtures on which the KERNEL printed 0 (`run_kernel_on_fixture`, `kcheck.py:416`), asserted at 0 in battery next to `:64`.

### 3.2 Why g1-g3 matter more than their size

A theorem is stated with definitions and proved by unfolding them (`F8-first-theorems.md` §2). `p04_dependent_app` was the fixture that made substitution observable; g1 is the fixture that makes delta observable, and today the kernel fails it. Nothing elaborated from Bebop source will check under a `k_conv` that cannot unfold `fp_mul`.

### 3.3 Why the probes are not committed by this lane

Landing g1-g5 as `p05`-`p07`/`n16`-`n17` makes `kernel_parity` 18/23 and `kernel_neg_bin` 2 by construction, which is red on `:65` until K-δ (§5) lands. That is the correct red, but it is the F7-row owner's to take with the fix in the same commit, not a blueprint lane's to leave behind. The texts are in appendix A, byte-for-byte what was run.

## 4. The Hurkens witness: where it is actually blocked, measured

### 4.1 n15 as it landed

`n15_hurkens_full.core` (`ddb534f`) declares `U : Sort 0`, `Pow : U -> Sort 0`, `tau : (U -> Sort 0) -> U`, `sigma : U -> (U -> Sort 0)` as AXIOMS and then applies `tau` to `sigma`. Both arms return 24 (argument type mismatch) under both level rules -- a structural property of application, as the fixture's own header says. So the row's sentence "the impredicative arm stays UNPROVEN until a full Hurkens encoding is authored and shown rejected under it" is still true after n15.

### 4.2 The encoding is expressible; a generator was written to prove it

The fixture header and the commit message claim the paradox's self-application "requires the same node at two syntactic depths" and is therefore inexpressible under the no-sharing rule (`kcheck.py:59-65,121-126`; `tkernel.bp:114-123`). Measured otherwise: the rule forbids one NODE with two parents, not one SUBTERM written twice, and the twin's own error text says what to do -- "Duplicate it in the elaborator" (`kcheck.py:125`). A 60-line generator (appendix B) takes named-variable terms, computes de Bruijn indices per occurrence and emits a fresh id per node; Hurkens' three definitions in the shape of Coq's `theories/Logic/Hurkens.v` (from memory: `U := forall X:Type. (PP X -> X) -> PP X`, `tau t X f p := t (fun x => p (f (x X f)))`, `sigma s := s U (fun t => tau t)`, with `Prop = Sort 0`, `Type = Sort 1`, `Pow X = X -> Sort 0`) come to **78 nodes and 3 declarations** -- against caps of 1024 file ids and 8192 uids (`tcheck_kernel.bp:26-29`). Sharing is a cost, not a wall.

### 4.3 What each implementation says about it

| probe | content | twin | kernel `max` | kernel `imax` |
|---|---|---|---|---|
| h1 | `def U : Sort 2 := forall X : Sort 1. (PP X -> X) -> PP X` | ACCEPTED (17 fuel) | 0 | 0 |
| h4 | the same `U` declared at **Sort 1** -- what `Type : Type` would admit | REJECTED, "declared Sort 1 but the body has type Sort 2" | 26 | 26 |
| h2 | h1 + `def tau : PP U -> U := ...` | ACCEPTED (184 fuel) | **23** | **23** |
| h3 | h2 + `def sigma : U -> PP U := fun s => s U (fun t => tau t)` | **REJECTED, "argument type mismatch: expected Sort 1, got Sort 2"** | 23 | 23 |

h3 is the witness. The twin accepts `U` and `tau` and refuses `sigma` at `s U`: `s : U` quantifies over `X : Sort 1` and `U : Sort 2`, so `U` cannot be a member of the universe it quantifies over -- the same point at which Coq's and Lean's kernels refuse the encoding, and the point a `Sort n : Sort n` kernel would pass (h4 shows that mutant's fingerprint: h4 flips to 0 under it, so the pair h3/h4 discriminates). It is rejected under BOTH level rules, which is the result the row wants: `imax` makes `Sort 0` impredicative and leaves `Sort 1` predicative, and Hurkens needs the latter.

The kernel never reaches that check. It stops at `tau` with 23 because `x : U` is a `const` and `k_infer_app` demands a literal `pi` at the head (`tcheck_kernel.bp:154-155`) -- **the witness is blocked on delta unfolding, the same gap as g1**, not on sharing and not on eliminators. Hurkens' paradox is the one that needs no inductive type at all; that is why it, and not Girard's, is the standard witness. Where eliminators DO enter is §5 K-ι: an eliminator from a `Sort 0` inductive into `Sort 1` under the `imax` arm is the classic inconsistency (impredicative `Set` with strong elimination, the reason Coq restricts elimination out of `Prop`; from memory), so K-ι must carry an elimination restriction on that arm and re-run h3 -- with the paradox restated through the eliminator -- when it lands.

### 4.4 Consequence for the row's owed list

The row lists "eliminators and recursors, universe polymorphism, that Hurkens witness". Measured order:

1. **K-δ first.** Beta/delta in `k_conv` and on the application head, with fuel. Unblocks g1, g2, h2, h3 and every future theorem. Without it the witness cannot be shown rejected FOR THE RIGHT REASON.
2. **K-H second**, the same week: land h3 as `n18_hurkens_sigma.core` and h4 as `n19_hurkens_type_in_type.core`, expected 24 and 26 under both arms; a mutation run with `k_infer`'s sort rule changed to `Sort n : Sort n` (`tcheck_kernel.bp:204`) must flip h4 to 0 or the fixture pins nothing.
3. **K-ι third.** Constructor well-formedness (g4/g5, kernel-accepted today), constructor registration (g3), recursor type generation, iota, elimination restriction under `imax`; re-run the witness through the eliminator.
4. **K-u last.** Universe polymorphism -- required by the operator (row F7, 2026-09-09, "both") but placed in the ELABORATOR by the design study (`RESEARCH-PROOF-DESIGN-2026-09-09.md:192`: "concrete levels; the elaborator solves and instantiates; the kernel computes `max` on two integers"). The two are reconcilable only if the kernel stays concrete and the elaborator instantiates; this blueprint assumes that reading and flags the alternative (level variables in `k_infer_pi`) as the ~8-fn cost the row quotes, UNMEASURED.

## 5. Sequencing: the kernel half, then the surface half

| step | name | lands in | first number |
|---|---|---|---|
| K-δ | `k_whnf` (beta + delta, fuel in `K`), `k_conv` after whnf, `k_infer_app` head through whnf; `kernel_neg_bin` line in `kcheck.py` and battery | `tcheck_kernel.bp`, `tools/kcheck.py`, `tools/battery.sh` | `kernel_parity: 21/21` with g1-g3 landed; `kernel_neg_bin: 0` |
| K-H | h3/h4 as fixtures; mutation run on the sort rule | `bench/kernel_neg/` | `kernel_neg: 0 accepted of 18`; h4 flips under the `Sort n : Sort n` mutant |
| K-ι | constructor type inference (g4/g5 as `n16`/`n17`), ctor registration, `k_rec_type`, iota in whnf, elimination restriction on the `imax` arm; `imax` arm run by the gate | `tcheck_kernel.bp`, `tkernel.bp`, `kcheck.py` (twin grows in step) | `kernel_checked >= 1` on a `Nat` recursor fixture; both arms in battery |
| K-u | level instantiation in the elaborator OR level variables in the kernel (operator to pick; §4.4) | `elab.bp` or `tcheck_kernel.bp` | a fixture stated once for all `u` |
| S-a | surface parse of `[T; n]` (F2's width-keyed side channel, `F2-bounds-by-type.md` §2.1 -- not re-derived), `{x : T \| p}`, `requires`/`ensures`/`invariant`, `theorem NAME : STMT := PROOF`; erase before emission; obligations into the trace zone | `bebop.bp` (`collect_fns:6023`, `compile_fn_at:6191`, `parse_params:277`, `emit_let_stmt:5466`) | `f8_dt` reports `4 parsed` instead of `0 parsed`; `theorem_neg: samples/theorem-false.bp COMPILEFAIL`; `K5_typed <= 1.5x 1.67 s` (`ROADMAP.md:248`); fixpoint unchanged |
| S-b | untrusted elaborator emitting `.core` text (the line-numbered format `kcheck.py:86-146` and `tkernel.bp:125-151` read -- NOT the s-expression form of `RESEARCH-PROOF-DESIGN-2026-09-09.md` §8.1, which exists nowhere), duplication instead of sharing | `selfhost/elab.bp` (standalone, own cap) or Lean off-box (row F4's shape) | `elab_neg: 0 accepted of N`; `dt_fns: 2/925` (`fp.bp`), then `19/925` (+`money.bp`), then `101/925` (+`store.bp`) |
| S-c | rename every `let` (REBIND, `docs/LANGUAGE.md:41-43`) to SSA in the elaborator; require `invariant` for every rebound register a type mentions | `elab.bp` | `dt_fns` grows past straight-line fns; first measurement: count of rebound `let`s per fn in the three targets (not measured here) |
| S-d | `Fin n`, length-indexed tensors for B7 | after S-c | B7's consumer gate |

What S-a inherits from F8 step 0 (`ddae413`, `1e51e48`), so it is not rebuilt: `scan_inert` (`bebop.bp:4554`, comment `:4548-4553`) lexically validates the fn-header region between `)` and `{` (`compile_fn_at`, call at `:6254`, exit 110 at `:6255`) and a top-level `theorem ` line to end of line (`collect_fns`, recognition `:6068-6084`, validation `:6096-6100`); code 110's text is at `:105`, its row at `docs/TRAPS.md:34`; constructs `neg/c143_contract_garbage.bp` and `c144_contract_ok.bp` bracket it. Measured on the two SUPERSEDED samples: `samples/theorem-sample.bp` -> **rc 110 at 9:21** (the `\x` lambda, not on the allow list); `samples/theorem-false.bp` (`theorem bad : 1 + 1 = 3 := refl`, `:7`) -> **rc 0**. A false theorem compiles green today; S-a's `theorem_neg` gate is that file.

Two anchors in the tree are stale and should not be copied: row F8 places `scan_inert` at `bebop.bp:4503` (it is 4554), and `docs/TRAPS.md:34` places the header check at `compile_fn_at:6146` (6146 is inside `compile_fn_at_total_saved`, `:6132`; the check is `:6250-6255`) and the theorem check at `collect_fns:6023-6027` (it is `:6068-6100`).

## 6. Gates

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| K-δ | unfolding conversion | kernel | `kernel_parity: 21/21`; `kernel_neg_bin: 0 accepted of 14`; `kernel_internal: 0` | success |
| K-H | Hurkens witness | corpus | `kernel_neg: 0 accepted of 18` under `max` AND `imax` (two battery lines); mutant flips h4 | success |
| K-ι | inductives complete | kernel + twin | `kernel_neg: 0 accepted of N` with n16/n17; `kernel_checked >= 1`; elimination-restriction fixture rejected under `imax` | success |
| K-u | universes | per §4.4 | one fixture stated for all levels; `kernel_parity: N/N` | success |
| S-a | inert surface + erasure | `bebop.bp` | `f8_dt: 4 parsed`; `theorem_neg: 1/1 COMPILEFAIL:<code>` with `line:col`; `K5_typed <= 2.5 s`; gen3 == gen4 | success |
| S-b | elaborator | `elab.bp` | `elab_neg: 0 accepted of N`; `dt_fns: 2/925 -> 19/925 -> 101/925` | success |
| S-c | rebinding | `elab.bp` | `dt_fns` past straight-line fns; every rejection carries `line:col` | success |

`dt_fns`' denominator: row F8 says 833; the design study measured 837 on 2026-09-09 (`RESEARCH-PROOF-DESIGN-2026-09-09.md:34`); today `grep -c '^fn '` over `selfhost/std/` + `selfhost/prelude/` gives **925** (776 + 149), 993 with the four top-level programs and `selfhost/tools/`. Re-derive at landing. The row's `store.bp 52/52` is 82 fns today (`selfhost/prelude/store.bp`); `fp.bp 2/2` and `money.bp 17/17` hold.

## 7. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| `selfhost/tcheck_kernel.bp:k_conv` | whnf both sides before the structural walk | `:94-109` (comment says structural, deliberately) |
| `selfhost/tcheck_kernel.bp:k_infer_app` | whnf the head's type before the tag-4 test | `:150-164`; the test at `:154-155` |
| `selfhost/tcheck_kernel.bp:` new `k_whnf` | beta + delta with a fuel cell in `K` (a free cell: `K[6..15]` per the layout at `:22-30`); exhaustion is a NEW reject code, never 0 | layout `:22-30`; `k_def_type` returns the type only, a `k_def_body` twin of `:182-194` is needed for delta |
| `selfhost/tcheck_kernel.bp:k_check_inductive_ctor` | `k_infer` the constructor type and require a sort before the spine walk | `:270-285` |
| `selfhost/tkernel.bp:kp_ctor` | register `(hash, ctype, 0)` via `k_def_add` | `:155-161`, TODO at `:157` |
| `selfhost/tkernel.bp:main` | print `K[0]` (uid high-water) on a line BEFORE the verdict so capacity is a number; `kcheck.py` reads only the last line (`:431-433`) | `:214-241` |
| `tools/kcheck.py:corpus` | `kernel_neg_bin` line; run the kernel twice (`imax` arg) and print `kernel_parity_imax` | `:483-524`, `:416` |
| `tools/battery.sh` | assert `kernel_neg_bin: 0`, `kernel_parity_imax` | `:64-65` |
| `bench/kernel_neg/` | `p05`-`p07` (g1-g3), `n16`-`n17` (g4-g5), `n18`-`n19` (h3, h4) | appendix A |
| `bebop.bp:collect_fns`, `compile_fn_at`, `parse_params`, `emit_let_stmt` | S-a parse + erase; add `\` to `scan_inert`'s allow list only if lambdas are admitted (row F8's own note) | `:6023`, `:6191`, `:277`, `:5466`, `:4554` |
| `selfhost/elab.bp` (new) | S-b/S-c | -- |

## 8. Cost and schedule

Kernel today: 19 + 11 = 30 fns. The design study's per-module estimates (`RESEARCH-PROOF-DESIGN-2026-09-09.md` §5.1): reduction 10-14 fns, conversion 3-5, inductives 25-40, environment 5-8; kernel total 137-206. K-δ is ~6-10 fns and the fixtures (one lane, one merge); K-H is fixtures only; K-ι is the study's K4 lane, 6 steps (`:318`); K-u 2-3 steps. Row F8's own schedule for the surface stands: (a) 2-3 weeks on the `bebop.bp` lane, (b) 8-16 weeks in parallel, (c) after (b). Nothing in this file shortens (b); what it changes is that (b) cannot start against today's kernel and K-δ is the two-day step that makes it startable.

## 9. Speculative, marked

- The elimination restriction under `imax` (§4.3) is designed from the CIC precedent, not measured here; the fixture that pins it does not exist until K-ι.
- Universe polymorphism's placement (§4.4) is an open operator decision; both costs are quoted from documents, not measured.
- `K5_typed <= 1.5x` assumes annotation parsing is one more `scan_inert`-shaped pass; unmeasured until S-a.
- Whether K-δ's allocation (every `k_subst`/`k_shift` allocates fresh uids, `:60-92`) fits under 8192 for h3's 78 nodes plus unfolding is unknown -- the driver prints no high-water mark; that is the first line to add.

## 10. Attribution

- Rows: `ROADMAP.md:201` (F7, kernel), `:202` (F8, this file), `:199` (F5, trace zone), `:248` (K5 = 1.67 s).
- Kernel and driver: `selfhost/tcheck_kernel.bp` (19 fns), `selfhost/tkernel.bp` (11 fns), digest `f48daac5`; twin `tools/kcheck.py` (536 lines), corpus `bench/kernel_neg/` (18 files).
- Today's commits, verified with `git cat-file -t`: `3a61acd` (parity becomes a measurement), `bccbde4` (tkernel.bin ignored), `ddb534f` (n15), `ddae413`/`1e51e48` (F8 step 0), `0ceb80b` (Phase F opens).
- Design study: `docs/RESEARCH-PROOF-DESIGN-2026-09-09.md` §4 (`:107`, `:120`, `:136`), §5.1, `:192`, `:318`; refutation instrument: `docs/KERNEL-REFUTATION.md` §0-6.
- Surface: `bebop.bp:4554` (`scan_inert`), `:6023` (`collect_fns`), `:6191` (`compile_fn_at`), `:105` (code 110), `docs/TRAPS.md:34`, `docs/LANGUAGE.md:41-43`, `samples/theorem-sample.bp:9`, `samples/theorem-false.bp:7`.
- Hurkens' construction: Coq `theories/Logic/Hurkens.v` (from memory; not in this tree).

---

## Appendix A -- the five gap probes, byte-for-byte as run

`g1_delta_def_as_type.core`
```
% g1: def A := B, check b : A needs delta
1 sort 0
axiom B : 1
2 sort 0
3 const B
def A : 2 := 3
4 const B
axiom b : 4
5 const b
6 const A
check 5 : 6
```
`g2_beta_in_type.core`
```
% g2: check b : ((fun A => A) B) needs beta
1 sort 0
axiom B : 1
2 const B
axiom b : 2
3 const b
4 sort 0
5 var 0
6 lam 4 5
7 const B
8 app 6 7
check 3 : 8
```
`g3_ctor_const.core`
```
% g3: constructor used as a const after its inductive
1 sort 0
2 const Nat
3 const Nat
4 const Nat
5 pi 3 4
inductive Nat : 1 | zero : 2 | succ : 5
6 const zero
7 const Nat
check 6 : 7
```
`g4_ctor_domain_value.core`
```
% g4: inductive whose constructor DOMAIN is a value, not a type (twin: sort_of(ctype) rejects)
1 sort 0
axiom A : 1
2 const A
axiom a : 2
3 sort 0
4 const a
5 const Foo
6 pi 4 5
inductive Foo : 3 | mk : 6
```
`g5_ctor_unbound_var.core`
```
% g5: inductive whose constructor domain is an UNBOUND var
1 sort 0
2 var 7
3 const Foo
4 pi 2 3
inductive Foo : 1 | mk : 4
```

## Appendix B -- the Hurkens witness (h3), 78 nodes, generated

Generator (scratchpad `gen.py`, kept out of the tree; reproduce from this text): named terms `Pi(x,A,B)`, `Lam(x,A,b)`, `App(f,a)`, `V(x)`, `S(n)`, `C(name)`; emission walks the tree, computes each `var` as its distance from the innermost binder, and hands out a fresh id per node so no id is referenced twice. `U`, `tau`, `sigma` as in §4.2.

```
% h3: sigma applies s : U to U itself: X : Sort 1 but U : Sort 2 -- the universe rejection
1 sort 2
2 sort 1
3 var 0
4 sort 0
5 pi 3 4
6 sort 0
7 pi 5 6
8 var 1
9 pi 7 8
10 var 1
11 sort 0
12 pi 10 11
13 sort 0
14 pi 12 13
15 pi 9 14
16 pi 2 15
def U : 1 := 16
17 const U
18 sort 0
19 pi 17 18
20 sort 0
21 pi 19 20
22 const U
23 pi 21 22
24 const U
25 sort 0
26 pi 24 25
27 sort 0
28 pi 26 27
29 sort 1
30 var 0
31 sort 0
32 pi 30 31
33 sort 0
34 pi 32 33
35 var 1
36 pi 34 35
37 var 1
38 sort 0
39 pi 37 38
40 var 3
41 const U
42 var 1
43 var 2
44 var 0
45 var 3
46 app 44 45
47 var 2
48 app 46 47
49 app 43 48
50 app 42 49
51 lam 41 50
52 app 40 51
53 lam 39 52
54 lam 36 53
55 lam 29 54
56 lam 28 55
def tau : 23 := 56
57 const U
58 const U
59 sort 0
60 pi 58 59
61 sort 0
62 pi 60 61
63 pi 57 62
64 const U
65 var 0
66 const U
67 app 65 66
68 const U
69 sort 0
70 pi 68 69
71 sort 0
72 pi 70 71
73 const tau
74 var 0
75 app 73 74
76 lam 72 75
77 app 67 76
78 lam 64 77
def sigma : 63 := 78
```
h4 is h3's first 16 nodes with line 1 replaced by `1 sort 1`; h1 is h3's first 16 nodes and first `def` alone.
