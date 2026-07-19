# Higher-Abstraction Infra Scan — Spectral / Statistical / Retrieval Reuse Across Telemetry, Testing, Notifications

**Date:** 2026-07-19
**Scope:** dowiz/DeliveryOS kernel + tools + design docs. Research-only; no code written, no branches touched.
**Method:** live `Read`/`Grep` against the working tree (the repo is Rust since the 2026-06-14 Repowise index; the old TS `apps/api` no longer exists on disk). Discipline matches the sibling batch/coalescing scan (`docs/research/OPUS-BATCH-CALLS-EVENTS-SCAN-2026-07-19.md`): a technique earns a target only when the real workload/volume justifies it. Named context docs `OPUS-SPECTRAL-EVERYWHERE-SWEEP` / `OPUS-PHYSICS-WAVE-ALGORITHMS` do **not** exist on disk under those names — the closest landed siblings are `OPUS-BATCH-*-SCAN-2026-07-19.md` and the `OPUS-PERF-*-2026-07-18.md` set, which this scan does not duplicate.

**Toolkit under consideration (what "reuse" would draw from):**
- `kernel/src/stats.rs` — E2 uncertainty primitives: `wilson_interval` (:100), `mean_se`/`normal_interval` (:69/:84), `within_clt_envelope` (:133, the CLT √N gate), seeded `bootstrap_interval` (:153).
- `kernel/src/csr.rs::personalized_pagerank` (:330) + `kernel/src/retrieval/{ppr,diffusion,bm25,index}.rs` — importance/relevance machinery.
- `kernel/src/markov.rs` + `kernel/src/bin/markov_attractor.rs` — first-order Markov chain + spectral gap (SLEM) over the tool-outcome token stream.
- `kernel/src/spectral*.rs`, `incidence.rs`, `householder.rs` — Laplacian/eigen surface.

---

## TL;DR — one small latent reuse, two honest-negatives

| Surface | Verdict | One line |
|---|---|---|
| **Telemetry/logging** | **HONEST-NEGATIVE, with ONE small latent reuse** | Real telemetry infra exists (P08 built, P24 designed) and already carries the right-sized detector (EMA+K·dev). PageRank-on-metrics and spectral-on-product-logs are both negative (wrong cardinality / an exact FSM check already dominates). The one genuine `stats.rs` reuse — a **Wilson lower bound on anomaly-flag RATES** — has no consumer today; latent, not live. |
| **Testing** | **HONEST-NEGATIVE (technique real, workload absent)** | A test-selection mechanism **already exists** (retest-all + red-line-forced re-exec), it is the industry-recommended shape (DO-178C, matching Google TAP's retreat from heuristics), the suite is ~1,161 fast pure-unit tests run unconditionally on every push, and the predictive-ranking substrate (RCI PPR) is **unbuilt AND explicitly barred from red-line gating**. |
| **Notifications** | **HONEST-NEGATIVE (clear)** | Single committed transition → single status message → per-recipient fan-out over a **proven-total** capability matrix; ~6 notifications per order lifecycle; dedup is provided *exactly* by the order FSM + `event_log` content-id (both stronger than similarity-dedup); no digest/feed/inbox surface exists where ranking would apply. |

---

## 1. Telemetry / logging

### 1.1 What real infra exists (this is not a greenfield)

dowiz has substantial, layered observability infra — built and designed:

- **P08 typed local-observability, BUILT.** `kernel/src/metrics.rs` defines a *closed* `LogEvent` enum (4 variants: `Metric`/`ClaimLatency`/`ClaimLatencyAnomaly`/`Bench`, `metrics.rs:102-108`) with deterministic parse-or-reject line I/O, plus the **claim-latency anomaly detector** — a named-constant floor predicate `claim_latency_floor` (`metrics.rs:322-329`, `MIN_SECONDS_PER_100_LINES = 5.0` at `:318`) and its typed emitter `check_claim_latency` (`:333`). `kernel/src/typed_metrics.rs` reads `/proc/self/stat` + `/proc/self/status` into typed `MetricSample`s (`:52`, `:83`), GPU typed-absent (`:222`, never a fake 0).
- **Self-improvement pattern surface, BUILT.** `kernel/src/telemetry.rs::surface_recurring_patterns` (`:37`) folds the tool-outcome token stream into ranked recurring trigrams (reuses `trigram.rs`); `kernel/src/markov.rs` + `bin/markov_attractor.rs` model the same stream as a first-order chain and already compute **spectral quantities (SLEM / spectral gap)** as the loop/attractor signal.
- **Product-event anomaly reducer, BUILT.** `kernel/src/analytics.rs::reduce_anomalies` (`:137`) detects illegal order-lifecycle sequences by folding through `order_machine::fold_transitions` — i.e. an *exact* FSM-law check, not a statistical estimate.
- **P24 native runtime telemetry, DESIGNED (not built).** `docs/design/BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` specifies an SPSC ring flight-recorder + explainable latency capsules. Its detector (§4.1) is **EMA mean + EMA absolute-deviation + `K_DEV=6.0` threshold**, built on `geo.rs:39 ema_next` — which is the 1-D steady-state Kalman (`kalman.rs:3-6`). It **explicitly rejects** the full n-D Kalman as "capacity is not need" (§6 anomaly-rule row; §4.1). Every §7 wave item is a proposal — nothing is coded.

Real volume (deciding evidence, consistent with the batch-scan): the live JSONL sink `tools/telemetry/logs/metric.jsonl` measured at **2,758,165 bytes** (P24 audit addendum §1) — a real but tiny, single-host stream; product lifecycle events run ~5k/day/hub, ~0.5 orders/sec system-wide (`OPUS-BATCH-CALLS-EVENTS-SCAN-2026-07-19.md:23-25`).

### 1.2 Spectral / statistical anomaly detection — where it lands honestly

**Statistical detector: already the right size.** P24's EMA+K·dev is a robust running z-score — the correct, zero-dep detector for per-site latency streams. `stats.rs`'s `within_clt_envelope` is a *convergence gate for estimators* (used in `causal.rs`), not a stream detector; `mean_se`/`normal_interval` assume large-n iid and would badly under-estimate error on the EMA-smoothed, autocorrelated latency stream — the module header (`stats.rs:37-39`) warns against exactly this misuse. So the heavy CLT machinery does **not** improve the detector.

**The one genuine reuse — a Wilson bound on anomaly RATES (latent).** Both live detectors (`claim_latency_floor`, and P24's per-site predicate) emit a *binary flag*. The moment any consumer reports an anomaly **rate** — "k of n drain ticks flagged", or the P24 backtest's own false-positive rate during tuning — that rate is today a naked point estimate, which is precisely the RC-1 self-certification failure `stats.rs` was built to fix (`stats.rs:1-19`). `wilson_interval` (`:100`) is the correct small-n binomial bound for it. This is the *identical* pattern RCI's M4 already adopted (`realtime-change-intelligence-2026-07-17/resolution.md:234-237`: "floor = Wilson lower bound on measured precision"). **But there is no rate-reporting consumer today** (P24 capsules report individual events; claim-latency is per-commit) — so this is a real, tasteful, ~5-line reuse that is **latent, not a live gap**. Sketch: when P24's §3.5 historical tier or a `latency-report` fold reports "anomaly rate per site over the window", wrap it in `wilson_interval(flags, ticks, 1.96)` so the rate ships with the error bar that could refute it.

**PageRank importance-scoring on metrics — HONEST-NEGATIVE.** "Rank which metrics matter most via centrality" needs cardinality + edge structure. The metric set is a **closed, flat, ~24-series schema** (P24 §3.5: host gauges + PSI + per-site aggregates) or the 4-variant `LogEvent` enum — independent gauges with no graph among them. Centrality earns its keep at thousands of nodes with rich edges where you must surface the important few; ~24 independent gauges have neither, and you simply read all of them. `csr.rs::personalized_pagerank` has no metric-graph to run on.

**Spectral on log/behavior patterns — already placed correctly.** Spectral analysis of a *stream* already lives where it fits: the Markov attractor detector computes SLEM over the tool-outcome transition matrix. For **product** log streams the "anomaly" is an illegal FSM transition, and `analytics.rs::reduce_anomalies` already decides it *exactly* via the order-machine Law — a spectral/statistical detector there would be strictly worse than the exact check it would replace.

**Verdict (Surface 1):** HONEST-NEGATIVE on PageRank-for-metrics and spectral-on-product-logs; the statistical detector is already right-sized (EMA/Kalman, full-Kalman consciously deferred). One real latent reuse: `stats.rs::wilson_interval` on anomaly-flag rates once a rate consumer exists. **Trigger to build:** a telemetry surface that reports an anomaly/error rate (P24 historical tier or a `latency-report` fold) — then the Wilson wrap is a small, honest win.

---

## 2. Testing

### 2.1 Suite scale and the mechanism that already exists

- **Suite size:** kernel = **1,033** `#[test]`/`#[tokio::test]` functions across 106 of 124 `.rs` files; engine = **128**. ~**1,161** dowiz test functions, overwhelmingly **pure std unit tests** (deterministic, no I/O — the modules are dependency-light by design).
- **CI runs retest-all, unconditionally.** `.github/workflows/ci.yml:120-134` — the `cargo-test` job runs `cargo test --offline` on **both** kernel and engine with **no `if:` path filter**, by explicit design ("this job is unconditional so a planted `assert!(false)` in any kernel test goes RED on every push", `ci.yml:116-119`). That the maintainers run the whole suite on every push is itself evidence they treat retest-all as cheap enough.
- **A test-SELECTION gate already exists — structural, not predictive.** `tools/ci-truth/src/main.rs` ships `is_redline(path)` (`:237-245`, matches `money.rs`/`order_machine.rs`/`event_log.rs`/`auth`/`otp`/`jwt`) and the `v5c-reexec` gate (`:383-394`): if a diff touches a red-line path, force a **full clean-worktree re-execution** of the suite; otherwise `SKIP`. This is retest-all + a DO-178C-shaped criticality override.

### 2.2 Predictive test selection (test-impact analysis) — HONEST-NEGATIVE, and already ruled dominated

The exact question ("which tests most likely catch a regression in this diff") was **already investigated in-repo** and answered negatively — `docs/design/realtime-change-intelligence-2026-07-17/H1-H4-prior-art-research.md:196-230`:

1. **Safe-RTS (Rothermel & Harrold, TSE 1996):** a *safe* technique must select every test that may behave differently; where safety can't be established, the fallback is **retest-all**.
2. **Google TAP (Memon et al., ICSE-SEIP 2017):** even at Google scale they gate on a *sound build-graph reverse-dependency* set (never scraped imports) **and explicitly abandoned fine-grained selection heuristics as unreliable**, keeping periodic retest-all safety nets.
3. **DO-178C criticality classification dominates any heuristic** — a *structural* classification evaluated *before* any analysis; no cost/analysis result may downgrade a critical component's verification. The research concludes verbatim: "the repo **already ships the DO-178C-shaped control**" (`is_redline` + v5c-reexec) and "**reuse it, do not re-implement**."

Three load-bearing reasons the workload doesn't justify predictive selection here:

- **The suite is cheap.** ~1,161 pure-unit Rust tests run offline (the reason CI runs them unconditionally). Test-impact analysis exists to avoid *expensive* suites (tens of minutes → hours, heavy E2E/integration). Below that threshold, retest-all is cheaper than *computing + maintaining + trusting* a selection model, and strictly safer (no false-negative regression escape). *(One honest un-measured item: I sized the suite by test count + nature, not a wall-clock `cargo test` run — but 1,161 pure-unit tests run on every push is decisive enough; a live timing was not warranted for a negative verdict.)*
- **The ranking substrate isn't built.** The PPR blast-radius ranker that *could* map changed-files → affected-tests is **RCI (Option D′), blueprint-only** — no `.rci/`, no `rci` binary (`kernel/src/bin/` holds only `lm.rs`, `markov_attractor.rs`), no `AnalysisFrame` in code. The PPR machinery itself exists and is tested (`csr.rs::personalized_pagerank:330`, `retrieval/ppr.rs`) but is wired to retrieval, not tests.
- **Even the designed ranker is barred where it matters most.** RCI's own resolution pins a permanent LOCK (`resolution.md:162-188`, H4): RCI **never** gets gating/blessing authority over red-line surfaces (money/auth/RLS/migrations), and its import graph is *structurally blind* to coupling that runs by runtime contract rather than `import`. So a predictive selector would be forbidden from gating exactly the diffs a regression most endangers — where the existing structural retest-all is correct.

**Verdict (Surface 2):** HONEST-NEGATIVE for now. The technique is real (PPR over an import graph → affected tests), but a selection mechanism already exists in the industry-recommended shape, the suite is fast enough that retest-all dominates, the ranking substrate (RCI) is unbuilt, and it is explicitly barred from red-line gating. **Trigger to revisit:** suite wall-clock crosses ~10 min (E2E/integration land) **AND** a measured regression-escape rate — **and** RCI must first be built as the substrate, with the H4 red-line LOCK intact.

---

## 3. Notifications

### 3.1 What the fabric actually does

`kernel/src/ports/notification.rs` (1,023 lines, BLUEPRINT-P61):

- **Single committed transition → single status message.** `StatusMsg::for_status` (`:81`) maps one `OrderStatus` to one fixed-template `{title, body}` (`:82-95`). The trigger is one committed, non-duplicate transition; the P61 blueprint confirms "idempotency is free" via `event_log` content-id dedup (`BLUEPRINT-P61-notification-fabric.md:36`).
- **Per-recipient fan-out, not selection among many messages.** `Notifier::notify(channel_ref, msg)` (`:434`) fans **one** message out across **one** customer's registered transports (push/sms/email/messenger), returning a `FanoutOutcome` that records every transport's fate (`:389-397`).
- **Channel routing is a proven-total capability classifier.** `reachability(kind, ctx)` (`:159`) is a total function over `(TransportKind × PlatformContext)` with a proven **X10 coverage invariant** (`:5`, `:158`) — e.g. iOS-Safari-web → push unreachable → falls back to SMS/email. This is a *correctness matrix*, not a score.
- **No ranking/dedup/queue anywhere.** `grep -niE 'dedup|coalesc|rank|priorit|score|similar|sort|queue|batch|digest|history|throttl|debounce'` over `notification.rs` returns **nothing** except `TokenBucket` push-retry throttling (`:379`, rate-limit, not ranking) and "mid-batch" meaning *iterating one customer's push subs* (`:446`). No digest/feed/inbox surface exists in the kernel or the P61 blueprint.

### 3.2 Retrieval-style relevance / dedup / prioritization — HONEST-NEGATIVE

- **Volume is trivially low.** ~6 notifications over one order's lifecycle (Pending→Confirmed→Preparing→Ready→InDelivery→Delivered), at ~0.5 orders/sec system-wide. P61's own scaling analysis (`BLUEPRINT-P61…:464-467`): `ChannelSet` is O(push subs + 3) — a handful of devices per customer; `ChannelRegistry` is O(live orders on one hub) — "hundreds not millions". There is no high-volume per-recipient stream to rank or dedup.
- **Dedup is already exact — and stronger than similarity.** Duplicate suppression comes from two exact mechanisms: the order FSM guarantees each status is entered once per order (`assert_transition`/`is_terminal`, `order_machine.rs:139`), and `event_log` dedups on content-id (`:148`, the idempotency key). A trigram/BM25 near-duplicate detector over 6 fixed templates would be strictly worse than the exact FSM+content-id guarantee it would replace.
- **Nothing to prioritize.** There is no queue of competing notifications and no inbox/digest surface — each message fires on its own committed transition. The only ordering logic is the Reachability *fallback order*, and that is a proven-total classifier (X10 invariant); replacing it with a learned/retrieval ranker would swap a proven-correct total function for a probabilistic one — worse on every axis. Cross-order isolation is a live property test (`ports/customer.rs:586-599`), which a similarity-based router would put at risk.

**Verdict (Surface 3):** HONEST-NEGATIVE, clear. Single-event → single-message, per-recipient fan-out over a proven-total matrix; volume is a handful per order; dedup is provided exactly by the FSM + content-id; no ranking/digest surface exists. Retrieval/ranking is real technique with no target here. **Trigger to revisit:** a genuine multi-item **notification digest** or **operator alert-feed** surface with real volume *and* a dedup/ordering problem the FSM+content-id doesn't already solve exactly — neither exists today.

---

## Bottom line

All three infrastructure surfaces have real, well-built primitives — but the reuse math is the same one the batch-scan found: **do not manufacture a target the workload doesn't justify.** Telemetry's statistical detector is already right-sized (EMA/1-D-Kalman, full-Kalman consciously deferred), leaving one *latent* Wilson-bound reuse pending a rate consumer; testing already has the industry-recommended structural selection gate and a suite cheap enough that retest-all dominates (with the predictive substrate unbuilt and red-line-barred); notifications are single-event→single-message with exact FSM+content-id dedup and no ranking surface. Two clean HONEST-NEGATIVEs and one small, honestly-latent reuse — consistent with today's research discipline.
