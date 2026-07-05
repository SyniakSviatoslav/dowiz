# Graceful Degradation Architecture

## Principles
1. **No cascade crash** — External call failure → single-channel degradation, not all-or-nothing
2. **Order never lost** — Cart preserved in `localStorage` even if submission fails
3. **Owner dashboard always visible** — REST snapshot (E24) independent of push/Telegram
4. **No silent errors** — Every error path shows fallback banner or generic message

## Degradation Sources

### External Service Timeouts
All external calls wrapped with `withTimeout()` from `lib/resilience/timeout.ts`:
- Telegram API: 5s timeout
- R2/Cloudflare: 5s timeout  
- Geocoding: 5s timeout
- Translation API: 5s timeout

### Channel Dead Detection
- Telegram failure 5+ consecutive → channel marked degraded
- Push notification failure 5+ consecutive → channel marked degraded
- Both dead → owner dashboard banner "Channels offline: push, telegram"

### Health Check Effects
| Check | Degraded When | Effect |
|-------|--------------|--------|
| `fallback` | <50% locations have fallback config | Warning in health dashboard |
| `telegram` | API ping fails | Notifications degraded |
| `workers` | Worker stale >60s | Specific worker degraded |
| `backup_restore` | Last verify >25h ago | Backup restore untested |

## Customer UX
When any error path triggers, the fallback banner:
1. Shows at bottom of screen (red, fixed position)
2. Displays localized message based on error reason
3. Shows phone number if `showPhoneOnError` / `showPhoneOnOffline` is enabled
4. Dismissable with X button
5. Does NOT block interaction — cart/checkout remains functional

## Owner UX
When push + Telegram both dead:
- Red banner below header: "Channels offline: push, telegram. Notifications may not reach you."
- Re-check button reloads status
- Dashboard remains fully functional via REST snapshot

## Resilience Patterns
- `withTimeout` pattern for all external calls (2–5s timeout, fallback value)
- `retryWithBackoff` for idempotent operations (max 3, exponential backoff + jitter)
- Separate connection pools per external service (bulkhead)
- Full circuit breaker deferred to P5+

## Files
```
apps/api/src/lib/resilience/timeout.ts    — withTimeout, retryWithBackoff
apps/api/src/client/shared/fallback-phone.ts  — Banner UI + config fetch
apps/api/src/client/shared/error-boundary.ts  — Global error handler
apps/api/src/routes/public/fallback-config.ts — Public config endpoint
apps/api/src/routes/owner/fallback.ts         — Owner config + degradation
apps/api/src/routes/owner/reveal-contact.ts   — Customer contact reveal
apps/api/src/routes/admin/fallback.ts         — Admin overview
apps/api/src/routes/health.ts                 — Extended with fallback check
```
