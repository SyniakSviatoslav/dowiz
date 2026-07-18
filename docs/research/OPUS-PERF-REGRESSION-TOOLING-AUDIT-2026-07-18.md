# Benchmark-regression tooling — deep audit (`bench_track.py` / `native-trackers` / `baseline.json`)

**Date:** 2026-07-18 · **Model:** Opus 4.8 · **Scope:** the dedicated deep-dive the operator asked
for — deltas, regressions, snapshot comparison over time. Is the regression-detection pipeline real,
rigorous, and complete, or does it have gaps? Read-only; no code changed.

**Bottom line up front.**
1. The pipeline is **real and wired into CI**, but its comparison math is **naive, not statistical**:
   a raw percentage delta of criterion's *reported mean* against a committed absolute baseline, at a
   flat 10 % threshold, on `--sample-size 10` runs. It **parses criterion's text output and throws
   away criterion's own bootstrap confidence intervals and change-significance test** — the exact
   statistical comparison the operator is asking whether it should use. So it is a *parallel and
   strictly-less-rigorous* re-implementation of a capability criterion already ships.
2. Of the **three fail-open paths** the earlier audit found this session (`AUDIT-2026-07-18-PERFORMANCE-metrics-benchmarks.md` §PERF-03, scorecard §4.4/§6): **two are now FIXED** (baseline coverage 5/9 → **13/13**; the exit-0-on-MISSING tracker is closed twice over). **The third is STILL OPEN** — the explicitly-rejected cross-host absolute comparison is still what CI is configured to do — **and it is now compounded by a new breakage**: the refactor that fixed path #2 turned `bench_track.py` into a thin wrapper that **cannot run on a fresh CI runner at all** (it `exit(2)`s because `native-trackers` is never built there). The gate that is supposed to catch silent regressions currently carries no usable CI signal.
3. This **is** the same gate as the memory's "3 independent fail-open paths" note (it traces to this session's PERF-03 finding, ledger row 23).
4. `BENCH_HISTORY.md` is **git-ignored** — there is no committed historical trend storage; CI only uploads it as a failure artifact.

---

## 1. Ground truth — what the pipeline actually is

### 1.1 Components and data flow

| Component | Path | Role |
|---|---|---|
| criterion harness | `kernel/benches/criterion.rs` (**13** `bench_function` IDs, `:17…:236`) | produces per-bench timing + criterion's own stats in `target/criterion/**` |
| committed baseline | `kernel/benches/baseline.json` (**13** keys — git-tracked) | the absolute reference numbers (measured on the Hetzner host) |
| **the real comparator** | `tools/telemetry/native-trackers/src/main.rs` → `cmd_bench` (`:143-299`) | runs `cargo bench`, parses text, computes deltas, gates |
| CI-path entry | `kernel/benches/bench_track.py` (HEAD = **thin wrapper**, `:1-102`) | locates + delegates to `native-trackers`; forwards its exit code |
| rolling history | `kernel/benches/BENCH_HISTORY.md` (**git-ignored**, `.gitignore:2`) | append-only trend log, never committed |
| CI job | `.github/workflows/ci.yml` `bench-regression` (`:150-168`) | `runs-on: ubuntu-latest`; runs `bench_track.py --threshold 10` |

The `bench_track.py` at HEAD is **not** the comparator — it is a 102-line delegation shim. All parsing
/ baseline-diff / gating lives in the zero-dep Rust binary `native-trackers` (`bench_track.py:5-11`).

### 1.2 How "10 %" is actually computed — mean-vs-mean raw delta, no significance test

`native-trackers::cmd_bench` (`main.rs:263-275`):

```rust
let delta = (cmean - bmean) / bmean * 100.0;
let verdict = if delta > threshold { worst = worst.max(delta); "REGRESS" }
             else if delta < -threshold { "improve" } else { "ok" };
```

- `bmean` = the committed baseline number; `cmean` = criterion's **reported mean** for this run,
  scraped from the text line `name  time: [lo unit mean unit hi unit]` — it takes token index 2 (the
  mean) and discards `lo`/`hi` (`parse_timing`, `main.rs:322-336`).
- The run is deliberately fast and **statistically thin**: `--warm-up-time 1 --measurement-time 2
  --sample-size 10` (`main.rs:176-181`). 10 is criterion's *minimum* allowed sample count.
- There is **no** t-test, **no** Mann–Whitney U, **no** confidence-interval overlap check, **no**
  variance/noise model, **no** per-bench threshold. One scalar (the mean) vs one scalar (the
  baseline), flat 10 % band, single run.

**This is the crux rigor finding.** criterion is *already computing* the rigorous version: every run
writes `target/criterion/<id>/report/regression.svg` and change-detection reports (these files exist
in-tree today — e.g. `ppr_rank_32x32_k20/report/regression.svg`), using its bootstrap resampling to
produce a confidence interval and a change p-value against a configurable `noise_threshold` /
`significance_level`. The homegrown comparator runs criterion, then **throws that away** and does a
raw percentage on the point estimate. So the pipeline is not "criterion's stats plus a gate" — it is
"criterion's stats **discarded** and replaced with a weaker heuristic."

The sibling PERF audit already proved this is not academic (`AUDIT-2026-07-18-PERFORMANCE…` §PERF-06):
back-to-back runs on this shared host swung **+75 %** on `fold_transitions` (3.28 → 5.74 ns) and the
n=30 CI width was ±14 %. A flat 10 % delta on a 4 ns bench, from a 10-sample run, on a QEMU box shared
with swarm agents, is — in the auditor's own words — "statistically meaningless on this host."

### 1.3 Homegrown vs. criterion-native — is it redundant?

Yes. criterion ships exactly the A/B comparison the operator is asking about:
`cargo bench -- --save-baseline <name>` saves a named run; `--baseline <name>` compares the current
run against it and reports regressed/improved/unchanged **with statistical significance**. `critcmp`
(the standard BurntSushi tool) reads those saved baselines and prints a comparison table. The
homegrown `native-trackers bench` path duplicates this comparison **and does it less rigorously**
(mean-only, no significance) **and cross-host** (see §2). The one thing criterion/critcmp do *not*
do out of the box is set a non-zero **exit code** on regression — so a thin exit-code gate is
legitimately needed, but re-implementing the *statistics* is not.

---

## 2. Is it CI-wired, or manual-only? — Wired, but currently non-functional in CI

Wired: `.github/workflows/ci.yml` `bench-regression` job (`:150-168`), `runs-on: ubuntu-latest`, on
every push/PR, step = `cd kernel && python3 benches/bench_track.py --threshold 10`.

Two problems, one of them new and serious:

**(A) The gate cannot execute on a fresh runner — it exits 2 before comparing anything.** HEAD's
`bench_track.py` delegates to the `native-trackers` binary and, if it can't find it, prints
"native-trackers binary not found" and `sys.exit(2)` (`bench_track.py:66-84`). On `ubuntu-latest`:
`native-trackers` is not on `PATH`, the binary is not committed (`target/` is git-ignored), and the
job **never builds it** — its only steps are checkout, `cargo fetch` for `kernel/`, then the wrapper
(`ci.yml:154-162`); there is no `cargo build` for `tools/telemetry/native-trackers`. The wrapper is
not invoked with `--build-native`. Therefore the step exits 2 on every CI run. This was introduced by
`f3c0687cf` ("bench_track.py → native wrapper"), which **deleted the python fallback comparator** that
previously let the job run on a fresh checkout — and ci.yml was not updated to compensate. A gate that
is red-for-the-wrong-reason on every run is indistinguishable from broken: it carries no
regression signal, invites `|| true`/`continue-on-error` "fixes" that flip it to genuinely fail-open,
and is exactly why the earlier audit noted "no evidence the job has ever run green" (`gh` returns 404
for Actions with current auth, so this could not be observed remotely; the conclusion follows from the
job definition + the wrapper source, which are unambiguous).

**(B) Even if it did run, it does the cross-host absolute comparison that was explicitly rejected.**
`native-trackers` compares the committed `baseline.json` (Hetzner-host absolute ns) against numbers
measured on the foreign `ubuntu-latest` runner. Ledger row 23's own honesty note and
`BLUEPRINT-P45 §4b.3` both say this must be criterion's same-runner `--save-baseline`/`--baseline`
A/B, precisely because a foreign runner's constant factor makes an absolute-baseline gate either
false-RED or (with a widened threshold) gate nothing. It is not using `--save-baseline` at all.

The rigorous *absolute-host* tracking is designed to stay local/scheduled (`native-trackers bench
kernel --threshold 10` on the Hetzner box) — that part is sound. The defect is that CI was pointed at
the same absolute-comparison mechanism on a different host, and now can't even reach it.

---

## 3. The "3 independent fail-open paths" — same gate, traced to current status

This **is** the gate in the memory note. It traces to this session's
`AUDIT-2026-07-18-PERFORMANCE-metrics-benchmarks.md` §PERF-03 and scorecard §4.4/§6, and to
`REGRESSION-LEDGER.md` row 23. Status of each of the three named paths at HEAD:

| # | Fail-open path (as named in the audit) | Status | Evidence |
|---|---|---|---|
| a | **Partial baseline coverage** (baseline covered 5 of 9 bench IDs; 4 newest invisible) | **FIXED** | `baseline.json` now has **13 keys = all 13** `criterion.rs` bench IDs (incl. both `spectral_cache/*`, both `graph_rebuild_rank/*`, `ppr/*`, `absorbing/*`, `attention/*`, `retrieval/*`). Coverage 13/13. |
| b | **Exit-0-always tracker** (python loop skipped `MISSING` benches → a *deleted* hot-path bench exited 0) | **FIXED (twice)** | First `6d7d0e155` made the python fallback record `MISSING` as `delta = threshold+1` → `exit 1`. Then `f3c0687cf` deleted the python comparator entirely; the surviving `native-trackers` path was already fail-closed: `MISSING` sets `worst = (threshold+1).max(worst)` → `exit 1` (`main.rs:257-260`). |
| c | **Rejected cross-host comparison still running in CI** | **OPEN — and worse** | CI still points at the absolute-baseline mechanism on `ubuntu-latest` (§2B), and post-refactor that mechanism **can't execute** on the runner at all (§2A, `exit 2`). Neither the same-runner `--save-baseline` A/B nor a build step was added. |

Net: **2 of 3 closed, 1 open.** The open one is the structurally hardest (it needs the CI job
re-architected to same-runner A/B, not a one-line change) and it is now masked by the execution break,
so it presents as "broken job" rather than "fail-open" — but the underlying design defect (cross-host
absolute gate) is unchanged.

---

## 4. Best-practice research — rigorous regression detection under noise

> **Epistemic status (disclosed).** This session's WebSearch budget was exhausted (200/200 by the
> sibling passes) before I could issue fresh queries, so the tool-capability claims below are from
> established knowledge of these tools and their canonical docs, **not re-verified live this session**.
> They are well-known, stable behaviours; treat the specific flag names as "verify before relying"
> where noted. In-repo facts (§1–§3, §5) are all live-verified.

The core problem is the one this pipeline gets wrong: **a percentage delta on a point estimate is not
a regression test on a noisy measurement.** The rigorous approaches, roughly in increasing order of
noise-immunity:

1. **Use criterion's built-in statistical comparison instead of re-deriving it.** criterion collects
   many samples, bootstrap-resamples to estimate the mean/median distribution, and for A/B
   (`--save-baseline` then `--baseline`) reports *"Performance has regressed/improved/no change"*
   against a `noise_threshold` and `significance_level` — i.e. a significance test, not a raw delta.
   Limitation: it is *reporting*, not *gating* (no non-zero exit on regression). Canonical:
   criterion.rs user guide, "Command-Line Options" and "Comparing Benchmarks".
2. **`critcmp`** (BurntSushi) — the standard tool to compare two saved criterion baselines and print a
   ratio table (`critcmp base new`). Same limitation (report, not exit-code gate), so a thin parser
   over its output supplies the CI exit code. This is the "just USE criterion's native comparison"
   path the operator asked about, and it is the right one. Canonical: `github.com/BurntSushi/critcmp`.
3. **Statistical significance tests** for a homegrown gate, if one insists on custom logic: **Welch's
   t-test** (unequal-variance, for approximately-normal timing samples) or the non-parametric
   **Mann–Whitney U** (no normality assumption — better for heavy-tailed latency) on the two sample
   sets, gating on the *p-value + effect size*, never on the mean delta alone. criterion effectively
   does the bootstrap equivalent internally; re-implementing this is only justified if criterion's
   output genuinely can't be consumed.
4. **Prefer a deterministic metric over wall-clock where possible.** The single biggest lever on a
   shared QEMU host: measure **instruction counts**, not time. `iai-callgrind` (callgrind-based, the
   modern Rust successor to `iai`) yields near-deterministic instruction/cache counts that are immune
   to the ±75 % host swings PERF-06 measured, letting a *tight* threshold actually mean something for
   the ns-scale benches that cannot support a wall-clock gate at all. Canonical:
   `github.com/iai-callgrind/iai-callgrind`.
5. **How large Rust projects do it at scale — `rustc-perf` (perf.rust-lang.org).** The reference
   design: (a) run on **dedicated, isolated hardware**, never shared CI runners; (b) primary metric is
   **`instructions:u`** (hardware/cachegrind instruction counts) because wall-time is too noisy;
   (c) **full historical trend stored in a database** with a dashboard, so drift is a visible curve,
   not a single delta; (d) significance judged against **each benchmark's own historically-derived
   noise level**, not one flat percentage; (e) automated regression comments on PRs. The transferable
   lessons for dowiz: same-runner (or dedicated-host) comparison, deterministic metric for the
   deterministic kernel benches, per-bench noise instead of a global 10 %, and committed historical
   trend.

The through-line: dowiz's `native-trackers bench` is a naive-delta gate that discards the statistics
criterion already computed. Every rigorous approach either *consumes* criterion's stats (options 1-2)
or *changes the metric* to something deterministic (options 4-5); none is a flat percentage on a
10-sample mean.

---

## 5. Snapshot / state comparison over time (`порівняння снепшотів`) beyond perf

The operator's broader "snapshot comparison" framing does connect to real kernel machinery — but for
**state integrity/equivalence**, which is a *different axis* from perf deltas. What exists:

1. **Content-addressed hash chain = O(1) full-state snapshot equivalence.** `event_log.rs` is a
   SHA3-256 hash chain: each event's content-id = `SHA3(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`
   (`event_log.rs:127-154`). The **tip content-id is a Merkle-style digest of the entire history** —
   comparing two nodes' (or two points-in-time) tips is a cheap, exact "are these two state snapshots
   identical" check. This is the literal basis of MESH-06 sync (`event_log.rs:1-6`) and is real,
   in-tree, tested.
2. **`EventLog::verify_chain()` = at-rest corruption / drift detection *within* a snapshot** (P-H
   W-H4, ledger row 24.A3). Walks the chain, recomputes each event's SHA3, returns
   `ChainDefect::HashMismatch{at}` on the first divergence between stored and recomputed
   (`event_log.rs:211-212`). This is "has this snapshot silently drifted from its own hashes."
3. **`spectral_cache::RetainedBase` = drift-gated spectral snapshot** (ledger rows 26/27). A
   canonicalized, content-addressed "retained snapshot" of a spectral operator, admitted only after
   `classify_drift` on the RAW operator rejects `Unstable` (`spectral_cache.rs:233-273`,
   `admit(raw, epoch)`); `SnapshotRejected::UnstableSpectrum` otherwise. This is stability-drift
   detection over epochs.
4. **`eqc-rs` digest pinning = golden-digest reproducibility over time** (ledger row 25). The CORDIC
   `empirical_identify` digest is pinned and re-verified (`tools/eqc-rs/tests/cordic_digest.rs`,
   `pin-and-verify`) so the *same computation* producing a *different digest* across
   machines/commits is caught. This is the closest existing analogue to a state-regression gate — but
   scoped to one specific computation.

**The genuine gap.** There is **no general "golden state-digest" regression gate** — the *state*
analogue of the perf bench gate. Nothing periodically folds the event log into its projections
(order totals, ledger balances, FSM states) and diffs a committed golden digest of those outputs
against HEAD to catch a `decide`/`fold` *logic* change the way the bench gate is meant to catch a
*timing* change. All the primitives exist (deterministic fold, content-addressing, digest-pinning
discipline, the 12-query retrieval oracle), so this is a **wiring/consolidation task, not
greenfield** — the Rust-ecosystem shape is snapshot testing (`insta`-style) or a committed
golden-digest KAT over `fold`. Worth proposing as a sibling to the perf gate; it is a real, coherent
extension, not busywork.

---

## 6. Concrete improvement proposal (grounded in verified capabilities)

Ordered by leverage. Items 1-2 fix the open fail-open path + the CI break together; 3-4 add the
rigor and the missing trend storage; 5 is the STEP-3 extension.

**1. Re-architect the CI gate to criterion's same-runner A/B (fixes path #3 and the exit-2 break).**
In the `bench-regression` job, on one runner: check out the merge-base and
`cargo bench --bench criterion -- --save-baseline base`; check out HEAD and
`cargo bench --bench criterion -- --baseline base`; then `critcmp base new` and gate the exit code on
a threshold via a thin parser. Both baselines are measured on the *same* `ubuntu-latest` instance, so
the host constant cancels — this is exactly what ledger row 23's honesty note and P45 §4b.3
prescribed. It also removes the dependency on the unbuilt `native-trackers` binary in CI entirely.
(Grounded: `--save-baseline`/`--baseline` and `critcmp` are stable, documented capabilities; verify
exact flag spelling against the pinned criterion 0.5 before wiring.)

**2. If the homegrown path is kept for the local/scheduled absolute-host tracker, make it runnable.**
That role (absolute Hetzner-host numbers, single environment) is legitimate and cross-host noise
doesn't apply. But then either (a) build `native-trackers` in any job that calls the wrapper
(`cargo build --release --manifest-path tools/telemetry/native-trackers/Cargo.toml`) or pass
`--build-native`, or (b) restrict `native-trackers bench` to the scheduled/local cron and take it off
the CI path. Do **not** leave the CI job calling a wrapper that `exit(2)`s.

**3. Stop discarding criterion's statistics.** Whichever path gates, consume criterion's own
significance verdict (regressed/improved/no-change against `noise_threshold`) rather than a raw mean
delta; raise `--sample-size` above 10 for the sub-100 ns benches; and adopt **`iai-callgrind`
instruction-count benches for the deterministic kernel hot paths** so the ns-scale benches PERF-06
flagged as ungateable get a metric that actually supports a tight threshold. Use per-bench thresholds
(rustc-perf's per-benchmark-noise model), not a single global 10 %.

**4. Add committed historical trend storage.** `BENCH_HISTORY.md` is git-ignored, so there is no
durable trend (CI only uploads it as a failure artifact). Commit the stable quantity — the same-runner
A/B **ratios** (and A/B benches as ratios per PERF-05's guidance), or resume the scheduled absolute-
host feed into the already-live `tools/telemetry/logs/bench.jsonl` (PERF-01/PERF-10 note it went
silent 2026-07-15). This is the minimal `rustc-perf`-style trend the operator asked for ("historical
trend storage if missing").

**5. Propose a golden state-digest regression gate (STEP-3 extension, separate wave).** Sibling to the
perf gate: a committed golden digest over `fold`/`decide` projections (built from the existing
content-addressing + eqc-rs pinning discipline), so a *behavioural* drift in kernel output trips CI the
same way a timing drift should. Building blocks all exist; this is wiring, and it should be gated on
operator direction (touches the money/FSM red-line surfaces).

---

## Sources

- **In-repo, live-verified this pass:** `tools/telemetry/native-trackers/src/main.rs:143-337`
  (comparator), `kernel/benches/bench_track.py:1-102` (thin wrapper + exit-2 path),
  `kernel/benches/baseline.json` (13 keys), `kernel/benches/criterion.rs` (13 bench IDs),
  `kernel/benches/.gitignore` (BENCH_HISTORY ignored), `.github/workflows/ci.yml:150-168`
  (bench-regression job), `kernel/src/event_log.rs:1-6,127-212` (content-addressing + verify_chain),
  `kernel/src/spectral_cache.rs:233-273` (RetainedBase drift-gate),
  `tools/eqc-rs/tests/cordic_digest.rs` (digest pinning).
- **Prior session artifacts:** `docs/research/AUDIT-2026-07-18-PERFORMANCE-metrics-benchmarks.md`
  (§PERF-03 the 3 fail-open paths, §PERF-05/06 drift + noise), `docs/research/AUDIT-2026-07-18-SYNTHESIS-SCORECARD.md`
  §4.4/§6, `docs/regressions/REGRESSION-LEDGER.md` row 23,
  `docs/design/CORE-ROADMAP-2026-07-17/P-H-audit-telemetry-regression-benchmarks.md`.
- **Commits traced:** `6d7d0e155` (closed python MISSING fail-open), `f3c0687cf` (python comparator
  → thin native wrapper; introduced the CI exit-2 break), `c08ccd28d` (native trackers).
- **External tools (from established knowledge; WebSearch unavailable this session — verify flag
  spellings before wiring):** criterion.rs user guide (`--save-baseline`/`--baseline`, noise_threshold
  / significance_level, bootstrap change-detection); `github.com/BurntSushi/critcmp`;
  `github.com/iai-callgrind/iai-callgrind`; `rustc-perf` / `perf.rust-lang.org` (dedicated hardware,
  `instructions:u`, per-benchmark noise, historical DB + dashboard); Welch's t-test / Mann–Whitney U
  as the significance tests appropriate to timing samples.
```
