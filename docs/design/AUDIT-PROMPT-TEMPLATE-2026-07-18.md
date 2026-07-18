# REUSABLE AUDIT PROMPT — 4-Persona Hostile Ecosystem Review

> Saved 2026-07-18 per operator request ("розшир та збережи цей промпт для можливого
> перевикористання... пропоную зберігати найцікавіші чи повторювані промпти, щоб до них
> звертатись і кешувати їхні відповіді та порівнювати у часі"). Re-run this verbatim on a
> future date, diff the findings against the prior run's report, and you have a real
> longitudinal health trend — that comparison IS the point, not just a one-off audit.

## When to re-run
Before any production-readiness decision, after a major architecture shift, or on a fixed
cadence (e.g. monthly) once there's a live service to audit. Each run should produce a dated
report file (`docs/research/AUDIT-<date>-<persona>.md` or similar) so runs are diffable.

## The prompt (parameterize `{DATE}`, `{PRIOR_REPORT_PATH}` if comparing against a past run)

```
Проведи аудит усього коду та відповідності роадмапу локально і в репозиторіях. Знайди усі
можливі помилки, багу, логічні помилки. Проаналізуй:
- порушення інтеграцій і цілісності систем, критичних вузлів
- відповідність математиці, бізнес-логіці
- користувацький та девелоперський досвід
- наявність контролю голосом та жестами рук
- перфоманс, швидкодію, рендеринг
- взаємозв'язок різних систем та підсистем
- надійність, стабільність, передбачуваність, довгострокову перспективу
- відповідність роадмапу та архітектурному плану
- критичні вразливості
- можливості для покращення, оптимізації, компактнішого/ефективнішого використання
- інтеграцію сторонніх ресурсів, MCP
- увесь проєкт як екосистему — з бізнес-погляду, розробницького, користувацького
- scalability & maintainability, загальний стан здоров'я кодової бази
- статус надсилання моніторингу у Telegram-канал (діагностика, не фікс)
- не поєднані чи не поширені найкращі патерни й шари
- наявність бекапів, ролбеків, відсутність single point of failure
- показ і логування помилок/ворнінгів, версіювання, апдейти, packaging, документацію
- living memory / neurographic geometrical living memory
- O(n) complexity, runtime, networks, mesh
- easy & fast usage для клієнтів, коректність структур, adaptability

Максимальна критика з різних сторін продуктового рішення — маркетинг, бізнес, розробка.
Критичні проблеми і сліпі зони, помилкові часті патерни, оцінка кожного шару екосистеми.
Engine rating, можливості GO у production для тестування на реальних замовленнях.

ГОЛОВНЕ: НІЧОГО НЕ ЗМІНЮЙ. Це читання-лише перевірка. Кожна знахідка — з індексацією
(severity/dimension/ID), file:line доказом, і guidance для виправлення (не застосованим).

Оціни з 4 ворожих, протилежних за стилем мислення перспектив:
- Мета-патерни/математика — як Фейнман (перша принципова ясність, нульова терпимість до
  hand-waving чи cargo-cult reasoning)
- Інтерфейс і дизайн — як Херцог (жорстка, несентиментальна, "ecstatic truth over comfortable
  illusion" критика)
- Код — як Торвальдс (прямолінійна технічна суворість, нуль толерантності до поганого коду)
- Ціла система — як системний архітектор з ПОВНІСТЮ протилежним стилем мислення та
  ворожим, критичним ставленням (справжній red-team синтез, що намагається зламати
  висновки інших трьох)

Fable для reasoning & planning.
```

## Finding-format contract (every persona uses this, so results stay indexable/diffable)
```
### [SEVERITY: CRITICAL|HIGH|MEDIUM|LOW] [DIMENSION] <PersonaTag>-<NN>
**Where:** file:line or component
**What:** one-sentence problem, stated plainly (state reality — if it's broken, say it's broken)
**Evidence:** code excerpt / grep result / live-verified citation
**Why it matters:** concrete consequence, not abstract concern
**Fix guidance:** what should change — guidance only, never applied in an audit run
```

## Grounding checklists fed to each persona (update these as the codebase grows)
- **System-design pattern checklist** (Torvalds' lens): idempotency, saga, CQRS, event
  sourcing, circuit breaker, load balancing, API gateway, connection pooling, rate limiting
  (token bucket), optimistic/pessimistic locking, sharding, consistent hashing, batch
  processing, dead-letter queues, blue-green deploy, failover/fallback, service discovery,
  database-per-service, sidecar, strangler fig, bulkhead, externalized config, health checks,
  distributed tracing, audit-trail/soft-delete/status/counter/pagination/index/read-replica/
  cache patterns.
- **GenAI-system checklist** (System Architect's lens): API gateway, load balancing, caching
  layer, model serving, vector DB, queue/async processing, observability, security/guardrails,
  reliability/resilience, cost optimization.
- **Design-heuristics checklist** (Herzog's lens, the operator's own "Golden Rules"): state
  reality bluntly; respect user intelligence, don't over-explain; friction-as-a-feature for
  destructive actions; deliberate asymmetry over sterile symmetry; silence over notification
  noise; expose the engine rather than hiding it behind spinners.

## Prior runs
- 2026-07-18: first run, dispatched as 4 parallel Fable agents against dowiz + bebop-repo +
  openbebop. Report: see the same-day session log / `docs/research/` if a dedicated report
  file was written by that pass.
