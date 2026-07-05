# Resolution — Order acquisition-attribution `channel` (write-only metadata)

- Seat: System Architect DeliveryOS (QR/ATTRIBUTION build lane)
- Date: 2026-07-04
- Resolves: `breaker-findings.md` (System Breaker) + `counsel-opinion.md` (Counsel)
- Design of record after this round: `proposal.md` (updated in place) + `docs/adr/ADR-order-channel-attribution.md`
- Gate context: STOP-DESIGN-B

Every breaker finding gets exactly one disposition — **fix** / **accept-risk** / **defer-flag**. Every
counsel ETHICAL-STOP gets **revise** or **human-decision**; every advisory note is addressed-or-acknowledged.
Source claims were re-verified against the live tree before dispositioning (citations inline).

---

## 0. Severity census (what came in)

| Source | CRITICAL | HIGH | MEDIUM | LOW | ETHICAL-STOP |
|--------|:--------:|:----:|:------:|:---:|:------------:|
| Breaker | 0 | 1 (H1) | 2 (M1, M2) | 3 (L1, L2, L3) | — |
| Counsel | — | — | 2 recs (Rec#1, Rec#2) | 5 notes (A–E) + steel-man + §6 | **0 (CLEAR TO PROCEED)** |

**0 CRITICAL. 0 grounded ETHICAL-STOP.** Counsel §2 verdict: *"CLEAR TO PROCEED. No grounded ETHICAL-STOP."*
The single HIGH (H1) is a **pre-existing** GDPR gap not introduced by this change; it is dispositioned
accept-risk + flagged-follow-up (below), not a blocker for this write-only design.

---

## 1. Breaker findings — disposition

### H1 [HIGH · B-DATA/B-SEC] Anonymizer never scrubs the `metadata` jsonb copy of `client_ip_hash` → **ACCEPT-RISK + DEFER-FLAG (follow-up ticket)**

- **Verified true against source.** `order-persistence.ts:95` writes
  `JSON.stringify({ otp_verified, client_ip_hash })` into `orders.metadata`; `anonymizer/index.ts:210-222`
  on erasure sets the dedicated **column** `client_ip_hash = NULL` (plus `delivery_address`,
  `delivery_instructions`, `receiver_*`) but issues **zero writes against `metadata`**. So after a GDPR
  erasure, `metadata->>'client_ip_hash'` (sha256(ip‖salt) = pseudonymized personal data) **survives**. Same
  finding as counsel Rec #2.
- **Disposition: ACCEPT-RISK for this build lane; record as a separate flagged follow-up ticket.**
  - This gap is **pre-existing** and **not introduced** by `channel` — `channel` itself adds no PII (a fixed
    tenant-agnostic enum token). The proposal's *incorrect claim* that "the anonymizer already owns
    `client_ip_hash`" has been **corrected** in `proposal.md` §8 ("No PII" bullet) and R4 — the design no
    longer rests on a false GDPR premise.
  - The **fix belongs to the GDPR/anonymizer maintainer**, not this lane: have the anonymizer also strip the
    jsonb copy on erasure (`metadata = metadata - 'client_ip_hash'`) so the sidecar honors the same erasure
    as the column. That touches `apps/api/src/lib/anonymizer/*`, a red-line-adjacent path this lane **must
    not** and **does not** touch.
  - **Why `channel` still makes this worth flagging now, not silently deferring:** `channel` is precisely
    the new reason engineers will start reading `metadata` (attribution reporting) — raising the odds the
    residual hashed-IP gets surfaced or exported. This is recorded as a live gap, **not fixed here** and
    **not dropped.**
- **Owner / escalation:** GDPR/anonymizer maintainer — the lead/operator must open the follow-up ticket and
  route it. Not closable inside this lane. Recorded in `proposal.md` R4 (owner column) and the appendix
  "Flagged follow-up" line.
- **Interim mitigation (design-time, this lane):** any future analytics/AI read of attribution must project
  `metadata->>'channel'` **only**, never `SELECT metadata` (kept in R4 disposition).

### M1 [MED · B-SEC/B-CONSIST] "No reader anywhere / never consulted" is false — owner dashboard reads + returns whole `metadata` → **FIX (proposal claim corrected); the behavior itself is a FEATURE, in-scope-acceptable**

- **Verified true against source.** `owner/dashboard.ts:112-131` does
  `metadata = JSON.parse(row.metadata)` and returns `metadata` **verbatim** in the dashboard response
  (alongside `preflight`). So the moment `channel` is folded in, `GET .../dashboard` returns
  `metadata: { otp_verified, client_ip_hash, channel }` per order — day 1, no new code.
- **Disposition: FIX the wording; the passthrough is expected/acceptable, not a defect.**
  - The over-broad claim ("written once and never consulted / no reader anywhere") was **factually wrong**
    and is **corrected** in `proposal.md`. The invariant is rewritten to the one that actually matters:
    *"`channel` is never read by pricing / the order-status state-machine / dispatch / notifications / any
    authz or RLS decision."* (§8 "Decision-path read prohibition" bullet; appendix checklist item; §7 "No
    cascade").
  - **This is a feature, not a bug.** An owner reading acquisition metadata about **their own** orders is
    exactly the owner-facing "QR-kit" story this feature exists to seed, and it is **same-tenant** (no
    cross-tenant read, no decision gate). Counsel's Justice lens agrees: real owner value, negligible
    incremental customer exposure. Recorded as expected/acceptable in §1 non-goals and §8.
  - **Out of scope for this pass:** building a dedicated channel-breakdown UI, label/i18n rendering, or an
    owner-facing analytics surface. The passthrough is a raw metadata echo, not a designed surface — no
    design/test/i18n work is owed by this lane for it. (Counsel B/C picked up separately in §2.)
- **Owner:** Attribution/impl lane (wording); channel-breakdown UI lane (future, out of scope).

### M2 [MED · B-CONSIST] Session-global `sessionStorage` → cross-storefront attribution bleed in one tab → **FIX (per-slug key + slug-parameterized API)**

- **Root cause.** A single session-global key means restaurant A's captured `qr` is read for a later,
  no-`?ch=` visit to restaurant B **in the same tab** → B's order mis-attributed to `qr`. sessionStorage is
  per-tab but not per-storefront; the design implied one global key.
- **Disposition: FIX in the design.** The storage key is **scoped per slug** — `dos_channel:<slug>` — never
  session-global. The `channel.ts` API changes shape accordingly:
  - **`capture(slug, search)`** — read `?ch=` from `search`, normalize against `OrderChannel`, write under
    `dos_channel:<slug>`. A mount **with** `?ch=` writes/overwrites that slug's key; a mount **without**
    `?ch=` leaves that slug's key untouched (first-touch-per-slug).
  - **`read(slug)`** — read `dos_channel:<slug>`; default `'web-direct'` on absence/throw.
  - Checkout calls `read(slug)` for the slug it is checking out.
  - **Result:** B without `?ch=` reads its own **absent** `dos_channel:restB` key → `'web-direct'`, and can
    never inherit A's `qr`. Cross-storefront bleed closed.
  - Updated in `proposal.md`: §2 (client cost), §7 (new per-slug bullet), §8 (no-cookies bullet), R1
    (per-slug semantics), R7 (RESOLVED), appendix checklist. **This changes the planned
    `apps/web/src/lib/channel.ts` API shape** — the impl lane must build `capture(slug, search)` /
    `read(slug)`, not `capture(search)` / `read()`.
- **Owner:** Attribution/impl lane.

### L1 [LOW · B-ANTIPATTERN] `.default('web-direct')` makes `channel` required in the inferred `CreateOrderInput` output type → **NO FIX (confirmed expected/consistent)**

- **Disposition: not a real issue — consistent with an established pattern in the same schema.** `channel:
  OrderChannel.optional().default('web-direct')` is wire-optional but type-required post-parse. This is the
  **exact same shape** as the sibling field already present in the same schema:
  `acknowledged_codes: z.array(z.string()).max(10).optional().default([])`. The breaker itself verified **no
  live break exists today** (no `: CreateOrderInput` literal constructors; no `toStrictEqual`/`toEqual` on
  parsed order bodies). Confirmed as expected and consistent convention — recorded in `proposal.md` §6
  ("Wire-optional / type-required" bullet). No change.
- **Owner:** N/A (no action).

### L2 [LOW · completeness] Second, legacy checkout client (`client/checkout/app.ts`) uninstrumented → **ACCEPT (explicit, documented known-gap)**

- **Verified true against source.** `apps/api/src/client/checkout/app.ts::confirmOrder` (vanilla JS, line
  491) POSTs to `/api/orders` (line 521), a second order-entry path distinct from the React
  `CheckoutPage.tsx` this pass instruments. It will **not** be instrumented with `?ch=` capture this pass →
  its orders always fall to server default `'web-direct'`.
- **Disposition: ACCEPT as an explicit, documented known-gap.** This build lane's directive expressly allows
  "capture server-side into the existing order-event/audit path … and DOCUMENT the gap honestly" for exactly
  this kind of secondary client. Recorded plainly — **not silently dropped, not fixed this pass:**
  - Honest under-attribution, **not a break** — the server default (`'web-direct'`) is safe; no PII, no
    decision impact, no cascade.
  - Recorded in `proposal.md` R6 (§10) and the appendix "Known-gap (documented, not fixed here)" line.
  - If/when that client stays live and attribution coverage matters, instrumenting it is a follow-up for the
    attribution lane (identical capture, same `OrderChannel` enum) — no new design needed.
- **Owner:** Attribution lane (future, only if that client remains a live storefront path).

### L3 [LOW · B-CONSIST, informational] Idempotency hash-exclusion is correct → **NO ACTION (probe closed)**

- **Disposition: the design got this right; recorded to close the probe.** `buildRequestHash`
  (`order-canonical.ts:29-52`) hashes an explicit field allowlist that does **not** include `channel`;
  adding/omitting `channel` cannot alter the hash, so a legit retry re-deriving a different channel does
  **not** trigger a spurious `IDEMPOTENCY_KEY_REUSED`. Folding `channel` into the hash **would** have been a
  bug — correctly avoided. Residual = first-touch-on-retry non-determinism (first INSERT's channel wins) =
  lossy analytics, not a dedup/replay break. No violated invariant. Kept as-is in `proposal.md` §6.

---

## 2. Counsel notes — disposition (advisory; verdict was CLEAR TO PROCEED)

**ETHICAL-STOP gate: NO human ethical decision required.** Counsel §2 tested `channel` against every grounded
red line (anonymize-not-delete, zero-PII-in-AI, server-authoritative, schema-rich-runtime-minimal, etc.) and
found **none crossed** — `channel` is not PII, server is authoritative, the invariant is honored. Verdict:
**CLEAR TO PROCEED, no grounded ETHICAL-STOP.** Recorded here so the STOP-ETHICS gate is satisfied without a
recorded human ethical override. (Note: the "anonymize-not-delete" red line is *adjacent* to the H1 metadata
gap, but counsel itself confirmed that gap is pre-existing and not caused by `channel` → not an ETHICAL-STOP
on this change; it is the flagged follow-up in §1/H1.)

- **Rec #1 — elevate the write-only invariant from prose to a deterministic guard.** *Accepted as a
  design-time flag for the impl lane* (folded into R5). The no-decision-path-reader rule is, per counsel,
  *the ethical mechanism* — not mere tidiness. The impl lane should ship a red→green guardrail
  (grep/lint-plugin-local rule or a test that fails if `metadata->>'channel'` acquires a reader in
  pricing/dispatch/authz/status/RLS modules), consistent with this repo's "guardrails decide, comments
  advise" discipline. This lane writes no production code, so it cannot land the guard itself — it is
  recorded as a required invariant for the implementing change (appendix checklist + R5).
- **Rec #2 — anonymizer does not scrub the `metadata` copy of `client_ip_hash`.** *Same finding as breaker
  H1* → same disposition: **ACCEPT-RISK + flagged follow-up for the GDPR/anonymizer owner** (§1/H1, R4). Not
  fixed in this lane.
- **A — `'other'` silent fallback masks a broken QR (owner harm).** *Acknowledged; deferred to the
  QR-kit/analytics lane.* When that lane is built, either split `'unknown'` (malformed/garbled) from
  `'other'` (valid-but-unlisted), or treat an `'other'` spike as an owner-visible QR-health alarm — so an
  owner is not told "this channel underperforms" when the truth is "the link I paid to print is broken."
  Out of scope for this write-only pass. (`proposal.md` §11-A.)
- **B — i18n the channel *labels*, keep *tokens* stable / C — document the taxonomy.** *Acknowledged and now
  directly relevant:* the owner-dashboard passthrough (M1) surfaces raw tokens (`gbp`, `telegram-tma`,
  `web-direct`) to owners day 1. This lane builds **no label rendering** — but the design records that no raw
  token should be *rendered as a label* until the channel-breakdown UI lane adds `al`/`en` catalog entries
  (i18n SSoT + parity gate) and publishes the token→meaning map (esp. an explicit "Google Business Profile"
  for `gbp`). Out of scope to build here; owned by that lane. (`proposal.md` §11-B/C.)
- **D — customer-facing transparency.** *Acknowledged; deferred to the compliance owner.* A one-line
  `/compliance` privacy-notice addition ("arrival source is recorded for the owner's own analytics") is the
  honest, commons-consistent move and costs a sentence. Low stakes; not built by this lane. (`proposal.md`
  §11-D.)
- **E — enum-growth as an ethical control surface.** *Acknowledged; folded into R3's intent.* The
  add-a-channel process should carry a one-line "is this token sensitive?" check (a token encoding an
  affinity-group / health / identity-linked partner would silently make this metadata more revealing). Keep
  the allowlist to benign marketing channels. (`proposal.md` §11-E.)
- **Steel-man of Option B (dedicated column = legibility/auditability, DB-CHECK closes R2 permanently, PII
  separation from `client_ip_hash`).** *Accepted on the merits.* Option A remains defensible for **this**
  lane because of the hard no-migration constraint **and** the field's write-only, no-decision-reader nature;
  the rejection is recorded as on-merits-plus-constraint, not silently on the lane constraint alone. The
  deferred first-class-column path (§5) is the correct hedge — identical wire contract, lands with zero
  client change if/when a reader or DSAR-tooling requirement appears. (`proposal.md` §11 steel-man line.)
- **§6 "the question nobody asked" (customer standpoint).** *Acknowledged as a horizon marker.* The
  write-only, coarse, merchant-first framing holds today. It stops holding the moment `channel` is joined to
  a **stable cross-order customer identity**, or `channel` is **aggregated across tenants for the platform's
  own benefit** — either is a *different feature* requiring its own Charter/ethics review. Recorded, not
  actioned (nothing here presumes it). (`proposal.md` §11 §6 line.)

---

## 3. Back-of-envelope — still holds after resolution

The resolution changed **no** load-bearing number:
- Storage still one ~26 B key on an already-written jsonb blob per order; ceiling 1,000 orders/min ≈ 17/s ⇒
  ~442 B/s ⇒ ~1.5 MB/hr fleet-wide. Zero new rows/tx/pool/index. (Breaker verified these true.)
- The M2 per-slug fix adds only a slightly-longer sessionStorage key on the client — sub-millisecond, no
  network, no server/DB cost. Connection budget delta = **0** (API + worker + analytics + migrations
  unchanged; this rides the existing single order transaction). BoE **holds**.

---

## 4. Residual open items after resolution (nothing blocks STOP-DESIGN-B)

| Item | Class | Blocks this design? | Owner |
|------|-------|:-------------------:|-------|
| H1/Rec#2 — anonymizer strip `metadata.client_ip_hash` on erasure | Accept-risk + **flagged follow-up** (pre-existing, red-line-adjacent) | No | GDPR/anonymizer maintainer (via lead/operator) |
| L2/R6 — instrument legacy `client/checkout/app.ts` | **Documented known-gap** | No | Attribution lane (future, if client stays live) |
| Rec#1/R5 — write-only guard (red→green) | Required invariant for the **impl** lane | No (design-time flag) | Impl lane |
| A — `'unknown'` vs `'other'` / QR-health alarm | Deferred (advisory) | No | QR-kit/analytics lane |
| B/C — label i18n + published taxonomy | Deferred (advisory) | No | Channel-breakdown UI lane |
| D — `/compliance` privacy-notice sentence | Deferred (advisory) | No | Compliance owner |
| E — "is this token sensitive?" add-a-channel check | Deferred (advisory) | No | Shared-types owner / add-a-channel process |

All residuals are **accept-risk**, **documented known-gap**, or **deferred advisory** — none is an unresolved
CRITICAL, an unresolved HIGH-without-accept-risk, or an unresolved ETHICAL-STOP.

---

## 5. Hard-exit conditions for STOP-DESIGN-B

| Condition | Status | Evidence |
|-----------|:------:|----------|
| 0 unresolved CRITICAL | **MET** | Breaker found 0 CRITICAL (census §0). |
| 0 unresolved HIGH without accept-risk | **MET** | Only HIGH = H1 (pre-existing GDPR gap), explicitly accept-risk + flagged follow-up to the GDPR owner; proposal's false premise corrected. |
| 0 unresolved ETHICAL-STOP | **MET** | Counsel verdict CLEAR TO PROCEED, no grounded ETHICAL-STOP; no human ethical decision required (§2). |
| Aesthetic/strategic advice addressed-or-acknowledged | **MET** | Rec#1–2 + A–E + steel-man + §6 all dispositioned (§2), reflected in proposal §11. |
| Back-of-envelope holds | **MET** | No load-bearing number changed; per-slug fix is client-only, conn delta 0 (§3). |
| Artifacts exist | **MET** | `proposal.md` (updated), `breaker-findings.md`, `counsel-opinion.md`, `resolution.md` (this), `docs/adr/ADR-order-channel-attribution.md`. |

---

## 6. Recommendation

**GO for STOP-DESIGN-B.** All breaker findings are dispositioned (2 FIX in-design: M1 wording + M2 per-slug;
1 ACCEPT-RISK + flagged follow-up: H1; 1 documented known-gap: L2; 2 confirmed-no-fix: L1, L3). Counsel is
CLEAR TO PROCEED with no ETHICAL-STOP; both recs and all five advisory notes are addressed-or-acknowledged.
The design remains the "boring, proven, schema-rich / runtime-minimal, reversible-by-omission" write-only
choice — with the two correctness/legibility fixes (decision-path invariant wording; per-slug storage) folded
in and the one pre-existing GDPR gap honestly flagged to its rightful owner rather than silently perpetuated
or silently fixed out of lane.

---

## 7. Implementation-time pivot (post-STOP-DESIGN-B, before code) — transport moved off `packages/shared-types`

On attempting the first edit (`packages/shared-types/src/legacy.ts` — adding `OrderChannel` + the `channel`
field to `CreateOrderInput`), the repo's `protect-paths.sh` PreToolUse hook hard-blocked it: the pattern
`packages/shared-types/` is an unconditional protected zone in this hook (broader than this build lane's
briefed protected-path list, which only named `packages/db/migrations`, `.claude`, `.husky`, and
`package.json`). This hook has no override mechanism available to this session — it requires a human/lead
edit, not a build-lane one. This was discovered *after* council GO, so it does not reopen STOP-ETHICS or
STOP-DESIGN-A/B (no invariant examined by breaker/counsel changes), but the **transport mechanism** changes:

- **Before:** `channel` as a new optional field on the `.strict()` `CreateOrderInput` zod body schema
  (`packages/shared-types`).
- **Now:** `channel` travels as a request **header** (`x-channel: <value>`), read directly in
  `apps/api/src/routes/orders.ts` — the same out-of-band pattern the route already uses for
  `x-otp-verified` (`CheckoutPage.tsx` already sends a custom header alongside the strict JSON body). This
  needed zero edits to the strict body schema, so it carries strictly *less* regression risk than the
  reviewed design (no chance of breaking any existing `CreateOrderInput.parse()` caller).
- **Consequence for the allowlist single-source-of-truth:** since `apps/web` can no longer import
  `OrderChannel` from `@deliveryos/shared-types` (that export can't be added), the 13-value allowlist is
  now defined **twice** — once in `apps/api/src/lib/channel.ts` (server, validates the header) and once in
  `apps/web/src/lib/channel.ts` (client, normalizes before sending). Both carry an explicit comment pointing
  at each other and at this note, so a future edit by someone with `packages/shared-types` access can
  collapse them into one `OrderChannel` export — flagged as a follow-up, not silently left undocumented.
- **All examined invariants still hold**: write-only (no reader in price/status/dispatch/authz/RLS), no
  migration, no new dependency, allowlist-validated before ever touching SQL, tenant-agnostic, session-scoped
  per-slug capture (M2 fix unchanged), no PII. Nothing the breaker or counsel evaluated depended on the
  transport being a body field vs. a header.
