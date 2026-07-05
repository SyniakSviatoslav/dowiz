# Reflection — proofs that assert shape instead of behavior, and rules duplicated instead of shared

- **Date:** 2026-07-05
- **Trigger:** meta-loop backlog item 6 (autonomous-continuation task) — retrospective read of
  commit `aaa0b1827430f2c8c6f85aa01149bca3c677c160` ("audit proof-hardening, GTM display fixes,
  closed-venue gate, voice safety guardrails + design research"), batch 2 of the 2026-07-03 audit
  remediation, sibling to `69ad3074`.
- **Class:** self-improvement loop / test-integrity false-green family (ledger #67) + duplicated
  invariant family (echoes the 3-secret-store lesson, `2026-07-03-secret-store-provenance-trace`).

## WHAT happened

Two distinct but related gaps surface in the same commit:

1. **Proofs that pass regardless of the bug.** LC9's `no-fabricated-fallback` red-arm never ran the
   detector against a fabricated fixture — it was self-referential, so it would pass whether or not
   the detector actually worked. LC2's PATCH-IDOR proof pinned the literal SQL string rather than
   exercising a real cross-tenant read, so a refactor that kept the string but broke the tenant
   check would still pass. Both are the *frontend/API-test* instance of the exact same shape as
   the prior commit's GDPR/backup gaps: a check that is structurally incapable of going RED on the
   real defect.
2. **The same business rule computed twice, independently, and drifting.** The closed-venue gate
   (`isVenueOpen`) had to be *added* server-side because open/closed was previously derived only in
   the storefront (`hours_json` + `delivery_paused`) — the server had no matching check, so a
   customer could see "closed" and still place an order. The money-display fix is the same
   pattern: a hardcoded `/100` in one render path desynced from `currency_minor_unit` used
   correctly elsewhere, and `default_locale` was read in some places but not honored in
   `resolveInitialLocale`.

## WHERE

`apps/api/src/...` LC9/LC2 test files (ledger #67) · `isVenueOpen` (server) vs `public/menu.ts:335-358`
(storefront) closed-venue derivation · SSR JSON-LD offer price + `PromotionsPage` (100×-off render) ·
`resolveInitialLocale`.

## WHY (causal root, not just location)

Gap 1 is literally the same root as the sibling reflection
(`2026-07-05-gdpr-backup-completion-was-unconditional.reflection.md`): a check was written to match
the *code's current shape* (a specific SQL string, a self-asserting fixture) instead of the
*invariant the code is supposed to hold* (no cross-tenant leak, a fallback-detector that actually
detects). This is now confirmed as a **pattern that recurs across layers** — backend workers,
backup scripts, and API/frontend security tests all produced the same "green-by-construction" shape
in the same 2026-07-03 audit sweep, which means it is not a one-off authoring mistake but a missing
authoring habit: nobody was asking "what pre-fix state must this test see, and does it actually see
it?" before landing the proof.

Gap 2's root is different: it is the **same-invariant-in-two-places** failure the codebase has
already named once (the 3-secret-store lesson — Fly runtime / GitHub Actions / Supabase role
password holding the "same" URL with no single source of truth). Here it's "is this venue open"
and "what is this item's real-currency price" computed once client-side and once (or not at all)
server-side, with no shared function or contract test keeping them identical. Business logic that
exists in two independent implementations will diverge; only a shared source (or a parity test)
prevents it, and neither existed here until this remediation added one reactively.

## CONFIDENCE

High — read directly from the commit body and cross-checked against ledger #65 (closed-venue),
#66 (money-display), and #67 (proof-hardening), all of which independently confirm the same
defects with red→green proof.

## NEXT-TIME

- When writing a negative/security-path test (IDOR, fabricated-fallback, cross-tenant), the test
  must construct the actual pre-fix condition and assert failure on the unfixed code — a proof
  that never runs its detector against a broken case is not a proof.
- When a business rule (venue-open, money conversion, locale resolution) must hold identically on
  client and server, either share the implementation or add an explicit parity test — don't let two
  independent derivations of "the same" rule ship without one checking the other.

## PROPAGATE (candidate — advisory; librarian/ratchet decides)

Feeds the curated lesson `docs/lessons/2026-07-05-proof-must-observe-the-effect.md` (this run),
which generalizes gap 1 across this reflection and its sibling. Gap 2 (duplicated invariants)
is a second, distinct candidate for a future lesson if a third recurrence appears — not promoted
here since two instances (secret-stores, venue/money) is not yet a strong pattern on its own.

## LINK

`docs/regressions/REGRESSION-LEDGER.md` #65, #66, #67 · commit `aaa0b182` ·
`docs/lessons/2026-07-03-secret-store-provenance-trace.md` (duplicated-invariant precedent) ·
[[2026-07-05-gdpr-backup-completion-was-unconditional.reflection.md]]
