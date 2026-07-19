# BLUEPRINT P75 — CI bench-regression gate re-architecture (same-runner criterion A/B) (2026-07-19)

> **Standalone INFRASTRUCTURE blueprint (dowiz kernel/engine CI + telemetry).** One coherent,
> independently buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md`
> §2. Research source: `docs/research/OPUS-PERF-REGRESSION-TOOLING-AUDIT-2026-07-18.md` ("R7") —
> the dedicated deep-audit of `bench_track.py` / `native-trackers` / `baseline.json`. Synthesis
> placement: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.1 finding **A1** (Tier A live bug) +
> §3.3 finding **C5** (Tier C rigor half), Wave **W0** (§5). Sequence context:
> `MASTER-STATUS-LEDGER-2026-07-19.md` §3 (P75 = Wave-0 "protection machinery first", the dowiz-lane
> item every other bench blueprint writes baselines into). Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/dowiz` at HEAD, every
> `file:line` re-read live this pass.
>
> **One sentence:** the CI perf gate currently `exit(2)`s on every fresh runner — carrying **zero**
> regression signal while reporting a green-adjacent "broken" state that invites permanent fail-open
> `|| true` — so P75 replaces the cross-host absolute-baseline comparison with a **same-runner
> criterion A/B** (`--save-baseline` at the merge-base → again at HEAD → a thin significance-aware
> exit-code parser), which closes **both** the exit-2 break **and** the still-open rejected
> cross-host design (fail-open path "#c") in one move, and **owns** the bench-id / baseline /
> threshold / exit-code schema that P80, P81, and P82 cite and never redefine.

---

## VERDICT (stated up front)

**GO — no operator pre-ruling required (ledger §4 confirms P75 needs none).** This is a genuine live
bug on the *protection machinery itself*, not a speculative optimisation, so the usual
"measure-first NO-GO" gate does not apply — the deliverable is a **working gate**, and its proof is
the falsifiable **RED-on-injected-regression** demonstration (§8), not a speedup number. Two honesty
calibrations bound the scope:

1. **URGENT — the A1 half (the exit-2 break) is the whole reason this is Wave 0.** It is a
   deterministic break provable by reading two files (`ci.yml` + `bench_track.py`, both re-verified
   §0); it must be fixed before ~40 new baselines from P80/P81/P82 are written into a schema whose
   gate does not run.
2. **HARDENING — the C5 half is real rigor debt, and one sub-item is honestly host-conditional.**
   Consuming criterion's significance verdict, per-bench thresholds, `sample-size > 10`, and committed
   trend storage are all sound and cheap. The **iai-callgrind instruction-count lane** is the one
   sub-item that is *not* free: it adds a new dev-dependency (⇒ a DECART report, `ci.yml:256` gate)
   and needs `valgrind` on the runner; it is worth building **only** for the handful of ns-scale
   benches PERF-06 measured as ungateable on wall-clock (±75% swings) — **not** a blanket adoption.

**One sequencing note (not a P75 blocker):** OD-2 in the ledger (push/merge of
`perf/contention-bench-2026-07-18`) interacts with P75 — the contended benches (`kernel/benches/
contention.rs`) already exist on that branch and, when merged, **must register into P75's schema
(§3), not be re-specified** (P90 owns their content). If that branch merges *before* P75 lands, its
benches have no gate schema until P75 does; if *after*, they slot in cleanly. Either order is safe.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**
> (`/root/dowiz`, HEAD), not inherited from R7's or the synthesis's line numbers. Where R7's evidence
> is derived rather than observed, that is stated explicitly (item 6 honesty).

### 0.1 The gate cannot execute — `exit(2)` on every fresh runner (finding A1)

| Element | Cite | State |
|---|---|---|
| CI job `bench-regression` | `.github/workflows/ci.yml:150-169` | `runs-on: ubuntu-latest`, on every push/PR; `fetch-depth: 0` is **already set** (`:155-156`) — the merge-base checkout M1 needs is already available. |
| the only run step | `ci.yml:159-162` | `cd kernel && python3 benches/bench_track.py --threshold 10` — **no `--build-native`**, and the job has **no `cargo build`** for `native-trackers`. |
| `bench_track.py` is a thin wrapper | `kernel/benches/bench_track.py:1-23` (docstring), `:66` (`bin_path = _find_native()`) | since `f3c0687cf` it does **no** parsing itself; it only locates + delegates to the Rust `native-trackers` binary. |
| the `exit(2)` path | `bench_track.py:77-84` | if `native-trackers` is not on `PATH` **and** not at `tools/telemetry/native-trackers/target/release/native-trackers` (which `target/` being git-ignored guarantees on a clean checkout) **and** `--build-native` was not passed → `sys.exit(2)`. |
| `--build-native` exists but is unused by CI | `bench_track.py:62-63,69-76` | the flag *would* build the binary, but `ci.yml:162` does not pass it. So on `ubuntu-latest` the step exits 2 before comparing anything. |
| the breaking commit | `git log f3c0687cf` (verified: title *"fix(bench): native-trackers parse bug + bench_track.py → native wrapper"*, 2026-07-18) | it **deleted the python fallback comparator** (that previously let the job run on a fresh checkout) and did **not** update `ci.yml` to build the native binary. `git show --stat` confirms it touched `bench_track.py` (−184 lines net) + `native-trackers/src/main.rs`, **not** `ci.yml`. |

**Honest evidence note (carried from R7 §2A):** the "job exits 2 on every run" conclusion is
**derived from the job definition + wrapper source** (both unambiguous, both re-read here), **not**
from an observed CI log — `gh` returns 404 for this repo's Actions under current auth, so no run was
inspected remotely. The derivation is sound because the exit-2 path is deterministic on a clean
runner, but it is derivation, not observation, and is labelled as such.

**Consequence (why this is Tier A, not Tier C):** a gate that is red-for-the-wrong-reason on every
run is indistinguishable from broken — it carries **no** regression signal, and it is exactly the
shape that invites a `|| true` / `continue-on-error: true` "fix" that flips it to **permanently
fail-open**. That is a worse state than no gate, because it *looks* like protection.

### 0.2 What the comparator actually does today (the design being replaced)

`native-trackers::cmd_bench` (`tools/telemetry/native-trackers/src/main.rs:143`+, re-read this pass):

| Fact | Cite | Detail |
|---|---|---|
| statistically thin run | `main.rs:~175-181` | `cargo bench` with `--warm-up-time 1 --measurement-time 2 --sample-size 10` (10 = criterion's *minimum*). |
| raw mean-vs-mean delta | `main.rs:~263` | `let delta = (cmean - bmean) / bmean * 100.0;` — scrapes criterion's reported **mean** (`parse_timing`, `main.rs:~166`), **discards** criterion's own bootstrap CI + change-significance. |
| flat single threshold | `main.rs:~264` | `if delta > threshold { "REGRESS" }` — one flat 10% band for **all** benches. |
| MISSING is fail-closed (good) | `main.rs:~119-120` | a baseline key with no matching bench → `worst = (threshold+1).max(worst)` → exit 1. This is correct and P75 **keeps** it. |
| cross-host absolute comparison (fail-open path #c, STILL OPEN) | `main.rs` baseline load + `ci.yml:152` | it compares the committed `baseline.json` (measured on the **Hetzner** host) against numbers measured on the **foreign `ubuntu-latest`** runner — an absolute gate across a host constant that is either false-RED or (widened) gates nothing. |
| exit-code contract today | `bench_track.py:16-17` (docstring) | `0 ok · 1 regression · 2 usage/IO error`. |

### 0.3 The committed baseline + bench IDs — the `<group>/<variant>` convention already exists de-facto

- `kernel/benches/baseline.json` — **13 keys**, flat `{"<bench-id>": mean_ns}` (verified:
  `"absorbing/fundamental_matrix_16": 26655`, `"attention/matmul_8x8": 1278`, …). git-**tracked**.
- `kernel/benches/criterion.rs` — **13** `c.bench_function("…")` IDs (verified `:17`–`:246`), one per
  baseline key: `place_order/5_items`, `fold_transitions/5_hops`, `empirical_identify/20k_samples`,
  `empirical_identify/end_to_end_20k`, `token_bucket/try_acquire_permit`,
  `spectral_cache/slem_cached_10x10_hit`, `spectral_cache/canonical_address_32x32`,
  `graph_rebuild_rank/heap`, `graph_rebuild_rank/arena`, `ppr/rank_32x32_k20`,
  `absorbing/fundamental_matrix_16`, `retrieval/recall_at_k_5`, `attention/matmul_8x8`.
- **Key finding:** every ID is already `"<group>/<variant>"`, and the variant *usually* encodes the
  fixed shape/size (`_16`, `32x32_k20`, `20k`, `5_hops`). P75 does **not** invent a convention — it
  **formalises the one already in use** and pins the forward rule for *sweep* benches (`<group>/<n>`).
- `[[bench]]` wiring: `kernel/Cargo.toml:146-148` declares exactly **one** bench target
  (`name = "criterion", harness = false`). A second target (e.g. `iai`, or the branch's `contention`)
  needs its own `[[bench]]` stanza.

### 0.4 Trend storage + the "noisy benches stay probes" principle already on record

- `kernel/benches/.gitignore:1-2` — `BENCH_HISTORY.md` **and** `_cur.json` are git-ignored (by design:
  "so the repo doesn't churn on every run"). CI only uploads `BENCH_HISTORY.md` as a **failure**
  artifact (`ci.yml:163-169`). So there is **no committed historical trend** today (C5 sub-item).
- A `bench.jsonl` feed **already has a real consumer**: `tools/ops-alert/bench-drift:17,65` reads
  `tools/telemetry/logs/bench.jsonl` and compares each bench's rolling window. Per the synthesis /
  memory it went silent 2026-07-15. Reviving that scheduled feed is the low-churn trend option (§9),
  strictly preferable to committing per-CI-run absolute numbers.
- **The "gateable: false" idea is already established policy**, not a P75 invention: `ci.yml:144-149`
  comment — *"Deterministic kernel benches only — harness/LLM benches stay probes (pass/fail), NOT
  baseline-gated (host/noisy variance)."* P75 promotes this from a prose exemption to a per-bench
  manifest field (§3.3).

### 0.5 What is NOT present (so nothing is double-counted)

- `iai-callgrind` — **absent** (grep of `kernel/Cargo.toml`, `tools/` empty). New dep ⇒ DECART report
  (`ci.yml:256-264` `decart-dep-lint` gate). Verify its API before wiring (R7 caveat, §0.6).
- `critcmp` — **absent** (grep of `*.toml`, `*.yml`, `*.rs` empty). It is a `cargo install` dev tool
  (BurntSushi), not a crate dependency, but it is not installed in any job today.
- `kernel/benches/contention.rs` — **not on main**; exists only on `perf/contention-bench-2026-07-18`
  (verified `git ls-tree`). It is a second bench target that must register into P75's schema on merge
  (OD-2 note above).

### 0.6 Epistemic caveat inherited verbatim from R7 (do not launder it away)

R7's external-tool claims (criterion's `--save-baseline`/`--baseline` A/B + significance,
`critcmp base new`, `iai-callgrind` instruction counts, `rustc-perf` per-bench noise) were from
**established knowledge with the session's WebSearch budget exhausted** — R7 itself flagged *"verify
exact flag spelling against the pinned criterion 0.5 before wiring."* This blueprint treats those
flag names as **"verify-before-relying"** (called out again at each build item), and pins the
in-repo facts (§0.1–§0.5, all live-verified) as the load-bearing ground truth.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

Each row is a real, standard construction and exactly how P75 uses it — and what it deliberately does **not** take.

| Prior art | What it is | How P75 uses it — and what it does NOT take |
|---|---|---|
| **criterion `--save-baseline <name>` / `--baseline <name>`** | criterion's native A/B: save a named run, compare a later run against it **with bootstrap significance** (regressed/improved/no-change vs `noise_threshold`/`significance_level`). | **Adopt as the core.** Save `base` at the merge-base and `pr` at HEAD **on the same runner** so the host constant cancels (§5). **NOT taken:** criterion's *reporting-only* posture — it never sets a non-zero exit, so P75 adds the thin exit-code parser (M2) on top. |
| **`critcmp base pr` (BurntSushi)** | reads two saved criterion baselines, prints a ratio table (optionally JSON). | **Adopt as the comparison surface** (a stable, greppable ratio table / JSON). **NOT taken:** re-implementing the *statistics* — the whole R7 finding is that the homegrown delta *discards* criterion's stats. Prefer reading criterion's own `estimates.json`/`change/estimates.json` (M2) so significance is consumed, not re-derived. |
| **`iai-callgrind` (callgrind instruction counts)** | near-deterministic instruction/cache counts, immune to the ±75% wall-clock host swings PERF-06 measured. | **Adopt narrowly** for the ns-scale benches that cannot support a wall-clock gate (M5). **NOT taken:** blanket adoption — it is a new dev-dep (DECART) + needs valgrind; scoped to the specific `fold_transitions`-class benches, everything else stays wall-clock. |
| **`rustc-perf` / perf.rust-lang.org** | the reference at-scale design: dedicated hardware, `instructions:u` primary metric, per-benchmark historical noise (not one flat %), committed historical DB. | **Adopt the *lessons*:** same-runner/dedicated-host comparison, deterministic metric for the deterministic kernel benches, **per-bench** thresholds, committed trend. **NOT taken:** a database/dashboard service — out of scope for a single-box sovereign repo; the `bench.jsonl` feed (§9) is the minimal trend. |
| **The existing `native-trackers` absolute comparator** | zero-dep Rust: run cargo bench, parse, diff vs committed `baseline.json`, gate. | **Keep it — but confine it** to the local **Hetzner absolute-tracking cron** (built where invoked, same host as `baseline.json` was measured on), where cross-host noise does not apply (M6). **NOT taken:** calling it in CI on a foreign runner (that *is* fail-open path #c). |
| **The `bench.jsonl` feed + `tools/ops-alert/bench-drift` consumer** | an append-only rolling-window drift consumer that already exists. | **Revive it** as the committed/scheduled trend (§9). **NOT taken:** committing per-CI-run absolute numbers (churn; `BENCH_HISTORY.md` is git-ignored for exactly this reason, §0.4). |
| **The `ci.yml` fence pattern** (`no-courier-scoring`, `no-pub-raw-matrix-hash`, `fence-check`) | grep/assert CI jobs that turn a bug-class regression into a hard RED. | **Adopt** for the anti-fail-open fences (M2/M7): a workflow-lint that RED-s if the bench step carries `|| true`/`continue-on-error`, or if CI calls the absolute comparator (§7). |

---

## 2. Scope — what P75 OWNS vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P75 OWNS — and is the single-owner authority for

1. **The bench-id schema** — the canonical `<group>/<variant>` form and the forward **sweep**
   convention `<group>/<n>` (§3.1). P80/P81/P82 (and the merged `contention.rs`) name benches by this
   rule; they never define their own.
2. **The baseline + threshold manifest format** (§3.2) — the on-disk shape of `baseline.json` (v2),
   what a per-bench record contains (metric, absolute reference, per-bench threshold, min-sample-size,
   `gateable` bit, sweep axis), and the non-breaking migration from today's flat map.
3. **The gate semantics + exit-code contract** (§3.3) — the total function from a comparison result to
   an exit code `{0,1,2,3}`, and the invariant that `{2,3}` are **hard RED, never a skip, never
   silenceable**.
4. **The same-runner A/B CI job** (M1) and **the significance-aware exit-code parser** (M2).
5. **The confinement of `native-trackers` absolute comparison to the Hetzner cron** (M6) + the fences
   that keep CI from ever calling it again (M7).
6. **The committed trend-storage decision** (§9) — revive the scheduled `bench.jsonl` feed; keep
   `BENCH_HISTORY.md` git-ignored.
7. **The iai-callgrind instruction-count lane** for the ns-scale ungateable benches (M5).

### 2.2 P75 does NOT own (anti-scope — prevents collision & scope-creep)

- **The *content* of any bench** — P75 adds **no** new `bench_function` bodies. The `money_ledger`
  tripwire, the PQ lane, the spectral/mesh/geo sweeps (P80), the engine harness (P81), the bebop
  sign/KEM/AEAD lanes (P82), and the branch's `contention.rs` benches (P90) all belong to their
  blueprints. P75 provides the *slots* (schema + gate); it does not fill them.
- **Re-specifying the contention benches** — `kernel/benches/contention.rs` already exists on
  `perf/contention-bench-2026-07-18` (P90 owns it). P75 only guarantees it a schema home on merge.
- **The golden state-digest / behavioural regression gate** — that is the *state* analogue (R7 §5,
  D-1, reserved P84, operator-gated OD-7). P75 is the *timing/perf* gate only; the two are siblings,
  not the same job.
- **The engine/bebop `[[bench]]` wiring** for their own new targets — P81/P82 add those; P75 only
  fixes the kernel gate and defines the schema they conform to.
- **Any operator-gated crypto/money bench decision** — none here; P75 touches CI tooling, not a
  red-line surface.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `ci.yml` `bench-regression` job (M1 rewrites it), `bench_track.py`
(M1 removes it from the CI path), `native-trackers::cmd_bench` (M2 extends it with a `compare`
subcommand; M6 confines its `bench` subcommand), `baseline.json` (M3 migrates it), `criterion.rs`
bench IDs (M3 grandfathers them), `kernel/Cargo.toml` `[[bench]]` (M5 adds the iai target),
`tools/ops-alert/bench-drift` + `tools/telemetry/logs/bench.jsonl` (M6/§9 revive the feed).

**Consumers (they cite this blueprint, never redefine it):** **P80** (kernel bench expansion — HARD:
~30 new baselines into P75's schema + working gate), **P81** (engine harness — HARD), **P82** (bebop
expansion — HARD; also feeds P92's D-BENCH measure-first gate), **P77/P79** (kernel algo/layout fixes
— SOFT: their red→green benches land in P75's schema), **P90** (contention benches register into the
schema on merge). Per ledger §3, P80–P82 are HARD-blocked on P75; P77/P79/P90 are SOFT.

---

## 3. Predefined types, constants & schemas — named BEFORE implementation (standard §2 item 4)

These four artifacts **are** the single-owner contract. Everything downstream references them by name.

### 3.1 The bench-id schema (`BenchId`)

```
bench-id  ::=  <group> "/" <variant>
<group>   ::=  snake_case logical hot-path family        e.g. money_ledger, mesh_verify, spectral_math
<variant> ::=  <sweep-n> | <fixed-shape>
<sweep-n> ::=  a bare non-negative integer = the primary scaling-axis value   e.g. 2, 8, 64, 256
              (multi-axis: "<a>x<b>" e.g. 128x128, or "<n>_<tag>" e.g. 32x32_k20 — but the FIRST
               token MUST be the dominant axis size so critcmp + the trend sort numerically)
<fixed-shape> ::= a descriptive token for a NON-swept single point           e.g. try_acquire_permit
```

**Rules (binding on P80/P81/P82/P90):**
- One criterion **group** = one logical hot path. One **variant** = one measured point on its axis.
- **New sweep benches use `<group>/<n>`** (bare integer variant) — this is the "sweep-size convention"
  the synthesis names. Example: P80's money tripwire is `money_ledger/2`, `money_ledger/8`,
  `money_ledger/64`, `money_ledger/256`.
- **The 13 existing IDs are grandfathered** (§0.3): they name fixed shapes, keep their current
  variants; their size stays encoded in the variant (`fundamental_matrix_16`, `20k_samples`). No
  rename churn.
- A bench-id is the **join key** across `criterion.rs` (`bench_function` name), `baseline.json` (v2
  manifest key), the criterion output dir (`target/criterion/<group>/<variant>/`), and the trend feed.

### 3.2 The baseline + threshold manifest (`baseline.json` v2)

Today: flat `{"<bench-id>": mean_ns}`. P75 evolves it **non-breakingly** to a per-bench record. A bare
number is still accepted and read as the defaults, so migration is incremental and the existing 13
keep working unchanged until touched.

```jsonc
// kernel/benches/baseline.json  (v2 — per crate; committed)
{
  "$schema_version": 2,
  "money_ledger/2": {
    "metric":          "wall_ns",     // "wall_ns" | "instructions"
    "absolute_ns":     41.0,          // Hetzner-host REFERENCE — used ONLY by the local cron (M6),
                                      //   NEVER the CI gate (CI is same-runner A/B, host-relative)
    "threshold_pct":   10.0,          // per-bench regression band for the A/B ratio
    "min_sample_size": 100,           // >10 for sub-100ns benches (C5); criterion --sample-size floor
    "gateable":        true,          // false => measured + trended, NOT gated (noisy wall-clock)
    "sweep_axis":      "ledger_len"   // documents the scaling axis (standard §2 item 8)
  },
  "fold_transitions/5_hops": {
    "metric":          "instructions",// ns-scale (~4ns), ungateable on wall-clock (PERF-06 ±75%) =>
    "threshold_pct":   1.0,           //   gated on iai instruction count with a TIGHT band (M5)
    "gateable":        true,
    "sweep_axis":      "hop_count",
    "wall_absolute_ns": 4.2           // kept for the cron's wall-clock trend, NOT the gate metric
  },
  "attention/matmul_8x8": 1278        // <- legacy bare number still valid: {wall_ns, absolute=1278,
                                      //    threshold_pct=DEFAULT_THRESHOLD_PCT, gateable=true}
}
```

Backward-compat parse rule (M2/M3): `parse_baseline` (native-trackers) is extended — a JSON **number**
`N` decodes to `BenchRecord { metric: WallNs, absolute_ns: N, threshold_pct: DEFAULT_THRESHOLD_PCT,
min_sample_size: 10, gateable: true, sweep_axis: None }`; a JSON **object** decodes field-by-field.

### 3.3 The gate semantics + exit-code contract (the enum P80–P82 cite)

```rust
// tools/telemetry/native-trackers/src/main.rs — the P75-owned exit contract (formalised)
// (return i32 from `cmd_compare`; the CI step forwards it unchanged)
pub enum GateExit {
    Pass       = 0, // every GATEABLE bench within its per-bench threshold AND (where criterion
                    //   emits it) no statistically-significant regression. THE ONLY green.
    Regress    = 1, // >=1 gateable bench regressed beyond its threshold; if significance is
                    //   available the change must be BOTH over-threshold AND significant.
    Harness    = 2, // could not run/parse: build failed, empty bench output, merge-base == HEAD,
                    //   or checkout failed. HARD RED. NEVER a pass. NEVER silenceable with `|| true`.
    SchemaDrift= 3, // a committed bench-id has no manifest record, OR a manifest record has no bench
                    //   (bench-set drift). Fail-CLOSED RED (preserves today's MISSING behaviour).
}
```

**Invariants (the anti-fail-open core, enforced by tests + fences):**
- `Pass` (0) is the **only** green. Any of `{Harness, SchemaDrift}` is RED — a gate that *cannot run*
  or whose *schema drifted* must never look like "no regression."
- The CI step **must not** carry `|| true` or `continue-on-error: true` (M7 fence RED-s if it does).
- A **statistically-significant** regression (from criterion's `change/estimates.json`) flags even
  when the point-estimate delta is under the flat band — significance is consumed, not discarded (M4).
- Named constant: `DEFAULT_THRESHOLD_PCT = 10.0` (matches today's `--threshold 10`); per-bench
  `threshold_pct` overrides it.

### 3.4 The bench-target registry (`[[bench]]` wiring)

P75 pins that a crate may declare multiple bench targets, each `harness = false`, each producing
`<group>/<variant>` IDs into `target/criterion/`. Kernel today: `criterion` (§0.3). P75 adds `iai`
(M5). On merge, the branch's `contention` target (P90) is a third — it needs its own `[[bench]]`
stanza + baseline records, both conforming to §3.1/§3.2.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a test that goes RED before the change, code, then GREEN.** Because this is
CI/tooling, most RED tests are **unit tests of the parser over committed fixtures** (deterministic,
host-independent, fast) plus **workflow-lint fences** — not live bench runs. That is deliberate: the
gate's correctness must itself be gated by something that does not depend on the noisy host.

### 4.1 M1 — the same-runner A/B CI job (fixes the exit-2 break AND fail-open path #c)

- **Spec:** rewrite the `bench-regression` job (`ci.yml:150-169`). It already has `fetch-depth: 0`.
  New steps, all on **one** runner:
  1. `BASE=$(git merge-base origin/main HEAD)`; **assert `BASE != HEAD`** (else `GateExit::Harness`).
  2. `git checkout $BASE`; `cargo bench --bench criterion -- --save-baseline base`
     (verify flag spelling vs pinned criterion 0.5 before wiring — §0.6).
  3. `git checkout -` (back to HEAD); `cargo bench --bench criterion -- --save-baseline pr`.
  4. `native-trackers compare base pr --manifest kernel/benches/baseline.json` (M2) → exit per §3.3.
  5. on failure, upload the criterion `change/` reports + a `critcmp base pr` table as the trend
     artifact (replaces the git-ignored `BENCH_HISTORY.md` upload).
  - **Remove** the `python3 benches/bench_track.py` call from CI entirely — CI no longer touches the
    absolute comparator (that moves to the cron, M6). This is the "one move closes both" step: the A/B
    is same-runner (kills #c) and needs no unbuilt binary (kills exit-2).
- **RED `red_ci_step_has_no_fail_open`:** a workflow-lint (new `ci-meta` job or extend `fence-check`)
  greps the `bench-regression` job body for `|| true`, `continue-on-error`, or a call to
  `native-trackers bench`/`bench_track.py`; any hit → RED. RED today would pass only because the step
  *is* broken; after M1 the lint asserts the *new* step stays honest.
- **RED `red_merge_base_equals_head_is_harness`:** parser fixture where `base` and `pr` dirs are
  identical *and* a sentinel marks "no merge-base advance" → exit `2`, not `0`. Prevents a shallow /
  first-commit clone silently comparing HEAD to itself and passing.
- **Adversarial `red_regressed_bench_makes_job_red` (the centerpiece, §8):** on a throwaway branch,
  inject a `std::thread::sleep`/extra work into one `criterion.rs` bench body; the real job must exit
  `1`. Machine-checkable proxy: the M2 fixture pair `(base, regressed)` → exit `1` in the parser
  self-test job (`bench-gate-selftest`), which runs on every PR.

### 4.2 M2 — the significance-aware exit-code parser (`native-trackers compare`)

- **Spec:** add a `compare <base> <pr> --manifest <path>` subcommand to `native-trackers`
  (reuse-first, standard §19 — extend the existing zero-dep binary, do **not** add a new one). For
  each bench-id present in both saved baselines: read criterion's emitted per-bench data —
  `target/criterion/<id>/<baseline>/estimate.json` (point estimate + CI) and, when present,
  `target/criterion/<id>/change/estimates.json` (the bootstrap change estimate + significance). Apply:
  regression iff `ratio > 1 + threshold_pct/100` **and** (if a change estimate exists) the change is
  statistically significant. Emit `GateExit` per §3.3. `critcmp` output is an acceptable alternate
  input surface (parse its JSON) if reading criterion's dirs proves brittle across the pinned version
  — decide at build time, document which was chosen.
- **RED `red_compare_clean_is_pass`:** fixture `(base, pr)` with identical estimates → exit `0`.
- **RED `red_compare_over_threshold_and_significant_is_regress`:** fixture where one bench is +50% with
  a non-overlapping CI → exit `1`.
- **RED `red_compare_over_threshold_but_insignificant_is_pass`:** fixture where a ns-scale bench shows
  +30% mean but the change estimate is **not** significant (wide overlapping CIs) → exit `0`. This is
  the whole C5 point: **consume significance**, don't fire on a noisy point-estimate.
- **RED `red_compare_significant_under_flat_band_still_flags`:** a small but statistically-significant
  regression (tight CIs, +4% but clearly significant on a `threshold_pct: 1.0` iai bench) → exit `1`.
- **RED `red_compare_missing_baseline_is_schema_drift`:** a bench in `pr` with no manifest record (or a
  manifest key with no bench) → exit `3` (fail-closed), never `0`. Preserves today's MISSING behaviour
  (`main.rs:~119`) under the new contract.
- **Adversarial `red_compare_empty_output_is_harness`:** `base` dir empty (bench didn't run) → exit
  `2`, never `0`. A non-run is never a pass.

### 4.3 M3 — the schema + non-breaking baseline migration

- **Spec:** implement §3.1/§3.2. Extend `parse_baseline` to accept both the bare-number and the object
  form (backward-compat rule §3.2). Add a `bench-schema-lint` CI job: every `bench_function` ID in
  `criterion.rs` must match the `<group>/<variant>` grammar and have a manifest record (or be a
  grandfathered bare number); every manifest key must have a bench. Drift → RED (`GateExit::SchemaDrift`
  semantics at lint time).
- **RED `red_schema_lint_rejects_orphan_bench`:** add a `bench_function("foo/1")` with no manifest
  record → the lint fails. Proves a new bench cannot dodge the gate by simply not having a baseline
  (the bench-set-drift fail-open).
- **RED `red_schema_lint_rejects_orphan_baseline`:** add a manifest key with no matching bench → fails.
  Proves a bench cannot be deleted to shed its baseline silently.
- **RED `red_legacy_bare_number_still_parses`:** the 13 existing bare-number keys load as their
  default records and the gate runs — migration is non-breaking.
- **Adversarial `red_bad_sweep_variant_rejected`:** a sweep bench named `money_ledger/big` (non-numeric
  first token where a sweep is declared) → lint fails; the convention is enforced, not advisory.

### 4.4 M4 — consume criterion significance + fix sample size (C5, hardening)

- **Spec:** M2 already reads `change/estimates.json`; M4 is the explicit obligation to **run enough
  samples** so significance is meaningful — pass per-bench `--sample-size = min_sample_size` (≥100 for
  sub-100ns benches, §3.2) instead of the flat `10`. Wire it in the A/B `cargo bench` invocation.
- **RED `red_subhundred_ns_bench_uses_large_sample`:** assert the invocation for a bench whose manifest
  `min_sample_size = 100` passes `--sample-size 100`. (Guards the "10-sample mean on a 4ns bench is
  meaningless" finding, R7 §1.2.)
- **Honest label:** hardening, cheap, no new dep. Do it.

### 4.5 M5 — iai-callgrind instruction-count lane for the ns-scale ungateable benches (C5, host-conditional)

- **Spec:** add a second bench target `kernel/benches/iai.rs` (`[[bench]] name = "iai", harness =
  false`) covering **only** the benches PERF-06 proved ungateable on wall-clock (start:
  `fold_transitions/5_hops`; extend to any sub-~50ns kernel bench). Metric = callgrind
  `instructions`; these benches' manifest records set `metric: "instructions"` + a **tight**
  `threshold_pct` (e.g. 1.0). The A/B parser gates them on instruction delta (deterministic), not
  wall-clock. Wall-clock for these stays *measured + trended* (cron) but `gateable`-on-wall is false.
- **RED `red_nsbench_gated_on_instructions_not_wall`:** the parser, for an `instructions`-metric bench,
  reads the iai output and ignores its wall-clock number; a +200-instruction regression → exit `1`; a
  ±75% wall-clock swing with 0 instruction delta → exit `0`.
- **DECART obligation (standard §19 + `ci.yml:256`):** `iai-callgrind` is a **new dev-dependency** →
  file a DECART report (honest falsifiable comparison; why the ns-scale gate can't be met otherwise)
  or the `decart-dep-lint` job RED-s. Runner must have `valgrind` (add to the job). **Verify the
  `iai-callgrind` bench-macro API against its current release before wiring** (§0.6).
- **Honest label:** hardening, **host-conditional** — only worth it for the specific ns-scale benches;
  do **not** convert the whole suite. If `valgrind` on the runner proves impractical, this item may be
  deferred with the ns-scale benches left `gateable: false` (measured, not gated) — that is a valid,
  honest fallback, not a failure.

### 4.6 M6 — confine `native-trackers` absolute comparison to the Hetzner cron

- **Spec:** the `native-trackers bench <crate>` path (absolute vs committed `baseline.json`) is
  **legitimate for one role only**: the local Hetzner absolute-tracking cron, where the runner **is**
  the host `baseline.json` was measured on, so cross-host noise does not apply. (a) Document a cron
  that runs `native-trackers bench kernel --threshold 10` on the Hetzner box and appends to
  `tools/telemetry/logs/bench.jsonl` (§9). (b) Add a guard so the absolute path is a **no-op / hard
  error in CI**: e.g. `cmd_bench` refuses to run when `CI=true` (GitHub sets it) unless
  `--allow-ci-absolute` is passed, printing "absolute comparison is cron-only; CI uses `compare`".
- **RED `red_absolute_bench_refuses_in_ci`:** invoke `native-trackers bench` with `CI=true` and no
  override → non-zero + the cron-only message. Prevents anyone re-pointing CI at the cross-host gate
  (re-opening fail-open path #c).
- **RED `red_ci_does_not_call_absolute` (fence, overlaps M7):** the workflow-lint asserts no CI job
  calls `native-trackers bench` or `bench_track.py`.

### 4.7 M7 — anti-fail-open fences (the smart index for this bug class, standard §2 item 14)

- **Spec:** turn the three fail-open vectors into **CI-time** RED, not runtime surprises, mirroring the
  existing `no-courier-scoring`/`fence-check` pattern (§1):
  1. **`|| true` / `continue-on-error` on the bench job** → RED (a grep fence over the workflow file).
  2. **CI calling the absolute comparator** (`native-trackers bench` / `bench_track.py`) → RED (M6).
  3. **A per-bench threshold widened without review** → made *visible*: thresholds live in the
     committed manifest, so loosening one is a reviewable diff (not a hidden flag). Optionally a fence
     that RED-s if any `threshold_pct > CEILING` (e.g. 25) without an inline justification marker.
- **RED `red_fence_catches_injected_fail_open`:** add `|| true` to the bench step on a test branch →
  the fence job RED-s. This is the exact `f3c0687cf`-class failure turned into a compile-time-equivalent
  guard.

---

## 5. The gate lifecycle in full (the CI job shape the DoD proves)

```yaml
# .github/workflows/ci.yml  — bench-regression (rewritten by M1). Same-runner A/B.
bench-regression:
  name: bench regression (kernel hot paths, same-runner A/B)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
      with: { fetch-depth: 0 }                 # already present (§0.1) — merge-base is reachable
    - name: Pre-fetch crates offline
      run: cargo fetch --manifest-path kernel/Cargo.toml
    - name: Build the gate binary (native-trackers) ONCE, on this runner
      run: cargo build --release --manifest-path tools/telemetry/native-trackers/Cargo.toml
    - name: Bench the merge-base
      run: |
        BASE=$(git merge-base origin/main HEAD)
        test "$BASE" != "$(git rev-parse HEAD)" || { echo "::error::merge-base==HEAD"; exit 2; }
        git checkout --quiet "$BASE"
        (cd kernel && cargo bench --bench criterion -- --save-baseline base)   # verify flags §0.6
        git checkout --quiet -
    - name: Bench HEAD + gate (same runner, host constant cancels)
      run: |
        (cd kernel && cargo bench --bench criterion -- --save-baseline pr)
        native-trackers compare base pr --manifest kernel/benches/baseline.json  # exit 0/1/2/3 (§3.3)
    - name: Upload A/B change report on regression (trend artifact)
      if: failure()
      uses: actions/upload-artifact@v4
      with: { name: bench-ab-report, path: kernel/target/criterion/**/change, if-no-files-found: warn }
```

Why this closes both defects at once (R7 §6 item 1): both `base` and `pr` are measured on the **same
`ubuntu-latest` instance**, so the foreign-host constant cancels (kills #c); and nothing depends on a
pre-built `native-trackers` absolute path (the build step is explicit, and `compare` reads criterion's
own dirs — kills the exit-2). The absolute `baseline.json` numbers are untouched by CI; they serve the
cron (M6) only.

---

## 6. What stays on the Hetzner cron (the absolute-tracking role — kept, not deleted)

The absolute comparison is **not wrong** — it is wrong *in CI*. On the Hetzner box the runner is the
same host the baseline was measured on, so `native-trackers bench kernel --threshold 10` is a valid,
low-noise **absolute drift** monitor. P75 keeps it there and feeds its output into the committed trend
(`bench.jsonl`, §9). This is the "confine, don't kill" half of the A1 fix: CI gets the host-relative
A/B gate; the cron keeps the absolute-host trend. Neither steps on the other.

---

## 7. Adversarial self-check — ways the gate can silently fail-open or be gamed (standard §2 items 3, 5)

This is the heart for an infra blueprint: not "can an attacker forge a frame" but **"can a regression
slip through green, or can a dev make red go away without fixing the perf?"**

| # | Fail-open / gaming vector | Defence (and where) |
|---|---|---|
| 1 | **`\|\| true` / `continue-on-error` on the step** (the classic response to a noisy/broken gate) | M7 fence RED-s on it; and the *reason* devs reach for it (a gate that's red-for-the-wrong-reason) is removed by M1 — a working gate is not one people want to silence. |
| 2 | **Exit 2 treated as pass** (today's exact bug) | §3.3 contract: `{2,3}` are RED. M1's build step + `merge-base != HEAD` assert make "couldn't run" a hard RED, not a green. |
| 3 | **A regression hides under a too-loose per-bench threshold** | M4: criterion **significance** flags a statistically-significant regression even under the flat band. And thresholds are committed (M7) so loosening one is a visible, reviewable diff — not a hidden `--threshold 50`. |
| 4 | **A flaky ns-scale bench → false RED → dev widens threshold to shut it up → gate rots** | M5: ns-scale benches move to iai **instruction counts** (deterministic) and keep a tight band, so they don't flap; ones that genuinely can't be stabilised are marked `gateable: false` **explicitly** (measured, not gated) rather than silently defanged. |
| 5 | **A new hot bench added with no baseline → invisible to the gate** | M3 `bench-schema-lint`: an orphan bench (no manifest record) RED-s. A bench cannot enter the tree ungated. |
| 6 | **A bench deleted to shed its failing baseline** | M2/M3 fail-closed `SchemaDrift` (exit 3) on an orphan baseline key (preserves today's MISSING behaviour). |
| 7 | **Cross-host noise re-creeps in** (someone re-points CI at the absolute comparator) | M6 `red_absolute_bench_refuses_in_ci` + M7 fence: CI calling `native-trackers bench`/`bench_track.py` RED-s. |
| 8 | **Shallow clone → merge-base == HEAD → HEAD-vs-HEAD always passes** | M1 `red_merge_base_equals_head_is_harness`: asserted → exit 2. |
| 9 | **The gate's own parser is wrong (a bug in the bug-detector)** | The parser is gated by its **own** fixture self-test job (`bench-gate-selftest`, M2 REDs), host-independent and RED-able — the gate that guards the gate. |
| 10 | **P80/P81/P82 invent their own schema, fragmenting the contract** | §2.1 single-owner + M3 schema-lint enforce `<group>/<variant>` repo-wide; a non-conforming bench-id RED-s regardless of which blueprint added it. |

**Honestly-stated residual:** on a **new** runner image or a criterion version bump, the *absolute*
cron numbers may shift (they are host-bound); the CI A/B gate is immune (host cancels), but the cron's
`bench.jsonl` trend has a discontinuity at the image/version change. Mitigation: record the runner
image + criterion version alongside each cron sample so a discontinuity is attributable, not mistaken
for a regression. This is a trend-readability caveat, not a gate-correctness hole.

---

## 8. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

**Centerpiece falsifier:** *inject a known regression → CI goes RED; HEAD (clean) → GREEN — proven IN
CI, not locally.*

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| **D1 (centerpiece)** | an injected regression makes the `bench-regression` job exit `1`, and clean HEAD exits `0` | live: `std::thread::sleep` in one `criterion.rs` bench on a throwaway branch → job RED (manual acceptance). machine: `bench-gate-selftest` fixtures `(base,regressed)→1`, `(base,clean)→0` on every PR |
| D2 | the CI gate is same-runner A/B and never calls the absolute comparator | `red_ci_step_has_no_fail_open`, `red_ci_does_not_call_absolute`; the job body greps clean |
| D3 | the exit-code contract is total: run/parse failure and schema drift are RED, never a pass | `red_compare_empty_output_is_harness` (2), `red_compare_missing_baseline_is_schema_drift` (3), `red_merge_base_equals_head_is_harness` (2) |
| D4 | criterion significance is consumed, not discarded | `red_compare_over_threshold_but_insignificant_is_pass`, `red_compare_significant_under_flat_band_still_flags` |
| D5 | per-bench thresholds + sample-size are honoured | `red_subhundred_ns_bench_uses_large_sample`; a manifest `threshold_pct` override changes the verdict in a fixture |
| D6 | the bench-id schema is enforced repo-wide; drift RED-s | `red_schema_lint_rejects_orphan_bench`, `red_schema_lint_rejects_orphan_baseline`, `red_bad_sweep_variant_rejected` |
| D7 | the legacy bare-number baselines still load (non-breaking migration) | `red_legacy_bare_number_still_parses`; all 13 existing IDs gate under v2 |
| D8 | ns-scale ungateable benches are gated on instructions (or explicitly `gateable:false`), not flaky wall-clock | `red_nsbench_gated_on_instructions_not_wall` (if M5 built); else the manifest marks them `gateable:false` and the parser skips them from the gate while still trending them |
| D9 | the absolute comparator refuses to run in CI (cron-only) | `red_absolute_bench_refuses_in_ci` |
| D10 | a fail-open edit (`\|\| true`) is caught at CI time | `red_fence_catches_injected_fail_open` |
| D-BUILD | `native-trackers` builds; the parser + fixtures pass; the workflow parses | `cargo build --release -p native-trackers`; `cargo test -p native-trackers`; `bench-gate-selftest` green |
| D-NOREG | the other CI jobs (`cargo-test`, `gitleaks`, `supply-chain`, fences) stay green; the kernel bench suite still runs | full `ci.yml` green on a no-op PR |

---

## 9. Benchmarks + telemetry + committed trend storage (standard §2 item 10)

- **The "win" is a working gate, not a speedup** — so the proof is D1 (RED-on-injected-regression),
  per the Performance Standing Rule. No speedup number is claimed or fabricated.
- **Committed trend (C5 sub-item), the low-churn way:** revive the scheduled **`bench.jsonl`** feed
  (`tools/telemetry/logs/bench.jsonl`, consumer `tools/ops-alert/bench-drift` already exists, §0.4).
  The Hetzner cron (M6) appends one absolute-host sample per bench per run; `bench-drift` compares
  rolling windows. **`BENCH_HISTORY.md` stays git-ignored** (committing per-run absolute numbers
  churns the repo — the exact reason it was ignored). Rationale on the record so a future reader does
  not "helpfully" commit it.
- **CI trend artifact:** on a RED, upload criterion's `change/` reports (+ optionally a `critcmp base
  pr` table) — the same-runner **A/B ratios** are the stable quantity to eyeball (≈1.0 baseline), per
  PERF-05's ratio guidance. These are artifacts, not committed (no bot commit-back loop).
- **The gate measures itself:** `bench-gate-selftest` runtime + the A/B job wall-time are themselves
  cheap signals; if the A/B job's *own* duration balloons (double bench run), that is visible in CI
  timing and is an accepted cost (correctness over CI-minutes for a Wave-0 protection item).

---

## 10. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as structure (item 6):** the unsafe state is a **fail-open gate** (a regression
  passing green). It is made unreachable by the **total** exit-code contract (§3.3): there is no input
  — build failure, empty output, missing baseline, HEAD-vs-HEAD, deleted bench — that maps to `0`.
  "Green" is a *positive* verdict (every gateable bench compared and within band), never a *default*.
  Argued from the enum's totality, not a prose assurance.
- **Schemas & scaling axis (item 8):** the bench-id schema's scaling axis is **number of benches** and
  **crates**. `baseline.json` is O(benches) flat JSON — fine to ~hundreds; it would need sharding only
  past ~thousands of benches (far off). The manifest carries each bench's *own* `sweep_axis` so the
  data it gates is self-describing about *its* scaling. Stated, not timeless.
- **Isolation / bulkhead (item 11):** the gate is a **bulkhead** — its failure mode is *block the
  merge* (fail-closed), never *corrupt the repo* or *break a runtime path*. It touches only CI +
  telemetry; a bug in it cannot reach the kernel/engine/money surfaces. The cron's absolute path and
  the CI A/B path are isolated (M6) so one's noise can't red the other.
- **Mesh-networking awareness (item 12):** **N/A, honestly** — this is CI tooling on a single-box
  build; no transport, no gossip, no node-local vs propagated distinction.
- **Rollback/self-healing as math (item 13):** **Self-termination** = the `Harness`/`SchemaDrift` exits
  (a gate that cannot validly run *stops* the merge, it does not guess). **Snapshot re-entry** = the
  A/B is stateless per run — it regenerates `base` from the merge-base each time, so there is no
  persistent state to corrupt or recover. **Self-healing is NOT claimed** (a broken bench needs a human
  fix; there is no error-correcting auto-recovery) — claiming it would be false.
- **Error-propagation / smart index (item 14):** the bug classes this gate's own machinery could
  introduce — a silently-widened threshold, a re-pointed cross-host gate, a `|| true`, an ungated new
  bench — are each turned into **CI-time RED** by M3/M6/M7 fences, mirroring the existing
  `no-courier-scoring`/`fence-check` pattern. Not runtime surprises.
- **Living-memory awareness (item 15):** the committed trend (`bench.jsonl`) **is** a temporal
  series — a per-bench time axis. It is deliberately kept in the append-only telemetry feed with an
  existing rolling-window consumer, **not** re-modelled into the retrieval/living-memory arc (that
  would be over-engineering a CI trend). One-line honest scope: time-series yes, personalized-recall
  no.
- **Tensor/spectral (item 16):** **N/A, honestly** — an exit-code parser + JSON manifest is not a
  linear-algebra kernel; forcing `spectral.rs` here would be ponytail-violating over-engineering.
- **Linux discipline (item 9):** **EXTENDS** the existing `native-trackers` binary (a new `compare`
  subcommand, not a new tool) + the `[[bench]]`/fence patterns; **REINFORCES** fail-closed defaults
  (MISSING → RED) and the "noisy benches stay probes" principle (`ci.yml:144`); **ALREADY-EQUIVALENT**
  on the CI-fence idiom (reuses the `no-courier-scoring` grep-gate shape); **DOES-NOT-TRANSFER** — no
  new daemon, no service, no dashboard (rustc-perf's DB is deliberately not taken, §1).

---

## 11. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (ci.yml/bench_track.py/native-trackers/baseline.json/criterion.rs/.gitignore/bench-drift all re-read; `f3c0687cf` confirmed; R7 derivation-vs-observation flagged) |
| 2 | Falsifiable DoD | §8 (D1 centerpiece = inject-regression→RED, each row a test/check) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; fixtures assert the exit-code *sequence* 0/1/2/3) |
| 4 | Predefined types & constants | §3 (`BenchId`, `baseline.json` v2, `GateExit`, `DEFAULT_THRESHOLD_PCT`, bench-target registry) |
| 5 | Adversarial/breaking tests | §4 (every M has RED adversarial cases), §7 (10 fail-open/gaming vectors) |
| 6 | Hazard-safety from structure | §10 (fail-open unreachable via the total exit-code contract) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §10 (benches/crates axis; per-bench `sweep_axis`) |
| 9 | Linux engineering discipline | §10 (EXTENDS/REINFORCES/ALREADY-EQUIVALENT/DOES-NOT-TRANSFER) |
| 10 | Benchmarks + telemetry | §9 (proof = working gate; revived `bench.jsonl` trend; A/B-ratio artifact) |
| 11 | Isolation / bulkhead | §10 (fail-closed = block merge; CI vs cron isolated) |
| 12 | Mesh awareness | §10 (N/A, honestly — CI tooling, no transport) |
| 13 | Rollback/self-heal as math | §10 (self-termination = Harness/SchemaDrift exits; stateless re-entry; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §10 + §4 M3/M6/M7 (fail-open vectors → CI-time RED fences) |
| 15 | Living-memory awareness | §10 (trend is a time-series in the telemetry feed; not re-modelled into the recall arc — stated) |
| 16 | Tensor/spectral where applicable | §10 (N/A, honestly) |
| 17 | Regression tracking | §12 (REGRESSION-LEDGER entry: the exit-2 fail-open + the gate self-test as the permanent regression) |
| 18 | Clear worker instructions | §12 |
| 19 | Reuse-first, upgrade-if-needed | §1 (adopt criterion/critcmp/iai; extend native-trackers not replace), §2.2 (anti-scope) |
| 20 | Hermetic principles | §12 (Polarity: pass/fail, no middle exit; Cause&Effect: every RED has a named cause; Correspondence: the gate result *is* a function of the two same-runner measurements) |

---

## 12. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-REGRESSION-TOOLING-AUDIT-2026-07-18.md` ("R7") — §1.2 (raw-delta discards
  criterion stats), §2A (the exit-2 break), §2B/§3 (fail-open path #c, cross-host), §4 (rigorous
  approaches: criterion A/B, critcmp, iai-callgrind, rustc-perf lessons; **its own "verify flag
  spelling / WebSearch exhausted" caveat is carried into every build item**), §6 (the concrete
  proposal P75 formalises).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` — §3.1 A1 (Tier A live bug), §3.3 C5 (rigor half), §4
  (P75 needs no operator pre-ruling; the sweep-size convention is P75-owned), §5 W0 (build order:
  P75 first so P80/P81/P82 write into the fixed schema), §6 E4/E12 (SeqCst→Relaxed declined,
  Mutex→CAS bench-first — context for why noisy-bench honesty matters).
- `MASTER-STATUS-LEDGER-2026-07-19.md` — §3 (P75 = Wave 0; HARD-feeds P80/P81/P82, SOFT P77/P79/P90),
  §4 item 1 (P75 owns the bench-id/baseline schema, P80–P82 cite never redefine), §5 **OD-2** (the
  `perf/contention-bench-2026-07-18` push/merge interacts with P75 — the sequencing note in the
  VERDICT), **OD-7** (D-1 golden state-digest = the sibling *state* gate, reserved P84, not P75).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`; contract:
  `CORE-ROADMAP-STANDARD-2026-07-17.md` §2.
- Standing rule: `.claude/CLAUDE.md` Performance Standing Rule (every claimed win carries a bench;
  here the win is a gate, proof = RED-on-injected-regression). Memory:
  `performance-priority-over-minimal-change-2026-07-17.md`, `verified-by-math-2026-07-07.md`
  (ship RED, falsifiable proof), `worktree-remote-push-collision-avoidance-2026-07-18.md` (why the
  contention branch is unpushed — the OD-2 context).

**Existing code this blueprint edits/extends (exact targets, dowiz — NOT bebop):**
- **EDIT** `.github/workflows/ci.yml` — rewrite the `bench-regression` job (`:150-169`) to the
  same-runner A/B (§5, M1); add `bench-gate-selftest` + `bench-schema-lint` + the anti-fail-open fence
  (extend `fence-check` or a new `ci-meta` job) (M2/M3/M7).
- **EDIT** `tools/telemetry/native-trackers/src/main.rs` — add the `compare <base> <pr> --manifest`
  subcommand + `GateExit` (M2); extend `parse_baseline` to the v2 object form (M3); guard the absolute
  `bench` path against `CI=true` (M6). **Reuse the binary — do not add a new tool.**
- **EDIT** `kernel/benches/baseline.json` — migrate to v2 records incrementally; the 13 existing bare
  numbers stay valid until touched (M3).
- **NEW** `kernel/benches/iai.rs` + `[[bench]]` in `kernel/Cargo.toml` — the instruction-count lane for
  ns-scale benches (M5); **file a DECART report** for the `iai-callgrind` dev-dep, add `valgrind` to
  the job.
- **NEW** `kernel/benches/testdata/{base,clean,regressed,missing}/` — committed criterion fixture dirs
  for `bench-gate-selftest` (M2). Host-independent; the gate that guards the gate.
- **REVIVE** the scheduled Hetzner cron → `tools/telemetry/logs/bench.jsonl` (consumer
  `tools/ops-alert/bench-drift` already exists) (§9, M6). **Do NOT** un-ignore `BENCH_HISTORY.md`.
- **DO NOT TOUCH** the *content* of any bench (`criterion.rs` bodies), the engine/bebop benches, or the
  `contention.rs` benches on the branch — P80/P81/P82/P90 own those. P75 provides schema + gate only.

**For the worker with zero session context — exact acceptance path:**
1. **Build M1 + M2 first (the gate itself)** — rewrite the CI job to same-runner A/B and add the
   `compare` subcommand + fixtures. Land `bench-gate-selftest` so the parser is gated by its own
   host-independent tests **before** trusting it on real benches.
2. **Prove D1 in CI, not locally:** on a throwaway branch, inject a `std::thread::sleep` into one
   `criterion.rs` bench body and confirm the real `bench-regression` job exits `1`; revert and confirm
   clean HEAD exits `0`. This is the whole point of the blueprint — do not mark P75 done without it.
3. Add M3 (schema + non-breaking migration + `bench-schema-lint`), M4 (significance + sample-size),
   M6 (confine absolute to cron + `CI=true` guard), M7 (fences). Each M's RED tests fail before its
   code and pass after.
4. **M5 (iai lane) is hardening and host-conditional** — build it only for the ns-scale benches, file
   the DECART report, add `valgrind`; if the runner can't host valgrind, defer M5 and mark those
   benches `gateable: false` (measured, not gated) — a valid honest fallback, **not** a failure.
5. Add the REGRESSION-LEDGER entry (item 17): the exit-2 fail-open (root: `f3c0687cf` deleted the
   fallback without updating `ci.yml`) + the `bench-gate-selftest` as the permanent regression guard.
6. **Sequencing:** land P75 **before** P80/P81/P82 (they write ~40 baselines into this schema). If the
   `perf/contention-bench-2026-07-18` branch merges around the same time, register its `contention.rs`
   benches into the schema (§3) — do **not** re-specify them (P90 owns their content). Neither order
   blocks P75.
7. **Anti-scope:** P75 adds no bench bodies; invents no schema P80–P82 could contradict; never
   re-points CI at the cross-host absolute gate; never un-ignores `BENCH_HISTORY.md`.
