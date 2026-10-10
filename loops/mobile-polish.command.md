<!-- STAGED slash-command. `.claude/` is protect-paths-blocked (manual approval). To enable the
     `/mobile-polish` trigger, copy this file to `.claude/commands/mobile-polish.md` and approve. -->
---
description: Петля полірування мобільного UX (390px) — кожна поверхня PASS за Mobile Rubric з 390px-артефактом; verify-before-fix; косметику фіксиш inline, логіку/контракти — flag-only.
argument-hint: <опц. скоуп: storefront|admin|courier|екран>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Agent, Task
---
Запусти петлю mobile-polish (loops/mobile-polish.yaml). Скоуп: «$ARGUMENTS».

🔴 PASS лише з 390px-скріншотом. Цикл SENSE→DIAGNOSE→ACT→VERIFY→REPEAT:
- **SENSE**: перезніми поверхні на 390px на staging — `CAPTURE=1 SLUG=demo VITE_BASE_URL=https://qa-durres.dowiz.org DEV_AUTH_SECRET=stg-e2e-secret CAPTURE_DIR=audit/mobile-polish-iN pnpm exec playwright test e2e/tests/capture-states.spec.ts --project=desktop` (бери `-m` шоти; іконки рендеряться — self-host).
- **DIAGNOSE**: оціни кожен `-m` шот за Mobile Rubric (tap-targets ≥44px+thumb-zone · нуль overflow · chrome ≤ content · inputs ≥16px/inputMode · safe-area · density+семантичні статус-кольори · стани · shared-компоненти+bottom-tab). **verify-before-fix**: познач кожну знахідку real / artifact / flag-only з доказом. Звіт → `docs/design-review/MOBILE-POLISH.md`.
- **ACT**: фіксиш лише verified, FE-only, токен-конформні мобільні знахідки (collision-free fan-out). Логіку/контракт/безпеку/seed-data — flag-only.
- **VERIFY**: перезніми 390px, доведи кожну фіксовану знахідку зеленою + нуль регресій; typecheck+build зелені.
- **REPEAT** до exit: усі поверхні PASS + 0 overflow + tap-target чисто. Ship: commit→deploy staging→validate.

Онови `loops/memory/mobile-polish.md`. Сертифікація DRAFT→CERTIFIED — окремо через loop-architect.
