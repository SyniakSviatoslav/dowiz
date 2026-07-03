# Security Policy

We take the security of this project and its users' data seriously. Because the
platform handles orders, addresses, and phone numbers for real diners and
restaurants, we treat security reports as high priority.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report privately via one of the following:

- **Email:** `TODO: security@<operator-domain>` *(operator to configure before
  the repository is made public)*
- **GitHub private advisory:** use the repository's **Security → Report a
  vulnerability** ("Private vulnerability reporting") once enabled.

> **Operator TODO (pre-public gate):** set up the private disclosure channel
> above — a monitored security mailbox and/or GitHub private vulnerability
> reporting — and replace this placeholder. Do not make the repository public
> without a working private channel.

When reporting, please include:

- A description of the vulnerability and its impact.
- Steps to reproduce (proof-of-concept, affected endpoint/route, request).
- Affected version / commit if known.

Please give us a reasonable window to remediate before any public disclosure.
We will acknowledge your report, keep you updated, and credit you if you wish.

### Please do not

- Access, modify, or exfiltrate data that is not yours.
- Run automated scanners against the **hosted production** service without prior
  arrangement. Test against a **local self-hosted instance** instead.
- Perform DoS/load testing against production.

## Supported versions

This project ships continuously; the **hosted cloud** service always runs the
latest released `main`.

| Version                     | Supported          |
| --------------------------- | ------------------ |
| Latest `main` (hosted)      | ✅ Yes             |
| Self-hosted from latest tag | ✅ Yes             |
| Older tags / forks          | ⚠️ Best-effort — please upgrade |

Self-hosters are responsible for tracking releases and applying security fixes
promptly. Security fixes land on `main` and the hosted service first.

## Security posture

The codebase is built with defence-in-depth as a default, not an afterthought:

- **Row-Level Security (RLS)** enforced at the Postgres layer for tenant
  isolation — application code is not the only line of defence. RLS is `FORCE`d
  and verified (`pnpm verify:rls`); DB roles do not carry `BYPASSRLS` in
  production.
- **Strict input validation** — request bodies are parsed with **Zod** (strict
  schemas); unknown/extra fields are rejected rather than silently accepted.
- **Constant-time comparisons** (`timingSafeEqual`) for tokens and secrets to
  avoid timing side-channels; auth tokens are revocable with per-request status
  checks.
- **Fail-closed auth** — dev-login and privileged paths fail closed; platform
  admin access is gated by an allowlist, not role inference alone.
- **PII discipline** — no secrets, cookies, or PII in logs; a PII-leak detector
  and privacy gate run in CI. GPS/location access is guarded to active
  deliveries only.
- **Secrets hygiene** — secrets live in environment / secret stores, never in
  the repo; a secrets scan runs before release. `.env*` is git-ignored.
- **Governance gates** — `plane-guard` and the `verify:all` suite (typecheck,
  lint, unit/integration, RLS, migrations, contract, privacy) gate every change;
  regressions must ship with a red→green guardrail.

Security is verified programmatically (tests, gates, guardrails), not by
assertion. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the guardrail rule
that every fix must satisfy.
