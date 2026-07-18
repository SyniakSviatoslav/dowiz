# BLUEPRINT P74 — Moderation: per-hub reports + shareable abuse blocklist (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Wave **W4** (final),
> component **MODERATION**. Source scope: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W4 table,
> row P74) and `OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §5. Operator-binding constraints:
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.51 (post-hoc report/blocklist, no
> pre-review), §16.59 (**no vendor quality bar of any kind** — hard red line), §16.26 (courier
> no-scoring red line, HRW-automatic), §16.14 (no central dowiz state, hub isolation), §16.54
> (dowiz.org's own infra is the only closed surface). Structural precedent for format:
> `BLUEPRINT-P51-open-map-routing.md`.
>
> **This is deliberately a THIN blueprint.** §16.51 itself defers the *full* trust-and-safety
> design and names only a report + optional-subscribe blocklist as the durable Wave-0 decision.
> This document ships exactly that and no more; its single most important property is the
> **falsifiable proof that no moderation datum ever reaches the HRW matcher or any discovery
> signal** (§4 M4, §5.1, §6). Everything else is intentionally minimal.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The load-bearing finding: **the report
event is a payload on the EXISTING content-addressed event log; the HRW matcher lives in a
different crate with no dependency edge to any moderation module — so the no-scoring guarantee is
structural, not policy.**

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| Event log EXISTS: per-node, content-addressed, idempotent `MeshEvent { prev, actor_pubkey, actor_seq, payload }` hashed to a SHA3-256 content-id (the idempotency key) | `kernel/src/event_log.rs:133-156` | **VERIFIED — P74's report rides this, adds no new store** |
| `EventLog::commit_after_decide(ev, decide)` — runs a `decide` Law before persist; a duplicate content-id is a structural no-op and `decide` is NOT re-run | `kernel/src/event_log.rs:366-391` | **VERIFIED — the report append path; its `decide` is P74's abuse-category validator** |
| Event-log module doc already declares the neutrality invariant: *"events carry an `actor_pubkey` (identity), never a score. The log is neutral, idempotent plumbing."* | `kernel/src/event_log.rs:22-24` | VERIFIED — P74 extends this invariant to the report payload, does not weaken it |
| Persistence seam is a trait (`EventStore`) with an in-memory default; production backs it with pgrust; report events inherit durability/replay for free | `kernel/src/event_log.rs:182-205` | VERIFIED — no new persistence design |
| HRW matcher EXISTS and is **structurally scoreless**: `Courier { pubkey: CourierKey }` carries "nothing else — no score / rating / trust / reputation / rank field" | `bebop-repo/bebop2/proto-cap/src/matcher.rs:33-37` (struct), `:1-19` (doc) | **VERIFIED — the surface P74 must never wire into** |
| `hrw_weight(order_id: u64, courier: &CourierKey) -> u64` and `assign(order, candidates, max)` — inputs are **only** `(order_id, pubkey)`; a pure hash order, no external state reachable | `bebop-repo/bebop2/proto-cap/src/matcher.rs:41-53`, `:63-77` | **VERIFIED — moderation state is not an input and cannot become one** |
| Matcher lives in `bebop2/proto-cap`; that crate has **no dependency edge** to `dowiz-kernel` (where P74's modules land) — the isolation is a missing dep, not a convention | `bebop-repo/bebop2/proto-cap/src/lib.rs:40` (`pub mod matcher;`); no `dowiz-kernel` in `proto-cap/Cargo.toml` | **VERIFIED — strongest form of the red-line: `matcher.rs` literally cannot `use` a moderation module** |
| The no-scoring CI gate EXISTS and is a HARD build-fail: any struct field named `score/rating/reputation/rank/trust_score/...` in `bebop2/` fails the build | `bebop-repo/scripts/ci-no-courier-scoring.sh:24-37` | **VERIFIED — P74 reuses/extends this, does not invent a new one** |
| Hybrid PQ signer EXISTS for the signed blocklist artifact (ML-DSA-65 ⊕ Ed25519, AND-verify both halves per the B4 batch-verify lesson) | `kernel/src/pq/hybrid.rs` (behind the `pq` feature); mesh sibling `bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs` | **VERIFIED — blocklist signing reuses this, defines no new crypto** |
| §16.51 names the gap precisely: *"A post-hoc report/blocklist mechanism for abuse is implied as necessary but not yet designed — named as a gap for the Tier-3/moderation blueprint."* | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2376-2379` | VERIFIED — P74 is that named blueprint |
| §16.59 red line (binding): *"No vendor quality bar at all — extends §16.26's courier no-scoring red line to vendors: dowiz does not gate, exclude, or rank vendors by quality/performance/rating."* | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2487-2489` | VERIFIED — the constraint §4 M4 tests |
| §16.14 isolation (binding): *"dowiz.org/the client holds zero server-side order state, ever"*; no central queue/buffer without an explicit named reopening | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1971-1983` | VERIFIED — forbids a central report DB and bounds the legal-takedown surface (§4 M3) |
| No `moderation`/`blocklist`/`report`/`abuse` module exists in the kernel today (grep, `node_modules`/`target` excluded) | grep this pass over `kernel/src/` | VERIFIED — the two new modules are genuinely new; the report *substrate* is not |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. The three mechanisms and their honest limits (research grounding: R5 §5)

R5 §5 grounds this in real, currently-deployed low-overhead moderation on comparable decentralized
platforms: **reactive (post-hoc) moderation** (review only what users flag) and **community-level
blocklists** (Mastodon-style: each admin is sovereign, may block others, and *optionally* shares a
blocklist others *may* subscribe to — arXiv 2506.05522; Jhaver et al. TOCHI'18; Hachyderm docs).
The isolated-hub mesh maps onto this exactly: each hub is sovereign; a *shareable* blocklist is the
natural cross-hub abuse signal with no central authority.

P74 ships three mechanisms, each with its limit stated plainly (the standard forbids prose
assurance dressed as a solution):

1. **Per-hub `report` event.** A customer or courier flags abusive/illegal *content* or an abusive
   *actor* on a specific hub. Stored in that hub's own event log only (§16.14 — no central dowiz
   moderation queue). **Limit:** a report is an unadjudicated signal. Wave-0 does *not* auto-act on
   it, does not rate reporters, and does not verify a report's truth — adjudication is the hub
   operator's human judgment. A report is a fact ("actor X was flagged for reason R by actor Y"),
   never a verdict.

2. **Optional signed, subscribable ABUSE blocklist.** A hub operator *may* publish a
   cryptographically-signed list of known-abusive actor identities, and another operator *may*
   subscribe. Named "abuse", never "reputation" or "quality". **Opt-in and hub-controlled; never a
   mandatory central ban list.** It is **advisory**: it surfaces a signal to the operator, who
   makes a manual pool-membership decision (§16.3 — the venue owns its courier pool). It never
   auto-filters and never re-ranks. **Limit — stated honestly:** this is a *starting point* for
   abuse-signal sharing, not a solved trust-and-safety system. There is no consensus on who is
   abusive, no dispute/appeal flow, and the well-documented echo-chamber failure mode of shared
   blocklists (memory `sovereign-event-exchange`: trust = signed *capability*, never central
   reputation) applies here in full. It shares *what an operator asserts*, signed, and nothing more.

3. **Legal-takedown endpoint — dowiz-side only.** For genuinely illegal content dowiz is *legally*
   compelled to act on, a narrow escalation path exists **only on dowiz's own closed surface**
   (§16.54; the closed `dowiz-infra` side, not the hub kernel). **Limit / hard boundary:** dowiz
   has zero visibility into hub data by design (§16.14). This endpoint acts on dowiz.org's own
   minimal public surface (§16.21) and, as a last resort, on hub *availability* at the CF-tunnel /
   claim layer (§16.2/§16.45 — decline to route, never read or edit hub content). It is **not** a
   moderation API into hubs and must never become one.

---

## 2. Scope — what P74 owns vs deliberately does NOT (anti-scope is load-bearing)

**P74 owns:**
- `kernel/src/moderation.rs` — the `report` payload types + the abuse-category `decide` validator,
  committed via the EXISTING `EventLog::commit_after_decide` (§0).
- `kernel/src/blocklist.rs` — the signed `AbuseBlocklist` artifact, its hybrid-signature
  verification, and the **advisory-only** `is_flagged` operator-facing query.
- The falsifiable no-scoring-leakage test + a new CI guard, next to the matcher (§4 M4).
- The legal-takedown *boundary contract* (§4 M3) — the invariant, not the closed-side code.

**P74 explicitly does NOT own / does NOT build (anti-scope — each would violate a red line):**
- **No quality / rating / reputation / discovery signal of any kind** (§16.59). A "slow service",
  "cold food", "wrong order" complaint is a *quality* complaint — out of scope, routed to the
  §16.29 vendor+payment-provider dispute channel. It is **unrepresentable** in P74's types: the
  `ReportReason` enum has no quality variant (§3). This is type-level anti-scope, not a rule.
- **No auto-filtering or re-ranking of couriers/vendors from moderation data** (§16.26/§16.59).
  The blocklist never touches `assign()` (§4 M4). Removing a blocklisted actor from a pool is the
  operator's manual §16.3 decision.
- **No central dowiz report database, moderation queue, or cross-hub aggregation** (§16.14).
  Reports are per-hub event-log rows; the blocklist is a pull/subscribe artifact between consenting
  hubs.
- **No per-hub review/rating system** — §16.26's per-hub reviews are a *separate* customer-facing
  signal (owned elsewhere, visible only within one brand); P74 is abuse-handling, not reviews.
- **No adjudication engine, reporter-scoring, or automated content classifier** — Wave-0 is human
  operator judgment over a raw signal.
- **No pre-publication review** (§16.51 — full vendor trust stands).

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// kernel/src/moderation.rs

/// Bounded free-text note on a report (opaque UTF-8). Scaling axis: report volume
/// (event-log rows); this bound keeps one report O(constant)-sized.
pub const MAX_REPORT_TEXT_BYTES: usize = 2048;

/// Abuse-only report reasons. There is deliberately NO quality/service variant:
/// a "slow"/"cold"/"wrong order" complaint is UNREPRESENTABLE here and routes to
/// the §16.29 dispute channel instead. This enum IS the type-level enforcement of
/// §16.59's no-quality-bar red line — moderation data and quality data cannot be
/// confused because quality data has no shape in this type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportReason {
    IllegalContent      = 0,
    Fraud               = 1,
    Harassment          = 2,
    Impersonation       = 3,
    ExploitativeContent = 4, // CSAM / exploitation — decode-flagged for legal escalation (M3)
    Spam                = 5,
    Other               = 255,
}

/// What a report is *about*. Content targets (hub/vendor/item) and actor targets
/// (a courier or customer identity). No target carries a score or count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportTarget {
    Hub,
    Vendor([u8; 16]),      // vendor_id (P62's row-scoping key)
    CatalogItem([u8; 32]), // content-hash of the flagged item
    Actor([u8; 32]),       // courier/customer pubkey — identity, NOT a rating subject
}

/// The report body carried in a `MeshEvent.payload`. The REPORTER is the event's
/// own `actor_pubkey` (a customer or courier) — no separate reporter field, no
/// reporter reputation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPayload {
    pub target: ReportTarget,
    pub reason: ReportReason,
    pub note:   Vec<u8>,   // len <= MAX_REPORT_TEXT_BYTES
}

/// Typed rejection from the report `decide` validator (the Law pole, never retried).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportRejected {
    Undecodable,          // payload bytes do not parse to a ReportPayload
    UnknownReason(u8),    // reason byte outside the abuse enum (a quality-code smuggle attempt)
    NoteTooLong(usize),   // note exceeds MAX_REPORT_TEXT_BYTES
}
```

```rust
// kernel/src/blocklist.rs

/// Scaling axis: entry count K. Whole-list re-sign + re-fetch is O(K); beyond this
/// bound a Merkle-delta sync would win — named future, NOT built (§5.4).
pub const MAX_BLOCKLIST_ENTRIES: usize = 10_000;

/// One blocklist entry. Carries an identity and a reason — NEVER a score, count,
/// rank, or weight (the field-level NO-COURIER-SCORING CI guard, §0, auto-covers
/// this type: no score-ish field name can be added).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedActor {
    pub actor:    [u8; 32],           // pubkey
    pub reason:   ReportReason,
    pub evidence: Option<[u8; 32]>,   // content-id of a supporting report event (optional)
}

/// A publisher's signed abuse list at a given epoch. Epoch replacement is the only
/// mutation (revoke = publish a new epoch omitting the entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbuseBlocklist {
    pub publisher: [u8; 32],
    pub epoch:     u64,
    pub entries:   Vec<BlockedActor>, // len <= MAX_BLOCKLIST_ENTRIES
}

/// The wire artifact: a canonically-encoded list + a hybrid signature over that
/// canonical encoding. Verification AND-checks both hybrid halves (B4 lesson) and
/// re-encodes canonically to defeat reorder/truncation forgery.
pub struct SignedBlocklist {
    pub list: AbuseBlocklist,
    pub sig:  crate::pq::hybrid::HybridSig, // reuse kernel/src/pq/hybrid.rs
}

/// How a subscriber treats a list. There is exactly ONE variant: a subscription is
/// ADVISORY. `Enforcing`/`Ranking` variants are intentionally absent — an auto-enforcing
/// or ranking blocklist is unrepresentable, which is how the §16.26/§16.59 red line
/// becomes a hard boundary rather than a code-review discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionTrust { Advisory }
```

Advisory query surface (the ONLY read path a UI gets — returns a membership signal, never a number):

```rust
/// Operator-facing advisory lookup. Returns the reason an actor was flagged on any
/// *verified, subscribed* list, or None. It returns NO score, count, or ordering,
/// and NOTHING in the dispatch/matching path may call it (enforced by M4's guard).
pub fn is_flagged(subscribed: &[AbuseBlocklist], actor: &[u8; 32]) -> Option<ReportReason>;
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Thin by design: four items, no hot-path changes, no new persistence.

### 4.1 M1 — `report` event on the existing event log
`kernel/src/moderation.rs`: deterministic encode/decode for `ReportPayload` (TLV, canonical), and
`decide_report(ev: &MeshEvent) -> Result<(), ReportRejected>` used as the `decide` closure to
`EventLog::commit_after_decide`. RED tests (fail before the module exists): (a) a valid abuse
report commits and is idempotent on replay (reuses the event-log dup path — a re-received report is
a structural no-op, not a duplicate row); (b) **adversarial** a payload whose reason byte is a
quality code (e.g. `100`) is rejected `UnknownReason` and **nothing persists** (the type-level
anti-scope, exercised at the byte boundary); (c) **adversarial** an over-long note is rejected
`NoteTooLong`; (d) a rejected report leaves the log empty (Law pole, `event_log.rs:756` pattern).

### 4.2 M2 — signed subscribable abuse blocklist
`kernel/src/blocklist.rs`: canonical encode of `AbuseBlocklist`; `sign_blocklist` /
`verify_blocklist` over `kernel/src/pq/hybrid.rs` (AND-verify both halves); `is_flagged` advisory
query. RED tests: (a) sign→verify round-trips; (b) **adversarial** a reordered entry vector fails
verification (canonical re-encode teeth); (c) **adversarial** a truncated entry list fails
verification, not a silent short-accept; (d) **adversarial** a single-half forgery (valid Ed25519,
junk ML-DSA, or vice-versa) is rejected — AND-verify, mirroring the B4 mixed-order forgery lesson;
(e) an over-`MAX_BLOCKLIST_ENTRIES` list is refused at construction; (f) `is_flagged` returns
`Some(reason)` / `None` and its return type is `Option<ReportReason>` — proving it cannot return a
number (compile-level).

### 4.3 M3 — legal-takedown boundary (contract + isolation invariant, not hub code)
`kernel/src/blocklist.rs` (doc + a boundary test): the takedown surface lives on the closed
`dowiz-infra` side (P67's split) and is specified here only as an **invariant the hub kernel
enforces by having no such API**. RED test (**adversarial isolation**): a grep/compile assertion
that the hub kernel exposes **no** function that deletes, edits, or reads another hub's content on
external instruction — the kernel has no cross-hub content mutation surface at all, so a "takedown
into a hub" is inexpressible. `ExploitativeContent` reports carry a decode flag so the *hub
operator* (never dowiz) is prompted to escalate through their own legal channel. Documented
boundary: dowiz's only cross-hub lever is §16.53 liveness/availability, never content.

### 4.4 M4 — THE red-line test: no moderation datum reaches HRW or discovery (the point of P74)
Two legs, both in `bebop-repo` next to the matcher (where a leak would occur and where the existing
no-scoring guard lives):

- **Runtime invariance test** (`bebop2/proto-cap/src/matcher.rs` test module): build a candidate
  courier set, capture `assign(order, cands, max)`. Then construct N reports and a *verified,
  subscribed* `AbuseBlocklist` naming one candidate, and capture `assign` over the **same** set
  again. **Assert byte-identical output.** This is RED only if someone wires report/blocklist state
  into matching — the regression P74 exists to forbid. It passes today *because `assign`'s
  signature cannot accept moderation state* (§0): the poisoned input is provably outside the
  function's domain.
- **Structural CI guard** (`bebop-repo/scripts/ci-no-moderation-in-matching.sh`, new, modeled on
  `ci-no-courier-scoring.sh`): FAIL the build if the matching/dispatch path (`matcher.rs` and any
  future P65 dispatch module) `use`s or references `moderation`/`blocklist`/`report`/`abuse`, and
  reaffirm that `BlockedActor`/`ReportPayload`/`ReportTarget` carry no `score/rating/reputation/rank`
  field (the existing `ci-no-courier-scoring.sh` field grep already covers this for any `bebop2/`
  struct; the new guard extends the *import* dimension). Registered in the CI test list beside the
  existing guard.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6) — the unsafe state is unreachable by construction
The forbidden state is "a report/blocklist datum changed a courier's dispatch priority or a
vendor's discovery visibility." It is unreachable on three independent structural grounds, any one
sufficient: **(1) domain** — `hrw_weight`/`assign` accept only `(order_id, pubkey)`
(`matcher.rs:41,63`); moderation state is not a parameter. **(2) crate topology** — `proto-cap`
has no dependency on `dowiz-kernel`, so `matcher.rs` cannot even `use` the moderation modules
(the missing dep edge is the wall). **(3) type** — `Courier { pubkey }` and `BlockedActor` cannot
hold a score (the field-name CI guard, `ci-no-courier-scoring.sh:24-37`), and `SubscriptionTrust`
has no enforcing/ranking variant. Discovery: §16.21 means dowiz.org hosts no cross-vendor catalog,
so there is no ranking surface to leak into; each vendor's `/s/:slug` static pack (P69) is
generated from catalog state whose input set excludes moderation state. M4 makes grounds (1)–(2)
falsifiable at runtime and CI-time.

### 5.2 Schemas & scaling axes (item 8)
Reports scale on **report-volume per hub** (event-log rows) — the same axis as any event, bounded
by the hub's own storage; break point when a hub prunes/aggregates old reports, handled by the
living-memory *demote-to-attic-never-delete* pattern (legal retention forbids hard delete).
Blocklist scales on **entry count K**: whole-list sign+fetch is O(K); break point at
`MAX_BLOCKLIST_ENTRIES = 10_000`, beyond which Merkle-delta sync replaces whole-list re-fetch
(named future, not built).

### 5.3 Isolation (11), mesh awareness (12), living memory (15)
**Isolation:** a report append is bulkheaded by the event-log's existing `decide`-before-persist
gate and the two typed failure poles (`CommitError::Rejected` vs `::Store`, `event_log.rs:269-275`)
— a malformed report is a Law reject that persists nothing and cannot corrupt the log.
**Mesh:** the blocklist is the *one* gossip/subscribe artifact here — **opt-in pull**, never
mandatory push, off the order hot path. Payload budget: `32 (publisher) + 8 (epoch) + K·(32+1+33)`
bytes + one hybrid signature; at K=1,000 ≈ 66 KB, fetched on an epoch change (low cadence, e.g.
on-demand or daily), well within the mesh transport budget. Reports are **node-local**, never
gossiped. **Living memory:** reports are content-addressed and time-ordered in the event log
(temporal access pattern); the attic/demote path (`internal-retrieval-living-memory-arc-2026-07-14`)
governs retention.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)
**Snapshot Re-entry** (cheap regenerative recovery): a report inherits the event log's
content-addressed idempotent replay — a re-received report is a structural no-op, so recovery from
a partial sync needs no reconciliation. **Self-Termination** (a hard invariant boundary, not a
supervisor decision): the no-scoring red line is enforced by *unrepresentable state* — a scored
blocklist entry and an enforcing subscription have no type, so the dangerous state cannot be
constructed. Blocklist "rollback" = epoch replacement (re-publish a prior signed epoch). No
Self-Healing / redundancy property is claimed here (it would be a false claim — moderation has no
error-correcting math).

### 5.5 Linux discipline (item 9), tensor/spectral (16), telemetry (10)
Linux-verdict framing (`BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION`): **REUSE** the event log,
the hybrid signer, and the no-scoring CI guard; **EXTENDS** the NO-COURIER-SCORING guard to an
import dimension; **DOES-NOT-TRANSFER** — no adjudication/classifier machinery (an honest scope
cut, not a gap). **Tensor/spectral: honestly NOT applicable** — moderation is discrete signed
records, not a field/graph computation; invoking `spectral.rs` here would be decorative. **Item 16
not claimed.** Telemetry (item 10): report append rides the existing event-log path (no new hot
path to bench); the one measurable is blocklist verify cost — bench `blocklist/verify_1k_entries`
on the existing criterion harness (a signature check; target sub-millisecond exclusive of the
one-time hybrid verify), result to `BENCH_HISTORY.md`, never a prose estimate.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no `moderation.rs`; report cannot commit; quality-code smuggle has no refusal | valid abuse report commits + idempotent replay; reason byte outside the enum ⇒ `UnknownReason`, nothing persists; over-long note ⇒ `NoteTooLong`; rejected report leaves log empty | quality-code-rejection test + report-idempotency test |
| M2 | no `blocklist.rs`; no signed artifact | sign→verify round-trips; reorder/truncation fail verify; single-half forgery rejected (AND-verify); over-`MAX_BLOCKLIST_ENTRIES` refused; `is_flagged` returns `Option<ReportReason>` | blocklist forgery-teeth test (ledger row) |
| M3 | no stated boundary; a cross-hub content API could be added unnoticed | hub kernel exposes no cross-hub content read/edit/delete surface (isolation assertion); `ExploitativeContent` decode-flags legal escalation to the *operator* | hub-content-isolation test |
| **M4** | **the invariance test and the import guard do not exist — a moderation→matching wire could ship silently** | **`assign` output byte-identical with vs without reports + a verified subscribed blocklist against a candidate; `ci-no-moderation-in-matching.sh` RED on any moderation import in the matching/dispatch path; field guard still green** | **no-scoring-leakage invariance test + import guard (ledger rows) — the load-bearing regression** |

**Not-done clauses:** any code path that filters or re-ranks `assign` candidates from
report/blocklist state = NOT done regardless of green totals (§16.26/§16.59 red line); any
`ReportReason` variant expressing a quality/service complaint = NOT done (§16.59); a central dowiz
report store or cross-hub aggregation = NOT done (§16.14); a legal-takedown path that reads or
edits hub content = NOT done (§16.14/§16.54); an `Enforcing`/`Ranking` `SubscriptionTrust` variant
= NOT done.

---

## 7. Benchmark plan (item 10)

One bench, existing harness, zero new infrastructure: `blocklist/verify_1k_entries` (canonical
re-encode + hybrid AND-verify of a 1,000-entry list — the only compute this blueprint adds).
Report append is deliberately un-benched: it is the existing `commit_after_decide` path with a
constant-size payload (no new hot path — benching it would re-measure `event_log.rs`). Telemetry
rides the existing native-trackers hooks so a verify-cost regression surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W4/P74 scope) ·
`OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §5 (research grounding, prior art) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.51 / §16.59 / §16.26 / §16.14 / §16.54 /
§16.21 / §16.53 (binding constraints) · `kernel/src/event_log.rs` (report substrate — existing
input) · `bebop-repo/bebop2/proto-cap/src/matcher.rs` + `bebop-repo/scripts/ci-no-courier-scoring.sh`
(the surface never wired into; the reused guard) · `kernel/src/pq/hybrid.rs` (blocklist signer) ·
`BLUEPRINT-P51-open-map-routing.md` (format precedent) · `BLUEPRINT-P62-catalog-multivendor-data-model.md`
(vendor_id target key) · `BLUEPRINT-P65-dispatch-orchestrator.md` (the future dispatch path M4's
guard also fences) · `docs/regressions/REGRESSION-LEDGER.md` (M2/M4 rows) ·
`HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9). Memory:
`sovereign-event-exchange` (trust = signed capability, NEVER central reputation/blacklist — the
honest-limit stance in §1) · `crypto-safe-first-pass-2026-07-14` (B4 AND-verify / canonical-TLV
lessons applied in M2) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style; no metaphor, no
market bias) · `verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` (legal-takedown
gates to a human/operator, §4 M3). Supersedes: nothing — closes the §16.51 named gap.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P2 CORRESPONDENCE** (one concept, one primitive): a report rides the ONE event log — no second
  moderation store; one identity concept (`actor_pubkey`) serves reporter and blocked actor alike.
- **P6 CAUSE-AND-EFFECT** (determinism as law): reports are content-addressed and idempotent; the
  blocklist is canonically encoded and deterministically verified; the `assign`-invariance test
  (M4) is a determinism falsifier — the same inputs must yield the same ordering irrespective of
  moderation state.
- **P7 GENDER** (paired verification, no self-certification): the blocklist signature is refereed
  by an independent AND-verifier of both hybrid halves; the no-scoring property is refereed by the
  CI import guard + the invariance test, never self-asserted; a report's *truth* is explicitly NOT
  self-certified (no adjudication — the honest limit in §1).

(P1/P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the event-log-as-substrate finding; the crate-topology isolation finding) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; M1/M4 assert on event sequences and function invariance |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4 (quality-code smuggle, reorder/truncation/single-half forgery, cross-hub takedown attempt, moderation→matching wire) |
| 6 hazard-safety as math | §5.1 (three independent unreachability grounds) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (report-volume; blocklist K with named break point) |
| 9 Linux discipline | §5.5 (REUSE/EXTENDS/DOES-NOT-TRANSFER, honest scope cut) |
| 10 benchmarks+telemetry | §7 |
| 11 isolation/bulkhead | §5.3 (decide-before-persist gate; typed failure poles) |
| 12 mesh awareness | §5.3 (opt-in pull blocklist, payload budget; reports node-local) |
| 13 rollback/self-heal vocabulary | §5.4 (Snapshot Re-entry + Self-Termination claimed precisely; Self-Healing refused) |
| 14 error-propagation gates | §6 (ledger rows), §3 (`ReportRejected`/verify typed refusals), §4 M4 (CI import guard) |
| 15 living memory | §5.3 (content-addressed, time-ordered; demote-to-attic retention) |
| 16 tensor/spectral + eqc reuse | §5.5 (honestly NOT applicable — not claimed) |
| 17 regression ledger | §6 (M1/M2/M3/M4 rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§2 (event log, hybrid signer, no-scoring guard all reused; two new modules justified as the genuinely-new gap) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Dependency order; all buildable today (the event log, hybrid signer, and matcher all exist — §0).

1. **T1 (M1).** Create `kernel/src/moderation.rs` with the §3 types verbatim; register
   `pub mod moderation;` in `kernel/src/lib.rs` (alphabetical). Implement canonical TLV
   encode/decode for `ReportPayload` and `decide_report`. Write the RED tests first (§4.1: valid
   commit + idempotent replay via `EventLog::commit_after_decide`; quality-code byte rejected;
   over-long note rejected; rejection persists nothing). Acceptance: `cargo test -p dowiz-kernel
   moderation` green. Do NOT add a quality variant to `ReportReason`; do NOT create a new store.
2. **T2 (M2).** Create `kernel/src/blocklist.rs` (§3 types); register `pub mod blocklist;`. Build
   `sign_blocklist`/`verify_blocklist` over `kernel/src/pq/hybrid.rs` (gate behind the `pq`
   feature, matching the signer) with canonical encoding; implement `is_flagged`. RED tests per
   §4.2 (round-trip; reorder/truncation/single-half forgery all fail verify; over-cap refused).
   Acceptance: `cargo test -p dowiz-kernel --features pq blocklist` green. Do NOT add a score field
   to `BlockedActor`; do NOT add an `Enforcing`/`Ranking` `SubscriptionTrust` variant.
3. **T3 (M3).** Add the legal-takedown boundary doc + isolation test to `kernel/src/blocklist.rs`
   per §4.3 (assert no cross-hub content mutation surface exists in the hub kernel; decode-flag
   `ExploitativeContent` for operator escalation). Acceptance: the isolation test green. Do NOT add
   any hub-content read/edit/delete API to the kernel.
4. **T4 (M4 — the load-bearing item).** In `bebop-repo`: add the `assign`-invariance test to
   `bebop2/proto-cap/src/matcher.rs`'s test module (build a candidate set; assert `assign` output
   is byte-identical with vs without reports + a verified subscribed blocklist against a candidate).
   Create `bebop-repo/scripts/ci-no-moderation-in-matching.sh` (model on `ci-no-courier-scoring.sh`)
   that RED-fails on any `moderation|blocklist|report|abuse` import in `matcher.rs` or a future
   dispatch module, and register it in the CI test list. Acceptance: the invariance test green AND
   the new guard PASS (green today because no such import exists) — and prove the guard's teeth by
   temporarily adding a `use ...moderation` line and confirming it goes RED, then remove it. Record
   the M2/M4 rows in `docs/regressions/REGRESSION-LEDGER.md`.
