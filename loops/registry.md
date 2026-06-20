# DeliveryOS — Реєстр петель

Джерело істини про наявні петлі. Картки — `loops/<id>.yaml`. Звіти — `loops/reports/`. Пам'ять — `loops/memory/`.
Статуси: **CERTIFIED** (пройшла M1–M11) · **DRAFT** (будується) · **REJECTED** (впала верифікацію) · **DEPRECATED**.

| id | intent (коротко) | version | статус | картка | звіт | пам'ять | тригер |
|---|---|---|---|---|---|---|---|
| error-fix-convergence | UI↔сервер у повну відповідність, кожен флоу зелений | 1.0 | CERTIFIED | loops/error-fix-convergence.yaml | loops/reports/error-fix-convergence-1.0.md | loops/memory/error-fix-convergence.md | /converge-loop |
| design-convergence | загартований план серйозної зміни (Тріадна Рада) | 1.0 | CERTIFIED | loops/design-convergence.yaml | loops/reports/design-convergence-1.0.md | loops/memory/design-convergence.md | /council |
| backend-contract-convergence | сервер↔контракт у відповідність | 0.1 | DRAFT | loops/backend-contract-convergence.yaml | loops/reports/backend-contract-convergence-0.1.md | loops/memory/backend-contract-convergence.md | /converge-server |
| investigation-triage | знайти корінь невідомого баґу (repro-first) | 0.1 | DRAFT | loops/investigation-triage.yaml | loops/reports/investigation-triage-0.1.md | loops/memory/investigation-triage.md | /investigate |
| regression-hunt | комміт-винуватець регресії (git-bisect) | 0.1 | DRAFT | loops/regression-hunt.yaml | loops/reports/regression-hunt-0.1.md | loops/memory/regression-hunt.md | /regression-hunt |
| incident-recovery | живий збій: стабілізувати→корінь→post-mortem | 0.1 | DRAFT | loops/incident-recovery.yaml | loops/reports/incident-recovery-0.1.md | loops/memory/incident-recovery.md | /incident |
| refactor-convergence | дублі→єдине джерело, поведінка незмінна | 0.1 | DRAFT | loops/refactor-convergence.yaml | loops/reports/refactor-convergence-0.1.md | loops/memory/refactor-convergence.md | /refactor-converge |
| performance | метрика в межі бюджету, без регресії | 0.1 | DRAFT | loops/performance.yaml | loops/reports/performance-0.1.md | loops/memory/performance.md | /perf |
| build-stage | етап роадмапу до ✅ чекпойнта | 0.1 | DRAFT | loops/build-stage.yaml | loops/reports/build-stage-0.1.md | loops/memory/build-stage.md | /build-stage |
| audit-gate | секції A–F PASS з артефактами | 0.1 | DRAFT | loops/audit-gate.yaml | loops/reports/audit-gate-0.1.md | loops/memory/audit-gate.md | /audit-gate |
| exit-audit | завершеність фази (змагально) | 0.1 | DRAFT | loops/exit-audit.yaml | loops/reports/exit-audit-0.1.md | loops/memory/exit-audit.md | /exit-audit |

## Здоров'я
Health-pass (Counsel) читає пам'ять/звіти й сигналить «хворі» петлі (flaky-під-green, training-ніколи-не-вимкнено, нема-пам'яті). Orchestrator на «хвору» петлю → improve-запит loop-architect.
🔴 DRAFT-петлі НЕ диспатчаться, доки `/build-verify-loop verify <id>` не поставить CERTIFIED.
