# Layer Blueprint — Reliability / Observability / Degrade for the Decentralized Local-First Hub

> **Blueprint, 2026-07-11 (late).** The operational spine that makes the hub "a Swiss watch that
> survives a nuclear war" — deterministic, self-healing, observable — under the HARDER constraint
> of no central backend. Zero code changed; repos read-only (code is being changed by a parallel
> session — every "fix" below is a spec that session or a successor executes). The only file
> created is this blueprint.
>
> **Ground truth inputs:** `G12-ops-broken-queue.md` §2.4 (the degrade-storm trap, item #15),
> `G04-rebuild-cutover-rebaseline.md` §2.3/A1 (the h_t frame + auto-degrade mechanism),
> `SYNTHESIS.md` + `C-runtime-transport-identity.md` + `05-protocol-tech-completion-blueprint.md`
> (the node this layer monitors), `docs/design/harness/META-CONTROLLER.md` (the deterministic-gate
> philosophy extended here), `.claude/skills/reliability-gate/SKILL.md` (the L0–L11 gate extended
> in §4), `docs/design/dowiz-brand/INTERFACE-DIRECTION-2026-07-11.md` (degrade must be FELT calmly).
>
> **Fresh verification, this session (2026-07-11):** the degrade-storm trap is **STILL ARMED** —
> `grep -iE "boot.?grace|bootAt|grace"` over `apps/api/src/lib/cutover/{front-door,flags}.ts` = 0
> hits; `autoDegrade` fires unguarded at `front-door.ts:337` (breaker trip) and `:422`
> (per-request); `flags.ts:140-161` alerts via `log.error` only (no bus event, no Sentry, no
> Telegram); `grep restart apps/api/tests/cutover-front-door.test.ts` = 0 hits. All three ratchet
> parts of task #15 remain missing. **VERIFIED.**
>
> Labels: **VERIFIED** (checked in-repo or via fetched source this session),
> **VERIFIED-in-repo-doc** (carried from a cited sibling doc that verified it),
> **VERIFIED-web-search** (claim confirmed at search-result level, source cited, page not
> deep-audited), **UNVERIFIED**, **DESIGN-JUDGMENT**.
>
> **Standing decisions honored throughout (binding, not re-litigated):** local-first is the
> destination; **COD is the most-robust rail and the design anchor**; **NO courier scoring** (no
> courier-keyed metric, ever); **anonymity** (metrics carry no PII; crypto-shred compatible);
> **multichannel, no dedicated app**; **storefront sovereignty** (`/s/:slug` gets no dowiz
> chrome); **hybrid-crypto rule** (every value-bearing signature verifies on the audited classical
> half — Phase A/H of doc 05).

---

## 1. Graceful degrade, FIXED — and the nuclear-war minimum

### 1.1 The trap, named exactly (why this layer exists)

The 2026-07-05 incident (h_t frame `docs/ops/rebuild-cutover-h_t.json:49`, G12 §2.4 — VERIFIED
against current code this session): a routine Node restart raced the health prober's
**immediate-on-boot** probe window; three consecutive fails (≈15 s of Rust unreachability during
its own redeploy) tripped the breaker, which **silently and persistently** flipped every non-money
surface to Node while Rust was healthy. Discovery was **accidental**, ~2 hours later. Three
structural absences made it possible, and all three are general laws this layer must encode:

| Absence | The general law it violates |
|---|---|
| No boot-grace — prober counts failures from t=0 (`front-door.ts:123-128`) | **A system booting is not a system failing.** No persistent decision may be taken on evidence gathered before the first successful health cycle (or a bounded grace deadline). |
| Alert = `log.error` only (`flags.ts:140-161`) | **A degrade is an event, not a log line.** Anything that changes what serves traffic must alert a human through a channel a human actually watches. Log-only alerting is the #1 reason discovery was accidental. |
| No restart-regression test | **Every incident becomes a RED test or it recurs.** The FakePool harness already records `degradeCalls` (`cutover-front-door.test.ts:33-41`) — the test was cheap and never written. |

The Azure/industry circuit-breaker literature covers trip/half-open/recovery extensively but is
thin on the boot-window case ([Microsoft circuit-breaker pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker)
— VERIFIED-web-search; the 07-05 incident is our own primary source and a sharper one). The
one-way flag ratchet (recovery re-enables forwarding but never un-flips flags, REV-C5 by design)
is what turned a 15-second race into a 2-hour silent outage-of-truth.

### 1.2 The fix for the CURRENT harness (spec — executed by the G12 batch-C / G04-A1 owner)

This is D4 in G04's operator sheet ("justified on every branch of D3 except full-inert") and
batch C of G12. Restated here once as the layer's first phase, then generalized. Three parts,
one session:

1. **Boot-grace** in `UpstreamHealth`: record `bootAt`; expose `inBootGrace` (true until first
   `recordOk()` OR `now - bootAt > BOOT_GRACE_MS`, default 120 s, env-tunable). Both degrade
   vectors — the trip callback (`:334-340`) and the per-request path (`:422`) — skip
   `autoDegrade` while `inBootGrace`. Forwarding still fails safe per-request to Node during
   grace: users lose nothing; only the *persistent flag write* is suppressed.
2. **Alert-on-degrade**: `flags.autoDegrade` success path publishes `ops.cutover_degrade`
   `{surface, reason, ts}` on the message bus + `getSentry()?.captureMessage` (the exact pattern
   of the refund-due fold, `orderStatusService.ts:182-195`), alongside the existing log.
3. **Restart-regression test** in the existing FakePool harness — **RED on today's code by
   construction**: (i) boot + 3 failed probes inside grace → `degradeCalls.length === 0`;
   (ii) after first OK, 3 fails → `degradeCalls > 0` (no infinite immunity);
   (iii) grace expiry with upstream never-up → degrade fires + exactly one alert event.

**VbM:** test (i) is the falsifier — revert the grace and it reproduces the 07-05 storm.
Staging drill (G12 C3): restart Node with Rust healthy → `SELECT` on `cutover_flags` shows zero
flips + one grace log line. **Effort:** 1 session (M). **Deps:** none. **Gate:** none (non-money,
additive, reversible via `BOOT_GRACE_MS=0`).

### 1.3 The degrade laws for the decentralized node (the lesson, generalized)

In the decentralized hub there is no `cutover_flags` table — but the same failure class exists
wherever a component *decides to stop trusting another component*. The node runtime (doc 05
Phase R: `sequencer.rs`, `dispatch.rs`, `sync.rs`, `adapters/*`, `fiscal.rs`) gets five degrade
laws, each enforced by a test that can go RED (DESIGN-JUDGMENT, derived from the incident):

- **D-LAW-1 (boot-grace is universal).** Any health-driven mode change (marking the relay dead,
  marking the warm-spare stale, closing an adapter) is suppressed until the first successful
  cycle of the thing being judged, or a bounded grace deadline. Per-request fail-safe behavior is
  always allowed; *persistent state changes* during boot are not.
- **D-LAW-2 (degrade is a signed event).** Every degrade/recover decision is appended to the
  node's own **ops stream** as a signed envelope `DegradeChanged{component, from, to, cause,
  ts}` — same store, same hash chain, same replay as orders. A degrade that isn't in the event
  log didn't happen; a degrade in the log is alertable, auditable, and replayable. This kills the
  log-only class structurally: the alerting layer (§2) subscribes to the stream, not to logs.
- **D-LAW-3 (one-way ratchets need a dead-man).** Anything that ratchets toward a safe-but-worse
  mode and waits for a human must emit a *repeating* signal until acknowledged (signed
  `DegradeAcked` event), never a single line. The 07-05 storm stayed invisible because the
  ratchet was silent after the first line.
- **D-LAW-4 (money never auto-degrades).** Carried verbatim from REV-C5: the COD/settlement path
  refuses automated mode flips. In the node this means `sequencer.rs` and `settlement.rs` have
  **no degrade modes at all** — they are either exactly correct or stopped (fail-closed HOLD, doc
  05 Phase X). Degrade vocabulary applies only to transport, adapters, sync, and ambience.
- **D-LAW-5 (every incident → a RED case in the gate).** The restart-regression test class is
  institutionalized as gate stage LD10 (§4) — a restart drill is part of every reliability-gate
  run, forever.

### 1.4 The honest fallback ladder (what actually keeps working, per failure)

The decentralized topology (SYNTHESIS §4, 02 §3, C-lens §§1-2): vendor always-on node (the only
true always-on participant) + one dumb €6 relay (encrypted-forward only) + push-woken courier PWA
+ one-shot customer browser + warm-spare replica (Litestream follow). The ladder below is the
design contract — each rung names what SURVIVES, what QUEUES, and what STOPS. COD is the anchor:
because no digital money moves (obligations settled by counter-signed custody hand-offs, 02 §3.8),
**an outage can delay bookkeeping but can never lose or invent money** — that is why cash is the
most robust rail and why every rung below bottoms out on it.

| Rung | Failure | SURVIVES | QUEUES (store-and-forward) | STOPS | Detection signal |
|---|---|---|---|---|---|
| F1 | **Relay down** (€6 box dies / DDoS) | Everything on-premises: LAN orders (Zenoh multicast / direct QUIC), counter orders, kitchen flow, courier handoff at the venue, COD collection | Off-premises customer web leg; courier sync while off-LAN (re-syncs on return or relay recovery) | New remote web orders | Node's relay-reachability check fails ≥N cycles post-grace → signed `DegradeChanged{relay}` → operator alert + customer channels fall back to phone/messenger adapters (multichannel is itself a fallback ladder — no-app ruling pays off here) |
| F2 | **Internet out at venue** | Island mode: full kernel (decide/fold/sign) local; LAN devices; phone-in orders typed by staff; COD; **fiscalization queues legally** — Law 87/2019 has a 48 h offline grace, `fiscal.rs` is queue-and-drain by design (02 §3.9 — VERIFIED-in-repo-doc) | Push wakes, remote orders, heartbeats (§2), fiscal drain | Remote channels | Heartbeat *absence* at the aggregator stub (dead-man, §2.3); locally the node knows and shows island-mode |
| F3 | **Vendor node restarts** | Everything after replay-verify-on-open (Phase S: walk streams, verify chain + sigs, fold, compare `head_hash`) — seconds | Inbound frames buffered by adapters during replay | Nothing, if replay is green | D-LAW-1: no degrade decisions during boot; one signed `NodeRestarted{replay_ok, head_seqs}` event. A red replay **refuses to serve the divergent stream and names the seq** (Phase S RED case) — never silently serves |
| F4 | **Vendor node DEAD** (the dinner-rush phone-dies case, B-lens §4.3) | Warm-spare promotion — but **single-writer handover is an explicit signed act**, never automatic: the spare serves reads immediately, and takes the sequencer role only on a signed `WriterHandover` (operator/owner act) + exclusive-writer lock (Phase R RED case c). Split-brain is structurally refused; a stale spare that can't prove `head_hash` continuity must resync first | New commands (customers see honest "venue reconnecting" state, §1.6) | Order intake, until promotion (minutes, drilled) | Spare detects missed heartbeats from its own node; alerts; promotion is push-button, not automatic — availability is traded for never-forking money truth (DESIGN-JUDGMENT, consistent with D-LAW-4) |
| F5 | **Courier device lost/offline** | Order flow (dispatch re-offers on capability `exp` timeout — no server callback needed, C-lens §3.3/§4.1); other couriers | The lost courier's unsynced signed events (recoverable: they're signed — re-sync or re-issue) | That courier's leg | Offer-unclaimed dwell timer on the node (dead-man per potential stuck point — the gate's thread 5); lost device → signed roster revocation, re-enrollment (C-lens §4.2 — key recovery is honestly impossible, re-enrollment is the answer) |
| F6 | **Push gateway (APNs/FCM) down** | Foregrounded courier apps; LAN; SMS/call fallback by staff | Wake signals (push is wake-only, never trusted state — C-lens §4.1) | Timely wake of backgrounded couriers | Push-send failure count in health vector; dispatch falls back to longer offer windows + voice-call escalation prompt |
| F7 | **EVERYTHING down** (the nuclear-war rung) | **The venue sells food for cash.** Pad-and-pencil order capture; courier carries cash; the counter-signed custody handoff is reconstructed on recovery as backdated signed events (both parties sign on reconnect — Σ=0 still provable, just late); fiscal within the 48 h grace | All digital records until any device recovers | Everything digital | Human eyes. The design goal is that F7 is a *bad day, not a data-loss event*: the event log is append-only truth, and truth arriving late is still truth |

### 1.5 The nuclear-war minimum (the invariant floor, stated once)

What must survive ANY combination of failures — the falsifiable definition of "survives a nuclear
war":

1. **Cash can always be taken.** COD requires no network, no server, no dowiz. (Anchor rail.)
2. **Money can never fork.** Single-writer + counter-signed custody + Σ=0 fold means no failure
   mode creates two truths about who owes what — at worst, truth is *delayed* (F7 backfill).
3. **Recovery is replay, and replay is verified.** Any surviving copy of the signed event log
   reconstitutes the exact state (hash-chain + signature verify + deterministic fold). A
   corrupted copy is *detected and refused*, never served (Phase S RED case).
4. **No silent mode changes, ever.** Every degrade is a signed event + a repeating alert
   (D-LAW-2/3). The 07-05 class is structurally extinct.
5. **The customer is never lied to.** Degrade states render truthfully and calmly (§1.6) — an
   honest "slow right now" outranks an infinite optimistic spinner (interface direction §5.3).

### 1.6 Degrade must be FELT, calmly (the interface tie — binding, not decorative)

The interface direction already reserves an exact vocabulary for this layer, and this layer is
its only legitimate data source (INTERFACE-DIRECTION §2.1, §3.1, §5.3-5.4 — VERIFIED read):

- **Degradation renders as desaturation along `--spectral-void`** — the room goes grey, never
  red-floods; `ops.degradation_changed` (Node era) / the `DegradeChanged` signed event (node era)
  is the ONLY trigger. "The desaturation vocabulary is reserved for genuinely degraded health" —
  which cuts both ways: this layer must emit the event truthfully, and nothing else may fake it.
- **Sustained states derive from store state, never replayed events** — after an offline window
  the UI reconciles to the final posture in one crest, not N animations of missed states.
- **Customer tracking in degrade:** the status ladder stays exact, the ETA range widens honestly,
  one plain line ("Connection to the venue is slow right now — your order is safe") per the
  stakes rule; the atmosphere desaturates one step. Island/reconnecting states (F2-F4) get the
  same treatment. On `/s/:slug` (sovereign storefront) degrade UI is the plain, unbranded system
  chrome dowiz owns — no spectral language on the vendor's stage (§4.0 sovereignty).
- **Courier PWA:** connection state + last-synced seq always visible (daylight register: density,
  not glow); an unsynced-events count is a number, not an alarm.
- **Errors carry a handle:** every degrade-adjacent error surfaces `code` + short correlationId
  handle ("this failure has a name") — the error-contract discipline made visible.

---

## 2. Observability WITHOUT a central server

### 2.1 The constraint set (harder than centralized ops, and the red lines)

Centralized observability assumes a trusted collector that sees everything. Here: no central
backend (the relay is dumb and untrusted), **metrics must carry zero PII** (anonymity ruling:
per-order PII lives in a sealed envelope, crypto-shredded after the dispute window — metrics must
not become a side-channel that reconstructs shredded orders), **NO courier scoring** (a courier
id may never key a metric series, a latency histogram, or a health dashboard — this is both a
red line and legally load-bearing, SYNTHESIS §9), and **storefront sovereignty** (no observability
chrome on `/s/:slug`). The honest architecture: **each node observes itself with falsifiable
checks and emits signed, content-free heartbeats; aggregation is optional, thin, read-only, and
untrusted** — the same trust shape as the relay floor (C-lens §2.4: carries, never decides).

Industry direction agrees: edge observability processes locally and ships only aggregated/critical
signals ([SigNoz edge-observability guide](https://signoz.io/guides/edge-observability/) —
VERIFIED-web-search); push-with-provenance beats pull for NATted edge devices, with signed
heartbeats for origin evidence ([SRE School heartbeat guide, 2026](https://sreschool.com/blog/heartbeat/),
[OneUptime IoT monitoring](https://oneuptime.com/blog/post/2025-09-24-monitoring-iot-devices-with-oneuptime/view)
— VERIFIED-web-search). We go one step further than the industry pattern: the collector is
*untrusted* — signatures make heartbeats verifiable by anyone, forgeable by no one.

### 2.2 Per-node health — the node monitors itself, falsifiably

`node/health.rs` (new module, Phase R adjunct): a deterministic health vector recomputed each
cycle (default 30 s), where every check is a real assertion with a RED input, never a vibe:

| Check | Assertion | RED input (VbM) |
|---|---|---|
| `chain_ok` | Incremental hash-chain + signature verify of events appended since last cycle | Flip a byte in a new event → RED |
| `replay_ok` | Last full replay-identity proof (§3.2) timestamp < 24 h and green | Corrupt a projection row → next proof RED |
| `store_ok` | WAL checkpoint lag < threshold; disk free > threshold; `synchronous=FULL` still set on the event-append path | Flip PRAGMA in a test double → RED |
| `relay_ok` / `push_ok` / `fiscal_ok` | Reachability + queue-depth bounds (fiscal queue age < 40 h — alarm BEFORE the 48 h legal wall, not at it) | Stub gateway down → RED after grace (D-LAW-1) |
| `clock_skew` | Median skew estimate vs signed peer timestamps within acceptance window (C-lens §4.3 — bounded windows, no trusted clock) | Skewed test peer → RED |
| `seq_progress` | Per-stream dwell: no open order stuck in one status past its stage dwell budget (the dead-man per stuck point — reliability-gate thread 5, generalized) | Freeze a fixture order → RED |
| `caps_ok` | Capability verifier self-test: one known-good verifies, one known-forged refuses (the hybrid-rule RED from doc 05 Phase A, run continuously) | If the forged cap ever verifies → RED (the gravest possible signal) |

Health is a **fold over the ops stream + current checks**, not a mutable global — deterministic,
replayable, and the D-LAW-2 events are its history.

### 2.3 Signed heartbeats — the wire format of health

A new frame type on the Phase W wire (`HEALTH`, reserved in the frame_type enum on day one):

```
Heartbeat = borsh {
  node_id,            // self-cert id (venue/node identity — NEVER a courier id)
  hb_seq, ts,         // monotonic + caller-supplied ts (not trusted, bounded)
  health_vector,      // the §2.2 bits + coarse gauges (queue depths, skew ms)
  stream_heads: [{stream_kind_bucket, count, max_head_seq, agg_head_hash}],
                      // COUNTS and HASHES only — no order ids, no content, no PII
  degrade_state,      // current D-LAW-2 fold (component → mode)
}  + signature block (hybrid rule: Ed25519 classical half MANDATORY)
```

Properties (DESIGN-JUDGMENT, each with a RED case in Phase RG-2):

- **Verifiable by anyone, forgeable by no one.** Any receiver (courier device, warm-spare,
  aggregator stub) verifies the signature against the node's self-cert id. A relay or aggregator
  compromise can *drop* heartbeats but never *fake health* — absence is detectable (dead-man),
  forgery is impossible. This is the decentralized answer to "who watches the watcher": math does.
- **Dead-man semantics.** The signal of failure is *missing* heartbeats, evaluated by every
  subscriber independently (spare: promote-prompt after N misses; aggregator: page the operator;
  courier PWA: show reconnecting). No component needs the node's cooperation to detect its death.
- **`agg_head_hash` makes health PROVABLE, not asserted** (§3.3): two parties claiming the same
  `max_head_seq` with different hashes have detected divergence — without either seeing content.
- **Anonymity by construction:** the heartbeat schema physically cannot carry PII (counts,
  hashes, gauges only); a schema-lint test asserts no string fields beyond enum names — RED if a
  free-text field is ever added.

### 2.4 Customer- and courier-visible status (observability as a product surface)

- **Customer (tracking page):** status = the signed event chain of *their* order, verified
  locally (the one-shot browser runs a keyless WASM verifier for its own slice — C-lens §1.4).
  Degrade shows per §1.6. The page never claims more than the chain proves — one crest per
  committed transition, "optimistic shimmer on unconfirmed state is lying with light."
- **Courier (installed PWA):** connection state, last-synced seq, unsynced-signed-events count,
  and the offer countdown made honest ("task offered" ring — interface §4.3). No self-score, no
  leaderboard, no per-courier stats screen — observability shows the *system's* health to the
  courier, never the courier's "performance" to anyone (red line).
- **Owner (console):** the room-feel is the ambient rendering of THIS layer's events (quiet amber
  shimmer = flowing; slow greying = `DegradeChanged` before you read the toast — interface §4.2),
  plus a plain health panel: the §2.2 vector, fiscal queue age, spare sync lag, last replay-proof
  badge (§3.5).

### 2.5 The operator fleet view — aggregating WITHOUT re-centralizing (the §2.4-stub option)

Three tiers, in order of preference; the design accepts tier 3 explicitly:

1. **No aggregator (pure).** The operator's own device subscribes to its venues' heartbeat topics
   over the same wire (iroh dial by NodeId). Works for 1–10 venues; the operator device is just
   another verifier. Cost: fleet view is down when the operator's laptop is.
2. **Peer witnessing (later, earn-it).** Venue nodes exchange heartbeats and gossip missing-peer
   alarms — no new box. Deferred: cross-venue topology is post-"one node among many" (02 §6);
   building it now is over-engineering before G11.
3. **The thin read-only aggregator — the accepted stub (the C-lens §2.4 relay-floor shape).** The
   SAME €6 Hetzner box that runs the relay also runs `hb-sink`: an append-only store of signed
   heartbeats + a read-only status page + alert fan-out (Telegram/ntfy to the operator). Trust
   analysis: it can **verify but not forge** (signatures), **sees counts and hashes, never
   content or PII** (schema §2.3), and **its loss loses only the fleet view, never truth**
   (heartbeats keep flowing peer-to-peer; every node keeps its own ops stream). It is a
   *convenience mirror of signed facts*, not a system of record — the exact trust shape of the
   relay itself ("carries, never decides"), which is why it does not re-centralize the
   architecture. It is also the natural home of the dead-man pager: the one component whose job
   is to notice silence. **This stub is the recommended MVP answer** (DESIGN-JUDGMENT): tier 1
   alone reproduces the 07-05 discovery-was-accidental failure at fleet scale.

### 2.6 The metric vocabulary (what may be measured, and what may not)

Allowed series — all keyed by `{venue_id, component}`, never finer:
orders/hour count, per-stage dwell histograms (order-level, venue-keyed), offer→claim latency
distribution (**venue-keyed, deliberately NOT courier-keyed** — it measures dispatch health, not
people), sync lag (node↔spare, node↔courier-fleet as an aggregate), fiscal queue age, relay RTT,
heartbeat gap distribution, replay-proof durations, Σ-check pass timestamps.

Forbidden by test, not by prose (CI greps + schema-lints, each RED-provable):
any metric keyed by courier id (grep-gate over `node/` + `hb-sink` — the doc-05 Phase X CI test
extended to the metrics namespace); any customer identifier, address, phone, or handle in any
metric/log/heartbeat; any free-text label field in the heartbeat schema; retention beyond the
crypto-shred window for anything order-linked (aggregates survive; order-linked rows die with the
envelope key).

---

## 3. Determinism guarantees made VISIBLE (Verified-by-Math, operationalized)

### 3.1 The three exactness claims this layer must prove continuously

The hub's whole trust story rests on three mathematical claims (SYNTHESIS §3, doc 05):
(1) **the state machine is exact** — the byte-frozen 10-status machine, one door
(`kernel::decide`), illegal transitions refused; (2) **replays are identical** — fold(events) is
deterministic, so any copy of the log reconstitutes the same state hash; (3) **money nets to
zero** — Σ over an order's obligation accounts returns to 0 at close (`ledger.rs:79-81` pattern,
Phase X). VbM's demand (CLAUDE.md standing rule): each claim needs a *continuously re-run,
falsifiable* proof — an assertion with a defined input that turns it RED — not a launch-day test
that rots.

### 3.2 The falsifiable self-checks (the node proves its own health)

`dowiz-node verify` — a subcommand suite, run nightly by the node itself + on every boot + on
demand; every result appended to the ops stream as a signed `SelfProof{kind, ok, detail_hash}`:

| Self-check | Proof | RED input (shipped as a fixture, per VbM) |
|---|---|---|
| **Replay-identity** | Fold every stream from seq 0 in a scratch context → `state_hash` must equal the live projection hash; and equal yesterday's for closed streams | Flip one byte of one `event_bytes` row in a copy → detect + name the exact seq; delete a middle event → `prev_hash` mismatch; reorder two → RED (Phase S REDs, run forever, not once) |
| **Conservation (Σ=0)** | Two independent computations — incremental balance vs full fold — agree; every CLOSED order's accounts sum to exactly 0 `Lek(i64)`; every OPEN order's imbalance equals its open obligations | Inject a single-signed custody handoff into a fixture → ledger refuses, Σ≠0 surfaces (doc 05 Phase X RED a); replay a handoff with a new nonce → second application rejected (RED b) |
| **Transition-legality** | Re-run the full event log through the transition table; count of refused transitions must be 0 in the committed log (illegal ones can never have been appended) | Fixture log with one `IN_DELIVERY→PICKED_UP` (the exact G12 §2.3 phantom class) → RED — the Node-era pickup-proxy bug becomes structurally impossible AND continuously proven impossible |
| **Signature audit** | Every value-bearing envelope verifies on the classical half (hybrid rule as a batch re-verify, not just at ingest) | A fixture envelope with valid ML-DSA + absent Ed25519 must refuse (doc 05 Phase A RED — the hybrid gate's own falsifier) |
| **Determinism cross-check** | `agg_head_hash(node) == agg_head_hash(spare)` at equal `max_head_seq` (via heartbeats, §2.3) | Diverge the spare's copy in a drill → both sides alarm on the next heartbeat exchange |

**The design stance: health is a theorem the node re-proves on schedule, not a status it
asserts.** A node that cannot produce a green `SelfProof` within its window is *treated as
degraded by its subscribers* regardless of what it claims — the dead-man applies to proofs, not
just to liveness.

### 3.3 Hash-comparison health — the decentralized trick that replaces the central dashboard

Because state is a deterministic fold of a signed log, **one 32-byte hash carries what a
centralized system needs a metrics pipeline to say**: same `head_seq` + same `head_hash` ⇒
byte-identical state, verified across any two parties (node↔spare, node↔courier slice,
node↔aggregator), with **zero content disclosure** — which is why this composes with anonymity
where a traditional trace pipeline cannot. Divergence detection is O(1) per heartbeat and
trustless. This is the load-bearing observability primitive of the whole layer.

### 3.4 The deterministic-simulation harness (how the gate exercises all of it)

The kernel is already clock-free and RNG-free (caller-supplied `Ts`, no clock reads in core —
C-lens §4.3 VERIFIED-in-repo-doc), and `transport-mem` is the deterministic test double every
transport must conformance-match (doc 05 Phase W). That means the FoundationDB/TigerBeetle-class
**deterministic simulation testing** pattern is unusually cheap here: a seeded simulator drives
full order lifecycles through real `decide`/fold/ledger code with fault injection — relay kill,
node `kill -9` mid-append, message duplication/reordering/delay, restart storms, clock skew — and
any failure replays exactly from `(seed, commit)` ([TigerBeetle VOPR internals](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md),
[WarpStream's whole-SaaS DST](https://www.warpstream.com/blog/deterministic-simulation-testing-for-our-entire-saas),
[Antithesis protocol-aware DST](https://antithesis.com/bugbash/talks/protocol-aware-deterministic-simulation-testing/),
[Jepsen × TigerBeetle × Antithesis](https://jepsen.io/analyses/tigerbeetle-0.16.11) — all
VERIFIED-web-search; the single-seed-replay property is the headline). The simulator's invariant
set IS §3.2's self-checks: fold-identity, Σ=0, transition-legality, single-assignment. **RED
case for the harness itself:** a seeded known-bug fixture (e.g., a build with the boot-grace
reverted) must fail under simulation — a simulator that passes a known-bad build is a
false-positive metric and does not validate (VbM).

### 3.5 Making it visible (the trust cues — determinism as a felt product property)

- **The proof badge:** owner console and (post-order) customer tracking footer carry a quiet mono
  line — `verified: replay ✓ seq 48211 · Σ=0 ✓ · 03:12` — the last `SelfProof`, stated, never
  performed (voice law: the machine states; "quiet flex" §5.5). No green-checkmark theater: the
  badge renders RED with the failing seq when a proof fails, or it isn't honest.
- **Money never tweens; digits cut to exact values** (§5.1) — the rendering rule that only a
  Σ=0-proven core has the right to.
- **One transition, one crest** (§5.2) — the tide swell fires off the committed signed event,
  which this layer guarantees exists exactly once.
- **Degrade desaturation is driven only by D-LAW-2 events** (§1.6) — the ambient layer becomes a
  *truthful* instrument because its only input is the proven ops stream.

---

## 4. The reliability gate, EXTENDED — LD0–LD11 for the decentralized path

### 4.1 The extension philosophy (META-CONTROLLER discipline, kept)

The existing gate (`.claude/skills/reliability-gate/SKILL.md`) traces ONE order L0–L11 through
the centralized stack with proof-by-artifact and a GO/NO-GO verdict. The extension follows the
meta-controller's law: **extend with new nodes, never rewrite authority** — the L0–L11 gate stays
as-is for the Node stack; a new parallel trace `LD0–LD11` covers the decentralized path, run by
the same skill when a node deployment exists. Deterministic checks, proof-by-artifact ("should
work" = FAIL), additive.

**The seven threads** (five inherited + two new, checked on every run):
1. 🔴 Exactly-once (one order, one assignment, one PoD, one settlement — on every surface)
2. 🔴 Recoverable (kill at any stage → no orphans, replay reconstitutes)
3. 🔴 Cross-surface consistent (customer chain == courier slice == node fold == spare fold)
4. 🔴 Proof-by-artifact (file:line / drill transcript / hash pair, or FAIL)
5. 🔴 Timely signal (a dwell/dead-man exists at every potential stuck point)
6. 🔴 **Signed-everything** (every value-bearing artifact in the trace verifies on the audited
   classical half; one deliberately-forged artifact per run must refuse)
7. 🔴 **Honest-degrade** (every degrade induced during the trace produced a signed event + a
   repeating alert + the truthful UI cue; any log-only degrade = NO-GO)

### 4.2 The LD0–LD11 trace (channel → node → dispatch → COD settlement)

| Stage | What is traced | PASS criteria (artifact required) |
|---|---|---|
| LD0 | **Channel entry** — QR/`/s/:slug` web, messenger adapter, (later `.onion`) → order intent | Intent reaches exactly one adapter; sovereign storefront carries zero dowiz chrome; channel privacy label rendered (04-revision: label≠enforce, but label must exist) |
| LD1 | **One door** — adapter → `Command::PlaceOrder` into the sequencer queue | Grep-gate artifact: no write path outside `sequencer.rs` (Phase R RED b); `cause_hash` real, never `"placeholder"` (the hub-review finding #1, closed and *kept* closed) |
| LD2 | **Decide + append** — `kernel::decide` → signed envelope → SQLite append | One atomic append; `content_hash` dedup proven by double-submit → single event; server-priced total in `Lek(i64)`; illegal transition fixture refused |
| LD3 | **Fan-out + wake** — EVENT frames to customer WSS; push-wake to courier | Customer page verifies its chain locally; push carries wake-only (no state); courier reconnect pulls + verifies the offer |
| LD4 | **Dispatch** — offer minted as capability with `exp` | Capability verifies offline (no DB, no network — Phase A VbM); **artifact: dispatch code path contains zero reputation/courier-score input** (red-line grep, RED-provable) |
| LD5 | **Claim** — single-writer arbitration | Two simultaneous claims → exactly one `Assigned`; the loser gets a refusal, not a race |
| LD6 | **Custody** — `CashCollected` counter-signed | Single-signed handoff refused (Σ stays ≠0 — Phase X RED a); customer OTP/countersign present |
| LD7 | **Delivery** — PoD signed at location | `pod.rs` verify green; replay-at-wrong-location fixture RED (`pod.rs:153-165` class, live); courier id in PoD is the pseudonymous vault id, no PII |
| LD8 | **COD settlement** — `SettlementReceived` counter-signed → close | **Σ = 0 over the order's accounts, computed by the independent fold** (thread 6 artifact = the hash + the fold transcript); double-settle fixture refused (RED b); dispute-timeout fixture lands in HOLD, not SETTLE (RED c) |
| LD9 | **Convergence** — customer slice, courier slice, warm-spare | Equal `head_seq` ⇒ equal `head_hash` across all three (§3.3 artifact: the hash triple); crypto-shred drill: PII envelope key destroyed → order still replays, PII unrecoverable, metrics unaffected |
| LD10 | **Degrade drills, mid-trace** (D-LAW-5 institutionalized) | (a) relay killed between LD3–LD7 → order completes on LAN or queues honestly; (b) node `kill -9` mid-append → restart, replay green, interrupted append absent-or-exactly-once (Phase S VbM); (c) **restart drill: zero persistent degrade decisions during boot-grace** (the 07-05 regression test, forever); (d) each induced degrade produced signed event + repeating alert + UI desaturation (thread 7) |
| LD11 | **Cross-cutting matrix** | 10-row surface matrix (customer page / courier PWA / owner console / heartbeat / aggregator stub / spare / fiscal queue / ops stream / metrics / alert channel); **anonymity audit artifact: zero PII and zero courier-keyed series in everything emitted during the whole trace** (mechanical grep over captured heartbeats/metrics/logs); sovereignty check on LD0 captures |

**Verdict:** GO = all LD0–LD11 PASS with artifacts, all seven threads green, all shipped RED
fixtures still RED. NO-GO = any FAIL, any duplicate on any surface, any log-only degrade, any
courier-keyed metric, any PII in observability output, Σ≠0 anywhere, or a RED fixture that has
gone green (a RED case that stops failing means a check died — meta-RED). A Known-debt flag-only
list may exist per the current skill's discipline, but threads 6/7 and the anonymity audit are
never flag-only.

### 4.3 Phase plan (entry precondition · module layout · VbM RED · effort · dependencies)

| Phase | Entry precondition | Module layout | VbM RED case (ships with the green) | Effort | Depends on |
|---|---|---|---|---|---|
| **RG-1 — kill the Node-era trap** (task #15; spec §1.2 — executed by the parallel/G12-C session, NOT this one) | none (current tree; FakePool harness exists) | `front-door.ts` (boot-grace in `UpstreamHealth`, both degrade vectors gated), `flags.ts` (bus + Sentry alert), `cutover-front-door.test.ts` (+3 tests) | Boot + 3 failed probes inside grace → `degradeCalls.length === 0` — **RED on today's code by construction** (verified again this session) | 1 session (M) | none |
| **RG-2 — health + signed heartbeats** | Phase W frame schema drafted (`HEALTH` frame type reserved day-one) | `node/health.rs` (the §2.2 vector), `node/ops_stream.rs` (D-LAW-2 signed degrade events + `DegradeAcked`), heartbeat emitter, `bebop-wire` HEALTH frame | Forged heartbeat (bad classical sig) refused by every subscriber; missed-heartbeat dead-man fires after N gaps (and does NOT fire at N-1 — no crying wolf); free-text field added to heartbeat schema → schema-lint RED | 3–4 sessions | W (frames); parallels S/R |
| **RG-3 — the self-proof suite** | Phase S landed (event store + replay-verify-on-open) | `dowiz-node verify` subcommands: replay-identity, Σ=0 dual-computation, transition-legality, signature audit; `SelfProof` events; nightly scheduler in `main.rs` | Flipped byte / deleted event / reordered pair in a scratch copy → detected, exact seq named, stream refused; single-signed handoff fixture → Σ≠0 surfaced; the `IN_DELIVERY→PICKED_UP` fixture → refused | 3–4 sessions | S; X for the Σ checks (Σ subset lands with X) |
| **RG-4 — the degrade-ladder runtime** | Phase R runnable node on staging | `node/degrade.rs` (D-LAW-1..5 as code: grace timers, ladder states F1–F7, island mode), `sync.rs` warm-spare lane + signed `WriterHandover`, adapter store-and-forward buffers, F7 backfill flow (backdated counter-signed custody events) | Relay killed mid-order → LAN completion (Phase R VbM kept); auto-promotion of spare WITHOUT signed handover → refused (split-brain structurally impossible); a degrade with no alert emission fails the gate's thread 7 harness | 4–6 sessions | R; RG-2 |
| **RG-5 — the aggregator stub (`hb-sink`)** | RG-2 heartbeats flowing; the €6 relay box exists | `hb-sink` bin on the relay box: append-only signed-heartbeat store, read-only status page, dead-man pager (Telegram/ntfy fan-out), retention = aggregates only past shred window | Sink cannot mint a healthy heartbeat for a dead node (signature — try it in a test, must fail); sink killed → nodes/spares/operator-device still detect each other's death (tier-1 paths intact); PII-grep over the sink's whole store = 0 hits, continuously | 2–3 sessions | RG-2; relay box (exists per relay research) |
| **RG-6 — the gate itself + DST harness** | RG-3/RG-4 drills exist; staging node runs an order end-to-end | `reliability-gate` skill extended with the LD0–LD11 protocol (additive section, L0–L11 untouched — meta-controller law); `sim/` seeded DST harness (transport-mem + fault injection + §3.2 invariants) wired to CI on the node crates | The known-bad build (boot-grace reverted / grace bypassed) MUST fail the sim and the gate; any shipped RED fixture going green = meta-RED = NO-GO; seed replay reproduces any failure bit-exactly | 4–6 sessions (gate 1–2, DST 3–4) | RG-1..RG-5; doc-05 phases W/S/R/X for the surfaces it traces |

**Total: ~17–24 focused sessions** for the full layer (RG-1 is due immediately and independently;
RG-2/RG-3 ride Phase W/S as library-lane work — pre-G11-safe per the SYNTHESIS §5 split; RG-4/5/6
land with R/X and gate any venue cutover: **no venue is cut over to the node before its first
LD0–LD11 GO**, which is this layer's own entry ratchet into production).

**Sequencing against the standing gates:** RG-1 now (every branch of G04-D3 justifies it);
RG-2/RG-3 are protocol-library lanes (zero pivot risk); RG-4+ follows the node runtime and stays
staging-only until G11 GREEN, same trigger as everything else. The layer never becomes the reason
protocol work displaces order #1 (doc 05 risk #1) — its pre-G11 footprint is one session of
Node-side fixes plus library-lane specs.

---

## Sources

**In-repo (read this session, VERIFIED):** `apps/api/src/lib/cutover/{front-door,flags}.ts` +
`apps/api/tests/cutover-front-door.test.ts` (trap re-verified armed);
`docs/design/gap-blueprints-2026-07-11/{G12-ops-broken-queue,G04-rebuild-cutover-rebaseline}.md`;
`docs/design/local-first-hub-2026-07-11/{SYNTHESIS,C-runtime-transport-identity,05-protocol-tech-completion-blueprint}.md`;
`docs/design/harness/META-CONTROLLER.md`; `.claude/skills/reliability-gate/SKILL.md`;
`docs/design/dowiz-brand/INTERFACE-DIRECTION-2026-07-11.md`; `.claude/CLAUDE.md` (VbM rule).

**Web (2026-07-11, VERIFIED-web-search — cited at search-result level, pages not deep-audited):**
[TigerBeetle VOPR internals](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md) ·
[Jepsen: TigerBeetle 0.16.11 (× Antithesis)](https://jepsen.io/analyses/tigerbeetle-0.16.11) ·
[WarpStream: DST for our entire SaaS](https://www.warpstream.com/blog/deterministic-simulation-testing-for-our-entire-saas) ·
[Antithesis: protocol-aware DST](https://antithesis.com/bugbash/talks/protocol-aware-deterministic-simulation-testing/) ·
[SRE School: heartbeat architecture 2026](https://sreschool.com/blog/heartbeat/) ·
[OneUptime: monitoring IoT/edge devices](https://oneuptime.com/blog/post/2025-09-24-monitoring-iot-devices-with-oneuptime/view) ·
[SigNoz: edge observability](https://signoz.io/guides/edge-observability/) ·
[Microsoft: circuit-breaker pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker).

*No code, flag, deploy, or DB row touched. This file is the only artifact created.*
