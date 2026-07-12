# D — Transition Blueprint + Honest Risk Verdict: two half-hubs → one local-first hub

> Lane D of the local-first-hub research fan-out, 2026-07-11. Read-only session: the ONLY file
> created is this report; both repos left exactly as found (bebop2/core uncommitted WIP untouched).
>
> **Dependency note:** sibling lane reports A (vision-reconcile), B (data-sync),
> C (runtime-transport-identity) were NOT yet present in this directory at write time
> (`ls` empty, 2026-07-11). Where this blueprint needs their conclusions (sync-engine choice,
> transport/identity floor) it derives them from primary sources + web research and marks them;
> reconcile against A/B/C when they land.
>
> **Evidence labels:** **VERIFIED** = file read directly this session (path cited).
> **INHERITED** = claim carried from a source doc that itself labels it verified (doc + section cited).
> **UNVERIFIED** = web research or estimate.
>
> Primary sources read in full this session: hub review
> `/root/dowiz/docs/research/2026-07-11-hub-architecture-review.md` (§0–7); unified blueprint
> `/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`;
> `/root/bebop-repo/docs/design/plan-audit-bebop-2026-07-11.md`;
> `MASTER-EXECUTION-PLAN.md`, `G04`, `G07`, `G09` (+G11 via master plan) in
> `/root/dowiz/docs/design/gap-blueprints-2026-07-11/`; living memory:
> `MEMORY.md`, `rebuild-decision-rust-astro-2026-07-04.md`, `open-source-goal-adr020-2026-07-03.md`,
> `sovereign-core-mvp-handoff-2026-07-06.md`, `l5-meta-controller-2026-07-03.md` in
> `/root/.claude/projects/-root-dowiz/memory/`.

---

## 0. The frame in one paragraph

The operator's own hub-review recommendation (§7) stands as written: ride Wave 0/1 of the
MASTER-EXECUTION-PLAN + the three small fixes (`/courier-invite` in SPA_ROUTES, the `c.name`
schema mismatch, the `backup.failed`/`settlement.disputed` event-name fix), hand one venue the QR
kit + one attribution card, give couriers one out-of-app signal, and build no messenger transport
until demand shows (hub review §7.1–7.2, VERIFIED). The new, bigger arc — drop
migrations/Supabase/Node/TS/Fly and go local-first (Rust/WASM kernel + bebop2 protocol + SQLite,
no central server) — is **not a contradiction of that recommendation; it is a destination that
must enter through it.** The bridge below is one ladder where the first rungs ARE Wave 0/1 and
each later rung is independently valuable, per-surface reversible (the strangler pattern the repo
already trusts — G04 §3, VERIFIED), and gated on venue evidence, not on enthusiasm. The honest
verdict (§2) is that the radical arc is MAX-EV **only in this staged form**: started whole and
now, it is the fifth serial pivot G07 named, aimed at a product with **zero real orders**
(G07 §1, quoting audit §7.8: "no evidence of a single real (non-demo) production order or claimed
venue" — INHERITED).

One doctrinal note that makes the whole ladder coherent: the EXPANSION-PLAN rule "doors carry,
kernel decides" (hub review §1.1, VERIFIED) already contains the local-first end-state. Local-first
done right does not delete servers — it **relocates the kernel to devices and demotes every
remaining server to a door** (relay, push gateway, static storefront host). That is what "no
central server" can honestly mean (§4).

---

## 1. The staged bridge — phase ladder

### 1.1 Ladder at a glance

| Ph | Name | Entry precondition | What's built | Infra dropped | VbM falsifiable proof | Reversibility | Effort | bebop2 must-be-ready |
|---|---|---|---|---|---|---|---|---|
| **0** | Stabilize + validate (= the operator's own Wave 0/1) | none — funnel is broken today | Wave-0 ops + Wave-1 prod PR (GDPR trio, G03 contract, `/claim`) + the 3 review fixes + QR kit to venue #1 + attribution card + courier out-of-app beep. Plus G08 0.2: protect the uncommitted bebop2 crypto WIP | none | **G11 GREEN**: a real order row from a non-operator customer on a claimed venue; RED pre-committed: 0 claims after 10 contacts across 5 venues | trivial (3 revertable commits, zero migrations — master plan Wave 1) | Waves 0–1 ≈ 2–3 days; validation week operator-personal; review fixes S–M | none |
| **1** | One door before decentralizing it | Wave-1 landed; venue #1 outreach running | Fix the `kernel::decide` bypass (hub review finding #1): Rust checkout via `Command::PlaceOrder → decide`, dual-write `Priced`; real `hub_checkout` gate, real replay-parity, real `cause_hash`; customer-cancel + courier callers through the same door; vocabulary-parity CI (G06 Option B + review amendment) | none | replay-parity oracle that goes RED on a dropped/forged event; a grep-gate proving no order write path outside `decide`; the current placeholder suites replaced by suites that can fail | staging-only until any flip; kernel is a library — no prod exposure | 4–6 sessions (G06 est. ~3 days + Phase-2 items) | none |
| **2** | SQLite beside Postgres (dual-run read model) | Phase 1 green (event log honest); **venue #1 first real order recorded** | Event-log → embedded-SQLite projector; owner console + courier surfaces read from the local replica; offline read + queued non-money actions; the parked 1.3 sync port gets its first real consumer | none (Postgres stays sole authority) | byte-compare oracle: `fold(events)` in the SQLite replica ≡ Postgres state per order; RED case: inject a dropped event → divergence alarm fires | delete the replica; reads flag back to server | 4–6 sessions | none (per-event content-hash may use bebop2 SHA-2/3 at **Tier 1** — G09 §4-P4 — or commodity crates) |
| **3** | First device-authoritative surface (non-money) | Phase 2 parity green ≥2 weeks; ≥1 venue actively ordering weekly | Menu / availability / stop-list / shift-presence become device-authoritative (CRDT/LWW-safe data); server demoted to replica+relay for these; per-actor **signed** events begin | server-side menu-write endpoints become optional (flag-off, not deleted) | partition test: edit menu offline, server down, reconnect → converge; RED: concurrent conflicting edits produce divergent replicas → alarm. Signature RED: unsigned/foreign-key event rejected | per-surface flag back to server-authoritative (strangler) | 6–8 sessions | signatures should be **host-crate Ed25519**, not bebop2 (`sign.rs` scalar_mul is secret-dependent-branch, non-CT — G09 §2.1, VERIFIED-in-doc); bebop2 usable only at Tier 1 for non-adversarial integrity |
| **4** | Money/order-state single-writer core on device + relay | ≥3 validated venues (or 1 venue ≥4 weeks ≥20 orders/wk); Phase 3 stable; crypto at **Tier 2**; money council | Venue device runs `kernel::decide` (native/WASM) as single writer for its own orders; customers still order via web storefront → signed order-intent → store-and-forward relay → venue device; settlement stays fail-closed; courier/customer events countersigned | Node order business logic; pg-boss order workers (server keeps carrying, stops deciding) | kill-the-relay drill: order completes from device log after reconnect; forge drill: relay-injected event without valid signature REJECTED (the RED case); no-money-invented: device totals ≡ pricing oracle | per-surface flag back to server-authoritative; money migration back is real work — this is the first rung with material re-entry cost | 12–18 sessions + councils | **Tier 2 hybrid minimum** for anything guarding money (G09 §4-P4: KATs + Wycheproof + differential-vs-oracle + dudect + fuzz + hybrid); wasm32 empty-import gate must PASS (currently FAILS ~94 errors — plan-audit-bebop (b), VERIFIED-in-doc; in-flight per v3 blueprint footer) |
| **5 (N)** | Drop Supabase / Node / Fly-as-app-host | all surfaces device-authoritative or relay-thin; ≥2 months dual-run parity; GDPR/fiscalization answered for device-held data | Postgres → archived export; Node decommissioned (subsumes/retires G04 Phase D); remaining infra = the irreducible floor (§4): relay(s) + push gateway + static storefront/TLS + backups | **Supabase, Node/TS, most of Fly** | staging drill: Postgres off for a full L0–L11 order lifecycle; RED = anything still reads it. Cost line-item goes to ~relay+DNS | poor by design — this is the burn-the-boats rung; do last, only after the floor is proven boring | 5–10 sessions + ops | Tier 3 (external audit — G09 §4-P3) before any bebop2 primitive is the **sole** guard of settlement/identity; until then hybrids or host crates |

Total to full local-first: **~30–45 focused sessions** past Phase 0 — roughly 2–3× the remaining
G04 cutover (12–18 sessions, VERIFIED G04 §2.6) and ~15× Phase 0. That ratio is the honest price
tag of the radical arc.

### 1.2 Why this ordering is load-bearing (not just prudent)

1. **Phase 1 before anything decentral.** You cannot decentralize a write path that does not
   exist as ONE path. Today the Rust checkout bypasses `kernel::decide` entirely
   (`Command::PlaceOrder` never constructed in the api crate; `cause_hash` = literal
   `"placeholder"`; replay-parity is a placeholder that can't fail — hub review §3.2, VERIFIED).
   Replicating THAT log to devices would replicate a decoration, and every later phase's proof
   (fold-parity, partition-converge, forge-reject) folds this log. Phase 1 is also the only
   G04/G06 slice that every one of the three futures in §2 needs — it is never wasted.
2. **Read-model before write-model (Phase 2 before 3).** The dual-run SQLite replica is the
   cheapest falsifiable test of the whole local-first premise — sync fidelity, device storage,
   projector performance — with zero authority risk. It is also immediately valuable on its own:
   the courier/owner surfaces get offline reads (couriers with a locked phone are today's single
   largest product gap — hub review §4.6, VERIFIED), independent of whether any later phase
   happens. This matches how the 2026 local-first ecosystem actually ships: server-authoritative
   Postgres → client SQLite replicas is the mature, production-tested pattern
   ([PowerSync](https://powersync.com/blog/electricsql-vs-powersync), [ElectricSQL
   alternatives](https://electric-sql.com/docs/reference/alternatives),
   [sqlite-sync](https://github.com/sqliteai/sqlite-sync) — UNVERIFIED web).
3. **CRDT-safe data before money (Phase 3 before 4).** Menu/presence tolerate merge; order state
   and money do not (the kernel's own law: forbidden transitions are refusals, integer money,
   single money surface — MANIFESTO C2/C3/C5 via v3 blueprint §2, VERIFIED). The correct money
   design is **single-writer-per-order on the venue device**, not CRDT merge — which dowiz can
   honestly do because MVP dispatch is single-owner (`attemptHonestDispatch`), and the
   "decentralize the matcher" invariant (DANGER #1) binds the *protocol* phase, not the
   single-venue hub (v3 blueprint §5 boundary: "the owner's hub is one matcher among many" —
   VERIFIED).
4. **Crypto ladder before value (G09 gate on Phase 3→4).** The v3 blueprint's "DONE + GREEN"
   crypto banner is true for KATs but G09 is blunt underneath it: bebop2's PQ set is **not
   FIPS-interoperable by construction** (coefficient-domain KEM, CBD-sampled A, 32-byte
   challenge — "bespoke schemes wearing FIPS names"), Ed25519 signing is non-constant-time, and
   KyberSlash-class division timing sits on secret data (G09 §2.1, VERIFIED-in-doc; master plan
   correction #9). The tiered policy (G09 §4-P4) is the right gate: **nothing bebop2-only guards
   money or identity before Tier 2 (hybrid) / Tier 3 (sole)**. Practically: Phases 2–3 use host
   crates or bebop2-at-Tier-1 hashes; Phase 4 requires the Tier-2 ladder (Wycheproof, differential
   oracle, dudect, fuzz) actually run; the PQ half stays hybrid until re-derivation + audit.
5. **Every rung is a product, not a bet.** Phase 0 = a working funnel + first venue. Phase 1 =
   an honest kernel (both stacks' declared law). Phase 2 = offline reads + a portable event log
   (also the real backup story). Phase 3 = venue keeps operating through outages. Phase 4 = orders
   survive relay death; venue owns its ledger. Phase 5 = infra bill ≈ 0 and the sovereignty claim
   is finally true. Stop at any rung and you keep everything below it.

---

## 2. The three-way EV verdict

Three futures, argued honestly. Baseline facts that bind all three: prod is 100% Node and earning
zero revenue but is the only funnel that exists (hub review §3.4, VERIFIED); the Rust twin is 69K
LOC/1,041 tests, staging-live on 6 surfaces, rotting measurably when unattended (G04 §2.6,
VERIFIED); the operator's attention since 07-08 is ~100% on bebop (5 dowiz commits vs ~110 bebop —
G07 §1, INHERITED); zero real orders exist (audit §7.8 via G07, INHERITED).

### Option S — Stay-Node-forever
- **For:** every revenue-path fix is a Node one-liner already drafted (G03 ~15 LOC, `/claim`
  1 line); the courier backend is the most hardened vertical in the product; nothing about
  "one hub, many sources, own couriers" is blocked on Rust (hub review §0, §3.4, VERIFIED).
  If validation fails, Stay-Node loses the least of the three. Lowest variance.
- **Against:** it silently converts the Rust twin into a write-off nobody signed (G04: "doing
  neither — the status quo — is the only indefensible option", VERIFIED); the two-stack tax
  persists; and the revealed-preference record says the operator will not actually work this
  future — attention collapsed off Node within days every time (G07 §2 scorecards + operator
  preferences 1–4, VERIFIED-in-doc). A plan the operator won't execute has an EV of its paper
  value × ~0.
- **EV shape:** highest probability-weighted *near-term* revenue per session, zero optionality on
  the sovereignty thesis, high abandonment risk by the person who has to run it.

### Option C — Finish the existing Rust cutover (G04 Path A)
- **For:** mechanism proven (2.4s rollback, 9/9 probe GREEN on staging — G04 §2.2, VERIFIED);
  tail enumerated (61 routes), not open-ended; kills the two-stack tax permanently; the Rust
  server story helps OSS/self-host (ADR-020 memory, VERIFIED).
- **Against:** 12–18 sessions + ≥8 operator acts of uniformly red-line money/PII work that
  **earns no new revenue by construction** (G07 scorecard A: "pure cost-reduction + optionality" —
  VERIFIED-in-doc) — and now a **new** argument the earlier docs couldn't make: under the
  operator's stated local-first destination, most of the tail is *architecture that Phase 4/5
  would dismantle*. Porting 17 S8 notification routes onto server-side DEFINER fns, 23 S5
  settlement/owner-action routes, and prod flip soak builds a better *server* — the destination
  says the server stops deciding. Finishing the cutover to prod is only max-EV if the destination
  is "Rust monolith on Fly", which the operator has just said it isn't.
- **EV shape:** dominated. Either validate first (do less) or go local-first (do different).
  **Except its kernel-honesty slice (G06/Phase 1), which every future needs and which this
  blueprint promotes to the ladder's rung 1.**

### Option L — Local-first rewrite (the radical arc)
- **For:** it is the only future aligned with every attractor the corpus records (sovereignty,
  no-central-server, PQ identity, determinism — G07 preference 3, VERIFIED-in-doc), which means it
  is the future the operator will actually show up for — motivation is a real resource with real
  EV. It is also the only future in which the 69K-LOC Rust twin, the sealed kernel, AND bebop2 all
  become the product instead of parallel truths: the kernel compiled to the device IS the unified
  blueprint's L0 (v3 §3, VERIFIED). Technically the moment is right: client-SQLite sync engines
  went production-grade in 2024–2026 (UNVERIFIED web, §1.2).
- **Against — the part the operator asked to hear straight:** rewriting the substrate before a
  single paying venue is the *exact* serial-pivot mechanism G07 documented — four "authoritative"
  futures in four weeks, each opened by parking the last one by omission, while the one
  invalidated assumption (demand) stayed untested (G07 §1, VERIFIED). Declaring local-first "the
  program" today would be pivot #5, and it would repeat the specific pattern of 07-04: operator
  overrides a validate-first verdict to build (G07 preference 1). The protocol's own cold-start
  doc says its path to value runs through dowiz having real venues (G07 scorecard D,
  INHERITED). A full-start now spends ~30–45 sessions to learn nothing about demand, atop a
  funnel that 400s three of six contact options *today*. And the sovereignty features have no
  demonstrated customer: an Albanian cash-first venue asked for QR tents and a beeping courier
  phone, not an offline-merge event log.
- **Cost to defer vs start now:** deferring costs bounded, *measured* rot (G04 B3 keep-alive
  ≈0.2 session/month keeps re-entry cost flat, VERIFIED) plus operator morale. Starting whole now
  costs the validation window itself — the G11 week is operator-personal and unparallelizable;
  every full-time local-first week pushes the only falsifiable business question another week out,
  while prod's front gate stays broken.

### Verdict

**MAX-EV is neither "local-first now" nor "cutover" nor "Node forever" — it is the ladder:
local-first as the committed destination, venue validation as the gate on every rung above 1.**
Concretely: run Phases 0–1 immediately (they are identical under all three futures and cost
days-to-a-week); **retire Option C as a program** (G04 Path B mothball for the *prod-flip*
program, kernel + Rust api stay live — exactly the carve-out G07's draft already makes for
`rebuild/crates/domain`); allow Phase 2 as the ONE active engineering track only after venue #1's
first real order; gate Phase 3+ on the triggers in §3. If G11 goes RED, the arc parks — see §4.
This keeps the operator's radical destination fully alive, converts it from a pivot into a
schedule, and never bets the only revenue funnel on it.

---

## 3. Start-trigger + sequencing vs venue validation

**Recommended start trigger for net-new local-first work (Phase 2):**
`(Wave-1 PR merged) AND (Phase 1 kernel-honesty landed on staging) AND (G11 GREEN: ≥1 real order
from a non-operator customer on a claimed venue)`. All three are observable events, not moods —
same standard G04 B4 sets for re-entry (VERIFIED).

**Phase escalation triggers (pre-committed, additive):**

| Rung | Trigger to start | Trigger source |
|---|---|---|
| Phase 0 | now — no precondition | master plan Waves 0–1 (VERIFIED) |
| Phase 1 | Wave-1 merged | G06 Option B is already ranked track #1 in Wave 4 (VERIFIED) |
| Phase 2 | G11 GREEN (venue #1 real order) | G11/G07: first real order = the re-rank event (VERIFIED) |
| Phase 3 | ≥1 venue with ≥20 orders/wk for 4 consecutive weeks, or ≥3 claimed venues | keeps device-authority behind demonstrated recurring usage |
| Phase 4 | ≥3 paying venues AND bebop2 interoperable set at Tier 2 AND money council GO AND wasm32 gate green | G09 D1–D3 + v3 blueprint G9/G10 (VERIFIED-in-doc) |
| Phase 5 | Phase 4 stable ≥2 months AND infra-cost or platform-EOL forcing event AND GDPR/fiscalization design signed | mirrors G04 B4(b) cost trigger (VERIFIED) |

**Sequencing against the validation week:** the validation week (Wave 2) is operator-personal;
agent sessions during that same week should burn on Phase 1 (kernel honesty) — it is staging-only,
red-line-reviewable, and blocks nothing the operator is doing. This is the one genuinely free
parallelism in the plan. Everything above Phase 1 is throttled by G07's R1 (one active engineering
future) — which this arc should enter by the arbiter's own re-entry clause (the draft already
schedules the protocol thread "ONLY once dowiz has real order flow", G07 §4 — VERIFIED); adopting
the ladder is an *amendment of D's entry, not a defeat of the ranking*.

**What NOT to build in any phase until its trigger fires** (carried unchanged from the hub review
§7.2, all still correct under the local-first arc): messenger transports before the G7 survey;
aggregator ingestion (doctrine-excluded); cart-token implementation before a conversational head
exists; libp2p/mesh/CRDT-for-money; voice/kiosk. The local-first arc changes none of these
verdicts — it only re-schedules the "1.3 sync port / signing / bebop2 coupling" line from
"Phase-3 seams, do not build" to "Phase 2+ of this ladder, triggers above".

---

## 4. Irreducible floor + kill-criteria

### 4.1 What "no single server" realistically means

Relay-assisted local-first, **not zero-infra**. Even at Phase 5 the following cannot be dropped
(this substitutes for the absent lanes B/C — re-verify against them when written):

1. **Push notifications.** iOS apps can only be woken through Apple's APNs (single shared TLS
   connection via `apsd`; backends never reach the device directly); Android equivalently through
   FCM. There is no self-hosted substitute on iOS — even self-hosted stacks ship an APNs-bridging
   relay ([networking behind iOS push](https://zonov.me/networking-behind-push-notifications/),
   [toot-relay](https://github.com/DagAgren/toot-relay) — UNVERIFIED web). The hub's own biggest
   product gap — deaf couriers (hub review §4.6, VERIFIED) — gets *worse*, not better, under naive
   P2P. Floor: a push gateway.
2. **NAT / reachability.** Venue phones and courier phones sit behind carrier-grade NAT; direct
   device-to-device connectivity is not generally possible; TURN-class relays are the standard
   answer (UNVERIFIED web, ibid.). Additionally, ordering is *asynchronous* — the customer orders
   while the venue device may be asleep — so a **store-and-forward relay** is required regardless
   of NAT. Floor: ≥1 dumb relay that carries signed events it cannot forge (Phase 4's forge drill
   is exactly the proof that the relay is a door, not a decider).
3. **The customer's door.** Customers arrive via QR → browser (the validated Tier-1 channel — hub
   review §7.1, VERIFIED). They will not install an app or run a node. A storefront must be served
   from a reachable TLS host on a domain, and checkout must POST somewhere online. It can be
   static + relay-backed, but it is a server. Floor: static hosting + the relay accepting signed
   order-intents.
4. **Fiscalization + GDPR.** The Albania fiskalizimi obligation attaches to the one checkout (hub
   review §6.1.2, VERIFIED) and presumes an online, identifiable money surface; GDPR erasure over
   an immutable replicated event log needs a crypto-shredding design and a reachable
   controller-of-record — the just-shipped GDPR trio assumes central deletion and does not carry
   over for free. Floor: a legal/erasure endpoint; a Phase-4/5 design task, not an afterthought.
5. **Durability.** "The venue's phone" is not a backup story. Floor: an encrypted log backup
   target (the relay can double as it; owner-held keys keep it sovereign).
6. **Distribution.** If courier/venue apps go native, app-store presence is unavoidable; a PWA
   escapes partially but re-tightens the push dependency (iOS web-push works only for installed
   PWAs — UNVERIFIED).

So the honest Phase-5 end-state is: **no server that decides; three or four small servers that
carry** — push gateway, relay/store-and-forward + backup, static storefront + TLS. That satisfies
C4 ("local-first, no central server" as *reachable-free from the signed event log* — v3 blueprint
§2, VERIFIED) in its own stated sense: the system survives any single carrier's death; it does not
pretend carriers don't exist.

### 4.2 Kill-criteria (abandon or park the arc when any fires)

| # | Condition (observable, not a mood) | Action |
|---|---|---|
| K1 | **G11 RED** — 0 claims after 10 contacts across 5 venues (the pre-committed stop — master plan Wave 2, VERIFIED) | Park the ENTIRE arc above Phase 1. The product question reopens; do not spend the answer's budget on substrate. The rewrite must never become consolation engineering |
| K2 | Phase-2 parity oracle RED twice consecutively without a fix landing | Rot is compounding (same rule as G04 B4-d) — stop, fix or formally retire the replica before any Phase-3 work |
| K3 | bebop2 cannot reach Tier 2 (timing leaks structural, or PQ re-derivation abandoned per G09 D1 = "No") | Cap the arc at host-crate crypto permanently OR halt Phase 4; never waive the tier gate to keep a schedule |
| K4 | Pilot evidence that the venue-device-as-sequencer premise fails (missed orders because the venue phone was locked/asleep at rates worse than today's server path) | Fall back to relay-authoritative for order intake (still local-first for reads/ops); re-scope Phase 4 |
| K5 | Any ladder rung idle >14 days while designated active | Auto-park with a dated PARKED state frame (the G04 §5 auto-fallback rule, generalized — VERIFIED). Parking is a decision with a document, never an omission — the exact failure G07 diagnosed |
| K6 | Venue demand signal for sovereignty ≈ 0 after N venues (all venues content on hosted; zero self-host/offline asks in the G7 survey) | Phases 4–5 lose their customer-value leg and stand on operator values + cost alone — re-argue them explicitly at that size, or stop at Phase 3 (which is already a strictly better product) |
| K7 | GDPR/fiscalization design for device-held PII cannot be made lawful/practical | Hard stop for Phase 5 (Postgres retirement); Phases 0–4 remain valid |

---

## 5. Operator decision points

| # | Decision | Recommendation | Existing hook |
|---|---|---|---|
| OD-1 | Approve the Wave-1 prod PR (+ the 3 hub-review fixes riding along) | YES — the only live legal exposure + the funnel, one merge | master plan D1/D2 (VERIFIED) |
| OD-2 | Adopt this ladder as D's (bebop/protocol's) entry path — i.e., amend the arbiter draft's D entry from "parked until real order flow" to "scheduled per §3 triggers", keeping B active-throttled | YES — it un-parks D *through* the arbiter's own re-entry clause instead of around it | G07 §4 draft + D6 (VERIFIED) |
| OD-3 | G04 disposition: Path B mothball for the **prod-flip program**; kernel + Rust api stay live as Phase-1/2 substrate; staging flag posture per B1 (money surfaces back to Node unless Phase 1 is actively worked) | YES — the cutover's value is absorbed by the ladder; its remaining tail is not | G04 D1/D3 (VERIFIED) |
| OD-4 | Crypto policy: adopt G09's 4-tier value-bearing policy now; host-crate signatures for Phases 3–4 until bebop2 reaches Tier 2; PQ stays hybrid; decide re-derivation (D1) only when Phase 4 is in sight | YES — this is master plan D14, unchanged by the arc | G09 §6 D1–D3 (VERIFIED) |
| OD-5 | Sign the Phase-2 start trigger (§3) and the kill-criteria table (§4.2) *before* any Phase-2 code | YES — pre-commitment is what makes the radical arc safe to want | this doc |
| OD-6 | Phase-4 gate bundle when reached: money-on-device council + GDPR/fiscalization legal design + Tier-2 evidence + wasm32 gate green | required, red-line | G09 P3/P4; v3 blueprint G9/G10 |
| OD-7 | 085-watermark disposition + draft renumbering (085–089 → 087–091) — orthogonal to this arc but blocks any future settlement work under every option | record now per G04 D2 | G04 §2.4 (VERIFIED) |

---

## 6. One-paragraph close

The operator's near-term recommendation and the operator's radical arc are the same plan at
different altitudes: fix the gate, get venue #1, make the kernel the one honest door — and then,
each time the venues say yes, move one more surface's authority from the cloud to the counter.
The radical arc is worth it **as a destination entered on evidence**; it is not worth it as a
fifth pivot. The single cheapest thing that makes the whole ladder real is unchanged from the hub
review: ship Wave 0/1 and put one QR tent on one real table.

*Read-only lane D, 2026-07-11. Only this file was created. Reconcile §1/§4 against lanes A/B/C
when they exist.*
