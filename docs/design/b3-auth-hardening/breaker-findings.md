# B3 Deep Auth Hardening — Breaker Findings

Атака проти `docs/design/b3-auth-hardening/proposal.md`, заземлена на фактичне джерело (2026-07-03).
Формат: `[SEVERITY] вектор · знахідка · сценарій/число · порушений інваріант`. Нуль фіксів.

Верифіковані факти-опори:
- `packages/platform/src/auth/tenant.ts` — `withTenant` ставить ЛИШЕ `app.user_id` (`is_local=true`), НЕ роль, НЕ `app.current_tenant`.
- `packages/db/migrations/1790000000077…` — RC2 policy `ops_all ON users/auth_refresh_tokens FOR ALL TO dowiz_app`; RC4 courier-writes keyed на `app.current_tenant`.
- `packages/db/migrations/1790000000080_grant-hardening.ts` — REVOKE/GRANT цілять роль `dowiz_app` (це логін-роль).
- `packages/db/migrations/1780421100065…` — `users` та `auth_refresh_tokens` = `NO FORCE ROW LEVEL SECURITY` (RLS enabled, force off).
- `memberships FORCE ROW LEVEL SECURITY` (`1780310071220`).
- `grep "SET LOCAL ROLE\|SET ROLE"` у apps/packages → **0 збігів** (механізм Option B ще не існує).
- `grep "RENAME COLUMN"` у `packages/db/migrations/` → **0 збігів** (жодного in-tree rename `telegram_connect_tokens`).
- 116 викликів `withTenant` проти 123 прямих `.db.query/pool.query` call-site.

---

## CRITICAL

### C1 — [B-FAIL / B-CONSIST] Тотальний login-lockout через невідповідність RLS role-targeting в Option B (реалізований R-2)
Option B робить `SET LOCAL ROLE dowiz_app_rls` всередині `withTenant`. Але:
- RC2 policy = `ops_all ON users … FOR ALL TO dowiz_app` — застосовується до ролі лише якщо `current_user` має привілеї `dowiz_app` (є ним або членом). Після `SET LOCAL ROLE dowiz_app_rls` `current_user = dowiz_app_rls`.
- Проєкт NIGDE не задає `GRANT dowiz_app TO dowiz_app_rls`; напрямок ґранту в §3 зворотний (`GRANT dowiz_app_rls TO login`). Сам proposal хеджує: «confirm pg_has_role mapping in the staging pre-flight» — тобто НЕ вирішено.
- `users`/`auth_refresh_tokens` = `NO FORCE` рятує лише ВЛАСНИКА таблиці; `dowiz_app_rls` — не власник → підпадає під RLS. Немає застосовної permissive-policy → default-deny.

Сценарій поломки: у мить, коли `RLS_ENFORCE_OWNER` вмикається, кожен owner-запит читає `users`/`auth_refresh_tokens` як `dowiz_app_rls`, жодна policy не admit → **0 рядків на всіх auth-читаннях → ніхто не логіниться, включно з оператором**. Це саме той «firebreak», який §7 називає гарантією проти lockout, а він структурно не з'єднаний. Blast radius = весь флот атомарно на весь ramp owner-lane.
Порушено інваріант: fail-closed для tenant-даних НЕ повинен ставати auth-lockout (§7 central tension); «RC2 = lockout firebreak» — недоведене.

### C2 — [B-SEC / B-CONSIST] `set_config('app.user_id', …, false)` на operational (Supavisor transaction-mode) пулі у provisioning-шляху
`apps/api/src/routes/owner/onboarding.ts:75` та `apps/api/src/routes/spa-proxy.ts:771` ставлять `app.user_id` з `is_local=false` (СЕСІЙНИЙ GUC) на клієнті з operational-пулу (`db.connect()` на :6543 transaction mode). В onboarding `BEGIN/COMMIT` стоять окремо (рядки 102/121) — UPDATE locations (85-88) і `set_config false` НЕ в одній транзакції; коментар прямо покладається на «set_config persists across statements on a single client».

Сценарій поломки (дві гілки, обидві погані під NOBYPASSRLS):
- (a) Supavisor transaction mode НЕ гарантує той самий backend між autocommit-стейтментами → сесійний GUC не доживає до наступного стейтмента → member-keyed FORCE-RLS write (`bootstrap_owner`→onboarding_state UPDATE→menu seed) **denied → onboarding нового owner ламається**.
- (b) якщо GUC доживає на переюзаному backend і не скидається на `release()` (немає RESET/ROLLBACK) → наступний запит ІНШОГО tenant на цьому backend успадковує stale `app.user_id` → під enforcement admit чужих рядків → **cross-tenant read/write**.
Порушено інваріант: транзакційно-локальний tenant-контекст (RC-модель `withTenant` `is_local=true`); no cross-tenant leak. Proposal аналізує Option B так, ніби всі шляхи = `is_local=true`; ці два `false`-шляхи не враховані.

---

## HIGH

### H1 — [B-SEC] Credential-rotation «dual-valid overlap window» (#4) — суперечність: або leak не закритий, або rollout не атомарний
Один PG-role не може мати два валідних паролі. `ALTER ROLE deliveryos_api_user PASSWORD 'new'` вбиває leaked-пароль МИТТЄВО (немає overlap → машини на старому секреті падають під час rollout = каскад). Щоб отримати «overlap», треба ТРИМАТИ leaked-роль живою → **leaked пароль з git продовжує працювати весь overlap-вікно** → мета #4/R-7 (закрити live-exposed cred) не досягнута протягом усього вікна.
Число/сценарій: тривалість «безпечного» overlap-вікна = тривалість, протягом якої скомпрометований креденшл лишається валідним. Proposal прирівнює overlap до idempotency, ігноруючи, що це = вимірюване вікно експлуатації активно-leaked cred (контекст `secrets-exposure-incident-2026-07-03`).
Порушено інваріант: ротація закриває leak; §5 «made idempotent by an overlap window (both passwords valid)» технічно неможливе для single-role і суперечить security-меті.

### H2 — [B-CONSIST / B-DATA money] RC4 money-writes під enforcement залежать від `app.current_tenant`, якого `withTenant` не ставить
RC4 (`orders` UPDATE/SELECT, `courier_cash_ledger` INSERT, `delivery_trace` INSERT) keyed на `app.current_tenant`. Механізм enforcement Option B (`SET LOCAL ROLE`) додається у `withTenant`, який ставить лише `app.user_id`. Courier money-шляхи (`courier/assignments.ts`, `courier/settlements.ts`, `owner/couriers.ts`, `lib/courier-room-authz.ts`) ставлять `app.current_tenant` на СИРОМУ checked-out клієнті, БЕЗ `withTenant` і БЕЗ `SET LOCAL ROLE`.
Сценарій поломки: (a) під час Option B ці шляхи лишаються на login-role (bypass) → **RLS-захист money-path відсутній весь ramp** (illusory hardening); (b) при convergence (Option A flip login-role NOBYPASSRLS) будь-який money-write, де `app.current_tenant` не заданий/заданий з невірним `is_local` → `UPDATE orders`/`INSERT courier_cash_ledger` матчить **0 рядків → тихий провал status-transition / cash-as-proof**. Це money+dispatch red-line (R-3).
Порушено інваріант: гроші-write під enforcement або guarded (rowcount>0 → помилка), або взагалі не enforced; тут — ані defense-in-depth під час ramp, ані визначена поведінка під час flip.

### H3 — [B-SCALE / B-SEC] Option B enforcement живе лише в `withTenant` → більшість write-шляхів лишаються BYPASSRLS (illusory hardening); IDOR-другий-нет відсутній саме там, де WHERE найлегше забути
`SET LOCAL ROLE` існував би тільки в `packages/platform/tenant.ts`. Enumerated шляхи, що НЕ проходять `withTenant` і лишаються на login-role (bypass) весь ramp: усі courier `app.current_tenant`-шляхи (assignments/settlements/couriers/room-authz), webhooks (`payments-webhook.ts:41`, `telegram-webhook.ts:281/411/631`), `public/funnel.ts:60` (anon insert), воркери (`courier-events`, `reconciliation`, `settlement-cron`, `dwell-monitor`, `signal-raiser`, `notifications/workers`), `getOwnerLocationId` (сирий read `memberships` до контексту), `spa-proxy`, `storefrontService`, `notificationPrefsService`.
Число: 123 прямих `.db.query/pool.query` проти 116 `withTenant` — приблизно половина DML-поверхні поза механізмом. Фреймінг «owner → courier → anon 3 lanes» приховує, що більшість raw-pool write-шляхів взагалі не в моделі lane.
Порушено інваріант: «defense-in-depth: forgotten WHERE no longer leaks» (§8) — не виконується для жодного raw-pool шляху; саме там app-layer WHERE найімовірніше забути.

---

## MEDIUM

### M1 — [B-OPS] W1 boot-guard = мертвий тягар весь Option B ramp
`createOperationalPool` `on('connect')` перевіряє `current_user` при коннекті = ЛОГІН-роль. Option B тримає логін-роль BYPASSRLS до convergence. Тож `SELECT NOT rolbypassrls FROM pg_roles WHERE rolname=current_user` або FATAL-ить boot назавжди (логін-роль bypass), або має бути вимкнений → **W1 не ловить випадковий `ALTER ROLE dowiz_app_rls BYPASSRLS`** (роль, що реально виконує enforced-запити, ніколи не бачиться connect-guard'ом, бо існує лише всередині txn). Зуби W1 працюють тільки post-convergence.
Порушено інваріант: guardrail, що має ловити повернення BYPASSRLS, не покриває enforcement-роль протягом усього періоду, коли enforcement нібито активний.

### M2 — [B-SEC / B-DATA] Grant-mirror на `dowiz_app_rls` ризикує re-open хардмінг міграції 080
§5.2 каже «mirror the login role's table/sequence grants onto `dowiz_app_rls`». Міграція 080 явно REVOKE `TRUNCATE, TRIGGER, REFERENCES` та `INSERT/UPDATE/DELETE ON platform_admins` з `dowiz_app`. Наївний `GRANT ALL … TO dowiz_app_rls` дає enforcement-ролі БІЛЬШЕ привілеїв, ніж аудитована `dowiz_app`: повертає TRUNCATE (не підпадає під RLS → wipe будь-якої tenant-таблиці) і write на `platform_admins` (self-promotion до platform-admin, `platform_admins` має RLS off → лише grants governing).
Сценарій: mirror робиться до post-080 стану, а не blanket — але §5.2 цього не специфікує; blanket-mirror = регрес двох HIGH-фіксів 080 на ролі, що фактично обслуговує трафік.
Порушено інваріант: enforcement-роль ≤ привілеї аудитованої логін-ролі; TRUNCATE/platform_admins-write лишаються закритими.

### M3 — [B-FAIL] Власник SECURITY DEFINER-функцій не визначений — load-bearing для owner-resolve і worker-sweep під convergence
RC3 `app_owner_location` (077) + ~19 Phase-2 sweep-fns (078) = `SECURITY DEFINER`, читають FORCE-RLS `memberships`/tenant-таблиці. Вони bypass RLS ЛИШЕ якщо їхній ВЛАСНИК зберігає BYPASSRLS. Власник = роль, що запускає міграцію (`DATABASE_URL_MIGRATIONS`), не специфіковано.
Сценарій поломки: якщо ці fns належать `dowiz_app`, то після convergence (`ALTER ROLE dowiz_app NOBYPASSRLS`, §5.5) вони стають subject to RLS → `app_owner_location` повертає 0 → `getOwnerLocationId` → null → owner трактується як не-owner → **locked out з власного дашборда**; worker cross-tenant sweep → 0 → **dispatch/reconciliation тихо зупиняється**. Convergence має зберегти BYPASSRLS-власника для DEFINER-fns; не адресовано.
Порушено інваріант: fail-closed не має вимикати легітимний owner-доступ і system-sweep.

### M4 — [B-DATA] Drift-reconciliation спроєктована на неперевіреному твердженні; re-run на both-columns стані невизначений
`grep "RENAME COLUMN"` по всіх міграціях = 0; base + 077 + 080 усі референсять `owner_id`. Твердження «prod keys on user_id (rename on staging, skipped on prod)» не має жодного in-tree артефакту, а §3 суперечливо описує напрямок дрифту (rename нібито applied on staging — тоді user_id на STAGING, не prod). Проєктувати forward-only `ALTER TABLE … RENAME COLUMN user_id TO owner_id` з «claim» без реального introspection-виводу:
Сценарій: (a) phantom no-op, якщо всі середовища вже `owner_id`; (b) якщо на частково-мігрованому env існують ОБИДВІ колонки — `RENAME user_id TO owner_id` падає (не можна перейменувати в наявну колонку); описаний DO-guard `IF user_id EXISTS THEN RENAME` цього кейсу не обробляє → re-run safety не визначена. Це той самий клас, що outage 2026-06-20.
Порушено інваріант: forward-only міграція idempotent + re-run-safe; policy пишеться проти реально інтроспектованої, а не «claimed» схеми (§3 сам це вимагає, але базує на claim).

### M5 — [B-SCALE] Rate-limiter pg-store: BoE невірний для adversarial-кейсу + hot-row contention = DoS-ампліфікатор
§5 BoE «~200 writes/min, single-digit/s under attack». Але лічильник робить upsert НА КОЖНУ спробу ДО рішення про ліміт → credential-stuffing flood (не «single-digit/s») пише на кожен запит. Таргет одного акаунта = один hot key = один рядок у `auth_rate_events(key, window_start)` → серіалізовані upsert-и під row-lock → лімітер сам себе серіалізує і з'їдає operational-pool конекти → **клас pool-starvation 2026-06-20**, перетворюючи anti-brute-force контроль на DoS-ампліфікатор.
Fallback degrade-to-in-memory прибирає cross-instance лімітування САМЕ під час DB-стресу: distributed brute-force повертає собі `N_machines × per-instance` бюджет тоді, коли shared-store лежить.
Порушено інваріант: back-of-envelope має сходитись на adversarial-N (лімітер існує для атаки); контроль не має ставати ампліфікатором.

---

## LOW

### L1 — [B-OPS / B-ANTIPATTERN] Convergence (Option A) — точка неповернення, не flag-reversible
`ALTER ROLE dowiz_app NOBYPASSRLS` + видалення `SET LOCAL ROLE`/ґранту (§5.5) = міграція; rollback = інша `ALTER ROLE … BYPASSRLS` міграція, не флаг. Теза §9 «every risky item reverts by flag» тримається ЛИШЕ під час Option B ramp; заявлений end-state відновлює повний Option-A blast radius з rollback рівня міграції (не «seconds»). Proposal це визнає, але класифікує convergence як «later/separate», що приховує, що фінальна конфігурація втрачає flag-reversibility.
Порушено інваріант: named, per-flag rollback для кожного risky-кроку (§4).

### L2 — [B-FAIL] WS URL-token removal gate «usage→0» без forcing-функції може не досягтись ніколи
`logTokenDeprecation` (`websocket.ts:350`) рахує клієнтський usage, але кешовані PWA/SW + FE `client/status/ws.ts` (досі URL-based) роблять usage асимптотичним, не 0. Gate #2 «usage→0» без дедлайну/примусу → URL-token vector (`?token=` у access-log/Referer/SW-cache) лишається відкритим невизначено довго. Додатково: при fail URL-token verify (`websocket.ts:352-354`) сокет НЕ закривається, покладається на 5s authTimeout.
Порушено інваріант: transport-leak (#2/§8) закривається; telemetry-gate має мати forcing-функцію, інакше «removal» не відбувається.

---

## Регресійна нотатка для RE-ATTACK
Найгостріше з'єднання: C1 (role-targeting) + M2 (grant-mirror) + M3 (DEFINER owner) — усі три залежать від того, ЯКОЮ роллю фактично виконуються enforced-запити (`dowiz_app` vs `dowiz_app_rls`) і хто власник DEFINER-fns. Proposal ніде цього однозначно не фіксує; поки не зафіксовано — Option B не можна вважати ані безпечним (lockout), ані повним (illusory). C2 (`is_local=false`) і H2 (courier `app.current_tenant`) — це конкретні вже-в-коді шляхи, що ламають модель Option B, а не гіпотетичні.
