# DeliveryOS — Реєстр петель

Джерело істини про наявні петлі. Картки — `loops/<id>.yaml`. Звіти — `loops/reports/`. Пам'ять — `loops/memory/`.
Статуси: **CERTIFIED** (пройшла M1–M11) · **DRAFT** (будується) · **REJECTED** (впала верифікацію) · **DEPRECATED**.
Машинний реєстр роутера `loops/runs/registry.json` — **ПОХІДНИЙ** від цієї таблиці: `node scripts/loops-registry-sync.mjs` (P2 2026-07-02; до цього роутер бачив 2 з 16 петель). `docs/agents/loops/REGISTRY.md` — історичний seed, не джерело.

**Intake (Loop Selection Router).** Над усім — роутер (`tools/loop-harness/src/router.ts`, спец `docs/operating-model/loop-selection-router-v1.md`) вирішує на КОЖНІЙ команді: DIRECT (типове — без петлі) · RUN наявну (`runs/registry.json`) · BUILD нову (loop-builder) · BOUNCE (нема метрики). Routes only — не переписує Class A/B / admissibility / security carve-out. Enforcement = `UserPromptSubmit` hook (`tools/loop-harness/router-hook.sh`, оператор вмикає в .claude) + CLAUDE.md директива.

**Harness (обов'язково для КОЖНОЇ петлі).** Жодної петлі поза харнесом. На фініші (success/stall/abort) петля ЗАВЖДИ емітить §5 LOOP REPORT (повністю в термінал) через `tools/loop-harness` finalize, який міряє git+session-токени+eco і пише незнищенно/lossless у `loops/runs/`. Дизайн — `docs/operating-model/living-loop-system-v3.md`; вузол `harness:` у картці. Телеметрія токенів/агентів/скілів ← session JSONL (те, що читає codeburn). Реалізовано §10 кроки 1–3 + колектори; відкладено §4/§6-recall/§8.

| id | intent (коротко) | version | статус | картка | звіт | пам'ять | тригер |
|---|---|---|---|---|---|---|---|
| error-fix-convergence | UI↔сервер у повну відповідність, кожен флоу зелений | 1.0 | CERTIFIED (звіт ВТРАЧЕНО — re-cert: /build-verify-loop verify) | loops/error-fix-convergence.yaml | — | loops/memory/error-fix-convergence.md | /converge-loop |
| design-convergence | загартований план серйозної зміни (Тріадна Рада) | 1.0 | CERTIFIED (звіт ВТРАЧЕНО — re-cert: /build-verify-loop verify) | loops/design-convergence.yaml | — | loops/memory/design-convergence.md | /council |
| backend-contract-convergence | сервер↔контракт у відповідність | 0.1 | DRAFT | loops/backend-contract-convergence.yaml | loops/reports/backend-contract-convergence-0.1.md | loops/memory/backend-contract-convergence.md | /converge-server |
| investigation-triage | знайти корінь невідомого баґу (repro-first) | 0.1 | DRAFT | loops/investigation-triage.yaml | loops/reports/investigation-triage-0.1.md | loops/memory/investigation-triage.md | /investigate |
| regression-hunt | комміт-винуватець регресії (git-bisect) | 0.1 | DRAFT | loops/regression-hunt.yaml | loops/reports/regression-hunt-0.1.md | loops/memory/regression-hunt.md | /regression-hunt |
| incident-recovery | живий збій: стабілізувати→корінь→post-mortem | 0.1 | DRAFT | loops/incident-recovery.yaml | loops/reports/incident-recovery-0.1.md | loops/memory/incident-recovery.md | /incident |
| refactor-convergence | дублі→єдине джерело, поведінка незмінна | 0.1 | DRAFT | loops/refactor-convergence.yaml | loops/reports/refactor-convergence-0.1.md | loops/memory/refactor-convergence.md | /refactor-converge |
| performance | метрика в межі бюджету, без регресії | 0.1 | DRAFT | loops/performance.yaml | loops/reports/performance-0.1.md | loops/memory/performance.md | /perf |
| build-stage | етап роадмапу до ✅ чекпойнта | 0.1 | DRAFT | loops/build-stage.yaml | loops/reports/build-stage-0.1.md | loops/memory/build-stage.md | /build-stage |
| autoupgrade | machine-gated self-upgrade (haste iteration, fewer resources); Class A авто/Class B пропозиція; v0.2 +proven-upgrade registry +decorrelated lenses (security·reversibility·perf) | 0.2 | CERTIFIED (report-only; звіт ВТРАЧЕНО — re-cert: /build-verify-loop verify) | loops/autoupgrade.yaml | — | loops/memory/autoupgrade.md | /autoupgrade |
| loop-builder | мета-петля: ціль G → найкраща harness-native петля (oracle-admissibility first, born-hardened) | 0.1 | DRAFT (report-only) | loops/loop-builder.yaml | loops/reports/loop-builder-0.1.md | loops/memory/loop-builder.md | /loop-builder |
| audit-gate | секції A–F PASS з артефактами | 0.1 | DRAFT | loops/audit-gate.yaml | loops/reports/audit-gate-0.1.md | loops/memory/audit-gate.md | /audit-gate |
| exit-audit | завершеність фази (змагально) | 0.1 | DRAFT | loops/exit-audit.yaml | loops/reports/exit-audit-0.1.md | loops/memory/exit-audit.md | /exit-audit |
| mobile-polish | мобільний UX (390px) кожної поверхні → PASS Mobile Rubric | 1.0 | CERTIFIED | loops/mobile-polish.yaml | loops/reports/mobile-polish-0.1.md | loops/memory/mobile-polish.md | /mobile-polish |
| acquisition-bulk-provision | батч ресторанів → claimable shadow + claim invite через /internal pipeline (idempotent, gated, no-cheat) | 0.1 | CERTIFIED | loops/acquisition-bulk-provision.yaml | loops/reports/acquisition-bulk-provision-0.1.md | loops/memory/acquisition-bulk-provision.md | node scripts/acquisition-bulk-provision.mjs &lt;list&gt; |
| demo-builder | prospect → ПОЛІРОВАНИЙ claimable demo storefront /s/:slug (menu-quality gate + coherent brand + VISUAL acceptance gate; preview-only default) | 0.1 | CERTIFIED | loops/demo-builder.yaml | loops/reports/demo-builder-0.1.md | loops/memory/demo-builder.md | node scripts/demo-builder.mjs &lt;prospects&gt; |
| test-hardening | полювання на false-green класи в тест-сюїті (10 заборонених класів) | 0.1 | DRAFT | loops/test-hardening.yaml | docs/design-review/test-hardening-findings.md | — | ad-hoc |
| offer-builder | prospect → outreach offer packet | 0.1 | DRAFT (без картки) | — | — | loops/memory/offer-builder.md | ad-hoc |

## Здоров'я
Health-pass (Counsel) читає пам'ять/звіти й сигналить «хворі» петлі (flaky-під-green, training-ніколи-не-вимкнено, нема-пам'яті). Orchestrator на «хвору» петлю → improve-запит loop-architect.
🔴 DRAFT-петлі НЕ диспатчаться, доки `/build-verify-loop verify <id>` не поставить CERTIFIED.

## cross-tenant-realtime-qa (QA · real staged service)
Cross-tenant, multi-role, real-time ordering validation проти РЕАЛЬНОГО staged сервісу (не мок). 3 ролі (customer UI + order + tracking · owner real-auth lifecycle · courier bus-dispatch) + real-time WS deltas + tenant isolation (ownerCanAccessRoom guard + 401 + customer-token-no-PATCH). Спец: `e2e/tests/cross-tenant-realtime-qa.spec.ts`. Run: `--project=desktop`. 6/6 GREEN staging.

## Test Integrity (cross-loop rule)
Будь-яка петля/агент, що ПИШЕ або РЕВ'Ю тести (test-hardening, audit-gate, convergence, QA), застосовує AGENTS.md «Test Integrity» (10 заборонених false-green класів + 🔴 money/RLS/PII). Ledger: docs/design-review/test-hardening-findings.md. Агент-визначення в .claude/agents/ успадковують AGENTS.md автоматично (CLAUDE.md).
