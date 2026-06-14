# Go-Live Runbook (G4/G6)

## Prerequisites

- [ ] All migrations applied: `pnpm migrate:up`
- [ ] `pnpm verify:launch` adapted and passing (see G7)
- [ ] Owner account created (email+password, non-Google path)
- [ ] First location configured with menu, pricing, branding
- [ ] Fallback phone configured
- [ ] Courier account created + invite code generated
- [ ] Test order completed end-to-end (owner→courier→delivery→settlement)
- [ ] UptimeRobot monitors active (4 monitors: `/health`, `/s/demo`, `/`, SSL)
- [ ] TLS cert valid, slug.dowiz.org resolves
- [ ] Sentry DSN configured, confirming no PII leaks
- [ ] R2 backup verified + restore-test green
- [ ] Free-tier monitoring worker running and reporting
- [ ] Keep-alive check active (health endpoint ping every 5min)

## Launch Sequence

### T-1 Hour: Final checks

```bash
# Verify all automated gates
pnpm verify:launch

# Verify RLS
pnpm verify:rls

# Verify privacy (no PII leaks)
pnpm verify:privacy

# Verify DB schema
pnpm verify:db

# Verify secrets
pnpm verify:secrets

# Quick backup verify
pnpm backup:verify
```

### T-30 Minutes: Owner onboarding

1. Share login URL with owner (email+password)
2. Owner signs in successfully
3. Owner views dashboard, menu, orders list
4. Owner confirms settings (fallback phone, notifications, branding)
5. Courier opens app → activates via invite code

### T-0: Launch

1. Flip DNS → production (if not already)
2. Owner places a **test order** as if they were a customer
3. Courier receives → accepts → delivers
4. Confirm alert received on owner dashboard
5. Confirm settlement generated correctly

### Post-Launch: First real order

1. External customer visits `slug.dowiz.org`
2. Customer views menu → adds items → checks out (PIN only — 77% cash)
3. Order appears on owner dashboard + courier queue
4. Courier delivers → marks delivered
5. Order status updated → customer notified (if push opt-in)
6. **Timestamp recorded in launch-journal.md**

## Success Metric

Launch is successful when:

1. `pnpm verify:launch` passes (all automated gates green)
2. Owner can log in via non-Google path
3. First real paid order completes full cycle:
   - `created → confirmed → preparing → in_delivery → delivered`
4. Owner alert received (Telegram or dashboard notification)
5. Health endpoint returns `healthy`
6. No 5xx errors in Sentry
7. Free-tier metrics below 80% threshold

## Post-Launch Elevated Watch (First 48h)

| Check | Interval | Action if fail |
|-------|----------|---------------|
| Health endpoint | Every 5 min (UptimeRobot) | Pager owner |
| Sentry error rate | Real-time | Triage immediately |
| Worker liveness | Every 60s | Pager on stale >3 critical workers |
| Free-tier metrics | Every 1h | Alert >80%, escalate >95% |
| Backup completion | Every 4h | Alert on miss |
| Order dwell | Every 5 min | Escalate >15min |
| Fallback phone | Daily | Test call to verify |

## Decision Authority

| Decision | Authority |
|----------|-----------|
| Rollback | On-call engineer (after criteria met) |
| Feature disable | On-call engineer |
| Scale up tier | Owner + engineer |
| Continue despite warning | Owner only |
