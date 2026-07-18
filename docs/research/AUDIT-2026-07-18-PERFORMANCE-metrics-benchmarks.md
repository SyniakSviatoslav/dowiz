# AUDIT 2026-07-18 — PERFORMANCE: real measurements, metrics completeness, benchmark rigor

Reviewer 5 of 5 (Performance Engineer / Data Scientist). Scope: ACTUAL NUMBERS — re-run benchmarks
vs doc-cited claims, metric-ID wiring reality, benchmark coverage, frame-time infra, measured
bottlenecks, latency-budget consistency. Math correctness, code quality, UX, and GO/NO-GO are the
other four reviewers' lanes and are not duplicated here.

**Measurement conditions (disclosed up front):** 8 vCPU AMD EPYC-Milan (QEMU), shared with 4
sibling audit agents running concurrently. Run 1 of the kernel suite overlapped a sibling
`cargo test` + a `cargo build --features gpu`; runs 3–4 (n=30, 5 s measurement) were taken on a
quiet-ish box (only a ~7 % CPU hermes python process + idle tsservers). Every number below states
which run it came from. Disk: QEMU virtio SSD, `/` at 80 % capacity.

---

## REAL NUMBERS TABLE

| # | Operation | Measured today (best run) | Source | Doc/committed claim | Verdict |
|---|-----------|---------------------------|--------|---------------------|---------|
| 1 | `place_order/5_items` | **73.19 ns** [72.95–73.51], n=30 | run 3 | baseline.json 72.822 ns | **HOLDS** (+0.5 %) |
| 2 | `fold_transitions/5_hops` | **3.75 ns** [3.23–4.26], n=30; run-to-run 3.28↔5.74 ns | runs 1–3 | baseline.json 4.2797 ns | **NOISE-BOUND** — ±40 % CI at n=30; a 10 % gate on a 4 ns bench is statistically meaningless on this host |
| 3 | `empirical_identify/20k_samples` | **116.82 µs** [116.34–117.40], n=30 | run 4 | baseline.json 112.56 µs | HOLDS (+3.8 %, inside gate) |
| 4 | `empirical_identify/end_to_end_20k` | **892.82 µs** [890.68–895.33], n=30 | run 4 | baseline.json 883.95 µs | **HOLDS** (+1.0 %) |
| 5 | `token_bucket/try_acquire_permit` | **52.04 ns** [51.93–52.27], n=30 | run 4 | baseline.json 51.924 ns | **HOLDS** (+0.2 %) |
| 6 | `spectral_cache/slem_cached_10x10_hit` | **15.38 µs** [15.34–15.43], n=30 | run 4 | — (no baseline entry, no doc number) | UNGUARDED |
| 7 | `spectral_cache/canonical_address_32x32` | **1.52 µs** [1.40–1.61], n=30 | run 4 | — (no baseline entry) | UNGUARDED |
| 8 | `graph_rebuild_rank/heap` | **118.71 µs** [118.07–119.47], n=30 (128.2 contended / 149.5 noisy) | run 3 | BENCH_HISTORY 2026-07-18 / ROADMAP-LIVE-STATUS: **109.51 µs** | **DRIFT +8.4 %** with tight CI; not baseline-guarded |
| 9 | `graph_rebuild_rank/arena` | **81.20 µs** [80.76–81.65], n=30 (83.6 / 88.6 other runs) | run 3 | same docs: **87.14 µs** | **REPRODUCES** (−6.8 %, better); CI tight in every run — allocation-immunity is real and visible |
| 10 | arena-vs-heap delta | **−31.6 %** (81.2 / 118.7) | run 3 | claimed **−20.4 %** | direction CONFIRMED, magnitude larger today (because heap side got worse) |
| 11 | `cache/exact_hit_decode` (llm-adapters) | **2.06 µs** [2.04–2.07] | bg run | — (no doc claim) | measured, unclaimed |
| 12 | Ollama qwen2.5-coder:7b **decode** (warm) | **9.45 / 10.44 / 12.04 tok/s** (3 probes, daemon's own `eval_count/eval_duration`) | live probes | P21: "4.8–10.5 tok/s measured" | **HOLDS** (top of band / slightly better) |
| 13 | Ollama **prefill** (warm, unique 758- and 1237-token prompts) | **31.1 and 29.0 tok/s** (`prompt_eval_count/prompt_eval_duration`) | live probes | P21: "**~636 tok/s** (≈130:1)" | **REFUTED — 20× lower.** Measured ratio ≈ 3:1, not 130:1 |
| 14 | Ollama cold load / warm load | **23.5 s / 332 ms** | live probes | P21: 25–31 s / 250 ms | HOLDS (approx) |
| 15 | Host fsync-append (100 × 256 B `oflag=dsync` writes) | **0.606 ms/write** (60.6 ms total) | dd probe | — (never measured anywhere in repo) | ⇒ `FileEventStore` durable-append ceiling ≈ **1,650 events/s** single-threaded |
| 16 | bebop2 ML-DSA-65 single verify | **778–808 µs/verify** (scalar) | verify_lane run | P-E §3.2 lane design | measured today |
| 17 | bebop2 ML-DSA-65 `verify_many` lane | **260–266 µs/verify → 2.93–3.00× speedup** at N∈{4,16,64} | verify_lane run | P-E lane-parallel claim | **REPRODUCES** (real 3× win) |
| 18 | bebop2 Ed25519 single verify | **500–527 µs/verify** | verify_lane run | — | ~6–10× slower than optimized libs (zero-dep tradeoff, unstated anywhere as a number) |
| 19 | bebop2 Ed25519 `verify_many` | **0.98–1.01×** (parity, no benefit) | verify_lane run | docs: "batching measured 3.26× slower" (`verify_batch`, batch/64) | CONSISTENT in conclusion (“no throughput benefit”), but the 3.26× number is for `verify_batch`, which `verify_lane.rs` does not cover — not directly re-runnable without new code |
| 20 | P29 API latency (1,000 calls) | not re-measurable this pass (no raw log located) | — | master-roadmap §8.11: p50 4.9 s / mean 10.6 s / p90 26.2 s, avg 1,232 output tok | **CITATION VERIFIED REAL + verbatim**; internally consistent (implies API decode ~120–130 tok/s ⇒ local is 10–12× slower, matching the doc's own "5–15×") |

Bench commands: `cargo bench --bench criterion -- --warm-up-time 1 --measurement-time 2
--sample-size 10` (runs 1–2, baseline-capture flags) and `-- <filter> --warm-up-time 2
--measurement-time 5 --sample-size 30` (runs 3–4). Raw outputs preserved in session scratchpad
(`bench_run2.txt`–`bench_run4.txt`, task logs).

---

### [SEVERITY: CRITICAL] [METRICS-COMPLETENESS] PERF-01
**Where:** grep across `kernel/src/`, `tools/`, `llm-adapters/`, `agent-adapters/` vs
`BLUEPRINT-P54 §` (lines 223–231), `BLUEPRINT-P21 §3.9 + §11.5`, `BLUEPRINT-P45 §4b.3/§4c/§4d`
**What:** **0 of 20 designed metric IDs are wired to any emission code** — P54's 9 `dowiz_agent_*`
IDs, P21's 8 `llm.*` IDs, and P45's 3 named `dowiz_delivery_*/dowiz_ops_*/dowiz_agent_*` example
IDs exist exclusively in blueprint prose.
**Evidence:** `grep -rn "dowiz_agent_|dowiz_delivery_|dowiz_ops_" --include=*.rs --include=*.py
--include=*.sh --include=*.ts` outside `docs/` → 1 hit, and it is a log-file *name* in
`agent-adapters/tests/e2e_admission.rs:88`, not a metric. `grep -rn "llm\.decode_tok_s|llm\.ttft1_ms|
llm\.queue_wait_ms|llm\.hol_block_ms|llm\.tier2_escalation|llm\.route_efficacy"` outside docs → 0 hits.
`tools/ops-alert/` (the §4b.3 checker crate) does not exist; `llm-adapters/src/bin/llm-bench.rs`
does not exist; `crontab -l` has no nightly bench entry (only the Monday curation script);
`BENCH_REGRESSION_PCT`/`BENCH_CONFIRM_RUNS` appear in zero code files.
What IS actually emitting today: host load/mem/disk samples every ~15 s to
`tools/telemetry/logs/metric.jsonl` (live, rows timestamped today), per-dispatch rows in
`llm-adapters/track_record.jsonl` (12 real ollama rows + 20 test-fixture `"backend":"fake"` rows),
and `bench.jsonl` (last row 2026-07-15 — the "nightly" bench feed has been silent for 3 days).
**Why it matters:** every alerting/regression/SLO argument in P21 §3.9/§11.5, P45 §4b.3–4e, and
P54 §4 rests on these IDs; today a decode regression, a router regression, or a probe-pass-rate
collapse is observable by NOBODY. The designs repeatedly say "mechanism already exists" — the log
transport exists; the metrics do not.
**Fix guidance:** before any further metric-ID design work, land the smallest producer: the
§3.7 `llm-bench` binary emitting the three §3.9 rows into the already-live `bench.jsonl`, then
`ops-alert bench-drift` as a plain cron. Track "designed vs emitting" as a counted ratio in the
weekly digest so this gap can't silently persist.

### [SEVERITY: HIGH] [BENCHMARK-RIGOR] PERF-02
**Where:** `BLUEPRINT-P21-local-llm-hermes-native.md:39` (and §3.4.3, §11.4.2 downstream uses) vs
live Ollama probes today
**What:** P21's headline **prefill "~636 tok/s (≈130:1)" does not reproduce: measured 29.0–31.1
tok/s (20× lower) on two independent warm, unique long prompts**; the decode band (4.8–10.5 tok/s)
DOES reproduce (9.45–12.04 measured).
**Evidence:** daemon's own counters, warm model: 758-token prompt → `prefill_tok_s 31.1`,
`decode_tok_s 10.44`; 1237-token varied prompt → `prefill_tok_s 29.0`, `decode_tok_s 12.04`;
cold call → `load_ms 23541`, `decode_tok_s 9.45`. Physics cross-check: 636 tok/s prefill on a
7.6B model ≈ 2·7.6e9·636 ≈ **9.7 TFLOPS**, implausible on 8 QEMU vCPUs; the measured ~30 tok/s
(~0.46 TFLOPS effective) is what this class of host can do. The original probe most plausibly hit
llama.cpp prompt-prefix caching (re-used prompt ⇒ tiny `prompt_eval_count` ⇒ inflated rate).
**Why it matters:** P21 derives arguments from the 130:1 ratio — e.g. "long-context prefill at
636 tok/s ≈ 3.4 min for 128k" (line ~879) is actually **~70 minutes** at measured rates; any
future design treating prefill as nearly-free (RAG stuffing, long tool transcripts, big
few-shot prompts) inherits a 20× error. Rejections that leaned on *decode* cost (MoA,
LLM-compression) are unaffected — decode was honest.
**Fix guidance:** re-measure prefill with unique prompts (cache-busting nonce prefix) in the
planned `llm-bench`, and record `prompt_eval_count` alongside the rate so a cached-prefix
measurement is self-evident. Amend P21's §0 measured-perf row.

### [SEVERITY: HIGH] [BENCHMARK-RIGOR / GATE-INTEGRITY] PERF-03
**Where:** `kernel/benches/baseline.json` (5 keys) vs `kernel/benches/criterion.rs` (9 bench IDs);
`kernel/benches/bench_track.py:111,139–142`; `.github/workflows/ci.yml:145–157`;
`docs/regressions/REGRESSION-LEDGER.md` row 23
**What:** the bench-regression gate is fail-open in three independent ways, and the exact defect
class the ledger fixed once ("baseline covered only 2 of 5 IDs") has silently regressed to
**5 of 9 IDs** — the 4 newest benches (both `spectral_cache/*`, both `graph_rebuild_rank/*`) are
invisible to the gate.
**Evidence:** (a) `baseline.json` contains exactly the 5 old keys. (b) `bench_track.py` (the CI
path — the native tracker binary is never built on a fresh runner) iterates `base.items()` only
(line 111), so run-only benches are ignored; and `MISSING` rows get `delta=None`, which the exit
loop (lines 139–142) skips → **even a deleted benchmark exits 0**, directly contradicting the CI
comment "a benchmark added without a baseline entry goes RED" (ci.yml:142). (c) ci.yml runs the
*absolute host baseline* compare on `ubuntu-latest` at threshold 10 % — the very design that both
ledger row 23's honesty note and P45 §4b.3 explicitly rejected as false-RED-prone ("deliberately
NOT a CI-runner gate"); three artifacts disagree with each other. (d) Remote verification
impossible: `gh` returns 404 for the repo/Actions API with current auth, so there is no evidence
the job has ever run green. (e) The native tracker auto-seeds missing baseline entries on run —
`baseline.json` still having 5 keys proves no tracked bench run has happened since the 4 new
benches landed.
**Why it matters:** my measured +8.4 % heap drift (PERF-05) is precisely the kind of change this
gate exists to catch, and it structurally cannot. The gate's *existence* is cited by P45/P21 as
the mechanism their alerting extends — the foundation is currently decorative.
**Fix guidance:** make unknown-current-ID and missing-baseline-ID both hard failures in the
python path (parity with the native tracker's MISSING→RED); commit the auto-seeded 9-key
baseline with the mandated ledger row; replace the CI absolute compare with criterion
`--save-baseline` A/B as ledger row 23 already prescribed.

### [SEVERITY: HIGH] [BOTTLENECK / COVERAGE] PERF-04
**Where:** `kernel/src/hydra.rs:1033–1066` (`FileEventStore::insert`); no bench anywhere touches IO
**What:** the durable event-append hot path does **open() + write + flush + `sync_all()` per
event, with no batching/group-commit**, and host fsync measures **0.606 ms/write** → a hard
ceiling of ≈ **1,650 durable events/s** single-threaded, ~8,000× the cost of the benched
`place_order` compute (73 ns) — and this path has zero benchmark coverage.
**Evidence:** `dd if=/dev/zero of=<scratch> bs=256 count=100 oflag=dsync` → 60.6 ms for 100
synced writes on this virtio disk. Source shows per-insert `OpenOptions::new().append(true).open()`
(re-open every event) then `f.sync_all()` before the in-memory index advances.
**Why it matters:** every mesh/agent design this session (B1 AgentBridge admission, P54 loop
events, event-sourced order flow) funnels through `EventLog::append` → store insert. The benched
numbers (ns-scale) describe the cheap 0.001 % of the write path; the real throughput budget is
fsync-bound and nobody has a number for it in-repo. At order+courier+agent event rates this is
fine today, but any "replay 20k events" or bulk-ingest path built on per-event fsync will run at
~12 s per 20k events, not the µs-scale intuition the current bench suite trains.
**Fix guidance:** add one criterion bench over `FileEventStore::insert` (tmpfs + real-disk
variants) so the number is owned; keep the fsync-before-index ordering (it is correct durability
discipline) but hold the file handle open and consider group-commit only if a measured workload
needs it.

### [SEVERITY: MEDIUM] [BENCHMARK-DRIFT] PERF-05
**Where:** `graph_rebuild_rank/heap` vs `kernel/benches/BENCH_HISTORY.md` (2026-07-18 entry) and
`docs/design/ROADMAP-LIVE-STATUS-2026-07-18.md:24`
**What:** the recorded heap median 109.51 µs does not reproduce — today's tightest measurement is
**118.71 µs (+8.4 %)**; the arena side reproduces (81.2–88.6 µs vs 87.14 µs recorded).
**Evidence:** n=30/5 s run: heap [118.07, 118.71, 119.47] µs — CI width < 1.2 %, so this is not
noise; two earlier same-day runs measured 128.2 µs (contended) and 149.5 µs (semi-contended). The
arena's CI was tight in every run regardless of box state ([80.76–81.65], [83.34–83.86],
[87.87–89.26]) — an incidental but real demonstration that the arena path is
allocator/system-state-immune while the heap path's cost is environment-dependent.
**Why it matters:** the −20.4 % delta is cited in two docs as "§3.3 hypothesis CONFIRMED". The
conclusion survives (today's delta is −31.6 %, stronger), but the *absolute* heap number is
already stale within hours of being recorded, and neither ID is baseline-guarded (PERF-03), so
this drift was only found by this manual re-run.
**Fix guidance:** record A/B benches as the *ratio* (the stable quantity) plus the absolutes with
their box-state; add both IDs to baseline.json.

### [SEVERITY: MEDIUM] [BENCHMARK-RIGOR / NOISE-FLOOR] PERF-06
**Where:** `BLUEPRINT-P45 §4b.3` ("measured host noise on this box is single-run ±3-5 %") vs
today's repeated runs
**What:** the ±3–5 % single-run noise-floor claim understates reality on this (agent-shared) box:
observed run-to-run swings today were **+75 %** (`fold_transitions` 3.28→5.74 ns), **+15 %**
(`empirical_identify/20k` 117.6→135.3 µs) and **+17 %** (`heap` 128.2→149.5 µs) between
back-to-back suite runs.
**Evidence:** runs 1/2 outputs (scratchpad `bench_run2.txt` vs task log run 1); the box hosts a
5-agent parallel audit today, which is exactly the workload it hosts most days (swarm waves).
**Why it matters:** the §4b.3 design (median-of-3, 2-consecutive-night confirm, 10 % threshold)
is *correctly shaped* for a ±5 % world but was tuned against a noise figure measured on a quiet
box; on the box's real duty cycle, single-sample nightly runs of ns-scale benches will breach
10 % routinely. The smallest benches (4–5 ns) cannot support a 10 % gate at all — CI width at
n=30 was already ±14 % for `fold_transitions`.
**Fix guidance:** when §4b.3 is built, pin the nightly run to a reserved core
(`core_pinning.rs` exists and is unused for this), raise sample counts for sub-100 ns benches, and
gate ns-scale benches on a wider (25 %+) threshold or on counted instructions rather than time.

### [SEVERITY: MEDIUM] [COVERAGE] PERF-07
**Where:** `kernel/benches/criterion.rs` (9 IDs) vs `kernel/src/lib.rs` (60+ public modules)
**What:** of the operator-named hot-path set, only ~half has any benchmark: order placement ✓,
status fold ✓, causal identify ✓, token-bucket admission ✓, spectral cache ✓ (10×10 only),
graph rebuild+PPR ✓ (n=1024) — while **capability/signature verification (`pq/dsa.rs::verify`),
event-log append (`event_log.rs::append`/`commit_after_decide`), chain verification
(`verify_chain`), retrieval (BM25/trigram/fusion), routing (`router.rs::route`/`shortest_path`),
money (`apply_tax`/ledger fold), and the engine `compose()` frame renderer have zero benches.**
**Evidence:** grep of bench file vs module list; `engine/` has no `benches/` dir at all;
kernel `pq/dsa.rs` verify is a *different implementation* from bebop2's (whose sibling numbers —
780 µs/verify ML-DSA, 500 µs Ed25519 — are the only measured indication of this cost class in
either repo).
**Why it matters:** the unbenchmarked half is where the expensive operations live (0.5–0.8 ms
signature verifies, 0.6 ms fsyncs) — the benched half is ns-µs compute that was never at risk.
Bench coverage is currently inversely correlated with cost.
**Fix guidance:** next three benches, in value order: (1) `pq::dsa::verify` (single +
batch-of-16), (2) `FileEventStore::insert` (PERF-04), (3) `retrieval` query over the committed
12-query oracle corpus.

### [SEVERITY: MEDIUM] [RENDERING] PERF-08
**Where:** `engine/src/bridge.rs:20–25` (`FrameProfiler`), `engine/src/loop_.rs`,
`engine/Cargo.toml:26–31`, `BLUEPRINT-P38-webgpu-render-engine.md`
**What:** there is **no frame-time measurement infrastructure at all — not even a stub timer**:
`FrameProfiler` counts *calls* (`json_parse_calls`, `write_buffer_calls`), never time; `engine/`
has no benches; `wgpu` is absent from the build (`gpu::new_gpu` is an honest `Err` stub); zero
pixels have ever been timed because zero pixels have ever been rendered on this branch.
**Evidence:** repo-wide grep for `frame_time|fps|requestAnimationFrame|performance.now|
GPUQuerySet|timestamp-query` outside docs hits only the fixed-timestep *clamp* in `loop_.rs`
(spiral-of-death guard — an input to stepping, not a measurement) and a false positive in
`geo.rs`. P38 names `PARTICLE_BUDGET = 10_000` and a "§6 frame budget" with no harness to check
either.
**Why it matters:** stated plainly, as asked: rendering performance is 100 % unmeasured because
the renderer is 100 % unbuilt — this is *consistent* (nothing fake), but P38's budget numbers are
currently untestable assertions, and the CPU-side `compose()` oracle (which DOES exist and is the
performance floor for any fallback path) has no bench either, so even the measurable part is
unmeasured.
**Fix guidance:** cheapest first real number: a criterion bench over `compose()` at the P38
target resolution — it bounds the CPU-fallback frame time today, needs no GPU, and turns the §6
frame budget into a checkable inequality.

### [SEVERITY: LOW] [LATENCY-BUDGET] PERF-09
**Where:** master roadmap §8.11 (line 451–455) · P21 §11.5 · P54 §4
**What:** the P29 citation is **real and verbatim** (p50 4.9 s / mean 10.6 s / p90 26.2 s,
avg 1,232 output tokens, 99.3 % cache-read) and internally consistent with today's local
measurements (implied API decode ~120–130 tok/s vs measured local 9.5–12 ⇒ 10–12× — inside the
doc's own "5–15×" claim). The genuine residual inconsistency: **P21's `Interactive` priority
class carries an SLO only on queue wait (p95 ≤ 5,000 ms) while the measured decode floor for even
a disciplined 200-token reply is 17–21 s** — no designed metric measures end-to-end turn latency
(`llm.ttft1_ms` is `max_tokens=1` by design), so the tier named "Interactive" can meet every one
of its designed gates while delivering ~20 s turns.
**Evidence:** measured decode 9.45–12.04 tok/s (three probes); 200 tok ÷ 10.4 tok/s ≈ 19 s;
P21 §11.5 row 1 caps only `llm.queue_wait_ms.Interactive`.
**Why it matters:** the alerting scheme as designed would certify an interactive lane green while
users experience batch-class latency; P29's data already implies local lanes must be reserved for
short-output or async shapes, and no metric enforces that boundary.
**Fix guidance:** add one designed ID — end-to-end wall time per priority class — before wiring
§11.5; it is the row the operator will actually feel.

### [SEVERITY: LOW] [BENCH-HYGIENE] PERF-10
**Where:** `kernel/benches/BENCH_RESULTS.md` · `tools/telemetry/logs/bench.jsonl` ·
`llm-adapters/track_record.jsonl`
**What:** three staleness/integrity nits in the measurement record: (a) `BENCH_RESULTS.md` still
presents the 2026-07-13 capture (90.4 ns / 5.59 ns) as "the baseline" while `baseline.json` says
72.8 / 4.28 — two committed sources of truth that disagree by 25 %; (b) `bench.jsonl`'s last row
is 2026-07-15 — the feed P45 calls "already live" has been silent 3 days; (c) `track_record.jsonl`
records `"ms":0` on ~⅓ of real ollama dispatches (repeat-prompt cache hits) — plausible, but a
true-0 latency field makes cache-hit latency unobservable and is indistinguishable from
"not measured".
**Evidence:** file contents quoted in audit session; 12 real rows (`backend":"ollama"`, 4 with
`ms:0`), 20 fixture rows (`backend":"fake"`) in the committed ledger.
**Why it matters:** small individually; together they show the measurement *record* is not yet
treated with the same discipline as the measurement *code*.
**Fix guidance:** delete or auto-generate BENCH_RESULTS.md from baseline.json; record sub-ms
cache hits as actual µs; keep fixture rows out of the committed ledger.

---

## GENUINE (real, solid, well-measured) list

Things that checked out under adversarial re-measurement — credit where the data earned it:

1. **The 5 baseline-tracked kernel benches all hold today** — `place_order` +0.5 %,
   `token_bucket` +0.2 %, `end_to_end_20k` +1.0 %, `empirical_identify` +3.8 %, all inside the
   gate. The committed absolute numbers are honest for this host.
2. **The arena claim reproduces, and then some** — 81.2 µs today vs 87.14 recorded; delta vs heap
   −31.6 % vs claimed −20.4 %; and the arena path's tight CI under every load condition is
   independent physical evidence that the allocation-immunity story is real.
3. **P21's decode band and load times are honest measurements** — re-probed live three times,
   9.45–12.04 tok/s vs claimed 4.8–10.5; cold 23.5 s vs claimed 25–31 s; warm 332 ms vs 250 ms.
   Only prefill (PERF-02) was wrong.
4. **The P29 latency citation is verbatim-real and internally consistent** — the implied API
   decode rate, the local-inversion claim, and the "5–15× slower" figure all cross-check against
   today's independent local measurements.
5. **bebop2's ML-DSA lane-parallel verify delivers a real, reproducible 3.0×** (2.93–3.00× at
   N∈{4,16,64}), and Ed25519 `verify_many` at measured parity (0.98–1.01×) is consistent with the
   repo's own honest post-SSR-2020 walk-back that batching has no throughput benefit.
6. **The BENCH_HISTORY discipline is genuinely good practice** — it contains recorded
   *refutations* of its own claims (the "≤8 heap allocs" estimate marked REFUTED, the Miri gate
   marked NOT RUN with reason). That is rare and worth protecting.
7. **`FileEventStore`'s fsync-before-index-advance ordering is correct durability engineering** —
   slow (PERF-04) but never dishonest: the in-memory state cannot claim an event the disk lacks.
8. **P21 §3.7's instrument-fit ruling is right** — criterion is the wrong tool for multi-second
   model calls, and using the daemon's own counters instead of a client stopwatch is the correct
   measurement design (it is how this audit could re-verify the claims at all).
9. **The native tracker is stricter than its python fallback** — MISSING → RED, auto-seed of new
   IDs — the good half of the gate already exists; it just doesn't run where it matters (PERF-03).

## Headline ratios

- **Designed metric IDs actually emitting: 0 / 20 (0 %).** Live emission today = host samples,
  dispatch ledger, and a 3-day-stale bench feed — none carrying a designed ID.
- **Bench IDs guarded by the regression baseline: 5 / 9 (56 %)**, and the guard itself fails open
  in CI.
- **Doc-cited perf claims re-verified: 12 hold, 2 refuted** (P21 prefill 20× off; heap 109.51 µs
  +8.4 % stale), 1 unverifiable from this host (CI gate outcomes), 1 not directly re-runnable
  (`verify_batch` 3.26×).
