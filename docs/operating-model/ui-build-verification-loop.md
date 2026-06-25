# dowiz / DeliveryOS — UI Build-Verification Loop

> **FE/QA execution loop. Per-change / per-component, NOT at phase boundaries.** Catches small,
> visually-wrong-but-valid UI before the phase gate. The automated, granular, always-on sibling of
> the phase-level `Frontend-Audit-Polish-Gate`. Invoked by the **VERIFY** step of the
> [Task-Exit Rule](task-exit-rule.md) when a change touches `packages/ui` or `apps/web`.

## Why it exists
The agent reasons in code tokens, not pixels — it emits syntactically valid but visually wrong UI
(off-rhythm spacing, dead state, clipped element, hardcoded colour, missing focus ring). Neither code
review nor the linter sees that. The fix: give the loop **eyes** (rendered output), **constrain** what
the agent can emit (author-time rails), and **shift verification down** from the gate onto each change.

## Position in the system (no duplication)
| Layer | What | This loop's relation |
|---|---|---|
| `lint:gates` / `post-edit-gates` / `require-classification` | mechanical floor | **runs** them as the floor, never reimplements |
| Task-Exit Rule | enrich→exit→verify per change | this loop = the **UI specialisation of VERIFY** |
| **This loop** | per-component visual + states + a11y, with eyes | inline-fixes cosmetics/states/tokens; **routes** the rest |
| Convergence-Playwright loop | full E2E flows, live backend | this loop **hands off** multi-step/cross-role/live-WS bugs |
| `Frontend-Audit-Polish-Gate` | phase-wide adversarial GO/NO-GO | this loop **feeds** it (fewer defects arrive) + **escalates** systemic drift; never issues a phase verdict |
| backend/contract | server handlers/schemas | **never touched** — `MISSING`/`BLOCKED-contract`, escalate |

## The 4 layers
1. **Rails (author-time).** ESLint design rules on top of `tools/eslint-plugin-local`: `no-arbitrary-tailwind` (bans `p-[13px]`/`text-[#fff]` — use the scale/tokens) + existing `no-hardcoded-color`/`-tailwind-color`; `eslint-plugin-jsx-a11y` (present); typed component variants over free `className` passthrough.
2. **Storybook (isolation + every state).** A story per state (loading/empty/error/success) × variant × {390,768,1280} × {al,en}. State list comes from the same Task-Exit enrich. — **status: not yet installed** (needs `package.json` deps → protect-paths-blocked; see `proposed-ui-loop-infra/APPLY.md`).
3. **Deterministic visual harness (Playwright-in-Docker).** `toHaveScreenshot()` vs Storybook/pages on 3 breakpoints × 2 langs, in a pinned-Chromium Docker image, animations off, time/tz frozen, dynamic zones masked (`RelativeTime`, MapLibre, Recharts, avatars, `pickup_code`/QR). Perceptual threshold. — **status: config provided, baselines NOT committed** (no Docker here → non-deterministic; generate in CI/Docker). Iron rule: never commit non-deterministic baselines.
4. **Vision review (agent-as-eye).** Screenshot each affected state → a vision model judges it against A–F + the design spec, returns structured JSON + a route. The spec routes to a cheap OpenRouter model; **here, since OpenRouter credits are dead, the loop uses Claude subagents as the eye** — parallel agents each review one screen (proven 2026-06-24). Scope = changed components only.

## The loop (per UI-change, repeat to clean)
`FLOOR → STATES → VISUAL → VISION → DIAGNOSE+ROUTE → FIX|ROUTE → RE-VERIFY → REPEAT` until diffs/FAIL = 0.
- **FLOOR:** `scripts/ui-verify-floor.sh` (lint + lint:gates + typecheck + i18n-parity + design-drift greps). Red → fix, not "done".
- **STATES:** every affected component state renders (Storybook when present; else direct render/screenshot).
- **VISUAL:** screenshot-diff in Docker × {390,768,1280} × {al,en} (CI/operator until Docker is local).
- **VISION:** agent-as-eye A–F verdict per affected state (skeleton in Appendix).
- **DIAGNOSE+ROUTE / FIX:** safe → minimal correct inline fix (root, not selector-hack) + before/after re-shot; rest → routed with the artifact.
- **Shared-component discipline:** a recurring root → consolidate to one source in `packages/ui`, then wider visual run.

## Routing matrix
| Finding class | Action | Where |
|---|---|---|
| cosmetic / token / missing state / aria / friction / i18n key | **inline-fix** + before/after | here |
| design-system drift across many components / phase boundary | **escalate** | Frontend-Audit-Polish-Gate |
| multi-step journey / cross-role / live-WS reconcile / idempotency | **hand off** | Convergence-Playwright loop |
| server lacks a field/state/endpoint | flag, don't fix | `MISSING`/`BLOCKED-contract` → backend |
| price/status logic, state-machine, security, cookie, PII | flag-only | backend/security dialog |
| mechanical check red | run/fix the floor | `lint:gates`/`typecheck` |
| flaky baseline (noise) | fix determinism (Docker/mask/anim) — don't weaken threshold blindly | here, Layer 3 |
> Always leave a **proof artifact** (screenshot/diff/vision-verdict) so the receiving loop doesn't restart.

## Definition of Done (per change)
🟢 FLOOR green · 🟢 every affected state renders · 🟢 visual diff clean (or new diffs consciously approved) · 🟢 vision verdict has no unresolved A–F FAIL · 🟢 every finding either inline-fixed (before/after) or routed with proof. Nothing silently skipped or weakened.

## Out of scope
❌ phase GO/NO-GO · ❌ owning E2E journeys · ❌ changing server contracts/price-status/security · ❌ reimplementing mechanical gates · ❌ weakening a threshold / masking a real bug · ❌ design-to-code generation · ❌ vision as the final gate (it's triage; regressions confirmed by pixel diff; baselines approved consciously).

## Appendix · vision prompt skeleton (agent-as-eye)
Per affected component/state, give the model the screenshot + tokens/spec and request JSON only:
`{ A_design_system, B_states, C_a11y, D_responsive, E_i18n, F_semantic, match_to_spec, findings:[{what,severity,route}] }`
with each dimension `{verdict:"PASS|FAIL", issue, evidence}`. FAIL + route → routing matrix.

## Environment status (this sandbox, 2026-06-24)
- ✅ Layer 1 rails: `no-arbitrary-tailwind` shipped; jsx-a11y + colour rules present. FLOOR runner = `scripts/ui-verify-floor.sh`.
- ⏳ Layer 2 Storybook: blocked on `package.json` deps (protect-paths) → `proposed-ui-loop-infra/APPLY.md`.
- ⏳ Layer 3 Docker visual: config in the proposal; no Docker locally → baselines generated in CI/Docker.
- ✅ Layer 4 vision: via Claude subagents (OpenRouter credits dead). First run: `e2e/journeys/UI_LOOP_RUN_2026-06-24.md`.
