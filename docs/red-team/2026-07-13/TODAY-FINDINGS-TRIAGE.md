# Triage — Today's red-team findings (2026-07-13) → status

Single source of truth mapping every finding from the 2026-07-13 D1–D7 reports +
MASTER-SYNTHESIS + ESCALATION-RETIRED-TREE to an in-repo status.

Scope rule (operator precedence, MANIFESTO D1 + DECISIONS): the centralized server
(apps/api Fastify + Supabase + Fly) was **retired to `attic/`** and is **not built**.
Patching `attic/` is a no-op against prod (D1 + ESCALATION-RETIRED-TREE.md). Therefore
findings filed against the retired tree are NOT patched in-repo; their classes are
ported into the NEW architecture as design gates (ADRs) so the replacement cannot repeat
them.

Legend:
- CLOSED-ADR   — class closed at the NEW-architecture design level (ADR written)
- CLOSED-REPO  — concrete in-repo fix shipped (verified)
- TODO-DEFER   — must land when the corresponding new-architecture component is built
- OPERATOR     — requires operator action (live DB / live deploy / GitHub-side), not repo-fixable
- OOS-D1       — out of scope per MANIFESTO D1 (retired stack; do not fake-fix)

## D1 — AppSec / AuthZ (legacy apps-api)
| Finding | Class | Status | Note |
|---|---|---|---|
| F1 CRITICAL seeded owner cred live in prod | weak-credential | OOS-D1 (file) / OPERATOR (live) | Retired tree; live prod seed needs operator DB action. New arch: ADR-0007 removes directory/phone-home → no seeded-cred class. |
| F2 HIGH couriers route missing requireRole | missing-role-gate | OOS-D1 (file) / TODO-DEFER (new) | New-seam mints+Zod-validates `role` (packages/platform/src/auth/jwt.ts); route-layer `requireRole` TODO when HTTP routes land. |
| F3 HIGH cross-tenant PII erasure | missing-ownership-check | OOS-D1 (file) / TODO-DEFER (new) | New arch: ADR-0008 per-node local SQLite → never aggregates cross-tenant data. ETL owner-check TODO when it lands. |
| F4 HIGH SSRF IPv6 literal bypass | ssrf-ipv6-gap | CLOSED-ADR | ADR-0009 mandatory canonicalize-then-allowlist + DNS-rebind pin; audit live fetcher TODO when it lands. |
| F5 MED telegram webhook no header enforce | auth-bypass | OOS-D1 | Retired tree. |
| F6 MED customer token order-scoped but endpoints check customer_id only | token-scope | OOS-D1 | Retired tree. |
| F7 MED `couriers/live` GPS leak | cross-role | OOS-D1 | Retired tree (sub-case of F2). |
| F8 MED admin kill-switch divergence prod vs attic | deploy-drift | OPERATOR | Live deploy observation; not repo-fixable from retired source. |
| F9–F12 (LOW/MED) misc authz/defense-in-depth | various | OOS-D1 | Retired tree. |
| F13 LOW defense-in-depth relies solely on RLS/flag | depth | OOS-D1 | Retired tree. |

## D2 — RLS / data governance (legacy Supabase)
| Finding | Class | Status | Note |
|---|---|---|---|
| 11 tables no RLS / USING(true) / fail-open (incl. couriers PII, telegram_login_tokens) | rls-gap | OOS-D1 (file) / CLOSED-ADR (new) | Retired Supabase stack. New arch: ADR-0008 local SQLite per-node, no central DB, no cross-tenant surface. |
| DSAR/data-export absent | compliance-gap | OOS-D1 | Retired stack. |

## D3 — API abuse / auth
| Finding | Class | Status | Note |
|---|---|---|---|
| F1 CRITICAL weak test cred live prod | weak-credential | OOS-D1 / OPERATOR | Same as D1-F1. |
| F2 MED rate-limit keys on req.ip (shared bucket) | ratelimit-keying | OOS-D1 | Retired stack (Fastify behind Fly). |
| F3 MED order-create limiter keys on body.phone | ratelimit-bypass | OOS-D1 | Retired stack. |
| F4 LOW anonymous telegram webhook on staging | auth-bypass | OOS-D1 | Retired stack. |
| F5 LOW /health topology disclosure + rateLimit:false | info-leak | OOS-D1 | Retired stack. |
| F6 LOW login user-enum oracle | user-enum | OOS-D1 | Retired stack. |
| F7 CSP missing on SPA shell / weak on SSR | csp-gap | **CLOSED-REPO** | **Dockerfile nginx now sets strict CSP + Referrer/Permissions-Policy** (verified patch). Applies to the staying static SPA. |

## D4 — Live web session / XSS
| Finding | Class | Status | Note |
|---|---|---|---|
| XSS blocked (React auto-escape) | xss | CLOSED-REPO (existing) | Confirmed by report; reinforced by D3-F7 CSP. |
| Clickjacking blocked (XFO SAMEORIGIN) | clickjacking | CLOSED-REPO (existing) | Dockerfile already sets X-Frame-Options DENY; report saw SAMEORIGIN on live (same effect). |

## D5 — Reliability / ops / secrets
| Finding | Class | Status | Note |
|---|---|---|---|
| H8 orphaned git blobs retain rotated JWT/PII/RSA | secret-hygiene | **CLOSED (local + GitHub)** | **2026-07-13: local purge removed the 2 RSA private-key blobs; `git fsck --unreachable` → 0. Reachable history proven clean. GitHub GC VERIFIED NOT NEEDED — `gh api /git/blobs/{sha}` → HTTP 404 for both RSA SHAs (never pushed). See H8-GITHUB-GC-REQUEST.md. Repo stays green (`pnpm verify:secrets`).** |
| `verify-secrets` only checked filenames (D5 blind spot) | gate-gap | CLOSED-REPO | verify-secrets hardened (step 4 enumerates secret filenames added to any ref; gitleaks robust against fork binary). Gate GREEN. |

## D6 — Business value
| Finding | Class | Status | Note |
|---|---|---|---|
| Strategy / ROI gaps | biz | OOS-D1 | Out of scope (MANIFESTO §6 declares business plan out of scope). |

## D7 — Design / UX / a11y (live SPA + deploy)
| Finding | Class | Status | Note |
|---|---|---|---|
| F-01 prod has no landing page (root → bare wizard) | ux/deploy-drift | OPERATOR | Live deploy divergence (staging has landing, prod doesn't). Repo: landing exists in source; deploy routing is operator/CD action. |
| F-02 login dead-end + brand mismatch (DeliveryOS vs dowiz) | ux/brand | OPERATOR | Live deploy + copy; not a security gap. |
| F-03 analytics money figures contradict | ux/data-integrity | OPERATOR | Live fixture/deploy; violates VERIFIED-BY-MATH but is deploy-state, not repo bug. |
| F-04 e2e fixture garbage customer-visible | test-leak | TODO-DEFER | Add guardrail asserting no UI-/E2E- names in public menu payload (when e2e suite + fixtures land in new arch). |
| F-05/F-06 checkout form a11y (no labels, EN fallback) | a11y | OPERATOR/TODO | Live SPA copy/labels; mostly deploy-state. Repo: ensure new SPA forms use associated labels. |
| F-07/F-08 owner surfaces contradict; hours form bug | ux/deploy-drift | OPERATOR | Live deploy/state. |
| Reduced-motion pass (one gap: courier marker tween) | a11y | TODO-DEFER | New-arch: ensure marker tween respects useReducedMotion (already guarded per report). |

## Summary
- **CLOSED-REPO this pass (verified):** D3-F7 (CSP on staying SPA) + D5-H8 (local secret purge) + D5-gate (verify-secrets hardened).
- **CLOSED-ADR:** F4 SSRF, F1/F2/F3 credential/PII classes (ADR-0007/0008/0009).
- **OPERATOR (not repo-fixable, runbooks provided):** D1-F1/D3-F1 live weak-cred prod (DB rotate via `PART1-LIVE-PROD-DECOMMISSION.md`), live `dowiz.fly.dev` teardown (same runbook), D7 F-01..F-08 live-deploy/UX (CD/copy). D5-H8 GitHub GC → VERIFIED NOT NEEDED (404), no Support ticket.
- **TODO-DEFER (land with new-arch components):** route-layer requireRole + RED test; live-fetcher SSRF audit + RED test; e2e-fixture guardrail; SPA form a11y labels.
- **OOS-D1 (explicitly not fixed — would be fake-fix):** all retired `attic/apps-api` + Supabase findings.
