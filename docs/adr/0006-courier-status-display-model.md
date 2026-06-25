# ADR-0006 — Courier status display model (account vs presence vs onboarding)

Status: Proposed (design-time) · Date: 2026-06-25 · Branch: fix/design-system-consistency

## Context

`apps/web/src/pages/admin/CouriersPage.tsx:179` maps the courier **account** status to a **presence**
label: `c.status === 'active' || 'available' ? 'online' : ...`. But `couriers.status` is
`active | deactivated | suspended` (migration `1780421029538`, default `active`), so **every active
account renders green "Online"** — including freshly-invited, phone-less couriers who have never logged
in — and inflates the `onlineCount` (CouriersPage.tsx:222) into a false "fleet is live" metric. The
`'available'` branch is dead: that is a `courier_shifts.status` value the list endpoint does not return.

The list contract `GET /api/owner/locations/:locationId/couriers` (`apps/api/src/routes/owner/couriers.ts:17-70`)
returns `status` (account), `onlineStatus: null` (presence NOT surfaced today, couriers.ts:53),
`maskedPhone` (null when no phone), `lastLoginAt` (null when never logged in).

Three orthogonal axes exist in the schema and were collapsed into one label:
- **ACCOUNT** — `couriers.status` ∈ {active, deactivated, suspended} (`1780421029538`).
- **PRESENCE** — `courier_shifts.status` ∈ {offline, available, on_delivery} (`1780421036157`); the real
  online set is already read by `GET .../couriers/live` (couriers.ts:141-193).
- **ONBOARDING** — **NOT derivable from the list endpoint** (the original draft assumed it was; the
  RESOLVE step disproved this — see below).

## H4 correction (Breaker inverse-lie — the original "Pending setup" criterion was unprovable)

Verified against source:
- `last_login_at` is stamped **only** by the password-login path (`courier/auth.ts:308`). The
  **invite-redeem** path (`auth.ts:88-148`) issues a JWT + 30-day session and **never stamps it**; the
  **refresh-rotation** path (`:353-468`) does not either → a fully onboarded, session/refresh-authed
  courier has `last_login_at == null` indefinitely.
- **Phone is optional at invite-redeem** (`auth.ts:38`) → a fully onboarded courier may have
  `maskedPhone == null`.
- A `couriers` row in this list **exists only because an invite was redeemed** (sole prod creation path
  `auth.ts:89`; `server.ts:732` is dev-gated) → **row existence already proves onboarding.**

So `(!maskedPhone || !lastLoginAt) → "Pending setup"` would brand a real, possibly on-shift courier as
un-onboarded forever — an inverse-lie, a reachable production state. The endpoint cannot prove
onboarding-incompleteness, so it must not assert it.

## Decision (re-pinned)

**Honest display = account status ONLY. Ship FE-only now; defer real presence to Option B.**

FE derivation (replaces CouriersPage.tsx:179 + :222), no server/contract change:

- `status === 'suspended'` → **"Suspended"**
- `status === 'deactivated'` → **"Inactive"**
- `status === 'active'` → **"Active"** (account ENABLED — explicitly NOT a presence claim, no green dot).

There is **no FE-derived "Pending setup"** (unprovable, H4). The "N online" badge is replaced by
**"N active"** = `count(status === 'active')`, labelled/tooltipped as "accounts enabled — see live map for
who's on shift" (Counsel A5) so it is not misread as dispatch capacity. Genuine live presence already
lives on the live-map screen (`couriers/live`, `cs.status IN ('available','on_delivery')`).

**Prerequisite for Option B (server-side, additive, follow-up — NOT gating this PR):** stamp
`last_login_at = now()` at **invite-redeem** (after `auth.ts:89`) and at **refresh rotation** (`:468`),
matching the password-login stamp (`:308`). Until then `lastLoginAt` is meaningless and MUST NOT drive any
display claim.

**Deferred (Option B, additive contract):** populate the already-present-but-null `onlineStatus`
(couriers.ts:53) from a LEFT JOIN on `courier_shifts` (online when a current shift is `available` with a
fresh heartbeat, busy when `on_delivery`, else offline). Requires agreeing a heartbeat-staleness threshold
AND the `last_login_at` prerequisite above. RLS already FORCE-isolated by `app.current_tenant` —
read-only, no schema change.

## Consequences

- The badge and chips stop lying immediately, with zero server/contract risk.
- No real courier is libelled as un-onboarded (H4 inverse-lie removed by refusing to assert what the
  endpoint can't prove).
- The list screen shows no live presence until Option B ships; the live map carries it meanwhile.
- `deriveDisplay` reads only `status` (always present) → no missing-field degrade path, no false-green.
- No migration. No write path touched (the `last_login_at` stamping is a deferred, additive follow-up).

## Cross-item guard (NEW-M2, round 2 — only if Item 3(b) ships)

The honest "N active" = `count(status === 'active')` is polluted by Item 3(b)'s persistent synthetic
`status='active'` courier on the shared staging DB. To keep "N active" honest, the owner couriers query
(`apps/api/src/routes/owner/couriers.ts:34`) excludes the synthetic row by its sentinel `email_hash`
(`AND c.email_hash <> SYNTHETIC_COURIER_EMAIL_HASH`), removing it from both the list and the count. Prod:
no-op (the synthetic row never exists behind the dark dev gate). This closes the irony where Item 3's
fixture would re-introduce the exact Item-2 inflation Item 2 removes. See proposal §3.4 pt.5.
