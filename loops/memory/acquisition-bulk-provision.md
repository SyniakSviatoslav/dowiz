# Пам'ять петлі · acquisition-bulk-provision

«Без запису що сталося — немає покращення, і палиш токени на тих самих граблях.»

## Уроки (lessons learned)
- 2026-06-28 — BUILD+CERTIFY. Корінь loop-shape: per-restaurant = fixed 6-stage pipeline with state-pinned transitions + a gate per stage + exit states + idempotency → a loop, not a script.
- 2026-06-28 — Idempotency seam (key): there is NO GET endpoint for a source. `POST /internal/acquisition {place_id}` is `ON CONFLICT (place_id) DO UPDATE … RETURNING`, so it returns the source's CURRENT `state` — that IS the resume read. The loop reads it first, then jumps to the right stage.
- 2026-06-28 — no-fake-green: the §5/§6 work this session was burned by "HTTP 200 = pass". Encoded the counter as an iron principle + proven by a LIAR backend (200 verified:false / 201 no-token) that the loop MUST classify needs-review. A status-only loop fails Scenarios C/D — that's the cheat the dry-run catches.
- 2026-06-28 — never-spine-an-exit-state: `provision/spine` requires state ENRICHED (the machine rejects SOURCED→PROVISIONED). The loop reads state and routes MENU_NOT_FOUND/LOW_QUALITY/MANUAL_REVIEW to needs-review BEFORE any mutation — proven by org_id staying null on the broken fixtures.
- 2026-06-28 — claim re-mint is NOT idempotent at the API (partial-unique index → 409 ACTIVE_INVITE_EXISTS on a 2nd active invite). So a CLAIM_OFFERED source is treated as skipped-already-invited, never re-minted.
- 2026-06-28 — a token-only claim invite (no invited_contact) is decline-only on the web (claim.ts CONTACT_REQUIRED on accept). Surfaced as a per-item WARNING, not a silent half-success.

## Історія прогонів (run history)
| дата | тригер/скоуп | результат | flaky? | нотатки |
|---|---|---|---|---|
| 2026-06-28 | dry-run.mjs (anti-cheat, mock) | GREEN 21/21 | no | CERTIFIED; A=mixed batch, B=idempotent, C/D=liar, E=fail-closed |
| — | live deployment run | (not yet) | — | first live run pending operator-set PROVISION_BASE_URL + PROVISION_OPS_SECRET |
