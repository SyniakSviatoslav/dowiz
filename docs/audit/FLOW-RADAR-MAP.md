# FLOW-RADAR-MAP.md — DeliveryOS Staging Radar

> Generated: 2026-06-12 · Target: `dowiz.fly.dev` · Status: UNKNOWN (all flows untested)

## Staging Info
- **URL:** https://dowiz.fly.dev
- **Health:** `/health` — postgres/ok, telegram/ok, workers/ok, backup_restore/degraded, fallback/degraded
- **Telegram bot:** `dowizbot_bot` (id: 8996764379)
- **Test accounts:** test@dowiz.com / test123456 (owner, no orgs), owner-p36-...@test.com (P36 Org owner)

---

## Flow Inventory

| ID | Domain | Trigger | Status | Notes |
|----|--------|---------|--------|-------|
| `order-create` | Orders | POST /orders | UNKNOWN | |
| `order-confirm` | Orders | POST confirm / PATCH status / Telegram callback | UNKNOWN | |
| `order-reject` | Orders | POST reject / PATCH status / Telegram callback | UNKNOWN | |
| `order-status-transition` | Orders | updateOrderStatus() | UNKNOWN | |
| `order-timeout-cancel` | Orders | order.timeout queue | UNKNOWN | delay 15-30min |
| `order-cancel-customer` | Orders | POST /orders/:id/cancel | UNKNOWN | 5min window |
| `order-message-send` | Orders | POST /api/orders/:id/messages | UNKNOWN | |
| `courier-dispatch-auto` | Courier | order.confirmed → dispatch | UNKNOWN | |
| `courier-assign-manual` | Courier | POST assign-courier (owner) | UNKNOWN | |
| `courier-accept` | Courier | POST /assignments/:id/accept | UNKNOWN | |
| `courier-reject` | Courier | POST /assignments/:id/reject | UNKNOWN | |
| `courier-picked-up` | Courier | POST /assignments/:id/picked-up | UNKNOWN | |
| `courier-delivered` | Courier | POST /assignments/:id/delivered | UNKNOWN | |
| `courier-assignment-cancel` | Courier | POST /assignments/:id/cancel | UNKNOWN | |
| `courier-shift-start` | Courier | POST /me/shift/start | UNKNOWN | |
| `courier-shift-end` | Courier | POST /me/shift/end | UNKNOWN | |
| `courier-gps-ping` | Courier | POST /shifts/ping | UNKNOWN | |
| `courier-stale-check` | Courier | Cron */2 * * * * | UNKNOWN | |
| `settlement-generate` | Settlement | Cron 0 2 * * * | UNKNOWN | daily 2AM UTC |
| `settlement-approve` | Settlement | POST /:locId/settlements/:id/approve | UNKNOWN | |
| `settlement-pay` | Settlement | POST /:locId/settlements/:id/pay | UNKNOWN | |
| `settlement-dispute` | Settlement | POST /:locId/settlements/:id/dispute | UNKNOWN | |
| `dwell-monitor` | Dwell | Cron * * * * * | UNKNOWN | 1-min cycle |
| `dwell-alert-acknowledge` | Dwell | POST /:locId/alerts/:id/acknowledge | UNKNOWN | |
| `signal-compute-order-time` | Signals | Inline during POST /orders | UNKNOWN | |
| `signal-batch-raise` | Signals | Cron */5 * * * * | UNKNOWN | |
| `signal-acknowledge` | Signals | POST /:locId/signals/:id/acknowledge | UNKNOWN | |
| `signal-dismiss` | Signals | POST /:locId/signals/:id/dismiss | UNKNOWN | |
| `no-show-mark` | Signals | POST /:locId/orders/:id/mark-no-show | UNKNOWN | |
| `phone-order-throttle` | Signals | Inline during POST /orders | UNKNOWN | |
| `otp-send` | OTP | POST /locations/:slug/otp/send | UNKNOWN | |
| `otp-verify` | OTP | POST /locations/:slug/otp/verify | UNKNOWN | |
| `notify-telegram-delivery` | Notification | notify.telegram.send queue | UNKNOWN | |
| `notify-dispatch` | Notification | notify.dispatch queue | UNKNOWN | |
| `customer-push` | Notification | notify.customer_status queue | UNKNOWN | |
| `telegram-webhook` | Notification | POST /webhook/telegram/:secret | UNKNOWN | |
| `auth-local-login` | Auth | POST /auth/local/login | UNKNOWN | |
| `auth-google-oauth` | Auth | GET /auth/google | UNKNOWN | |
| `auth-refresh` | Auth | POST /auth/refresh | UNKNOWN | |
| `auth-courier-activate` | Auth | POST /auth/courier/activate | UNKNOWN | |
| `ws-subscribe` | Realtime | WS auth + subscribe | UNKNOWN | |
| `ws-client-location` | Realtime | WS client_location msg | UNKNOWN | |
| `ws-ping-heartbeat` | Realtime | WS ping (implicit) | UNKNOWN | |
| `backup-hourly` | Backup | Cron hourly | UNKNOWN | |
| `backup-daily` | Backup | Cron daily | UNKNOWN | |
| `backup-weekly` | Backup | Cron weekly | UNKNOWN | |
| `backup-monthly` | Backup | Cron monthly | UNKNOWN | |
| `backup-verify-restore` | Backup | Cron 0 4 * * * | UNKNOWN | |
| `backup-verify-r2` | Backup | Cron 0 */6 * * * | UNKNOWN | |
| `anonymizer-retention` | Anonymizer | Cron 0 3 * * * | UNKNOWN | |
| `anonymizer-gdpr` | Anonymizer | POST /:locId/gdpr-requests | UNKNOWN | |
| `free-tier-watch` | Ops | Cron | UNKNOWN | |
| `worker-liveness` | Ops | Cron */60 * * * * * (60s) | UNKNOWN | |
| `order-rls-isolation` | Security | Cross-tenant query | UNKNOWN | |
| `courier-rls-isolation` | Security | Cross-tenant query | UNKNOWN | |
| `fallback-degradation` | Resilience | PUT /:locId/settings/fallback | UNKNOWN | |
| `telegram-callback-confirm` | Telegram | Inline keyboard callback_data | UNKNOWN | |
| `telegram-callback-reject` | Telegram | Inline keyboard callback_data | UNKNOWN | |
| `telegram-cmd-start` | Telegram | /start command | UNKNOWN | |
| `telegram-cmd-stop` | Telegram | /stop command | UNKNOWN | |
| `telegram-cmd-open` | Telegram | /open command | UNKNOWN | |
| `telegram-cmd-close` | Telegram | /close command | UNKNOWN | |

---

## Harness

### Auth
```bash
AUTH=$(curl -s -X POST https://dowiz.fly.dev/auth/local/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@dowiz.com","password":"test123456"}')
TOKEN=$(echo $AUTH | jq -r '.access_token')
```

### Observability queries
```sql
-- Check queue jobs
SELECT name, state, count(*) FROM pgboss.job GROUP BY name, state;

-- Check audit trail
SELECT * FROM notification_outbox_audit ORDER BY created_at DESC LIMIT 20;

-- Check alerts
SELECT * FROM location_alerts ORDER BY created_at DESC LIMIT 10;
```
