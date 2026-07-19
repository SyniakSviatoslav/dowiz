# AUDIT — Pervasive Telemetry Coverage + AI-Optional as an OS-Style Feature

**Date:** 2026-07-19 · **Kind:** consistency-enforcement audit (inventory + assessment; no code
changed) · **One of three parallel audits dispatched by the operator this session.**

**Operator directives audited against:**
1. *"telemetry for efficiency/latency/resources usage/memory/cpu/gpu/energy & calculation of
   efficiency should be enforced everywhere in the code, for all repos, libraries, processes,
   functions, etc."*
2. *"Like a real OS which has AI as togglable flag & feature, it should be possible to function
   even without it."*

**Method:** direct source inspection of `/root/dowiz` (main, `0cfc9df28`), the space-grade
execution worktree `/root/dowiz-wt-space-grade-exec` (branch `exec/space-grade-tier0-2026-07-19`,
`c64ca923b`), and `/root/bebop-repo` (`1b90803`). All citations verified against live files, not
the index. This doc mirrors item 26's discipline: findings and a design, **zero production code
landed**.

---

## Dimension 1 — Pervasive efficiency/latency/resource telemetry

### §1.1 What already exists (real, verified)

| Machinery | Where | Status |
|---|---|---|
| **FDR envelope with first-class hw stamp** — `HwStamp { cpu_ticks, rss_kb, joules_uj }`, each a `Reading<T> = Value \| Unavailable(Absence)` with a CLOSED reason enum; field ALWAYS serialized (named absence, never a missing key or fake 0). RAPL energy reader is genuinely new code; on this host it truthfully reports `{"unavailable":"no_rapl_interface"}` (`/sys/class/powercap` empty). `StampPolicy::{Full,Cheap}` is the honest hot-path cost control. | `kernel/src/fdr/schema.rs` (344 lines; readers REUSED from `typed_metrics.rs`) | **Built this session — UNMERGED.** Lives only on `exec/space-grade-tier0-2026-07-19`; `kernel/src/fdr/` does not exist on main. |
| **PMU companion stamps (item 27)** — `PmuStamp`: Tier A zero-permission (rdtsc, minflt/majflt, ctx switches from `/proc`), Tier B via hand-rolled `perf_event_open(2)` raw syscall, no libc (instructions, cycles, cache/branch misses → IPC/miss-rates). On this host `perf_event_paranoid = 4` ⇒ every Tier-B field degrades to `Unavailable(PermissionDenied)`. Attached ONLY to `Verdict`/`DriftClass` verdict-emission records; P3 forensic plane, excluded from every hash/gate surface. `PmuStamp::delta` = the one sanctioned bracketed subtraction. | `kernel/src/fdr/pmu.rs` (635 lines) + `kernel/tests/markov_pmu_fdr.rs` (same branch) | Built this session — UNMERGED. |
| **FDR ring** — segment-amortized fsync (measured 3.87 µs/record normal vs 571 µs alarm; ~8,000 records amortize one fsync barrier). Item 26 validated the design AS-IS. | `kernel/src/fdr/ring.rs` (same branch) | Built + baselined this session. |
| **Item 26 batching measurements** — real wall-clock + `strace -c` syscall-count telemetry: event-log 637 µs p50 (exactly 1 fsync **and 1 open/close** per event; 53× group-commit potential, operator-gated), FDR ring keep-as-is, `import_unit` 0.9 µs don't-batch. PMU honestly reported UNAVAILABLE, nothing fabricated. | `docs/design/AUDIT-ITEM-26-batching-measurements-2026-07-19.md` (staged on main) | DONE. |
| **`typed_metrics.rs`** — `/proc/self/stat` CPU ticks + VmRSS readers, `mono_now_ns`, GPU typed-absent (`Option<GpuSample> = None`, never fake 0). The precedent `HwStamp` upgrades. | `kernel/src/typed_metrics.rs` (main, always compiled) | Live. |
| **`metrics.rs` (P08)** — typed `LogEvent` schema + claim-latency anomaly detector; "CPU-% is a derived consumer concern; emit raw ticks only" losslessness rule. | `kernel/src/metrics.rs` (main) | Live. |
| **`span_metrics/` (P83)** — `SpanMetricsLayer` latency histograms over 8 verified hot functions (`place_order`, `place_order_priced`, `fold_transitions`, `route`, `commit_after_decide`, `decide_settlement`, `cap::verify_chain`, `mldsa_verify`) + load-breach `perf record` trigger. | `kernel/src/span_metrics/` — **behind non-default `telemetry` feature** (`kernel/Cargo.toml:93-102`, `lib.rs:86-87`) | Live but OFF in every default build. |
| **Bench corpus** — criterion + contention + crypto/money/ppr/spectral benches, `baseline.json`, `BENCH_HISTORY.md` (actively appended today), `bench_track.py`; engine criterion bench. | `kernel/benches/`, `engine/benches/`, `agent-adapters/benches/BENCH_HISTORY.md` | Live (bench-time, not runtime). |
| **Agent metrics** — `dowiz_agent_*` counts (iterations/tool_calls/tokens) into the shared `track_record.jsonl` harvest ledger; Dispatcher path times chat calls (`ms` into `TrackRecord`). | `agent-loop/src/main.rs`; `llm-adapters/src/dispatch.rs:148-150` | Live, partial (see G1). |
| **Engine `FrameProfiler`** — counts `json_parse_calls` / `write_buffer_calls` per frame (FE-01 zero-copy gate). | `engine/src/bridge.rs:20-26` | Live; counts only, no time. |
| **HOT-PATHS manifest** — machine-read hot-zone roster for the hardening gate (item 6): `pq/dsa`, `order_machine`, `householder`, `spectral`, `token_bucket`, `event_log`, `retrieval/pattern`, `fdr/json`, `ct_gate`. | `docs/audits/hardening/HOT-PATHS.tsv` (exec worktree) | Built this session. |

Naming note: `kernel/src/telemetry.rs` on main is a trigram pattern surface over tool outcomes —
NOT resource telemetry. Do not count it toward this directive.

### §1.2 High-consequence gaps (prioritized; file:line verified)

Priority = (hot-path flagged elsewhere this session) ∧ (crosses a process/module boundary) ∧
(an efficiency ratio could be derived from it).

| # | Gap | Evidence | Why it matters |
|---|---|---|---|
| **G1** | **Agent-loop turn: no latency at all.** The host binary folds counts only (iterations/tool_calls/tokens); it drives `OllamaAdapter` directly through `AgentLoop`, bypassing the ONE timed path (`Dispatcher`, `dispatch.rs:148-150`). No wall-clock per turn ⇒ tokens/sec, ms/tool-call underivable. | `agent-loop/src/main.rs:37-70` (fold_log: counts only) | agent-loop↔network↔LLM double boundary; tokens are already counted — one `Instant` pair away from a real efficiency ratio. |
| **G2** | **Kernel LLM port carries no timing field.** `ChatResponse { content, usage, tool_calls }` — no duration/TTFT; the port contract structurally cannot transport latency even where adapters measure it. | `kernel/src/ports/llm.rs:112-119` | The kernel↔adapter seam drops the measurement on the floor. |
| **G3** | **Engine frame loop: zero timing.** `EngineLoop::frame()` (the only production caller of `InputRouter::tick`) has no frame-time, no budget check; `FrameProfiler` counts calls but never time. Whole `engine/src/` has zero `Instant::now` (grep-verified). | `engine/src/engine_loop.rs`; `engine/src/bridge.rs:20-26` | THE per-frame hot path of the UI engine; frames/joule and frame-time-p99 are the canonical UI efficiency metrics. |
| **G4** | **wasm↔JS boundary: 24 un-instrumented pub fns.** `FdrEvent::stamp` is `cfg`'d off wasm (Instant panics there) — so the FDR plan structurally EXCLUDES the wasm surface; no wasm-safe clock alternative is designed. | `kernel/src/wasm.rs` (24 `pub fn`); `fdr/schema.rs:233-237` | Cross-runtime boundary; the named-absence pattern needs a wasm leg (`performance.now()` import or `Absence::NonLinuxHost`-class reason). |
| **G5** | **`FileEventStore::insert` / `EventLog::append`: bench-measured, runtime-blind.** Item 26 measured 637 µs p50 and found the re-open-per-event defect; but production has NO continuous counters here — the operator-gated 53× group-commit decision has no ongoing data feed. | `kernel/src/hydra.rs:1036`; `kernel/src/event_log.rs:302` (both HOT-PATHS zones) | Durability hot path; events/joule and fsync-stall-p99 live here. |
| **G6** | **Subprocess spawns: no child accounting.** `sh` and `node` children spawned with no duration, exit-rusage (`wait4`), or FDR record. | `kernel/src/living_knowledge.rs:88,185` | Process boundary; a hung/expensive child is invisible to FDR (adjacent to item 48's liveness blind spot). |
| **G7** | **Eigensolver hot zones have no runtime spans.** `spectral.rs` / `householder.rs` are HOT-PATHS zones (33/14 min-tests) but are NOT among span_metrics' 8 instrumented functions. | `docs/audits/hardening/HOT-PATHS.tsv:21-22,36-37`; `kernel/src/span_metrics/mod.rs:15-20` | The kernel's heaviest pure-CPU math; cycles/eigensolve is the cleanest Tier-2 efficiency metric available. |
| **G8** | **Crypto timing double-gated.** `mldsa_verify` span requires `telemetry` AND `pq`; a `pq`-only production build has zero crypto latency telemetry. | `kernel/src/span_metrics/mod.rs:29-30` | Signing gate is a live product surface (HybridSigner CLOSED on main). |
| **G9** | **All span telemetry is opt-IN.** `telemetry` is a non-default feature ⇒ every default/shipping binary carries ZERO span latency instrumentation. Defensible for perf-neutrality, but it inverts the operator's "enforced everywhere" — today the default is "enforced nowhere at runtime." | `kernel/Cargo.toml:93-102` | The single biggest structural gap: pervasiveness is currently a build flag someone must remember. |
| **G10** | **bebop-repo: no comparable machinery at all.** Repo-wide grep for `HwStamp\|PmuStamp\|perf_event_open\|joules\|rapl` → zero hits. This session's NTT work (`feat(pq_kem)` `986646a`, re-derived ML-KEM-768 NTT alongside schoolbook, **NOT wired**, proven bit-identical) has correctness proof but ZERO performance numbers — no bench dir in `bebop2/core`. The wire-in decision (NTT vs schoolbook) is precisely a cycles-per-op question with no data. | `/root/bebop-repo/bebop2/core/src/pq_kem.rs`; commit `986646a` | Cross-repo consistency: dowiz's `Reading<T>` pattern is entirely absent where the heaviest new math landed. |
| **G11** | **Engine voice/ASR: energy claim with no energy telemetry.** `WakeWordSpotter` is documented as "the battery lever" yet nothing measures `feed()` latency or energy; `InferError::Timeout` exists with no timer feeding it. | `engine/src/voice.rs:110-135` | Inference-shaped boundary + an explicit efficiency claim, unmeasured. |
| **G12** | **kernel agent executor: no per-iteration timing.** Always-compiled loop machinery, counts folded downstream only. | `kernel/src/agent/loop.rs` | agent↔kernel boundary; pairs with G1. |
| **G13** | **apps/api (Node): no metrics instrumentation found.** No `prom-client`/`process.hrtime`/`performance.now` in `apps/api/src`; `spa-proxy.ts` is a 99.4th-percentile churn hotspot with no latency telemetry. | `apps/api/src/routes/spa-proxy.ts` (grep-verified absence) | Lower priority for the kernel roadmap, but squarely inside "all repos." |

**Non-gaps (checked, honestly fine):** the FDR ring's own overhead (measured: 3.9 µs encode,
item 26 §2); `hub_provisioning` batch (cold path, item 26 §0); `import_unit` (0.9 µs, ~1M/s —
measured, don't instrument the inner loop, stamp at span granularity only).

### §1.3 Proposed efficiency calculation (design only — nothing built)

**Name:** the *work-normalized cost ledger*. Grounded in three already-landed laws: raw monotone
counters only (deltas at the consumer; `PmuStamp::delta` the one sanctioned exception), named
absence over fabricated zeros (`Reading<T>`), and P3-plane exclusion from all hash/gate surfaces.

1. **Emit pairs, never ratios.** On `SpanClose`-class FDR records for a named workload, emit
   `(work: {kind, Δcount}, cost: HwStamp-delta ⊕ PmuStamp-delta)` — both raw `u64`. The ratio
   (work/cost) is a display/analysis concern, exactly like CPU-% in `metrics.rs`. This keeps the
   record lossless and lets ANY later question (per-joule, per-cycle, per-tick) be answered from
   the same bytes.
2. **Closed workload-kind enum** (the countable work units that already exist):
   `DecisionUnitsImported` (`import_unit` count), `FdrRecordsAppended` (seq delta),
   `TransitionsFolded` (`fold_transitions`), `TokensGenerated` (agent, G1),
   `FramesRendered` (engine, G3), `EigensolvesCompleted` (G7), `SignaturesVerified` (G8).
3. **Degradation ladder, self-describing per field** — the host decides the tier, the record
   names it, mechanically, because every cost field is a `Reading<u64>`:
   - **Tier E (energy):** work per Δ`joules_uj` — requires RAPL. **This host: named-absent**
     (`no_rapl_interface`). The design MUST NOT block on it; it lights up automatically on a
     RAPL-capable deploy with zero schema change.
   - **Tier C (cycles/instructions):** work per Δ`hw_instructions` (+ IPC context) — requires
     `perf_event_open`. **This host: `permission_denied`** (`paranoid=4`). Same auto-light-up.
   - **Tier T (ticks+wall — ALWAYS available on Linux today):** work per Δ`cpu_ticks` and per
     Δ`mono_ns`, with `rss_kb` as the memory ceiling and Tier-A PMU proxies (minflt =
     allocation churn, nonvol_ctxt_switches = contention) as free context. **This is the tier
     the ledger actually runs at on this host** — honest, not aspirational.
   - A consumer MUST NOT compare efficiency numbers across tiers; the tier is part of the value
     (enforced structurally: absent counters are absent, so a cross-tier ratio is
     uncomputable rather than silently wrong).
4. **Consistency check for free:** on hosts where Tier C AND Tier T are both live, work/cycles
   vs work/ticks must agree within a stated band — a cheap self-test of the counters themselves.
5. **First three deployments** (matching §1.2 priorities): agent turn (G1: tokens + Δwall +
   Δticks — closes the only boundary where work is already counted but time is not), event-log
   append (G5: events + Δticks + fsync count — feeds the operator's group-commit decision with
   live data), engine frame (G3: frames + Δwall against a stated frame budget).
6. **Enforcement posture** (answers G9 without blanket instrumentation): extend HOT-PATHS.tsv
   with an `eff` column — every hot-zone row must either name its workload-kind or carry a
   ledgered `gap:` entry, exactly the item-6 mechanism. "Enforced everywhere" becomes "every
   hot zone either measures or explains," which is the version of the directive that survives
   contact with `StampPolicy::Cheap` economics — the same consequence-first judgment items 26/27
   already applied, extended rather than replaced.

---

## Dimension 2 — AI-optional as a genuine OS-style toggleable feature

### §2.1 Honest assessment of where item 45 actually stands

**Item 45 is a design, not yet even a feature flag.** Roadmap §I is explicit ("Planning only —
no item starts before the operator dispatches it"), and the `inference` feature does NOT exist in
`kernel/Cargo.toml` on main or the exec branch (verified: features are `std`/`json-api`/`wasm`/
`chaos`/`count-allocs`/`pgrust`/`pq`/`gpu`/`slot-arena`/`telemetry`). So today's guarantee is
one notch below "a feature flag exists" — it is "a gate is specified for a subsystem (items
33–44) that has not landed."

**And yet the kernel is genuinely AI-free today — verified, not assumed:** a grep of every core
decision module (`order_machine.rs`, `decision/`, `hydra.rs`, `event_log.rs`, `markov.rs`,
`spectral.rs`, `domain.rs`, `money.rs`) for `micrograd|online::|attention::` returns ZERO hits.
`attention.rs` is deterministic math (one diffusion step over a learned-affinity matrix — a
lens, no learned weights, no inference), and its doc states the law: "the kernel stays non-AI…
learning lives in `online`/`micrograd` at the edge if ever needed."

**Is compile-time the right "toggleable" level?** Yes — honest call: for a build-once-deploy
deterministic kernel, compile-time exclusion (the `CONFIG_FOO=n` analog) is STRONGER than
Linux's runtime-loadable modules, not weaker: absent code cannot be invoked, carries no dormant
symbols, and cannot introduce nondeterminism. What Linux modules add — load/unload without
rebuild — serves a deployment model (heterogeneous hardware, third-party drivers) this kernel
does not have. The runtime half of a genuine OS-style guarantee is already designed as **item
47's `Option<Proposal>`**: `None` (AI absent/crashed/rejected) is a first-class tested input
with a bit-identical-output proof obligation — that is *stronger* than "the OS tolerates the
module's absence"; it is "absence is a tested, provably-equivalent execution path." **Verdict:
45 + 47 together constitute a genuine OS-style guarantee. Neither is built. Today's safety is
real but incidental — maintained by convention (module docs + grep-clean), not by any gate.**

### §2.2 Coupling-risk evidence (the leak check)

No hard leak of AI types into always-compiled core decision code was found. Three **boundary-
definition gaps** were found — places item 45's current spec does not disposition:

1. **Ambient learning modules are always-compiled and ungated.** `micrograd` (`lib.rs:225`) and
   `online` (`lib.rs:232`) — the very modules the attention doc names as where "learning lives"
   — ship in every default build. Their only driver is `evals.rs` (wasm-gated, which explicitly
   "un-strands the two STRANDED learner organs," `evals.rs:746-747`); `noether.rs` references
   them in prose only (`noether.rs:9`). Item 45's dependency-direction list names the *future*
   AI module paths but is silent on these three pre-existing ones — when `inference` lands, are
   `micrograd`/`online` inside or outside the gate? Undefined = grandfathered leak.
2. **AI-shaped port surfaces are always-compiled:** `ports/llm.rs` (LlmBackend trait — verified
   zero network/serde, a deliberate compile firewall per its own doc), `agent/` (`lib.rs:173`),
   `ports/agent/` (admission/cap/manifest/scope). A trait-only seam in core is the CORRECT
   OS-style shape (the syscall interface exists even when the driver is absent) — but item 45
   never *states* that port traits are the sanctioned always-compiled surface, so the gate's
   grep can't distinguish a legal seam from an illegal reference.
3. **The engine is outside the gate's scope entirely.** `engine/src/voice.rs` carries an
   inference-shaped surface (`AsrModel::feed`, `InferError` "mirrors `LlmError` shape",
   `WakeWordSpotter` — "real spotters would run an always-on tiny net"); `engine/src/intent.rs:14`
   references an `inference/` firewall in tests. Item 45's module list is kernel-only.

### §2.3 Proposal (design only — nothing built)

- **P1 — Dispatch item 45 now, as an asserting gate.** It is READY-NOW/zero-prereq by its own
  spec and "asserts today's truth": CI job = default-features build + FULL suite green +
  dependency-direction check over the named module list, red-proven via a planted core→AI
  import before it counts as landed (P7). Cost is one CI job; it converts §2.1's
  "safe by convention" into "safe by gate" *before* items 33–44 create real risk.
- **P2 — Add a disposition table to item 45's spec** covering the pre-existing surfaces:
  `{micrograd, online, attention, evals, ports/llm, ports/agent, agent/, engine/voice.rs}` →
  each classified as CORE-DETERMINISTIC (stays ungated; `attention` belongs here — it is math),
  AI-EDGE (moves behind `inference` when it lands; `micrograd`/`online` are the candidates), or
  SANCTIONED-SEAM (trait-only, always-compiled, zero-dep verified via `cargo tree` — the ports).
  Extend the gate's scope clause to name the engine's `voice`/`inference` firewall.
- **P3 — Confirm compile-time + item-47 typed-`None` as the final toggle level.** Explicitly
  REJECT dlopen-style hot-swap, a runtime kill-switch service, and an AI-health monitor (item
  45 already lists these as not-built; this audit endorses that as the *correct* call, not a
  deferral — runtime module dynamism is over-engineering for build-once-deploy and would trade
  determinism for a capability nobody deploys).
- **P4 — Build-provenance FDR record.** One `Kind::Event` at startup naming the compiled
  feature set (`inference` on/off, `pq`, `telemetry`, …), so forensics can distinguish an
  AI-absent binary from an AI-present one from the flight recorder alone. Reuses the envelope;
  pairs naturally with item 48's heartbeat.
- **P5 — Feature-matrix CI legs.** At minimum `default` and `default+inference` must compile
  AND pass the full suite on every PR once the flag exists (extending item 45's single
  default-features leg), so the absent leg stays green forever rather than only at
  gate-landing time.

---

## Cross-dimension note

The two directives meet in one place: `HwStamp`/`Reading<T>` named-absence is ALSO the right
pattern for AI-absence (a `None` proposal is a named, typed absence — never a silent skip), and
P4's build-provenance record serves both audits. The kernel already owns the one idea both
directives need: **absence is data.**

## Disposition of this audit's own outputs

- This file: staged on main immediately after write (untracked-file-safety rule).
- No code, no roadmap edits, no bebop-repo writes — synthesis pass follows.
