# Triadic Council Verdict — P6 CLAIM PHASE (shadow → consented owner → live)

**Date:** 2026-06-28 · **Seats:** architect · breaker · counsel · grounded against live source.
**Stage:** Council-light, 🔴 RLS + AUTH red-line, design-only.
**Proposal:** `scratchpad/p6-claim-proposal.md`. Owner claims a shadow → authenticates → reviews/approves
the AI menu → publishes (the moment the never-orderable B3 invariant lifts + ownership of unconsented data
transfers). **VERDICT: APPROVE-WITH-CONDITIONS.**

## Verified correct (do not re-litigate)
- The state machine reaches CLAIM_OFFERED legally (PROVISIONED→VERIFIED→CLAIM_OFFERED→CLAIMED, `state-machine.ts:16-19`).
- **No route anywhere writes `organizations.owner_id` on an existing org** — onboarding only INSERTs fresh
  orgs and slug-collides a shadow → 409 (`onboarding.ts:57-59,73`). So "claim is the only way owner_id gets
  set on a shadow" holds by construction → B3 ("published only by an authenticated owner") holds.
- **Publish is double-gated:** `activation.ts` requires `menu_confirmed_at IS NOT NULL`, which
  `provisionShadowSpine` NEVER sets → a claimed-but-unreviewed shadow physically cannot publish.
- 065 = live `read_public_menu`, 035 = live `read_public_menu_all_locales` (066/067 touch neither).

## 🔴 CRITICAL / blocking conditions
- **K1 — claim_invites uses the `provision_grants` RLS template, NOT courier-invites.** courier_invites is
  ENABLE-only + tenant-isolated (`current_setting('app.current_tenant')`) — unreadable by a not-yet-member
  claimer. Use ENABLE+FORCE + `FOR ALL USING(true)` ops policy + REVOKE anon/auth/service + orders-grant-mirror.
  Opaque **256-bit** token (`randomBytes(32)`), built-in `sha256` hash (no search_path dep), single-use
  (used_at), TTL (hours), revoked_at, `invited_contact_hash`, partial-unique one-active-per-source.
- **K2 — token is the SOLE transfer authority (IDOR).** The claimer presents ONLY the token; `acquisition_source_id`
  / org / location are DERIVED from the matched invite, never from the request body. No request parameter
  selects a shadow → no enumeration. Red→green: authed user w/o token cannot claim any shadow; bogus/expired/
  used/revoked → 4xx, owner_id stays NULL.
- **K3 — claim-accept is `verifyAuth`-only** (NOT requireRole/requireLocationAccess — the claimer has neither
  membership nor owner role yet). After claim, role re-derives from membership per-request (ADR-0004); membership
  INSERT sets `status='active'` explicitly.
- **K4 — the approve subsystem is net-new (no `allergens_confirmed=true` writer exists).** Build it; without it
  the flag is forever false. Per-product owner write, `requireRole(owner)`+`requireLocationAccess`.
- **K5 — C2 live read-gate sequencing:** the read_public_menu/all_locales re-version (verbatim 065/035 + the
  `source='place' AND allergens_confirmed=false → attributes - 'bom'` CASE) is a TOTAL-blast-radius hot-path fn.
  REQUIRES 068 applied (else `source`/`allergens_confirmed` columns don't exist → CREATE OR REPLACE errors →
  every tenant's menu down). Ships in the SAME change that enables publish-on-claimed-shadow, proven on the
  FULL schema + staging Playwright (golden-snapshot: an owner tenant's menu byte-identical before/after).

## 🟠 Safety architecture (the load-bearing insight)
AI allergens never reach a live menu via TWO proven layers, so K5's hot-path re-version is **defense-in-depth,
not the sole guard**: (1) the P6-3 **write-strip** already nulls `bom[].allergens` to `[]` for place products
→ a published place product surfaces an EMPTY allergen list, never an AI guess; (2) **owner AUTHORS allergens
into empty fields** post-claim (counsel C3 — never "confirm" an AI-prefilled guess; anchoring an owner onto a
hallucinated allergen is the worst failure on the safety line). The read-gate (K5) covers the residual
re-populate-bom-while-unconfirmed window. So a safe claim phase ships now; the read-gate is staged + staging-proven.

## 🟠 HIGH fix-conditions
- **H-erase-on-claim:** a successful CLAIMED must ALSO clear `place_raw` + `menu_draft` on the source (raw
  unconsented third-party blob must not outlive provisioning). Today only explicit hardDelete clears them.
- **H-decline:** DECLINE+ERASE is mandatory, **one action, token-only, NO registration**, equally prominent to
  claim (counsel C2) → calls `hardDeleteShadow`. Claim-prominence > decline-prominence is the dark-pattern tell.
- **H-abandoned-TTL:** a CLAIM_OFFERED never accepted must self-`ABANDONED` + `hardDeleteShadow` on a SHORT TTL
  (public shadow = aggregator-squatting if left); sibling reaper to `reapExpiredGrants`.
- **H-publish-coupling:** never-orderable rests on `published_at` (orders.ts:134); decision-3 status-reject still
  unbuilt. Keep `published_at` NULL through claim; publish only via the gated activation path.
- **H-void-grants:** the claim tx voids any outstanding `provision_grants` for the source (stale-token hygiene).

## 🟢 Counsel binding conditions (consent integrity)
- **CC1 — invite = honest Art-14 notice written for the HOSTILE recipient:** controller identity+contact,
  purpose+legal basis, data categories, **source = your public site / Places (named)**, retention, **erasure
  right** + supervisory-authority complaint. Tone: "we built a preview from your public site; you didn't ask
  for this; here's exactly what we did + your options" — NOT "Claim your free store!".
- **CC2 — claim → review → publish stays THREE acts.** No one-click "claim & go live" (would launder unreviewed
  AI descriptions as the owner's word — violates decision #4).
- **CC3 — allergen confirmation is a DISTINCT deliberate act into EMPTY fields**, decoupled from description
  approval. Ingredients (bom) may show as authoring context; an AI allergen VALUE must never be pre-filled.
- **CC4 — track decline-without-complaint (+ zero C&Ds) as a first-class health signal**, not claim-rate.

## Decisions taken (within the autonomy grant; operator may override)
- **Token-only transfer authority** (architect proved it closes IDOR); `invited_contact_hash` is stored so an
  **email-match hardening** (counsel-recommended, "dignity stake") can be enabled later — built as a stage, not
  blocking. An owner-initiated "this is my restaurant" verified-invite request (counsel steel-man) is a follow-up.
- **Approve sets `allergens_confirmed=true` only — never mutates `source`** (preserves the `place` provenance/
  liability audit; the C2 gate keys on it).

## Migration head: 071 (claim_invites + claim_accept policies) · 072 (staged C2 read-gate). Both REQUIRE 068+069+070.
