# Telegram Ops-Observability Channel — Design Proposal (2026-07-20)

> **Status: DESIGN PROPOSAL, not built.** Produced from an exhaustive Opus inventory of every
> real+planned telemetry field in the repo, synthesized by Fable per operator instruction. No code,
> no build items executed — this is what the operator asked to see before anything gets wired.

**Scope:** design only, for operator review.
**Baseline extended:** chat `-1003901655568` (forum mode), bot `@dowizbot_bot`, existing topics
257/267/291/292/294, transport = `tools/telemetry/lib.sh` (`tg_spool`/`tg_send`) +
`tools/telemetry/rust-spool` drain + `tools/telemetry/topics` native aggregator +
`tools/ops-alert` (fence-check, regression-digest, deadman-check) +
`.github/workflows/heartbeat-monitor.yml`. This design extends that live registry rather than
inventing a parallel one.

## 1. Channel/topic architecture

**One forum chat, two orthogonal routing dimensions.** The existing system already implies this
split and the design makes it explicit:

- **Topic = category** (where a message *lives*, for browsing/history).
- **Severity = mirroring rule** (whether a *copy* also pages). S0/S1 — the levels `ops-alert`
  already emits — are **not a topic**; they are a rule: *any* message classified S0/S1, from any
  category, is additionally posted to topic 257 (OPS_ALERTS), which stays a low-volume pure pager.
  This generalizes what heartbeat-monitor already does (it posts S0 to 257) and what its nested-
  pager fallback assumes (257 must never be noisy, or pager-failure messages drown). FDR `level`
  maps in: `Error`/`kind=Alarm`/`kind=PostMortem` → S1 minimum; `load_breach`, `Hydra BreachAlert`,
  deadman/heartbeat failures, CI-red-on-main → S0/S1 per a small fixed table. `Warn` and below
  never mirror.

**Topic map** (existing IDs kept; new topics get IDs at creation time):

| Topic | Content | Cadence |
|---|---|---|
| **257 OPS_ALERTS** (exists) | S0/S1 mirror only + pager-failure fallback | live, rare |
| **267 HERMES** (exists) | agent ops, DOD plan/step/retro | as today |
| **291 PLANS** (exists) | DOD+git+roadmap aggregate | as today |
| **292 GIT/DEPLOY** (exists) | commits (today) + deploy events (start/success/rollback) with commit SHA and target (Hetzner/Cloudflare) | live, low-volume |
| **294 BENCH** (exists) | bench-regression gate verdicts; daily drift digest over the 47 baseline-gated IDs in `kernel/benches/baseline.json`, rendered as criterion A/B verdicts, not raw means | per-CI-run + daily |
| **NEW: RESOURCES** | the flagship resource/efficiency report (§3) | hourly pulse + daily rollup + pinned live board |
| **NEW: CI/CD & CRON** | one rollup message per workflow run on main (all 20 job verdicts in one message, never 20 messages); cron-liveness scoreboard (did the 03:17 UTC security scan and the 10-min heartbeat actually fire) | per-run + daily scoreboard |
| **NEW: LOGS** | FDR-derived: `Error` records near-live (batched ≤1/min); `Warn` hourly digest (top-N by name with counts); `CleanShutdown`/restart lifecycle events live | mixed |
| **NEW: KERNEL/MESH** | `EventLog::append` outcome counters (Committed/Duplicate ratio), `Hydra BreachAlert{node_id, group_size}` (live, S1-mirrored), and — once they exist — peer events. Until then this topic honestly carries the *absence* (§5) | hourly + live for breaches |
| **NEW: MEMORY/DOCS** | living-memory delta (MEMORY.md index changes, new topic files), new/changed `docs/design/*` blueprints, roadmap-item status flips | daily digest |

**Volume discipline is structural, not aspirational.** Nothing per-event high-frequency ever
reaches Telegram. A per-`SpanClose` message at kernel rates would be tens of thousands of
messages/hour against Telegram's ~20 msg/min group limit — the design forbids it at the
architecture level: span data reaches the channel *only* through the `span_latency_us` histogram
(count/sum/min/max/mean + 22 power-of-two buckets), already an aggregate. Same for load gauges —
`disk_pct/load1/mem_pct` continue flowing through the exporter/spool at native cadence; Telegram
sees windowed summaries. One Telegram-native trick worth adopting: the RESOURCES topic keeps a
**pinned message updated via `editMessageText`** — a "current status board" that changes in place
instead of scrolling.

## 2. Live vs digested — cadence design

**Page now (live, S0/S1-mirrored to 257):** heartbeat/deadman failures (unchanged); fence-check/
regression-digest CI-red pages (unchanged); FDR `kind=Alarm`/`kind=PostMortem`; `load_breach`
(normalized_load1 > 4.0); `Hydra BreachAlert`; deploy start/finish/rollback; CI red on main.

**Near-live batched (≤1 msg/min per topic):** FDR `Error` records; git commits (existing 292
behavior) — batching prevents an error storm from becoming a Telegram storm.

**Hourly digest (RESOURCES, LOGS-Warn, KERNEL/MESH):** the resource pulse (§3); Warn top-N;
EventLog counters; span-latency windows — the shortest cadence at which histogram buckets carry
statistically meaningful counts.

**Daily rollup (03:30 UTC, after the 03:17 security scan):** bench drift over all 47 IDs (only
*changed* verdicts itemized); CI scoreboard; cron-liveness; resource day-over-day trend including
daily joule/CO2e totals *when measurable/configured*; MEMORY/DOCS delta. Efficiency coefficients
are **daily-first** — hourly work/cost ratios are noisy, daily ones trend.

**Weekly (optional, operator-toggleable):** trend synthesis + the §5 gap tracker as a checklist.

Rule of thumb: *live = something changed that demands action; digest = something you want to
watch drift.* The operator's "efficiency coefficient, joules, carbon" asks are drift-class; their
"warnings, errors, ci/cd, deployment" asks are action-class.

## 3. The resource/efficiency report — concrete format

Hourly pulse in RESOURCES (`pre` block, well under Telegram's 4096-char limit):

```
RESOURCES dowiz-dev (Hetzner fsn1) 2026-07-20 14:00 UTC  window=1h
build: a1b2c3d  features: [n/a — item 63 build-provenance not built]

CPU    load1 1.24 | norm 0.16 / breach 4.0 OK | cpu_ticks d=4.31e9
       ctxt vol/nonvol 182k/9.4k | faults min/maj 88k/12
       IPC: [absent — PMU Tier B PermissionDenied (perf_event_open)]
MEM    rss 412 MB (kernel proc) | host mem 61.2%
DISK   71.3% (^0.4 vs prev hour)
NET    [absent — no network metric in hetzner-exporter; unplanned gap]
GPU    [absent — zero GPU telemetry exists; declared-empty seam]
PWR    joules: [absent — NoRaplInterface on this host]
CO2e   [not configured — NoRegionalConstant; to enable: set
       grid intensity gCO2e/kWh for Hetzner fsn1 (DE grid), item 69]

LATENCY span_latency_us, 1h
 decide/fold    n=18204  mean 118us  max 2.9ms   p99<=4096us*
 eigensolve     n=42     mean 8.4ms  max 21ms    p99<=32768us*
 event_log      n=9107   mean 61us   max 840us
 llm.turn       n=17     wall mean 3.1s (TrackRecord.ms) tok=41k
                ttft: [absent — item 59 not built]
 *bucket-quantile: upper bound of the power-of-two bucket holding p99

EFFICIENCY (item 58 Work-Normalized Cost Ledger — NOT BUILT)
 will render here as consumer-side ratios from (Work, Cost) pairs, e.g.
   FdrRecordsAppended per 1e6 cpu_ticks
   TransitionsFolded per 1e6 cpu_ticks
   TokensGenerated per joule        [also gated on RAPL]
 until then this section prints exactly this notice — never a number
```

Design commitments embedded in that mock:

- **Named absence everywhere, never zero.** Every unmeasured line renders `[absent — <named
  reason>]`, mirroring the kernel's `Reading<T>` philosophy (`NoRaplInterface`,
  `PermissionDenied`, `NoRegionalConstant`). A reader can always distinguish "measured 0" from
  "not measured." The CO2e line is **actionable** — it names exactly the one input (a Hetzner-
  region gCO2e/kWh number) that flips it on, honoring item 69's rule that the constant is
  operator-supplied, never hardcoded.
- **Efficiency is computed at render time, never stored.** Per item 58's design law, the report
  consumer divides `Work.delta_count` by the cost delta when *formatting the message*. No
  "efficiency" field enters FDR, the spool, or any ledger. If item 58 isn't built, the section
  shows the notice above — never a synthesized pseudo-coefficient.
- **Bucket-derived quantiles are labeled as bounds** (`p99<=…*`), because a 22-bucket power-of-two
  histogram gives an upper bound within the bucket, not an exact percentile — the items-55/56
  honesty discipline applied to the report layer.
- **The daily rollup** reuses the same skeleton with day-over-day deltas, daily joule/CO2e totals
  (when live), and 7-day sparklines (`▁▂▄▂▃▅▄`, ASCII-safe).

## 4. Native-telemetry-only confirmation

Every data source is already dowiz-native: the FDR envelope and `HwStamp`/`PmuStamp`
(`kernel/src/fdr/schema.rs`), `span_latency_us` histograms, the hetzner-exporter gauges,
`EventLog` outcomes, `Hydra` breach events, agent `TrackRecord`, `kernel/benches/baseline.json`,
and CI's own job results. Every delivery path is already dowiz-native: `tg_spool` →
`rust-spool` async drain (so report generation never sits on a kernel hot path), `lib.sh`, and the
`tools/telemetry/topics` crate (pure-std + `ureq`, gated by its own `ZERO-DEP-ALLOWLIST.txt`).
**The only external surface is the Telegram Bot API itself** — the incumbent, accepted transport.
No tracker SDK, no analytics service, no new external dependency. New digest generators are new
*subcommands of the existing `topics` binary* (matching the `git-watch`/`plans`/`bench-watch`
pattern), not new crates.

## 5. Honest gap map

**Ready today — real code only, wire directly:**
- **CPU:** cpu_ticks, PMU Tier A (tsc_cycles, faults, ctxt switches), load1/normalized_load1/
  load_breach. Tier B (IPC, cache/branch misses) is real code but will likely render
  `[PermissionDenied]` until perf_event_open is granted — the report shows that honestly either way.
- **Memory:** rss_kb + mem_pct. **Disk:** disk_pct. **Timestamps:** ts_unix_ns/mono_ns on every
  FDR record, `ts` on exporter samples.
- **Latency:** span_latency_us histograms, dur_us, TrackRecord.ms + total_tokens.
- **CI/CD & cron:** 20 named jobs, 2 real cron schedules, S0/S1 paging already built.
- **Benchmarks:** 47 baseline-gated kernel bench IDs + criterion A/B verdicts.
- **Logs/warnings/errors:** the full FDR level/kind taxonomy.
- **Nearest-to-"connections":** EventLog Committed/Duplicate + Hydra BreachAlert.

**Needs a planned roadmap item first:**
- **Efficiency coefficient** → item 58 (and partially RAPL for joule-denominated ratios).
- **Carbon** → item 69 **plus** the operator-supplied grid constant — two gates, both surfaced
  in-message.
- **Joules** → code is real; *measurability* depends on RAPL, which Hetzner VMs likely lack →
  expect `NoRaplInterface` (worth one empirical check on the actual box).
- **Grouped/threaded reports** ("this slow order = 50ms import + 300ms LLM + 50ms render" as a
  tree) → item 62 span linkage; until then all latency reporting is flat, stated honestly.
- **Build-provenance line** → item 63. **ttft/turn timing** → item 59. **Frame budget p50/p99** →
  item 60. **Verdict honesty tags** → items 55/56.

**Structural gaps — no planned home; operator decision needed:**
- **GPU:** zero telemetry anywhere (consistent with the declared-but-empty GPU seam). Renders
  `[absent]` forever until a seam is built — or the operator drops the ask.
- **Mesh peer connect/disconnect:** no peer-session telemetry exists in the decentralized mesh;
  needs its own blueprint (bebop2/mesh-adapter surface).
- **Network metric in the exporter:** absent; small, unplanned addition.
- **LLM-quality model benchmarks:** the 47 benches are *kernel algorithm* benches. There is **no
  LLM-quality benchmark suite anywhere in the repo** — if "model benchmarks" means LLM eval,
  that's a new arc, not a wiring task. The BENCH topic's header should carry this distinction
  permanently so the digest never masquerades as model-quality data.

## Prioritized build order (proposal, not execution)

1. **RESOURCES topic + `topics resources` hourly digest + pinned live board** — real fields only.
   Maximum operator value, zero roadmap dependency.
2. **CI/CD & CRON topic:** per-run 20-job rollup + daily cron-liveness scoreboard.
3. **Formalize the S0/S1 mirror rule** in lib.sh/topics, plus the FDR level→severity table.
4. **LOGS topic:** FDR Error near-live batches + Warn hourly digest, off the existing spool.
5. **KERNEL/MESH + MEMORY/DOCS topics:** EventLog/Hydra rollups; daily memory/docs delta.
6. **Roadmap-gated enrichment as items land:** 58 → efficiency section goes live; 69 + operator
   constant → CO2e; 62 → threaded slow-order traces; 63 → build line; 59/60 → ttft/frame; 55/56 →
   basis tags.
7. **Operator decisions queue:** GPU seam (build vs drop), mesh peer-telemetry blueprint, exporter
   network metric, LLM-quality bench arc.

One empirical pre-check worth doing before step 1: read RAPL availability on the actual Hetzner
box once, so the PWR line's first rendering states measured truth rather than an assumption.

---

**Files anchoring this design:** `.github/workflows/heartbeat-monitor.yml` (chat ID, topic 257,
S0 convention, nested-pager fallback), `tools/telemetry/lib.sh` (tg_spool/tg_send, topic 267
default), `tools/telemetry/topics/src/main.rs` (existing topics 291/292/294, subcommand pattern),
`tools/telemetry/topics/ZERO-DEP-ALLOWLIST.txt`, `tools/ops-alert/src/{fence_check.rs,
regression_digest.rs}` (S0/S1 emitters the mirror rule generalizes).
