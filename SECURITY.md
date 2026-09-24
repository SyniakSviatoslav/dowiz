# Security Policy

## Reporting a vulnerability

Please report security problems **privately**:

- **Primary:** GitHub private vulnerability reporting,
  <https://github.com/SyniakSviatoslav/dowiz/security/advisories/new>.
- **Fallback:** contact the maintainer through the GitHub profile of
  [@SyniakSviatoslav](https://github.com/SyniakSviatoslav) and ask for a private channel.

Do not open a public issue, pull request or discussion for a live exploit, and do not test against
a live venue's data beyond what is needed to show the problem. Include the host, the route, the
request and the response, and what an attacker gains. We aim to acknowledge a report within three
working days. Cross-tenant access, unauthenticated writes, money or consent bypasses are treated as
the highest priority.

## Supported versions

| Version | Supported |
|---|---|
| `main` and the latest `v*` release | yes |
| older tags (`2026.07.0` and before) | no |

## What protects a venue today

Each line names the code or gate that holds it, so it can be checked rather than trusted.

- **Tenant isolation.** The venue is resolved once per request from the Host header; each venue's
  records live in its own Durable Object. `tools/gates/one-venue.sh` refuses a handler that
  authorises one venue and acts on another.
- **Write routes.** The 2026-09-21 red-team pass deleted two unauthenticated write routes and closed
  three route families (commit `727bc591`). The principal-binding gaps still open (threads, the
  room's wallet debit, booking-token scope) are listed in
  `docs/design/AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24.md`, Wave G row G6. An order id alone is not a key: reading an order needs the
  guest's bearer key, and `tools/live-checks/health.sh` checks that refusal on production every 15
  minutes.
- **Tokens** are HMAC-SHA256 with the algorithm fixed in code, never read from the token
  (`crates/dowiz-hub/src/token.rs`, `workers/api/src/auth.rs`).
- **Staff act through signed capabilities** (`crates/dowiz-hub/src/caps.rs`), a closed set; red-line
  agent capabilities deny by default (`crates/dowiz-core/src/ports/agent/scope.rs`).
- **Card data never reaches the Worker.** Payments are Stripe PaymentIntents; card details go from
  the browser to Stripe and no type in the Worker can hold a card number
  (`workers/api/src/stripe.rs`).
- **Content Security Policy** on every served document; `e2e/kit-regression/render.mjs` catches
  violations in a real browser.
- **Idempotent replays.** A retried request cannot create a second order or payment
  (`workers/api/src/idempotency/`, `tools/gates/idempotent.sh`).
- **Allow-listed outbound calls** to the fiscal platform (`workers/api/src/ebills/client.rs`) and a
  circuit breaker on third-party rails (`workers/api/src/rail.rs`).
- **Supply chain.** `deny.toml` (yanked crates, wildcards, licences) is enforced by `cargo deny` in
  CI; Dependabot proposes updates weekly.

## Cryptography, stated precisely

- `crates/dowiz-core/src/pq/` contains an ML-DSA-65 (FIPS 204) implementation verified byte-exact
  against the vendored NIST ACVP vectors. It sits behind the kernel's off-by-default `pq` feature
  and is not on the live request path.
- A sealing path for the nightly off-site copies exists (`workers/api/src/cloud/seal.rs`) and is
  switched off until the operator sets `BACKUP_SEAL_PK`.
- dowiz does not claim post-quantum encryption of any live traffic or stored data.

## Personal data

Consent is an append-only log and withdrawal is a new record; a marketing send needs a value only
the consent fold can produce (`tools/gates/consent.sh`). A person can be forgotten in place, with the
erasure declared as a `Forgotten` event so the log's chain still verifies. Every look at a
customer's contact details is logged. Gaps against GDPR and Albania's Law 124/2024 are listed openly
in `docs/design/BLUEPRINT-GDPR-AND-MCP-2026-09-24.md` (Wave P).

## Out of scope

- The AI endpoint a venue connects for its assistant (the venue's own processor).
- A venue's own Telegram bot, Meta app, Stripe account and S3 bucket credentials, beyond how dowiz
  stores and uses them.
- Denial of service by volume against Cloudflare's edge.
