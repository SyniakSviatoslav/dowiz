# Agent Invariants — index of GATED red-lines (ADR-0012, Area C2)

> **Authority model (B11):** the **executable guardrail is authoritative** — this file only
> **links** to it. An invariant appears here **only if a gate enforces it**; nothing is restated
> as prose-without-a-gate. If you think a rule belongs here but has no gate, that is a request to
> **build the gate first** (via the ratchet), not to add an ungated line.
>
> **Append path (Counsel 5c):** do **not** hand-edit this file in-session to "remember" a new rule.
> A correction routes through the **librarian promotion gate** (reflection → lesson → human-reviewed
> guardrail → linked entry here). Memory is advisory; the guardrail/human is authoritative. This
> keeps the ratchet monotonic — no self-authored authority reaches the next agent ungated.
>
> Run the whole set with the individual commands below (or the relevant `pnpm verify:*`). The
> pre-commit hook + `.claude/hooks/post-edit-gates.sh` enforce the always-on subset on every edit.

| # | Invariant (agent-facing) | Authoritative gate | How it fires |
|---|---|---|---|
| 1 | **Money is integer minor-units — never `parseFloat` a price/amount/total** | `.claude/hooks/post-edit-gates.sh` → `red_lines()` (`parseFloat.*(price\|amount\|total)`) | post-edit hook, every edit; holds in spike+challenge |
| 2 | **Never bypass RLS; tenant tables are FORCE-RLS** | `pnpm verify:rls` + `packages/db/migrations/*force-rls*` | CI / manual; fails on missing FORCE for a tenant table |
| 3 | **Never edit an applied migration; forward-only, in-order** | `pnpm verify:migrations` | detects out-of-sequence + dangling migration files |
| 4 | **All realtime goes through `useWebSocket` — no `new WebSocket(...)`** | `local/no-direct-websocket` (eslint-plugin-local) | lint, every edit |
| 5 | **Zero hardcoded secrets/tokens** | `pnpm verify:secrets` + `local/no-hardcoded-string` | CI + lint |
| 6 | **Security randomness is `crypto.randomUUID`/CSPRNG — no `Math.random()` for token/otp/secret/nonce** | `local/no-insecure-random` + `.claude/hooks/post-edit-gates.sh` `red_lines()` | lint + post-edit hook |
| 7 | **JWTs are RS256 (asymmetric, kid-rotated)** | `apps/api/tests/phase5/jwt-rotation.test.ts` (`algorithms:['RS256']`) | test |
| 8 | **Owner/courier/customer route files carry an auth hook (verifyAuth + requireRole)** | `local/require-auth-hook` | lint, every edit |
| 9 | **No raw SQL string-interpolation — parameterize (`$1,$2`)** | `local/no-raw-sql` | lint |
| 10 | **No PII to logs or the LLM — redact before egress** | `pnpm verify:privacy` + `piiRedactor` (ai-ocr-parser redact-by-default, ADR-0011) | test + runtime |
| 11 | **Error responses use the one envelope; `code` is the stable BE↔FE contract** | `pnpm verify:error-contract` (ADR-0010) | static gate: FE-consumed codes must exist BE+FE; `item_unavailable` stays lowercase |
| 12 | **No `@ts-nocheck`, no `reply.status` permissive assertions, no mock-auth in prod** | `local/no-ts-nocheck` · `local/no-permissive-status-assertion` · `local/no-mock-in-prod` | lint |

**Not yet gated (do NOT treat as enforced — candidates for a future ratchet artifact):** none
currently promoted here. If a recurring bug warrants a new invariant, file a reflection → let the
librarian promote a guardrail → then it earns a row above.
