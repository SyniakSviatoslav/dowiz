# Certimate pilot — out-of-tree custom-domain TLS candidate

**STATUS: SCAFFOLDED — DO NOT USE. PARKED WITH TRIGGER.** Out-of-band. Not wired, not in CI, not a
dependency. Registered as a *future* candidate, dark.

## What it is
[certimate-go/certimate](https://github.com/certimate-go/certimate) (MIT) — self-hosted ACME tool that
automates full-cycle SSL: issuance → deployment → renewal → monitoring. Supports DNS-01 + HTTP-01,
multiple CAs (Let's Encrypt, ZeroSSL, Google Trust Services, …), wildcard + IP certs. ~16 MB, zero external
deps (no DB/runtime). Cross-platform.

## Why it's a candidate (trigger, not now)
Fly.io already terminates TLS for `*.fly.dev` and manually-added custom domains. Certimate becomes
relevant **only when we offer white-label storefronts on tenants' own domains** (`shop.theircafe.al`),
where we'd need programmatic, per-tenant cert issuance/renewal at scale that Fly's manual cert flow
doesn't cover cleanly.

## Boundary
- **Trigger to revisit:** the first paid custom-domain storefront request, OR a per-tenant-domain line in
  the roadmap. Until then: parked.
- Ops/infra plane only — not a product code dependency. If adopted, runs as an isolated service with its
  own ACME account + DNS-provider token (never a dowiz app/RLS secret).
- Scope it against the existing storefront routing (`/s/:slug`, `spa-proxy`) before wiring — custom-domain
  → tenant mapping is the design question, cert automation is downstream of it.
