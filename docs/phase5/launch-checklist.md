# Pre-Launch Checklist (P5-5)

Run: `pnpm verify:launch`

## Automated Gates (`pnpm verify:launch`)

### Infrastructure
- [ ] Supabase tier active (Free accepted for pilot, Pro required for scaling)
- [ ] PITR not available (Free tier). R2/logical backup is sole recovery net.
- [ ] TLS certificate valid (not expired, not self-signed)
- [ ] Wildcard DNS `*.dowiz.org` resolves
- [ ] HTTPS enforced (no plain HTTP)

### Data Protection
- [ ] Restore-test passed within last 48 hours
- [ ] Anonymizer retention job registered in pg-boss
- [ ] Fallback phone configured for initial locations
- [ ] Backup R2 lifecycle ≤ DB retention period
- [ ] Free-tier monitoring active (free_tier_snapshots table, hourly watch)

### Observability
- [ ] Sentry DSN configured
- [ ] UptimeRobot monitors active: `/health`, `/s/demo`, `/`, SSL
- [ ] Worker liveness checker active
- [ ] Health endpoint returns 200 (including free_tier check)

### Code Quality
- [ ] All migrations applied (`pnpm migrate:up`)
- [ ] RLS verification passed (`pnpm verify:rls`)
- [ ] All Stage 30-35 tests pass (`pnpm test:phase5`)
- [ ] DB schema verified (`pnpm verify:db`)
- [ ] No secrets in repo (`pnpm verify:secrets`)
- [ ] Stage 35 tests pass (`pnpm test:phase5-step5`)

### Environment
- [ ] Env parity: all `.env.example` keys present in production `.env`
- [ ] `NODE_ENV=production` in production
- [ ] `JWT_KID` matches active signing key
- [ ] Rate-limit env defaults configured

### Go-Live Specific (P5-5)
- [ ] Owner non-Google login works (email+password or test-user)
- [ ] Free-tier metrics below 80% threshold
- [ ] Keep-alive health pings active (prevents auto-pause)
- [ ] Scaling gate documented
- [ ] Rollback playbook documented + rehearsed

## Manual Gates

### Security & Compliance
- [ ] Google OAuth verification for `dowiz.org` — DEFERRED (info gate for pilot)
- [ ] CSP nonce pattern verified on all pages
- [ ] CORS restricted to menu GET + order POST only
- [ ] Upload MIME validation active

### Operations
- [ ] Rollback deploys rehearsed: forward-only → roll-forward via `git revert`
- [ ] `pnpm backup:drill --full` run successfully
- [ ] DR runbook reviewed: DB corrupt / R2 loss / region down / auto-pause scenarios
- [ ] Incident comms templates (SQ + EN) ready

### Go-Live Readiness
- [ ] First location selected for launch
- [ ] Owner account created + on-boarded (via non-Google path)
- [ ] Menu loaded + verified
- [ ] Test order placed end-to-end
- [ ] Courier account created + shift started

## Launch Sequence
1. Run `pnpm verify:launch` — all automated gates pass (info gates noted)
2. Review manual gates — all checked
3. Flip DNS to production
4. Monitor health endpoint + Sentry for 15 minutes
5. Owner signs in (non-Google path) → places test order
6. Courier receives → delivers
7. Confirm payment + settlement
8. Record first real order in launch-journal.md
9. Declare launch successful
10. If issues → roll-forward via `git revert` + redeploy

## Rollback Plan
- **DB**: Forward-only migrations → roll-forward by reverting code, not DB
- **DNS**: Keep old IP available for instant DNS revert
- **Deploy**: Previous container image tagged and ready for immediate redeploy
- **Criteria**: 5xx spike >1%, lost orders, free-limit breach >95%, RLS regression, auth broken
