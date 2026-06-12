# FLOW-RADAR-REPORT.md — Full Issue Matrix

> Generated: 2026-06-12 · Target: `dowiz.fly.dev` (staging)
> Method: live API probes + static analysis (orphans, event wiring, connection lifecycle)
> Coverage: 29 HTTP endpoints, 5 complete order lifecycle flows, 11 health checks, 3 static verify scripts

---

## Executive Summary

| Metric | Count |
|---|---|
| **Flows tested** | 35 |
| **OK** | 29 |
| **🔴 Divergences** | **4** |
| **⚪ Blocked** | **2** |
| **Severity mix** | 🔴 1 · 🟠 2 · 🟡 2 · ⚪ 1 |

---

## 🔴 Critical Issues

### 1. Settlements endpoint returns 500
| | |
|---|---|
| **Flow** | `settlement-list` |
| **Endpoint** | `GET /api/owner/locations/:locId/settlements` |
| **Expected** | 200 with settlement list (or empty array) |
| **Actual** | 500 `{"code":500,"error":"Internal server error"}` |
| **Evidence** | `GET /api/owner/locations/1f609add-062a-4bb5-89bf-d695f963ede6/settlements` |
| **Severity** | 🟠 — blocks owner from viewing settlement history |
| **Hypothesis** | `SettlementCronWorker` generates payouts daily at 2AM. If no settlements exist yet for this location (or period boundaries fail), the query may throw. Check `getSettlementPeriodBoundaries()` or missing period record. |

### 2. Backup restore test degraded
| | |
|---|---|
| **Flow** | `backup-verify-restore` |
| **Endpoint** | `/health` → `backup_restore.status = "degraded"` |
| **Expected** | Restore test passes (run nightly at 4AM) |
| **Actual** | `status="degraded"`, `last_result="failed"`, `stale=true` |
| **Evidence** | Health check response |
| **Severity** | 🟠 — R2 backup exists but restore path is not verified |
| **Hypothesis** | `BackupVerifyWorker` runs at 4AM. If the worker or R2 credentials have an issue, restore verification fails. Check worker logs for the last error. |

### 3. Fallback phone coverage at 0%
| | |
|---|---|
| **Flow** | `fallback-degradation` |
| **Endpoint** | `/health` → `fallback.detail = "0/150 locations have fallback phone configured (0%)"` |
| **Expected** | >0 locations have fallback phone configured for SMS/phone alerts |
| **Actual** | 0/150 (0%) |
| **Evidence** | Health check response |
| **Severity** | 🟡 — no automated phone fallback for any location |
| **Hypothesis** | Fallback config endpoint exists (`PUT /:locId/settings/fallback`) but never used. Either owners don't know about it or the UI doesn't surface it. |

### 4. Customer push subscription endpoint returns 400 for owner role
| | |
|---|---|
| **Flow** | `customer-push` |
| **Endpoint** | `POST /api/customer/push/subscribe` |
| **Expected** | 401/403 for non-customer role |
| **Actual** | 400 with owner token (validation error, not auth error) |
| **Evidence** | `POST /api/customer/push/subscribe` with owner JWT |
| **Severity** | 🟡 — wrong error code (400 instead of 401/403). Low impact since customer routes properly require customer role. |
| **Hypothesis** | The route validates the request body before checking auth. Validation fails first because the body uses different field names than expected. |

---

## ⚪ Blocked (could not observe)

### 5. Notification audit endpoint
| | |
|---|---|
| **Flow** | `notify-dispatch-audit` |
| **Endpoint** | No public/api route for `notification_outbox_audit` |
| **Blocked reason** | The audit table exists but has no HTTP endpoint to query it. `notification_outbox_audit` can only be queried via direct DB access. |
| **Severity** | ⚪ — infrastructure observation gap |

### 6. Telegram message delivery verification
| | |
|---|---|
| **Flow** | `notify-telegram-delivery` |
| **Testing approach** | Requires Telegram bot API access + known test chat |
| **Blocked reason** | Telegram bot token and test chat ID not available in current environment |
| **Severity** | ⚪ — cannot verify external notification delivery without credentials |

---

## ✅ Passed (29 flows)

| Category | Flows |
|---|---|
| **Health** | postgres, workers, messageBus, telegram, r2, settlement, anonymizer, backup, free_tier |
| **Public** | menu (200, 88 categories, version 579), info (200), theme (200) |
| **Auth** | local-login (200, role=owner), mock-auth (200) |
| **Order** | create (201, PENDING, 1400 ALL), confirm (200), reject (200), status-transitions (3x200), verify-shape (id+status+PENDING+total>0) |
| **Owner endpoints** | settings (200), orders (200, 50 items), categories (200), couriers (200), brand (200), analytics (200), dashboard-snapshot (200), alerts (200), signals (200), dwell-settings (200), fallback-settings (200), retention-settings (200), notification-targets (200), push-state (200), couriers-live (200), courier-invites (200) |
| **Role isolation** | courier endpoints return 403 for owner token (3/3) |
| **Admin** | backups (200), dr-report (200), fallback-health (200) |

---

## Static Analysis Results

| Script | Verdict | Issues |
|---|---|---|
| `verify-orphans.ts` | ✅ Pass | Zero raw string leaks. 3 info-level dead workers (anonymizer.retention, velocity.flush, free_tier.watch) |
| `verify-event-wiring.ts` | ✅ Pass | All 20 event types fully wired in all 4 required locations |
| `verify-connection-lifecycle.ts` | ⚠️ 2 flagged | `server.ts:237` (messageBus.connect() — intentional) and `client/status/ws.ts:69` (class-level reconnect) |

---

## Cluster by Root Cause

| Cluster | Issues | Shared cause | Estimated fix effort |
|---|---|---|---|
| **Settlement engine** | #1 (500 error) | Settlement period boundaries or missing configuration for locations created before settlement cron was deployed | 1-2h: investigate the SQL error, add null-safety |
| **Backup infrastructure** | #2 (restore degraded) | R2 restore verify worker failure. Could be IAM permissions, bucket naming, or last successful backup timing. | 2-4h: debug worker logs, test manual restore |
| **Owners not configuring** | #3 (fallback 0%) | UI gap or onboarding flow doesn't prompt for fallback phone. Feature exists but is invisible. | 1h: add to onboarding step or dashboard readiness checklist |
| **Auth error codes** | #4 (400 vs 403) | Body validation runs before auth check. Reorder hooks or change to accept empty body until auth verified. | 30m: reorder preValidation hooks |
| **Observability** | #5-#6 (audit+Telegram) | No HTTP surface for audit table; no test chat integrated into radar harness. | 2h: expose read-only audit endpoint; add Telegram test harness |

---

## Backlog (ordered severity → effort)

1. **🔴 Fix settlements 500** — investigate SQL error, add try/catch with empty array fallback
2. **🟠 Debug backup restore** — check `BackupVerifyWorker` logs, test R2 access
3. **🟡 Add fallback phone prompt** — add to onboarding step 2 or readiness checklist
4. **🟡 Fix auth error code** — reorder `preValidation` hooks in customer push route
5. **⚪ Expose audit HTTP endpoint** — read-only filtered by location_id + event
6. **⚪ Add Telegram test harness** — wire test chat into radar for notification delivery verification

---

## Safety Confirmation

- ✅ Staging only (`dowiz.fly.dev`)
- ✅ Test accounts only (`test@dowiz.com`, mock-auth)
- ✅ 0 real customer data accessed
- ✅ All orders created during radar are test orders with test phone numbers
- ✅ No destructive operations performed
- ✅ Teardown: test orders remain in staging DB (expected for audit)
