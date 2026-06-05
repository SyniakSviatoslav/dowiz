# Rollback Playbook (G5)

## Principle: Forward-Only Roll-Forward

**No down-migration in production.** Database migrations are forward-only. Rollback means:

1. **Code roll-forward**: Revert the application code (git revert) and redeploy the previous image
2. **Database stays forward**: Migrations already applied remain applied. The old code must be compatible with the current schema
3. **Data is preserved**: No data is lost in a roll-forward scenario

## Rollback Triggers

| Trigger | Criteria | Action |
|---------|----------|--------|
| **5xx spike** | >1% error rate over 5min, sustained | Initiate rollback |
| **Lost orders** | Orders disappear or fail to persist | Initiate rollback |
| **free-limit breach** | DB size >95% or storage >95% or connections exhausted | Initiate rollback (disable features) |
| **RLS regression** | Cross-tenant data leak detected | Emergency rollback |
| **Auth broken** | Owner or courier cannot authenticate | Initiate rollback |
| **Payment integrity** | Money total mismatch or duplicate charges | Initiate rollback |
| **Owners request it** | Explicit request from location owner | As agreed |

## Rollback Procedure

### Phase 1: Assess (2 min)

```bash
# 1. Check health
curl https://api.dowiz.org/health | jq .

# 2. Check Sentry for error rate
# 3. Check recent orders
curl https://api.dowiz.org/api/owner/locations/<id>/orders?limit=10

# 4. Decide: rollback or fix-forward?
#    - Rollback if: data integrity, security, auth, >1% 5xx
#    - Fix-forward if: minor UI bug, non-critical performance
```

### Phase 2: Execute Rollback (5 min)

```bash
# 1. Identify the last known-good commit
git log --oneline -20

# 2. Revert to stable commit
git revert --no-commit HEAD..<stable-sha>
git commit -m "rollback: revert to <stable-sha>"

# 3. Tag rollback point
git tag rollback-$(date +%Y%m%d-%H%M)

# 4. Deploy
#    Fly.io: flyctl deploy --image <previous-image-tag>
#    Or: git push production main (if auto-deploy)

# 5. Verify rollback
curl https://api.dowiz.org/health | jq .status
# Should return "healthy"
```

### Phase 3: Verify (3 min)

- [ ] Health endpoint → 200 healthy
- [ ] Owner can log in
- [ ] Customer menu loads at slug.dowiz.org
- [ ] Place test order → full cycle
- [ ] Sentry error rate back to baseline
- [ ] Free-tier metrics stable

### Phase 4: Communicate

- Notify owner via fallback phone
- Post-mortem scheduled within 24h
- Document root cause in launch-journal.md

## Communication Template

```
🛑 Rollback initiated at <timestamp>
Trigger: <trigger criteria>
Reverted to: <commit SHA>
Status: <in-progress / complete>
ETA for fix-forward: <hours>

Next update in: 30 min
```

## Preventing Rollback

| Risk | Prevention |
|------|-----------|
| Schema incompatibility | Test migration against production DB clone before deploy |
| Bad config | `.env` diff check in CI, validate all env vars at startup |
| Latent bug | Canary deploy to staging, smoke tests, spike tests |
| Free-tier limit | Monitor at 80%, upgrade before breach |
| OAuth failure | Non-Google fallback path always available |

## Post-Rollback

1. Root cause analysis within 24h
2. Fix identified → tested in staging
3. Deploy fix as forward-only (new migration if needed)
4. Verify fix in staging
5. Deploy to production (same forward-only process)
6. Add regression test to prevent recurrence

## Emergency Contacts

| Role | Contact | Backup |
|------|---------|--------|
| On-call engineer | (phone) | (phone) |
| Infrastructure | (phone) | (email) |
| Owner (pilot) | Fallback phone | Telegram |
