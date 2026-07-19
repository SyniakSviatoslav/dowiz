# OPUS — Fraud / Risk / Anomaly Surface Scan (soft→hard bridging target audit)

Date: 2026-07-19
Mode: research-only (zero code, zero branch/git mutation, this doc is the sole write)
Question: Does dowiz have any EXISTING or clearly-needed fraud / anomaly / risk-scoring surface —
trained or heuristic — where a "soft" probabilistic score resolved to a boolean via a threshold
BEFORE crossing into a hard layer (`RiskScore { score } → verify() -> bool`) would have a genuine
target? Or is this speculative with nothing concrete to attach to (the same honest-negative shape as
`OPUS-TERNARY-BITNET-QUANTIZATION-SCAN-2026-07-18.md`)?

Method: `grep` for `fraud|risk|anomaly|suspicious|trust_score|confidence|spoof|chargeback|abuse`
across `kernel/`, `apps/`, `web/`, `engine/`, and `bebop-repo` (`crates/`, `bebop2/`), plus targeted
reads of every hit. All findings cite `file:line` against the live working tree.

---

## VERDICT (headline)

**No live float-based fraud/risk *scoring* surface exists in the product path today — AND the
architecture carries an explicit, repeatedly-stated policy against ever building one for the money /
courier / vendor paths.** In that sense this is the same honest-negative as the BitNet scan.

**BUT it is not a pure negative.** Two concrete things exist that the operator's pattern touches:

1. **One typed-but-unwired FUTURE slot** where the soft→hard bridge has a real (not-yet-built) home:
   the kernel `FraudAuth` DecisionUnit (`FraudInput → FraudVerdict{ NotAnomalous | Escalate }`).
   It is a scaffold — no live predicate, no data feed — but it is *deliberately shaped* and named.
2. **One LIVE, shipping instance of the exact soft→hard bridging pattern**, in a non-fraud domain:
   `apps/courier/src/voice.rs::classify` — `confidence: f64` thresholded at `< 0.5` → a closed
   `Rejected | Resolved` verdict that **never auto-accepts**. This is the proven in-repo template
   for how the operator's pattern is already done here.

The important design nuance (below) is that dowiz's existing shape is **stricter** than the proposed
`RiskScore { score: f64 } → verify() -> bool`: the kernel keeps floats *out*, quantizes the soft
signal to integer bands at the type boundary (mirroring `money.rs` i64 discipline), and the hard
resolution is **escalate-to-human, never auto-block** (auto-block is made *unrepresentable*).

---

## §1 — The one genuine (future) target: kernel `FraudAuth` DecisionUnit  `[VERIFIED-CODE]`

`kernel/src/decision/mod.rs` — the MoE "mesh" DecisionUnit family (BLUEPRINT-P-F, Layer F,
`kernel/src/lib.rs:178-183`). It defines a closed capability slot for fraud:

- Input (`decision/mod.rs:186-189`):
  ```rust
  pub struct FraudInput {
      pub pattern_score: u32,   // NOTE: u32 integers, NOT f64 — the float never crosses in
      pub velocity: u32,
  }
  ```
- Output (`decision/mod.rs:193-197`): `FraudVerdict { NotAnomalous, Escalate(EscalateReason) }`.
  The module header (`:15-16`, `:190-192`) is explicit: **an auto-block verdict is
  UNREPRESENTABLE by construction** — "a unit that would silently block can never be written… the
  human/operator still decides on escalate."
- Routing (`:296-303`, `AnyUnit::FraudAuth`): selection is an exhaustive `match` on `DomainTag`
  (`:33-40`), which derives `PartialEq/Eq/Hash` but **not `Ord`/`PartialOrd`** (`:29-31`) — the
  no-scoring red-line is enforced at the type level (a router literally has no `cmp` to rank on).

**Wiring status: SCAFFOLD, not built.** The only `FraudAuth` predicate anywhere is a **test stub**
under `#[cfg(test)]` (`decision/mod.rs:399-413`, `fn fraud_unit()`) that ignores its input
(`|_in: &FraudInput|`) and unconditionally answers `NotAnomalous`. `pattern_score`/`velocity` are
only ever set to `0` in tests (`:468`, `:522`). No production construction, no data source, no caller
computes those fields. Grep confirms zero `FraudInput`/`FraudVerdict` usage outside `decision/mod.rs`.

So: a real, deliberately-shaped, escalate-only fraud capability *type* exists — with **no soft
signal, no threshold predicate, and no hard action wired to any product surface.**

---

## §2 — The LIVE soft→hard bridge that already ships (non-fraud): courier voice  `[VERIFIED-CODE]`

`apps/courier/src/voice.rs` is the proven, shipping instance of the operator's exact pattern:

- Soft signal (`:37-42`): `struct VoicePhrase { transcript, confidence: f64, is_final: bool }` —
  a speech-recognition **float confidence**, the closest thing in the repo to `RiskScore { score }`.
- Threshold → hard verdict (`:154-157`):
  ```rust
  pub fn classify(phrase: &VoicePhrase) -> Classification {
      if !phrase.is_final || phrase.confidence < 0.5 {
          return Classification::Rejected;   // never auto-accepts
      }
      ...
  }
  ```
- Closed output (`:62-69`): `enum Classification { Resolved(Intent), Rejected }`. Header comment:
  "ambiguous / low-confidence / consequential-but-unconfirmed ⇒ NEVER auto-accepts… The AI never
  resolves a consequential courier action" (P64 §3).

This is structurally identical to what the operator proposed (`f64 → threshold → bool-ish verdict`),
and it shares `FraudVerdict`'s **escalate/reject-biased** posture: low confidence resolves to the
safe pole, never to a consequential auto-commit. It is the ready-made template for the pattern here.

---

## §3 — GDPR / anonymizer / courier / orders: the premise paths are STALE  `[VERIFIED-CODE]`

The task named `apps/api/src/lib/anonymizer/`, `apps/api/src/routes/owner/gdpr.ts`,
`apps/api/src/routes/courier/`, and `apps/api/src/routes/orders.ts` (from the Repowise index, last
indexed 2026-06-14). **`apps/api` no longer exists.** `find apps/api -name '*.ts'` = 0 files;
`apps/` now contains only `apps/courier/` — a **Rust** crate (`apps/courier/src/{battery,dispatch,
lib,render,surface,types,voice}.rs`). The TS API layer was retired (cf. `crates/bebop/src/detect.rs:3`
"Replaces the TS-retired `anomaly`/`cycle`/`liveness` behaviors as real, tested Rust"). No
`server.ts`/express/fastify remains (`find` = none). The GDPR/anonymizer/orders TS hotspots the task
asked about **are gone**, so there is no risk logic there to find — the index was stale on this point.

The surviving courier crate is **purely deterministic** on the trust/risk axis:
- `apps/courier/src/dispatch.rs:48-49`: `AdvanceReason` is "an ORDER property, not a courier score —
  no variant carries a courier-quality signal (no-scoring red line)."
- `apps/courier/src/render.rs:128,145`: GPS is handled as **presence/absence** ("No live track —
  waiting for GPS", island/no-GPS fallback) — there is **no GPS-spoofing detector/scorer**.

---

## §4 — Everything else that matched is DEV-TOOLING or INFRA self-monitoring, not product fraud

These are real soft→hard bridges but none is a customer/courier/payment fraud surface:

- `kernel/src/metrics.rs:305-349` — **claim-latency anomaly detector** (`claim_latency_floor`,
  `MIN_SECONDS_PER_100_LINES: f64 = 5.0`). Detects an agent claiming an implausibly-fast GREEN on a
  large diff (the "52s on 1610 lines" self-certification residue). **Advisory only, does not gate**
  (`:339` "ADVISORY only — does not gate a merge"). This is *harness* self-honesty, not delivery.
- `kernel/src/analytics.rs` / `kernel/src/wasm.rs` (`reduce_anomalies_js`) / `web/src/...` —
  order-status **transition-legality** anomaly *counting* (illegal state jumps). Deterministic
  FSM check, no score, no threshold-to-action. Surfaced in the web app only as a count.
- `bebop-repo/crates/bebop/src/detect.rs` (N1 z-score utilization anomaly, etc.) +
  `analytics.rs:18 kalman_anomaly` + `stabilizer.rs` "distrust"/drift — **operational-graph /
  agent-mesh self-regulation** (node CPU/mem load, consensus-field drift). Infra health, explicitly
  not courier/customer trust.
- `kernel/src/ports/agent/admission.rs:254-262` — "spoofing" appears only in a **rate-limiter**
  (`AdmissionLimiter` built from `TokenBucket`) that bounds pre-crypto work "regardless of source
  cardinality/spoofing." Mechanical throttle, not a spoof *scorer*.
- `docs/security/skill-scanning.md:27` (skillspector) — a supply-chain "Risk score is a heuristic…
  decide on the categories that fired, not by the number." Dev-tooling for vetting agent skills.
- `kernel/src/hub_provisioning.rs` — `AnomalyFlag` for hub cap-alert / heartbeat-silence. Ops alert.

---

## §5 — Documented product stance: dowiz deliberately does NOT build fraud scoring  `[VERIFIED-DOCS]`

The design record is recent (2026-07-18) and unambiguous — anti-fraud/anti-abuse is **mechanical,
delegated, and reputation-free**, not a scoring layer dowiz owns:

- `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md:173-194` (§16.53): the **mandatory
  online-payment gate is the primary spam defense** (no card, no order); **fraudulent-but-paid
  orders are delegated to provider fraud tools (Stripe Radar / Adyen RevenueProtect)** — dowiz does
  not re-implement payment fraud scoring. The only named gap = **abandoned/attempted-checkout spam**
  (a DoS/nuisance vector), and its assigned fix is the kernel **`TokenBucket`** rate-limiter +
  **Cloudflare Turnstile** edge challenge — *rate/challenge/payment-gate only*, never scoring.
- Same doc `:183`, `:194`: "What to explicitly NOT build: vendor scoring, customer reputation,
  cross-hub blocklists… No reputation, ever." (§16.26/§16.59 no-quality-bar + mesh red-line.)
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P74-moderation-reports-blocklist.md` — moderation is
  **per-hub abuse reports + an opt-in signed subscribable ABUSE blocklist** (categorical
  `ReportReason::Fraud = 1`, `:135`), explicitly "**never reputation or quality**" (`:67-73`), with a
  `decide`-Law category validator (`:31`), NOT a probabilistic score.
- Mesh stance (memory + `docs/design/SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT`): "**trust = signed
  capability, NEVER reputation/blacklist**" (echo-chamber rejection).

Net: the money/courier/vendor paths are red-lined *against* the scoring layer. The `FraudAuth` slot
in §1 is the one place the architecture left a door open, and even it is escalate-only by type.

---

## §6 — IF the FraudAuth slot were built: concrete soft→hard sketch (not a full blueprint)

This is what the operator's pattern would look like targeted at the one real slot (`FraudAuth`),
adjusted to the repo's stricter existing discipline. **This is a sketch to reconcile, not a
recommendation to build — see the caveat.**

- **Soft signal (stays OUTSIDE the kernel).** Float features — checkout velocity, order-pattern
  similarity, retry bursts — computed app-side (storefront-checkout) or by the LLM oracle that
  *compiles* a unit. Floats never cross the kernel firewall (`decision/mod.rs:3` "ZERO network /
  HTTP / JSON / serde"; parallels the `voice.rs` split where the recognizer is upstream).
- **Quantize at the boundary.** `f64 features → u32 bands` = `FraudInput{ pattern_score, velocity }`.
  The integer type IS the firewall, exactly as `money.rs` uses i64 and `metrics.rs` keeps the float
  floor outside the committed decision. This is the key divergence from `RiskScore{ score: f64 }`:
  **the raw score is not the kernel's input; a coarse integer band is.**
- **Threshold / decision (pure).** The `FraudAuth` DecisionUnit's pure `decide()` maps
  `(pattern_score, velocity) → FraudVerdict::NotAnomalous | Escalate`. **No auto-block branch is
  representable** (`:193-197`).
- **Hard action gated.** `Escalate ⇒` route to an **operator review / hold-for-review** signal —
  never auto-cancel the order, never auto-block the courier (courier scoring is type-forbidden,
  §1/§3). Matches `voice.rs`'s "never resolves a consequential action" bias.
- **Audit: commit the outcome, not the score.** The `FraudVerdict` (outcome) is registered as an
  event in the EXISTING sha3 content-addressed event log — `DecisionUnitMeta.content_id`/`prev`
  already ARE the lineage (`decision/mod.rs:54-66`; substrate = `kernel/src/event_log.rs`). The raw
  float never enters the log. This directly satisfies the operator's "commit the outcome, not the
  raw score" idea with zero new machinery.

**Caveat (honest).** Before building this, reconcile with §5: §16.53 assigns **paid-order fraud to
the PSP** and **abandoned-checkout spam to TokenBucket + Turnstile**. So `FraudAuth` today is a
capability slot **without an assigned, un-delegated product problem** — the named gaps are already
owned by cheaper mechanical means. A build should first identify a fraud signal that (a) the PSP
cannot see, (b) rate-limiting cannot catch, and (c) does not smuggle in courier/vendor/customer
reputation. Absent that, `FraudAuth` stays a well-shaped door with no room behind it yet.

---

## §7 — Bottom line

| Question | Answer |
|---|---|
| Existing live float risk/fraud **scoring** in the product path? | **No.** |
| Existing soft→hard **bridge pattern** anywhere live? | **Yes** — `apps/courier/src/voice.rs` (voice confidence, non-fraud). |
| A deliberately-shaped **future fraud slot**? | **Yes but unwired** — kernel `FraudAuth` (escalate-only, integer input, test-stub predicate). |
| A documented **future need** for fraud scoring? | **No — the opposite.** Docs red-line *against* it; anti-abuse is mechanical + PSP-delegated + reputation-free. |
| Speculative-with-nothing-concrete (BitNet shape)? | **Partly.** Concrete *type* + concrete *template* exist; a concrete *un-delegated product problem* does not. |

The operator's pattern is architecturally coherent and even already-instantiated (`voice.rs`), and
the kernel left it a typed home (`FraudAuth`) that is *stricter and safer* than the proposed
`verify() -> bool` (escalate-only, integer-firewalled, outcome-committed). What is missing is not the
mechanism but the **problem**: the product's fraud/abuse surface is deliberately delegated (PSP) or
mechanical (rate-limit/challenge), so there is no un-owned scoring gap to attach to today. Recommend
**do not build** speculatively; keep `FraudAuth` as the documented reservation, and revisit only if a
PSP-invisible, rate-limit-immune, reputation-free signal is identified.

### File:line evidence index
- `kernel/src/decision/mod.rs:15-16,29-40,186-197,296-303,399-413,468,522` — FraudAuth slot + test stub
- `kernel/src/lib.rs:178-183` — BLUEPRINT-P-F Layer F description (FraudAuth escalate-only)
- `apps/courier/src/voice.rs:37-42,62-69,154-157` — live confidence→verdict bridge
- `apps/courier/src/dispatch.rs:48-49`, `render.rs:128,145` — no courier score, no GPS-spoof detector
- `kernel/src/metrics.rs:305-349` — claim-latency anomaly (dev-tooling, advisory)
- `kernel/src/ports/agent/admission.rs:254-262` — spoofing = rate-limiter, not scorer
- `bebop-repo/crates/bebop/src/{detect.rs,analytics.rs:18,stabilizer.rs}` — infra/mesh anomaly, not product
- `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md:173-194` — PSP-delegated, mechanical anti-abuse
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P74-moderation-reports-blocklist.md:67-135` — abuse reports, never scoring
- `apps/api/**` — GONE (stale index); no TS GDPR/anonymizer/orders risk logic exists
