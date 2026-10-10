# RELEASE-GATE.md — Post-deploy smoke gate

> Design: 2026-06-12 · Target platform: the legacy host (single-instance, blue-green via a deploy)
> Budget: ≤90s total · Verdict: PASS → promote / FAIL → rollback / INCONCLUSIVE → block

---

## Platform context

| Aspect | Detail |
|---|---|
| **Platform** | the legacy host — single deploy replaces the running instance |
| **Traffic switch** | deploy (no explicit canary — new instance replaces old when healthy) |
| **Rollback** | redeploy `<previous-image>` |
| **Health endpoint** | `GET /health` — returns JSON with per-component status |
| **Migrations** | `pnpm migrate:up` runs at startup (`node-pg-migrate` in `server.ts` bootstrap) |
| **Test tenant** | Demo location (`1f609add-062a-4bb5-89bf-d695f963ede6`), account `test@dowiz.com` |
| **Notification proof** | `notification_outbox_audit` table — written by dispatch/telegram workers |

---

## Blocker definitions

### B1 · Health & version
| | |
|---|---|
| **What it checks** | Service responds to `GET /health` with 200; all critical subsystems report `ok` |
| **Sub-checks** | `postgres.status=ok`, `messageBus.status=ok`, `workers.status=ok`, `telegram.status=ok`, `r2.status=ok` |
| **Budget** | ≤10s — single HTTP request |
| **Fail condition** | Any critical subsystem non-ok (excluding backup_restore/fallback which are pre-existing known degradations) |
| **Rollback** | redeploy previous image |

### B2 · Migrations
| | |
|---|---|
| **What it checks** | `pgmigrations` table contains the latest migration (`1790000000010_audit-drop-reasons`) |
| **How** | Query `SELECT name FROM pgmigrations ORDER BY name DESC LIMIT 1` via health endpoint `data` field, or direct HTTP probe |
| **Budget** | ≤10s |
| **Fail condition** | Expected migration not applied (schema drift) |
| **Rollback** | Run `pnpm migrate:down` or redeploy previous image |

### B3 · Workers on session connection
| | |
|---|---|
| **What it checks** | pg-boss worker can accept and process a **synthetic probe job** on session connection; MessageBus listener shows `connected` |
| **Sub-checks** | (a) Send `notify.telegram.send` probe job → wait for `completed` state in `pgboss.job`; (b) Health endpoint shows `messageBus.status=ok` |
| **Budget** | ≤30s — send + poll for completion (uses connected workers, not a new pool) |
| **Fail condition** | Probe job not processed within budget → pooler mismatch (jobs enqueued but never consumed) |
| **Rollback** | redeploy previous image |

### B4 · Config & secrets
| | |
|---|---|
| **What it checks** | Mandatory env vars present; Telegram bot token responds to `getMe`; R2 bucket reachable |
| **How** | (a) Health endpoint shows `telegram.status=ok` (confirms token works); (b) Health endpoint shows `r2.status=ok` |
| **Budget** | ≤10s — already covered by health check |
| **Fail condition** | Telegram or R2 check non-ok |
| **Rollback** | redeploy previous image + fix secrets |

### B5 · Critical end-to-end flow
| | |
|---|---|
| **What it checks** | Synthetic order → confirm → Telegram notification delivered (proven via audit) |
| **Steps** | (1) Place test order via `POST /api/orders`; (2) Confirm via `POST .../orders/:id/confirm`; (3) Wait for `notification_outbox_audit` entry with `event=order.confirmed status=delivered`; (4) Verify delivery to test target |
| **Budget** | ≤30s — order creation + confirm + poll audit |
| **Fail condition** | Audit entry not created, or status not `delivered` within budget |
| **Rollback** | redeploy previous image |

### B6 · RLS / tenant isolation
| | |
|---|---|
| **What it checks** | Cross-tenant query with wrong `location_id` returns 0 rows (or 404, never 200+data) |
| **How** | Query with randomly generated UUID as location_id against `/api/owner/locations/:random/...` — expect 404 |
| **Budget** | ≤10s |
| **Fail condition** | Returns 200 or leaks data for non-existent location |
| **Rollback** | redeploy previous image |

### B7 · Public menu has content
| | |
|---|---|
| **What it checks** | Menu API returns non-empty categories and products — what the client browser sees |
| **How** | Fetch `GET /public/locations/:locationId/menu` — verify ≥1 category with ≥1 product |
| **Budget** | ≤10s |
| **Fail condition** | 0 categories or 0 products returned (empty menu = zero orders) |
| **Rollback** | redeploy previous image |

### B8 · Critical assets load
| | |
|---|---|
| **What it checks** | SPA route returns 200 and all referenced JS/CSS assets are fetchable |
| **How** | Fetch SPA route `/s/:slug` HTML → extract `<script src>` and `<link href>` → verify every referenced asset returns 200 |
| **Budget** | ≤15s |
| **Fail condition** | Any critical asset returns 4xx/5xx or fails to load |
| **Rollback** | redeploy previous image |

---

## Gate runner

Script at `apps/api/scripts/release-gate.ts` — run as `npx tsx apps/api/scripts/release-gate.ts <release-version>`.

### Flow
```
B1 Health ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B2 Migrations ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B4 Config/Secrets ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B6 RLS isolation ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B3 Worker probe ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B5 End-to-end flow ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B7 Menu content ──FAIL──→ ROLLBACK
  │ PASS
  ▼
B8 Assets load ──FAIL──→ ROLLBACK
  │ PASS
  ▼
PASS → promote
```

### Output
`RELEASE-GATE-RUN.md` — per-blocker result + overall verdict + rollback action taken.

### Rollback
```bash
# Redeploy the previous release (Cloudflare Worker version; see workers/api)
```

---

## What is NOT a blocker (caught by radars, not gate)
- Non-critical UI surfaces (courier login, branding preview)
- Analytics charts rendering
- Courier shift/settlement flows
- Backup restore verification
- Fallback phone coverage
