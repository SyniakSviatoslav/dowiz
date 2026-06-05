# Scaling Gate (G8)

## Current Status

DeliveryOS is live with **1 pilot location** on:

- **Supabase Free tier** — 500 MB DB, 1 GB storage, 2 GB egress, no PITR
- **OAuth unverified** — Google login limited to 100 test users, unverified-app warning
- **Backup-only recovery** — No PITR, R2 logical backups only, RPO = 4h

**This is acceptable for a single location.** It is NOT acceptable for multi-location.

## Gate Requirements: Single → Multi-Location

Before onboarding a second location, ALL of these must be satisfied:

### 1. Supabase Pro Tier ✅

| Requirement | Reason |
|-------------|--------|
| **PITR enabled** (7-day) | RPO < 5min, recovery from any point in time |
| **DB size ≥ 8 GB** | Headroom for 10+ locations |
| **Storage ≥ 100 GB** | Menu photos, branding assets |
| **Egress ≥ 250 GB/month** | Customer menu views, API calls |
| **No auto-pause** | Production reliability |
| **Priority support** | Critical issue SLA |

**Migration from Free to Pro:**
```bash
# 1. Upgrade in Supabase dashboard (non-destructive)
# 2. Update DATABASE_URL if pooler connection changes
# 3. Run pnpm migrate:up (if any Pro-only migrations)
# 4. Verify: pnpm verify:launch
```

### 2. OAuth Verified ✅

| Requirement | Reason |
|-------------|--------|
| **Google OAuth consent screen verified** | All owners can log in without warning |
| **No test-user limit** | Unlimited owner accounts |
| **Sensitive scopes approved** | If needed for future features |

**Verification process:**
1. Submit `dowiz.org` to Google for OAuth verification
2. Complete privacy policy + terms of service links
3. Pass Google's app review
4. Remove test-user restriction

### 3. Pilot Stable ≥ N Days ✅

| Metric | Threshold |
|--------|-----------|
| Uptime (health endpoint) | ≥ 99.9% over N days |
| Free-tier resources | ≤ 60% utilization at peak |
| Orders processed | ≥ 10 real paid orders |
| Support requests | ≤ 3 per week |
| Rollbacks | 0 unplanned |
| Sentry error rate | ≤ 0.1% of requests |

**N = 14 days** minimum (30 recommended).

### 4. Documentation Current ✅

- [ ] CONVENTIONS.md lists all G1-G9 red lines
- [ ] DR runbook updated with Free tier specifics
- [ ] Owner auth launch doc finalized
- [ ] Free-tier ops guide current
- [ ] Rollback playbook rehearsed
- [ ] Launch journal populated
- [ ] verify:launch adapted for current tier

## How to Check Gate Status

```bash
# Run adapted launch verification
pnpm verify:launch

# Check this returns specific info about each gate
# Gate 1: Supabase tier — if Free, shows "Free tier OK for pilot"
# Gate 11: Scaling gate — shows "Not met: require Pro + OAuth verified + stability"
```

## If Gate Not Met

| Scenario | Action |
|----------|--------|
| Owner requests second location | Explain scaling gate requirements |
| Business needs rapid scaling | Fast-track Pro upgrade + OAuth verification + reduce stability period to 7 days |
| Gate partially met | List which requirements remain, timeline estimate |

## Future Scaling Considerations

Beyond the first gate (2-5 locations), additional scaling will require:

- **Connection pool sizing**: Review `BACKUP_POOL_SIZE`, `max` operational pool
- **Rate-limit tuning**: Adjust per-tenant limits based on observed traffic
- **Database indexing**: Review query performance for multi-tenant patterns
- **Caching strategy**: Cloudflare cache hit ratio, menu version invalidation
- **Worker throughput**: pg-boss queue depth, worker concurrency
- **Storage architecture**: R2 for asset serving, CDN for menu photos

Documented in `docs/scaling-beyond-pilot.md` when gate is met.
