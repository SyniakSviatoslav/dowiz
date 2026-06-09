# Episode: 2026-06-09--ssr-to-spa-migration

- **model**: deepseek-v4-flash-free / opencode
- **task**: Replace old Preact SSR menu page with new React SPA at /s/:slug
- **actions**:
  1. Investigated user report of "old menu" and different language selector
  2. Found SSR route at `/s/:slug` renders Preact-based page (no images, no modifiers, old `<select>` dropdown)
  3. Found SPA fallback serves correct React app but SSR intercepts the route
  4. Replaced Preact rendering pipeline with `reply.sendFile('index.html')`
  5. Removed heavy dependencies (ssr-renderer.ts, LRU cache, PII detector) from SSR route
  6. Deployed to Fly.io
- **diffs**: 1 file, -72/+7 lines (apps/api/src/routes/public/ssr.ts)
- **gate_results**: health green, SPA serves correctly
- **interventions**: none
- **diagnose**: systemic — dual rendering pipeline with SSR shadowing SPA
- **health**: 3 tool calls, 1 edit, 1 deploy
- **verdict**: passed
