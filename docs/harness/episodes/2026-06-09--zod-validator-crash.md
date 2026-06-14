# Episode: 2026-06-09--zod-validator-crash

- **model**: deepseek-v4-flash-free / opencode
- **task**: Fix fastify-type-provider-zod v6 crash (Zod v3/v4 incompatibility on POST endpoints)
- **actions**:
  1. User reported UI edit/menu pages crash on save
  2. Traced to 500 on all POST/PUT/PATCH with body schemas
  3. Found `fastify-type-provider-zod@6.1.0` requires Zod ≥4.1.x but project uses Zod 3
  4. Replaced Zod validator/serializer compilers with custom Zod v3-safe `safeParse()` implementation
  5. Applied in server.ts
- **diffs**: 1 file (server.ts — validator/serializer compiler section)
- **gate_results**: POST endpoints return 200, health green
- **interventions**: none
- **diagnose**: systemic — type-provider version mismatch with locked Zod version
- **health**: 4 tool calls, 1 edit, 1 deploy
- **verdict**: passed
