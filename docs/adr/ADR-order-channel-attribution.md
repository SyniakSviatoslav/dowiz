# ADR-0021 — Order acquisition-attribution `channel` (write-only order metadata)

- Status: **DRAFT / PROPOSED** (design-time only; no production code in this change). ADR number `0021`
  provisional — confirm against the live sequence before ratification (last taken: ADR-0020 open-source).
  See `docs/design/order-channel-attribution/proposal.md`.
- Date: 2026-07-04
- Seat: System Architect (QR/ATTRIBUTION build lane)
- Supersedes/relates: mig `1780421100057_anti-fake-signals.ts` (introduced `orders.metadata jsonb NOT NULL
  DEFAULT '{}'` + `CHECK jsonb_typeof = object`); ADR-0016 (checkout Communication channel); the
  OTP/velocity anti-abuse seam (which already writes `otp_verified` / `client_ip_hash` into that same jsonb).

## Context

Storefront orders are attributionally anonymous: nothing in the order-creation contract records *how the
customer arrived* — a table/menu QR, an NFC tag, a Google Business Profile link, Instagram/Facebook/
WhatsApp/Telegram, an in-app kiosk, an embedded widget, an agent, etc. Owners (and, later, first-party
analytics) cannot separate paid/organic acquisition channels.

We want a single, **purely descriptive** `channel` tag, captured from a `?ch=<value>` link param on the
`/s/:slug` storefront, threaded write-only into the row we already insert per order. Two hard lane
constraints frame every option: `packages/db/migrations/` is a **red-line/operator-gated protected path**
(no migration), and `package.json` is protected (no new dependency).

The `orders` table already carries a `metadata jsonb NOT NULL DEFAULT '{}'` column (mig `…057`), currently
holding `{ otp_verified, client_ip_hash }`, written as one positional parameter inside the single order
INSERT. That column is the pre-built per-order descriptive sidecar for exactly this class of signal.

## Decision (proposed — pending council)

1. Add an `OrderChannel` **Zod enum** to `@deliveryos/shared-types` — a **13-value allowlist**: `web-direct`
   (default), `qr`, `nfc`, `gbp`, `apple-maps`, `instagram`, `facebook`, `whatsapp`, `telegram-tma`, `kiosk`,
   `widget`, `agent`, `other`. This is the **single source of truth** for the allowlist.
2. Add **one optional field** `channel: OrderChannel.optional().default('web-direct')` to the existing
   `.strict()` `CreateOrderInput` schema. `.strict()` rejects undeclared fields, so this must be declared
   explicitly for the client to send it. Every **other** field's behavior is unchanged; the safe default
   means no existing (`?ch=`-less) client breaks.
3. Persist by **folding into the existing `orders.metadata` jsonb** — extend
   `JSON.stringify({ otp_verified, client_ip_hash })` to `{ …, channel }` at the existing INSERT. **No new
   DB column, no migration, no new positional parameter, no INSERT column-list change.** Adding a string
   key keeps `metadata` an object, so the existing `CHECK (jsonb_typeof(metadata) = 'object')` still holds.
4. Thread `channel: input.channel` from the POST `/orders` handler through `insertOrderWithItems`
   (add `channel` to `InsertOrderInput`). `channel` is **not** folded into the idempotency `request_hash`
   (two bodies differing only in `channel` are the same order intent).
5. **Client capture:** a new `apps/web/src/lib/channel.ts` reads `?ch=<value>` once on `/s/:slug` shell
   mount (`ClientLayout`), validates against the **same** `OrderChannel` enum imported from
   `@deliveryos/shared-types` (no duplicated list), and persists to **`sessionStorage`** (session-scoped,
   per-tab; a fresh tab/session re-attributes). Unknown/invalid → `'other'`; missing → default
   `'web-direct'`. All storage access is try/catch-guarded (degrades to `'web-direct'`, never throws).
   `CheckoutPage` reads it back and includes `channel` in the POST `/orders` body.
6. **Write-only.** `channel` gains **no reader** in pricing, order status/state-machine, dispatch/
   courier-assignment, notifications, or authz/RLS in this change. Reading it back out (analytics) is
   out of scope; if ever needed it is a reporting read, never a decision gate.
7. **Deferred, operator-gated:** if a first-class, indexable `orders.channel text` column is ever wanted,
   draft — do not build — a forward-only migration under `docs/design/channel-hub/migration-drafts/` for
   operator review (separate red-line change, its own ADR). The wire contract is identical, so it can land
   later with zero client change.

## Invariants honored

- **No migration; no DDL.** `packages/db/migrations/` untouched (red-line, operator-gated).
- **No new dependency.** No `package.json` touched.
- **Write-only / no new read path** — never read by price (subtotal/fee/tax/total), status transitions,
  dispatch, or any authz/RLS predicate.
- **Integer-money untouched** (no monetary field involved).
- **Server-authoritative default** (`web-direct`); unknown → `other`; parameterized query either way.
- **No injection/XSS** — Zod-enum-validated before it touches SQL; bounded length via the enum; never
  rendered as HTML.
- **No PII, no new AI egress.** A fixed enum token is not personal data; the GDPR anonymizer needs no change
  (unlike the sibling `client_ip_hash`, already handled).
- **No cookie / no `localStorage`.** Client persistence is `sessionStorage` only.
- **Tenant-agnostic.** Same `orders` row, same location-scoped INSERT, same ENABLE+FORCE RLS; no bypass, no
  cross-tenant read/write, no new policy.
- **Single source of truth** for the allowlist: `OrderChannel` in `@deliveryos/shared-types`.
- Idempotency preserved: `channel` excluded from `request_hash`.

## Consequences

- (+) Order acquisition source becomes captureable at effectively zero cost — additive to an existing write
  (~20–30 bytes on a jsonb blob we already stringify; no new row/tx/pool/index/consistency surface).
- (+) Reversible by omission — nothing reads it, so removal or a wrong value is inert (lossy-analytics, not
  an incident). No launch flag required (the field is behaviorally invisible).
- (+) Clean upgrade path to a first-class column later with an identical wire contract.
- (−) Not indexable/queryable-by-index at rest (jsonb); analytics must reach through `metadata->>'channel'`
  (accepted — no analytics built now; first-class column drafted-not-built for later).
- (−) Weak typing at rest — mitigated by enum-validation-before-SQL and the single order write path.
- (−) Enum growth needs a shared-types release rather than a migration (accepted — cheaper while write-only).

## Open questions (must close before ratification)

- **R1** First-touch vs last-touch semantics (shell mounts ~once/session → effectively first-touch). Accepted
  for v1; document chosen behavior in `channel.ts`.
- **R2** Confirm ADR sequence number (`0021` provisional).
- **R3** Whether to add a design-time guard/lint asserting `metadata->>'channel'` has no reader in
  pricing/dispatch/authz modules (recommended to lock the write-only invariant).
- **R4** Optional operator kill-switch (env-gated pass-through returning `web-direct`) — needed or not?
  See proposal §9. Not built by default.

See `docs/design/order-channel-attribution/proposal.md` §3 (options), §7 (failure/degradation), §10 (risks).
