# OpenAPI Contract Conventions — S1 storefront-read · S2 auth (strangler SSOT)

- **Date:** 2026-07-04 · **Lane:** OPENAPI-CONTRACT (rebuild program) · **Status:** authored-from-live-source
- **⚠️ Staging path:** the mandated home `docs/design/rebuild-plan/contracts/` is inside the
  `protect-paths.sh` protected zone (pattern `/contracts/`, `.claude/hooks/protect-paths.sh:54`).
  These files are therefore staged at `docs/design/rebuild-plan/openapi-contracts/`; the move into
  `contracts/` is the operator's manual-approval act:
  `git mv docs/design/rebuild-plan/openapi-contracts docs/design/rebuild-plan/contracts`.
- **Authority:** these YAML files are the **single source of truth** for the first two strangler
  surfaces. They document **CURRENT Node behavior verbatim** — extracted from
  `apps/api/src/routes/*` (Zod schemas translated faithfully, file:line cited per operation),
  `inventory/10-api-realtime-jobs.md` (route census) and `inventory/14-crosscutting-proofnet.md`
  (§3 error contract, §4 auth flows). Quirks are **annotated (`x-quirk`), never silently fixed** —
  fixing a quirk is a separate, explicit contract change with FE + E2E lockstep.
- 🔴 **S2 is council-gated** (every auth flow is a red-line row per inventory 14 §4). This contract
  is the *description* input to that council, not an approval to port.

## Files

| File | Surface | Operations |
|---|---|---|
| `openapi-s1-storefront-read.yaml` | S1 — public storefront read (`/s/:slug` + its API reads) | 20 |
| `openapi-s2-auth.yaml` | S2 — auth flows (owner/courier/claim/track/dev) 🔴 | 20 |
| `traceability-s1-s2.csv` | operationId → file:line → inventory row → E2E proof | 40 rows |

## Contract-first rule (Rust/utoipa side)

1. **This YAML is authority; generated output is evidence.** The Rust port annotates handlers with
   `utoipa` and CI runs `openapi-diff <authored>.yaml <generated>.json` — the gate passes only when
   the diff is **empty** (modulo `x-*` extensions, which utoipa must re-emit via
   `#[schema(extensions(...))]` where load-bearing: `x-money-minor-unit`, `x-quirk`, `x-cache`).
   A mismatch means the Rust code is wrong OR a deliberate contract change is being smuggled —
   either way the build fails until the YAML is amended first (contract-first, code-second).
2. **Additive-only versioning** (REBUILD-MAP §2 decision register / §6 hub invariants): published
   operations never change shape incompatibly. Allowed without a version bump: new optional request
   fields, new response fields, new enum values *on enums marked `x-open-enum: true`*, new
   endpoints. Forbidden: renaming/removing fields, changing types, tightening request validation,
   dropping error codes. A breaking change requires a NEW path or an explicit council-approved
   migration with FE/E2E lockstep. Error `code` values are append-only forever (ADR-0010 B1).
3. During strangler Phase B both stacks serve the same paths behind the proxy — the E2E net
   (174 specs) is the parity oracle; the CSV's E2E column tells you which specs must stay green
   per operation when it cuts over.

## Naming

- **Paths:** verbatim from the live Fastify registrations, including their inconsistencies
  (`/public/locations/...` vs `/api/public/...` vs `/s/...` vs `/v1/rates`). The rebuild does NOT
  normalize paths — the FE and the SW cache rules depend on them (e.g. `voice-config` deliberately
  under `/api/` so the service worker never caches it, `voice-config.ts:5-9`).
- **operationId:** `camelCase`, verb-first (`getPublicMenu`, `ownerLocalLogin`,
  `courierRedeemInvite`). Stable forever once published (E2E + traceability key on it).
- **Field names:** verbatim wire names — the live API mixes `snake_case` and `camelCase`
  (sometimes in one payload: `/public/locations/:slug/info` returns `currency_code` next to
  `deliveryFeeFlat`). Documented as-is with `x-quirk` where mixed; the Rust serde derives must
  reproduce the exact names (`#[serde(rename = ...)]`), never re-case.

## Error envelope (shared component — ADR-0010)

Single schema `ErrorEnvelope`, mirrored **byte-identically** in both files (they must stay in sync;
the map-coverage gate diffs the two components):

```jsonc
{ "code": "SCREAMING_SNAKE",   // machine code, FE-branchable — append-only, never rename/drop
  "message": "human text",     // generic on 5xx
  "fields": [{"path","code"}], // validation only — field PATHS, never values (PII-safe)
  "correlationId": "uuid",     // server-authoritative (inbound header NOT trusted), echoed in x-correlation-id
  "retryAfterMs": 1234,        // 429 only (+ Retry-After header)
  "status": 422,               // numeric HTTP status (legacy field — keep until FE re-audit)
  "error": "= message" }       // legacy alias (apiClient.ts:211 reads message || error) — keep
```

Source: `apps/api/src/lib/api-error.ts:56-72` (`buildErrorEnvelope`), central handler
`apps/api/src/server.ts:443-520`. Validation failures → `400 VALIDATION_FAILED` (status
**preserved at 400**, not 422 — a deliberate ADR-0010 code-preserving decision; ~10 E2E contract
tests assert it).

**Divergent (non-envelope) shapes are part of the current contract** and are documented per
operation with `x-quirk` instead of being mapped to the envelope. The S1+S2 set contains these
divergent families (subset of the 51 ad-hoc sites counted in inventory 14 §3):
`claim.ts` bare `{error: CODE}`; courier-auth manual-Zod `{error:'Validation failed', details}`;
`auth.ts` refresh-race `409 {error:'concurrent_refresh'}`; telegram-poll status bodies
(`404 {status:'unknown'}` etc.); track-exchange `410 {error:'TRACK_LINK_EXPIRED', message}`;
global Bearer gate `401 {error:'Unauthorized'}`; dev-guard `404 {error:'Not found'}`. In Rust
there is one exit (`IntoResponse`) — each of these is a **council decision row**: keep-verbatim
(FE branches on it today) or migrate-with-FE-lockstep. Never silently canonicalize.

**Separate namespace (never merge):** preflight `reasons[].code` — lowercase business-outcome
tokens (`item_unavailable`, `velocity`, `no_show_history`, `otp_required` —
`apps/api/src/lib/preflight.ts:71-138`; 4 unique codes across 8 emission sites). They ride
**success-path** payloads (S5 order-create `{outcome, reasons}`), not error envelopes. S1 defines
the shared `PreflightReason` component (reserved, referenced by no S1 route) so S5 imports rather
than re-invents it.

## Money / dates / ids

- **Money = integer minor units, always** (ADR-0005). Every money field is `type: integer` with
  `x-money-minor-unit: ALL` (Albanian lek, `minor_unit: 0` — 1 lek = smallest unit; the payload's
  `currency.minor_unit` / `currency_minor_unit` field is the per-tenant runtime authority).
  Never `number`, never floats, never strings. Non-money numerics that look money-adjacent are
  explicitly NOT marked: `taxRate` (config fraction), `rate` (FX float), `googleRating`.
- **Dates:** `type: string, format: date-time` (ISO-8601, `Date#toISOString()` output).
  Times-of-day (venue hours `open`/`close`/`closesAt`) are `HH:MM` strings — documented with
  `pattern`, NOT date-time.
- **IDs:** `type: string, format: uuid` wherever the source binds `::uuid` / `z.string().uuid()`.
  Slugs: `pattern: ^[a-z0-9-]+$` where the source enforces it (`claim.ts:12`), otherwise plain string.
- **JWTs:** opaque strings to the HTTP contract; claim shapes are documented in
  `openapi-s2-auth.yaml` `components.schemas.AuthTokenClaims*` (from the Zod union
  `packages/shared-types/src/legacy.ts:161-175`) because Phase-B token compatibility is the
  strangler's load-bearing seam (both stacks verify the same RS256 tokens, same `kid`).

## Auth / security schemes (reality, verified)

- **Bearer RS256 only. Zero cookies.** The API sets no cookie anywhere (inventory 14 §4:
  `grep -rn cookie apps/api/src` → only redaction lists; no `@fastify/cookie`). Session state is
  client-side `localStorage` (`dos_access_token`/`dos_refresh_token`) — so the S2 file declares a
  single `bearerAuth` (http/bearer/JWT) scheme and **no** cookie scheme. Moving to httpOnly cookies
  is AUTH-GAP-5, an explicit council decision, not a port default.
- The global Bearer-presence pre-gate (`server.ts:399-427`: `AUTH_PREFIXES` minus `NO_AUTH_PATHS`
  minus the OTP regex) returns `401 {error:'Unauthorized'}` (divergent shape) before any route
  handler — documented once in S2 (`GlobalBearerGate401`) and referenced.
- Dev routes are described **but flag-gated**: without `ALLOW_DEV_LOGIN='true'` AND matching
  `x-dev-auth-secret` they are `404 {error:'Not found'}` — existence-hiding fail-closed (ADR-0003).
  The Rust build should compile them out (`#[cfg(feature = "dev-routes")]`) AND keep the runtime gate.

## Cache semantics

Documented per-operation via `x-cache` (and response `Cache-Control` headers), from the live
`reply.header` calls — the storefront's perceived speed depends on these being ported exactly:

| Route | Cache-Control | Server-side cache |
|---|---|---|
| `/public/.../menu` | `public, max-age=60, stale-while-revalidate=300` + `X-Menu-Version` | in-proc 30s fresh / 300s SWR / 1h stale-on-error, ≤500 keys FIFO (`menu.ts:89-99`) |
| `/public/.../info` | none (uncached JSON) | in-proc 30s row cache + 1h stale-on-error (`menu.ts:104-111`) |
| `/public/.../products/:id/media` | `public, max-age=60, stale-while-revalidate=300` | none |
| `/public/.../theme.css` | `?hash=` → `public, max-age=31536000, immutable`; else `public, max-age=60` | none |
| `/images/*`, `/media/*` | `public, max-age=31536000, immutable` (content-hashed keys) | none |
| `/s/:slug*` HTML shells | `no-cache, no-store, must-revalidate` (`spa-shell.ts:162`) | none |
| `/s/:slug/manifest.webmanifest` | `public, max-age=3600` | none |
| `/api/public/voice-config` | `no-store` (kill-switch must propagate instantly) | none |
| `/v1/rates` | `public, max-age=300` **on the static-fallback branch only** | none |
| `/robots.txt` | `public, max-age=86400` | none |
| `/sitemap*.xml` | `public, max-age=3600` | none |
| all S2 auth responses | none (default no-cache JSON) | n/a |

## Channel extension (reserved for S5 — hub taxonomy)

`components.schemas.Channel` is defined in BOTH files (S1 defines, S2 mirrors) but referenced by
**no** S1/S2 operation. It reserves the order-attribution enum so S5 (`orders/money` +
`sales_channel` entity, REBUILD-MAP §3 Phase B / §6 channel-hub) extends the order-create request
with an **optional** `channel` field instead of inventing a second taxonomy:

```yaml
Channel:
  type: string
  description: Order-attribution channel (hub taxonomy, REBUILD-MAP §6). Optional everywhere; absent = web-direct.
  enum: [web-direct, qr, nfc, gbp, apple-maps, instagram, facebook, whatsapp, telegram-tma, kiosk, widget, agent, other]
  x-open-enum: true   # additive-only: new channels append, consumers must tolerate unknown values
  x-reserved-for: S5
```

## Validation status

See the lane report: `npx -y @redocly/cli lint` / `swagger-cli validate` attempted; output pasted
in the returning summary. If the sandbox has no registry access, the files ship **unvalidated**
and CI must lint them on first PR (add `openapi-lint` to `verify:all`).
