# BLUEPRINT — Item 51: Shadow-Mode Divergence Telemetry at the Decision Seam

- **Date:** 2026-07-19 · **Tier:** roadmap §J (fourth wave) · **Status:** BLUEPRINT v1 (planning
  artifact, no code). Genuinely new pattern (synthesis §2.4). Dispatch after {item 47 wiring +
  item 50}; the FDR-branch-merge prerequisite named in the roadmap is now **satisfied** (see §0).
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §J item 51 (lines 799–824);
  `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` §2.4 (full design) + §1.3 (Truthfulness vs
  Validity — the plane this does NOT live on); `docs/audits/hardening/CHECKLIST.md`;
  `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (style/depth template).
- **Ground-truth code cited (branch `main`, verified in-tree this session):**
  `kernel/src/fdr/schema.rs` (Kind enum, FdrEvent, the `pmu: Option` optional-field precedent);
  `kernel/src/fdr/mod.rs` (emission path, sink); `kernel/src/fdr/ring.rs` (CRC32 + bounded ring);
  `kernel/src/event_log.rs` (`sha3_256`); `kernel/src/decision/import.rs` (ReplayDisagreement — the
  fail-on-disagree kin); `kernel/src/metrics.rs` (advisory anomaly flag — the nearest advisory kin).
- **Upstream (hard):** item 47 (decision seam + `Option<Proposal>`); item 50 (`RejectionClass`,
  which this logs). Both spec-level; item 47's seam does not exist yet (verified — see item-50
  blueprint §2.2).
- **Downstream:** the field-measurement source for any future AI-authority-widening (item-33-class
  re-measurement); item 9's breaker still gets `Refuted` counts via item 47's OWN reject events, not
  via shadow mode (roadmap:815).

---

## 0. Dependency-status correction (load-bearing)

The roadmap gates item 51 on *"the FDR/exec branch merge"* (lines 799–800, 901). **That merge has
happened** — `main` HEAD is `6701bbb6f Merge … origin/exec/space-grade-tier0-2026-07-19`. The FDR
module is live in-tree: `kernel/src/fdr/{schema.rs,ring.rs,mod.rs,pmu.rs}` all present and tested.
So item 51's ONLY remaining prerequisites are the genuinely-unbuilt items 47 (decision seam) and 50
(`RejectionClass`). The FDR ring, the `Kind` closed-enum-growth mechanism, the optional-field
discipline, and both digest-primitive candidates are all **already shipped and usable**.

## 1. Scope / goal

Add advisory, non-gating telemetry that, on every decision where AI advice was present, records
**whether the admitted/rejected proposal's proposed action AGREED with the kernel's deterministic
decision D** — without building a second execution lane (D already exists and is total). This is the
measurement infrastructure that answers *"is the AI advice worth anything, per decision site, in the
field"* before any advice is ever trusted (synthesis §2.4). Deliverable: a new FDR record variant +
a bounded emission policy + the proof that it changes no behavior.

**Non-goals:** no second decision computation; no decision ever changed by a shadow event; no build
failure or breaker trip on a shadow event (advisory by definition AND by test, roadmap:813–816); no
full-payload logging (digests only); no new hash algorithm (synthesis §2.4 minimal-statistic rule).

## 2. Current-state grounding

### 2.1 The comparison object is free — D already exists

Item 47's seam computes the deterministic decision D as the **total, primary** function; advice
arrives as `Option<Proposal>` (roadmap:685–687). So on `Some(proposal)`, after `admit` returns, both
sides of the comparison already exist in hand — no re-run. On `None`, absence is already logged by
item 47's own path; shadow mode does nothing. (Item 47's seam is future code — see item-50
blueprint §2.2 — so §3 is a co-spec, hooking the seam where item 47 builds it.)

### 2.2 The FDR `Kind` enum is a closed enum that already grows deliberately

`kernel/src/fdr/schema.rs:186`:
```
pub enum Kind { Event, SpanClose, Alarm, PostMortem, Tuning, CleanShutdown }
```
`Tuning` is a **reserved** variant ("reserved for item-21's FDR-logged adjustments", `schema.rs:184`)
and `CleanShutdown` was added for the recovery path — i.e. this enum's established idiom is exactly
closed-enum growth with a documented owner per variant. Adding `ShadowDivergence` follows that idiom
one-for-one. (The roadmap's cited "item-48 `Heartbeat` precedent" is another *planned* variant;
`Heartbeat` is not yet in the enum — verified — so the precedent this item actually leans on is the
live `Tuning`/`CleanShutdown` growth pattern, which is stronger because it is already in-tree.)

### 2.3 The optional-field "byte-identical-elsewhere" discipline is live and proven

`FdrEvent` carries `pub pmu: Option<super::pmu::PmuStamp>` (`schema.rs:228`) that is `Some` ONLY on
verdict-emission records and `None` — and therefore **absent from the serialized JSON** — everywhere
else (`schema.rs:276–279`: `match self.pmu { Some(p) => p.write(w), None => w }`). This is the exact
item-27 optional-field precedent item 51 needs: the `ShadowDivergence` payload lives in the record's
`fields` bag (`schema.rs:212`, `fields: Vec<(&'static str, String)>`) or a dedicated optional field,
so every non-shadow FDR record stays byte-identical.

### 2.4 Two digest primitives already exist — a real choice (§7.1)

- **CRC32** — `kernel/src/fdr/ring.rs:65` `pub fn crc32(data: &[u8]) -> u32` (IEEE reflected,
  table-on-first-use `ring.rs:44–62`). **Caveat, verified:** the `ring` module is
  `#[cfg(not(target_arch = "wasm32"))]` (`fdr/mod.rs:52–53`), so `crc32` is **not compiled on
  wasm32**. This is fine for shadow mode *iff* the digest is computed only inside the FDR emission
  path (which is itself non-wasm — no sink is ever installed on wasm, `fdr/mod.rs:122`,
  `mod.rs:257`). If a future need computes the digest of D *before* the emission guard in a
  wasm-compiled decision path, the CRC32 must first be lifted out of the wasm gate (see item-54
  blueprint §2.3, same finding).
- **SHA3-256** — `kernel/src/event_log.rs::sha3_256` (default-build, pure, wasm-safe; used by
  `capability_cert.rs:37`). Truncated to 8 bytes it is a stronger-but-costlier fingerprint.

### 2.5 It is genuinely distinct from every in-tree differential

- `kernel/src/decision/import.rs` `ReplayDisagreement` **rejects** on disagreement (emits telemetry
  but is authoritative) — the opposite of advisory.
- pq / spool / spine / stats differentials are **tests**, not runtime observers.
- The nearest advisory kin is `kernel/src/metrics.rs`'s merge-plane anomaly flag (deterministic,
  merge plane) — cited by the synthesis as a *different plane*, not extended (roadmap:817–819).
- The swarm-arc replay probe is on the **Truthfulness** plane (byte-reproducibility), disjoint from
  this **Validity/advice-quality** plane (synthesis §1.3) — no overlap.

## 3. Implementation plan (co-spec with item 47's seam)

1. **New `Kind::ShadowDivergence` variant** (`fdr/schema.rs:186` enum + its `as_str` arm at
   `schema.rs:197`, `"shadow_divergence"`). Closed-enum growth, the `Tuning`/`CleanShutdown` idiom.
2. **The record payload — digests, never payloads.** On the `ShadowDivergence` record's `fields`:
   - `("site", decision_site_id)` — which decision seam;
   - `("verdict", "admitted" | "refuted" | "undecidable")` — the §2.1 admission class from item 50;
   - `("agree", "0" | "1")` — did the proposed action match D;
   - `("d_digest", hex)` and `("act_digest", hex)` — short digests of D and the proposed action
     (§7.1 primitive), a minimal statistic, never the full payloads.
3. **The hook — after `admit`, on `Some`.** In item 47's seam, on `Some(proposal)`: compute the
   agreement bit `proposal.action == D` (or the domain's equality), then emit ONE
   `ShadowDivergence` record per the emission policy (step 4). On `None`: emit nothing (absence is
   item 47's own log). No second lane; the whole hook is one comparison + one bounded emit.
4. **Emission policy (bounded — preserves the ring's replay-bounded property).**
   - every **disagreement** → logged;
   - every **Admitted-but-differs** (admitted yet ≠ D) → logged;
   - **Undecidable-while-D-decides** → logged at a bounded rate (the "model adds nothing on this
     domain" base-rate signal, roadmap:810–812);
   - **agreement** → SAMPLED at a low fixed rate (the base-rate denominator), never per-event.
   The bound is a per-site counter/rate limiter (reuse the existing `TokenBucket`
   (`kernel/src/token_bucket.rs`) or a fixed modulo counter — executor's call, §7.2). Bounded
   emission keeps the FDR ring's *replay-bounded-by-construction* property (item 49's rationale) —
   the ring is already fixed-cap A/B (`ring.rs:33` `DEFAULT_SEG_CAP = 1 MiB`).
5. **Digest primitive** — §7.1 decision; recommended default = truncated `event_log::sha3_256`
   (wasm-safe, no lift needed) unless the CRC32 lift lands for item 54 first, in which case reuse it
   (one primitive across items 40/51/54, the max-nativeness law, roadmap:808–809).
6. **Advisory wiring.** The variant is emitted through the existing FDR sink (`fdr/mod.rs` sink), so
   it is a no-op unless a sink is installed and inert on wasm — inheriting the FDR module's
   zero-default-cost posture. No control-flow branch anywhere consumes a `ShadowDivergence` record
   to change a decision or trip a gate.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

1. **Oracle.** Not a hot arithmetic path — the "oracle" here is the behavioral pin: a planted
   disagreeing proposal yields **exactly one** recovered `ShadowDivergence` record with the correct
   class + agreement bit + digests, verified by driving the real FDR ring and reading it back with
   `fdr::ring::recover` (`ring.rs:230`). Corpus-style behavioral oracle → manifest
   `N/A(behavioral-oracle)` for the strong differential form.
2. **dudect — `N/A`.** No secret-dependent timing (a digest of D/action + a rate check). Records
   `N/A(no-secret-timing)`.
3. **Debug cross-check.** `debug_assert!` that the emitted agreement bit equals the direct
   `proposal.action == D` recomputation at the emit site — a per-call self-consistency check,
   compiled out of release.
4. **Assembly spot-check — `N/A`** (no branch-free CT path introduced).
5. **Kani — `N/A` / optional.** Nothing here is a non-enumerable arithmetic contract; if a bound on
   the rate-limiter is worth a machine proof, it folds into item 8's GCRA/token-bucket Kani work,
   not a new item-51 harness.

**RED→GREEN proofs (P7, in the PR):**
- **the non-gating proof (the load-bearing one):** the deterministic decision output is
  **bit-identical with shadow logging ON vs OFF** — the item-47 `None`-path test pattern reused
  (roadmap:820–821). A diff that lets a shadow event alter D fails this test.
- a planted disagreeing proposal produces exactly one `ShadowDivergence` record, correct class +
  digests, recovered through the real ring (red→green).
- the **emission-rate bound** holds under a flood of planted disagreements (assert recovered
  shadow-record count ≤ the policy bound — proves bounded emission, roadmap:823).
- all **non-shadow** FDR records are byte-identical before/after adding the variant (the item-27
  optional-field guarantee, §2.3), asserted by a golden-JSON diff.

## 5. Falsifiable acceptance criteria

- Toggling shadow logging changes **zero** bytes of D across the full decision test corpus.
- A `ShadowDivergence` record is recoverable via `fdr::ring::recover` and carries `site`, `verdict`,
  `agree`, `d_digest`, `act_digest` — no full payload string appears in any FDR line (grep the
  recovered records for a payload marker = zero).
- Under N planted disagreements at one site, recovered shadow records ≤ the configured bound (never N
  unbounded).
- No code path reads a `ShadowDivergence` record to make or change a decision (structural review +
  grep: the variant is write-only from the kernel's perspective).
- Every non-shadow FDR record's serialized JSON is byte-identical to pre-item-51 goldens.

## 6. Dependency gates (honest)

| Gate | Status | Effect |
|---|---|---|
| FDR / exec branch merge | **MET** (§0 — stale roadmap flag corrected) | ring, `Kind` growth, optional-field discipline, both digest primitives all live. |
| Item 47 seam (`Option<Proposal>`, D) | **NOT MET** (item 47 spec-level) | hard blocker to LAND — the hook site does not exist yet. Specced here to land with item 47's wiring (after item 42). |
| Item 50 (`RejectionClass`) | **NOT MET** (co-spec) | the `verdict` field's `refuted`/`undecidable` values come from item 50. Land item 50 first or same-change. |
| Item 49 (replay-bounded ring) | **MET** (ring is fixed-cap) | the bounded-emission policy preserves that property; nothing new needed. |
| CRC32 wasm-lift (if CRC32 chosen) | **OPEN** (§7.1) | only if the digest is CRC32 AND computed in a wasm-compiled path; avoidable by choosing SHA3-256 or by landing the item-54 lift. |

## 7. Operator / executor decision points (flagged)

1. **Digest primitive: truncated SHA3-256 vs CRC32.** SHA3-256 is wasm-safe (no lift), stronger
   collision resistance, costlier. CRC32 is cheaper and unifies with items 40/54's fault-plane
   primitive but is wasm-gated today (§2.4) and weaker (adequate for a telemetry fingerprint, not for
   adversarial resistance — but the threat here is *content divergence detection*, not an adversary).
   **Recommendation:** truncated SHA3-256 by default (avoids the lift dependency); switch to the
   shared CRC32 only if/after item 54 lifts it out of the wasm gate. Executor's call, ledgered in the
   manifest row.
2. **Rate-limiter mechanism for the bounded policy.** Reuse `TokenBucket` vs a fixed modulo sampler.
   A modulo sampler is deterministic and simplest for the "sample agreement at 1/N"; a `TokenBucket`
   gives smoother bounding for bursty disagreements. Executor's call.
3. **`decision_site_id` scheme.** Whether sites are a `&'static str` label (greppable, human) or a
   small enum (typed, exhaustible). Recommend the enum for exhaustive per-site base-rate rollups; the
   final naming is item 47's decision-seam owner's call.
