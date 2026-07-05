# RELEASE-GATE-RUN

Release: v1.0.0-deploy-final
Target: https://dowiz.fly.dev
Duration: 5163ms
Verdict: FAIL

| Blocker | Status | Duration | Detail |
|---------|--------|----------|--------|
| B1 Health & version | PASS | 332ms | All 5 critical subsystems ok |
| B2 Migrations applied | PASS | 167ms | Server running — migrations applied (expected at least 1790000000010) |
| B4 Config & secrets valid | PASS | 102ms | Telegram token + R2 both valid |
| B6 RLS / tenant isolation | PASS | 572ms | Cross-tenant query returns 404 (expected) |
| B3 Workers on session connection | PASS | 200ms | 5 workers registered, 0 stale |
| B5 End-to-end order → confirm → Telegram audit | FAIL | 3787ms | Audit shows 0 entries for order — notification pipeline may be broken |

## Action
🔴 Rollback — deploy previous image