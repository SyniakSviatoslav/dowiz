# Episode: 2026-06-09--subdomain-asset-404

- **model**: deepseek-v4-flash-free / opencode
- **task**: Fix tenant subdomain (demo-location.dowiz.org) static assets returning 404
- **actions**:
  1. Reproduced asset 404 on subdomain (200 on main domain)
  2. Traced to subdomain middleware rewriting ALL non-API paths to `/s/:slug`
  3. Added file-extension exclusion + `/s/` prefix exclusion to condition
  4. Deployed to Fly.io
  5. Verified assets return 200 on subdomain
- **diffs**: 1 file, +1/-1 lines (apps/api/src/server.ts:198)
- **gate_results**: assets return 200, health endpoint green
- **interventions**: none
- **diagnose**: systemic — middleware lacked exclusion for static file paths
- **health**: 5 tool calls, 1 edit, 1 deploy
- **verdict**: passed
