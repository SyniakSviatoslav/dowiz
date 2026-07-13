# dowiz / bebop2 — Ops · Reliability · Single-Pane · Migration to pgrust — PLAN

> **Дата:** 2026-07-13 · **Тип:** дослідження → аналіз → синтез → план з блюпринтами (НЕ код) ·
> восьма задача сьогодні. Збережено в репо + Telegram.
> **Стоїть на:** консолідації на Hetzner (цієї сесії) + [[ecosystem-strategy-arc-2026-07-13]] (3 кити) +
> [[integration-ports-reactive-arc-2026-07-13]] + kernel/bebop2 PQ. 7 паралельних дослідницьких смуг + інвентар.
> **Головні цілі оператора (дослівно):** менше точок відмови · один надійний real-time моніторинг з УСІХ джерел
> в одному місці + Telegram-алерти · запобіжники (circuit-breakers/rate-limit/rollbacks/fallbacks) · оптимізація
> головних метрик (особливо LATENCY) · зменшити DevOps-пекло, зробити зручно під одного оператора · безпека.
> **Рішення оператора (mid-turn):** **pgrust — одразу** (не Postgres-interim). Дроп Fly + Supabase.

---

## 0. Каркас і головна теза

Три кити ([[ecosystem-strategy-arc-2026-07-13]]): **ЯДРО** недоторкане (kernel decide/fold + pgrust як local-first
сховище-субстрат), цей план — про **ІНФРАСТРУКТУРУ** (deploy/backup/edge/secrets) та **ПОТОКИ** (моніторинг-,
alert-, rollback-потоки). Головна теза: **менше рухомих частин + швидко відновлюваний ОДИН вузол + один пульт
спостереження + запобіжники, що деградують закрито — краще за надлишкову складність.**

**Дві правди, що визначають усе (з інвентаря):**
1. **Майже вся ops-зрілість УЖЕ написана — але в `attic/`.** `health.ts` (11 перевірок), Sentry+PII-redaction,
   notification/outbox стек (telegram/email/push adapters, quiet-hours, retry), backup drill/restore/verify,
   node-pg-migrate (140 міграцій), rate-limit token-bucket, timeout/retry — усе реальне й зріле, карантиновано
   рішенням D1. **Правило плану: «воскресити з attic + перенацілити», не «будувати з нуля».** Живим лишився один
   примітив — `CircuitBreaker` у `packages/platform/routing-provider.ts` — і orphaned `audit-sentinel/` watchdog
   з Telegram-ескалацією (цілить у мертвий Fly → перенацілити).
2. **Репо в стані переходу з суперечностями, які план мусить розв'язати:** `main` (прод) ДОСІ живий на Fly
   (`dowiz.fly.dev`) з **незахищеним auto-deploy** (D5-F2); поточна гілка = static-SPA-only; D1/MANIFESTO кажуть
   «no Supabase/Fly», але `.secrets.local` (сьогодні) має ЖИВІ Supabase/R2/CF креди; ADR-0008 пропонує
   SQLite-per-node, а MIGRATION-PLAN — Postgres-via-SQLx. **Рішення «pgrust одразу» резолвить це в бік pgrust.**

**Архітектурне уточнення (потребує твого підтвердження):** bebop2-меш ще НЕ реальний (per red-team B4: IrohTransport
100%-stub, roster-auth unwired, **нуль delivery-domain коду**). Тому НАРАЗІ pgrust = сховище застосунку на
**Hetzner-хабі** (прагматичний single-node), а децентралізований per-node local-first — це **напрям**, коли bebop2
стане несучим. Отже: **pgrust-зараз-на-хабі, per-node-пізніше.** ADR-0008 треба оновити (SQLite→pgrust) — це red-line
ADR, зміна фіксується явно.

---

## 1. ЦІЛЬ 1 — Менше точок відмови (FMEA + принцип)

**Принцип: усувати режим відмови > додавати надлишковий компонент.** Кожен зайвий вузол = ще одне, що патчити,
моніторити, платити. Консолідація на один Hetzner уже прибрала найбільший клас відмов (координація Fly+Supabase+
worker) — не відкочуй це другим вузлом; **зроби відмови одного вузла нудними й швидко-відновлюваними.**

| # | Точка відмови | L×I | Ризик | Запобіжник (нижче) |
|---|---|---|---|---|
| 1 | **Deploy ламає прод** | H×H | 🔴 Critical | symlink-atomic-swap + health-gate + 1-cmd rollback (§4) |
| 2 | **DB-корупція/втрата** | M×H | 🔴 Critical | WAL-G PITR + pre-change snapshot + off-Hetzner копія (§7) |
| 3 | Сервер down / OOM | M×H | High | Hetzner snapshot + cold-boot runbook (не 2-й вузол) |
| 4 | Диск повний | M×H | High | alert на % + predict_linear ДО корупції (§2) |
| 5 | DDoS/сплеск на 1 бокс | M×MH | High | Cloudflare orange + Tunnel (origin-IP прихований, §9) |
| 6 | Cert-expiry | LM×H | MedHigh | CF Origin-CA cert (15р, no-cron) + heartbeat |
| 7 | Cloudflare outage | L×H | MedHigh | прийняти чесно + Always-Online static-floor |
| 8 | Region outage FSN1 | L×H | MedHigh | cold-boot runbook (rebuild-from-code + restore) |
| 9 | Object-storage outage | L×M | Med | degrade-not-down (health вже так моделює) |
| 10 | Міграція ламає застосунок | M×MH | MedHigh | pre-change snapshot + fail-closed schema-guard |

**#1 і #2 домінують — і обидва ОПЕРАТОРСЬКІ** (не інфраструктурні) → найкеровніші **процесом**, не залізом. **#7/#8 —
«податок на консолідацію в одного провайдера»** (рідко-але-тотально): це єдине місце, куди варто витратити ОДИН
долар надлишковості (off-Hetzner бекап + CF-edge), а не на #3-#6, що дешево усуваються.

**Що робити ЗАРАЗ (дешево, без нових рухомих частин):** CF-orange + origin-hidden; CF Always-Online (origin down →
CF віддає кешовану статику = «сайт стоїть» не «сайт ліг»); WAL-G continuous archiving (point-in-time за секунди);
Hetzner daily snapshots; воскресити backup-drill. **НЕ зараз (надлишковість):** 2-й always-on вузол (натомість
cold-start runbook як **скрипт**, не запущений бокс), read-replica (воює з архітектурою), меш-надлишковість (не реальна).

---

## 2. ЦІЛЬ 2 — Один real-time моніторинг з УСІХ джерел + Telegram

**Рекомендований стек (найменше ops на 1 боксі):**
`VictoriaMetrics (single-node) + VictoriaLogs` (1.3GB RAM vs Loki 6-7GB, single-binary, PromQL/LogQL drop-in) як
**один store** → `Grafana` = **один пульт + вбудований unified-alerting = один alert-мозок** (БЕЗ окремого
Alertmanager) → джерела: `Netdata` agent (zero-config host/Docker/Postgres, per-second, remote_write) + Rust-app
`/metrics` (crate `metrics-exporter-prometheus`, НЕ мертвий opentelemetry-prometheus, без OTel-collector) + `Gatus`
(synthetic/cert/DNS, Prometheus-native).

**Усі джерела → один store:**

| Джерело | Колектор | Нотатка |
|---|---|---|
| Хост (CPU/mem/disk/net/temp) | Netdata → remote_write | zero-config |
| Rust-застосунок метрики | `metrics-exporter-prometheus` `/metrics` | без OTel-ваги |
| Логи | `tracing`-JSON → Alloy/Vector → VictoriaLogs | traces пропустити (YAGNI) |
| Postgres/pgrust | Netdata built-in collector | + postgres_exporter пізніше |
| Hetzner (сервер/volume) | Prometheus Hetzner-SD / hcloud_exporter | auto-register targets |
| **Object storage** | `s3_exporter` `s3_latest_file_timestamp` | = **метрика свіжості бекапу** |
| Cloudflare | polling GraphQL Analytics API → VM | community Grafana dashboards |
| Uptime/cert/DNS | Gatus (`probe_ssl_earliest_cert_expiry`) | ззовні процесу застосунку |

**Telegram-алерти:** Grafana unified-alerting → notification-policies по `severity` → **Telegram contact-point
(ОКРЕМИЙ infra-бот, НЕ бізнес-бот замовлень** — щоб page не тонув у чаті замовлень). Anti-fatigue: тільки
critical/warning у Telegram (нижче = лише дашборд), `for:` durations (anti-flap), 30-50% actionable-ratio, місячний
prune. НЕ запускати 2 Telegram-alert-tool паралельно.

**8 pager-правил:** (1) error-rate 5xx/order-fail >2%/5хв; (2) **LATENCY** order-placement p95 SLO 5хв-warn/15хв-crit;
(3) disk >85%-warn/>95%-crit + time-to-full; (4) cert <14д-warn/<3д-crit; (5) **backup-staleness** last-object >1.5×
інтервалу (тихий-fail!); (6) DB connections/locks/WAL; (7) storage quota/раптове-падіння-розміру; (8) uptime external
synthetic storefront+admin 2-fail-crit.

**★ Найбільша пастка — «моніторинг моніторить сам себе»:** стек co-located на боксі, який моніторить → не може
сповістити про власну тотальну відмову (бокс/Docker/Grafana помер = тиша = виглядає-зелено). **ФІКС: зовнішній
DEAD-MAN'S-SWITCH** повністю відв'язаний — безкоштовний heartbeat-pinger (healthchecks.io-style / Cloudflare-Worker-
cron) чекає періодичний ping ВІД сервера, і в мить, коли ping зникає, шле Telegram ОКРЕМИМ шляхом.

---

## 3. ЦІЛЬ 3 — Запобіжники (circuit-breakers · rate-limit · rollbacks · fallbacks)

**Circuit-breakers — ВОСКРЕСИТИ+уніфікувати наявний патерн.** `routing-provider.ts:76-99 CircuitBreaker` (threshold+
cooldown+half-open → `haversineRoute` fallback) = канонічний **degrade-closed «caller ніколи не бачить помилки»**.
Застосувати той самий shape на КОЖНОМУ зовнішньому порту: SMS/notify → queue-local+backoff (не блокує замовлення;
fix sw.js-push D5-F9); **payment → fall-back-cash** («замовлення завжди розміщуване, спосіб оплати = advisory»);
object-storage → mark-degraded keep-serving; PQ-порти → widen-retry (локальний запис авторитетний незалежно від
досяжності піра = local-first payoff). **Правило: resolve-never-reject, log-fallback, ніколи не блокувати money-path.**

**Rollbacks (одна команда, single-box):** deploy = `releases/<sha>/` dir + `current` symlink; deploy = build→health-
check-local→`ln -sfn` (один `rename()` syscall, нема half-deployed стану); **rollback = `ln -sfn releases/<prev> current
&& systemctl reload nginx`** (`deploy-rollback.sh`, викликаний по SSH з телефона). DB = snapshot pgrust-файлу/volume
ПЕРЕД зміною + restore-from-pre-snapshot (не hand-written down); fail-closed schema-guard (halt boot якщо schema-version
≠ binary). Config = у git → `git checkout <prev>` + restart. Whole-box = Hetzner snapshot перед ризиковою зміною.

**Rate-limiting (шарами, ВОСКРЕСИТИ):** Шар-1 Cloudflare-edge IP fixed-window на `/api/*` + OTP/auth (Free=1-rule →
на OTP; throttle не hard-block). Шар-2 `attic/apps-api/src/lib/resilience/rate-limit.ts` = ПОВНИЙ dep-free token-bucket
(tenant+IP-SHA256, inflight-cap проти slow-loris, presets AUTH/ORDER/STRICT/PROMO, recordAbuse) → **винести з attic**
у живий процес. Прив'язати до capability-token (rate-limit by subject-key) — АЛЕ спершу дротувати roster-check на live-
path (B4-F1: зараз unwired → rate-limit неenforced-identity = театр). Валідація: `load/spike.js` (k6) vs Hetzner = regression-gate.

---

## 4. ЦІЛЬ 4 — Latency (дві РІЗНІ задачі)

**(A) Customer-HTTP — оптимізуємо тут.** Ранжовано за впливом на p95/p99 UX:
1. **Local-first WASM-рендеринг** = найбільший структурний виграш (interaction latency → ZERO-network/device-compute,
   kernel decide/fold у WASM). Це вже напрям — тримати.
2. **CF-edge PoPs у регіоні — ПІДТВЕРДЖЕНО: Belgrade / Sofia / Skopje / ТIРАНА** → Balkans/Албанія б'ють ЛОКАЛЬНИЙ
   edge (TLS+static-cache+edge-dynamic), лише uncached round-trip до FSN1-Frankfurt (Hetzner NO-Balkans-DC). Тобто
   FSN1+CF латентно-здорові для Балкан БЕЗ Balkans-origin.
3. **PgBouncer** перед pgrust/Postgres (~0.5ms overhead, p99 падає ~95% від усунення connection-slot-queuing під
   навантаженням) = near-mandatory.
4. **HTTP/3+QUIC** на edge (mobile/lossy tail-latency — кур'єри+клієнти на мобільному).
5. Origin-region FSN1 = non-issue щойно (2) на місці.

**(B) Inter-node PQ-sync — НАВМИСНЕ НЕ оптимізуємо** (D3 locked: DTN/BPv7 reliability-over-latency, store-and-forward —
відхилено libp2p/Zenoh). НЕ застосовувати CDN-трюки до цього шару.

---

## 5. ЦІЛЬ 5 — Менше DevOps-пекла (5-tool toolchain)

Найменша поверхня, яку один оператор тримає в голові:

| Роль | Інструмент | Чому |
|---|---|---|
| Provision | **OpenTofu** | Hetzner-бокс+firewall+volume + CF-DNS одним `apply` = вся edge/origin-топологія з git; rebuild-from-code |
| Deploy/rollback | **Dokploy** | native docker-compose UNMODIFIED, ~350MB idle, один дашборд+health-rollback; push via GHA-webhook; **skip GitOps/Argo (K8s-shaped FOMO)** |
| Secrets | **SOPS + age** | тримаємо age (вже є), обгортка bespoke→SOPS: encrypted-in-git, decrypt-at-deploy, БЕЗ нового сервісу; rotation = re-encrypt+commit. Skip Infisical |
| DB pooling | **PgBouncer** | p99-виграш (§4), майже нульовий overhead |
| DB PITR | **WAL-G** | pgBackRest EOL (archived Apr2026) → WAL→Object-Storage |
| Edge | **Cloudflare** | proxied-DNS, Origin-CA-cert (no-renewal-cron), Free-WAF/DDoS |

Config-as-code (cattle-not-pets, нуль-IaC-сьогодні = найбільший gap): OpenTofu-модуль + cloud-init (Docker/non-root/
SSH-harden/unattended-upgrades) + усе-решта-в-git → full-rebuild = `tofu apply`→cloud-init→Dokploy→deploy = **хвилини,
без SSH-ручного-тюну** (проти snowflake-drift). Skip NixOS (learning-curve не варта для solo). Chmod-600 на secret-файли
(закриває D5-F8 `.env` 0666).

---

## 6. Дані: Supabase-дамп → pgrust (ОДРАЗУ) + дроп Fly/Supabase

**pgrust — рішення оператора, immediate.** Ризик сконцентрований в ОДНИХ воротах (compat), решта — чистий малий restore.

**Реальність дампу (перевірено з quarantined-міграцій):** extensions = ТІЛЬКИ `citext`+`pgcrypto` (обидва stock
contrib — і саме вони «ще не generally-compatible» на pgrust = ТА сама поверхня ризику); НЕМАЄ реальної залежності від
Supabase Auth/Storage/Realtime/Edge-Functions (нуль `@supabase/supabase-js`; ролі authenticated/anon/service_role/
deliveryos_api_user = лише RLS-convention-mirror). 76 таблиць, 140 міграцій. **11 таблиць із RLS-дірами** (fail-open;
`couriers` password_hash+PII + `telegram_login_tokens` owner-auth = 2 HIGH per D2) — **ФІКСИМО, не відновлюємо as-is.**

**Послідовність (verify → cutover → monitor → THEN drop, ніколи drop-first):**
1. Ротнути seeded owner-cred `test@dowiz.com/test123456` (FIRST, freeze-gap risk).
2. **★ COMPAT-GATE** (весь ризик pgrust-immediate): pgrust у docker → `CREATE EXTENSION citext + pgcrypto`. Якщо
   падає → (a) порт contrib-модуля, (b) app-substitute (`gen_random_uuid()`→app-UUID; `citext`→`lower()`/app-normalize +
   переписати DDL), (c) **тільки якщо непрацездатно** — Postgres-17 fallback (оператор пере-вирішує).
3. `pg_restore --list` → confirm public-schema-only. Pre-create 4 convention-ролі. `CREATE EXTENSION`. `pg_restore
   --schema=public --no-owner --no-privileges`. Дані з `session_replication_role=replica`.
4. **Фікс 11 RLS-дір** (D2 R1-R6). Falsifiable: NOBYPASSRLS-роль читає `couriers`/`telegram_login_tokens` → 0 cross-tenant.
5. **Row-count verify** vs інтроспекція (168k/41k/19.7k/8.5k/151/108/21). Mismatch > write-drift = HARD STOP.
6. App cutover (`DATABASE_URL`→pgrust), Fly/Supabase ЛИШАЮТЬСЯ ЖИВІ як rollback. Boot + read/write + E2E green.
7. **Monitor 24-72h** (нуль write-errors). Rollback = repoint назад (миттєво).
8. **THEN Fly**: export volumes(snapshot, permanent-if-destroyed)+secrets(лише-назви)+certs → `scale count 0` →
   `apps suspend` (reversible) → `apps destroy` (IRREVERSIBLE, після monitor).
9. **THEN Supabase delete** (IRREVERSIBLE — wipes auth/storage/edge-fns/realtime/PITR/API-keys; але тут real-loss =
   лише PG-DB = вже в age-дампі; confirm dashboard Edge-fns/Storage/Cron порожні перед delete).

**Rollback:** нічого незворотного до закриття monitor-вікна; після дропу — age-дамп у бакеті = last-resort.

---

## 7. Бекапи / снапшоти / DR (3-2-1-1-0)

**3-2-1-1-0** (3 копії, 2 медіа, 1 offsite, 1 IMMUTABLE, 0 verified-restore-errors). **Топ-gap ЗАРАЗ: НЕМАЄ off-Hetzner
копії НІЧОГО** → один скомпрометований/призупинений Hetzner-акаунт = total-loss.

- **Копія 1 (жива):** pgrust на **окремому attached VOLUME** (не boot-disk — інакше Hetzner snapshot його НЕ покриває).
- **Копія 2 (Hetzner near):** nightly `pg_dump`(-Fc, age) + daily volume-snapshot → бакет `dowiz`, lifecycle tiers
  (hourly24h/daily30d/weekly90d/monthly7y). Сервер НЕ backup-залежний (rebuild-from-Tofu+cloud-init, захищаємо лише
  volume+dump).
- **Копія 3 (off-Hetzner, cold, immutable):** той самий age-`pg_dump` → **rsync.net** (нуль egress, SSH-only=менша
  attack-surface, credential-isolated — переживає скомпрометований Hetzner-акаунт), daily 90д.
- **Immutability:** пересворити бакет для DB-dump-префіксу з **Object-Lock COMPLIANCE mode** (задається при СТВОРЕННІ
  бакета, НЕ retrofit; навіть holder-API-ключа не видалить рано — б'є ransomware+stolen-key). Versioning + lifecycle-
  expire.
- **PITR:** WAL-G continuous → Object-Storage — але як STRETCH після drill (pgrust WAL-binary-format неперевірений;
  logical `pg_dump` = trusted baseline). Event-log replay = complementary (rebuild projections) не substitute.
- **Restore-verify:** воскресити `backup-drill.ts` (download→decrypt→checksum→row-count→smoke→auto-report), swap
  R2→Hetzner + AES-GCM→age, monthly = leg «0».
- **age key custody:** rotate-by-risk (ніколи не знищувати старий — старі бекапи під ним); **lose-key=lose-everything
  no-recovery** → ≥2 offline-копії окремої custody + **age MULTI-RECIPIENT** (шифрувати кожен бекап на 2 pubkeys:
  primary + cold-escrow окремо) + ніколи не тримати ключ у тому ж Hetzner-blast-radius.

---

## 8. Cloudflare hardening + безпека (Cloudflare = тільки edge/hosting)

**Найбільший gap: origin, ймовірно, відкритий/IP-allowlisted → адоптувати TUNNEL першим.**
- **Origin-hiding: Cloudflare Tunnel** (`cloudflared` outbound-only, Hetzner firewall DENY-ALL inbound 80/443 → origin
  БЕЗ публічного listener, immune direct-IP-DDoS/portscan/spoof, FREE) >> IP-allowlist CF-ranges (shared, spoofable,
  false-confidence). DNS→CNAME cfargotunnel. SSH gate (residual risk).
- **DNS:** proxy every web A/AAAA/CNAME, grey-cloud TXT/MX, DNSSEC on, **exposed-IP audit** (старі Fly/R2 A-records =
  leak), delete stale subdomains.
- **TLS:** Full(Strict) + CF Origin-CA-cert (15р); min-TLS-1.2 + TLS1.3; Always-HTTPS; HSTS 1р+subdomains+preload
  (one-way-door, verify subdomains first); AOP mTLS (defense-in-depth).
- **WAF/bot:** Free-Managed-Ruleset + Bot-Fight-Mode; 1-free-rate-limit-rule → OTP/login; Turnstile on OTP/login/
  checkout; custom-rule block `/admin*` non-Access; НЕ geo-block broadly (Balkans+diaspora target).
- **Admin:** Cloudflare Access (Zero-Trust Free ≤50) gating `/admin*`; service-tokens for automation; **hardware-key
  2FA на CF-акаунт** (≥2 keys); scoped per-job API-tokens (додати Zone:Read audit-token; НІКОЛИ Global-Key; не reuse
  R2-token для zone-ops).
- **Caching/latency:** Cache-Rules (cache-everything /assets/* immutable, bypass /api/*) + Smart-Tiered-Cache (region-
  hint FSN1) + HTTP/3 + Early-Hints + Brotli. Argo = measure-first (не default; RUM Албанії спершу).
- **Плани:** Free покриває всі must-do; Pro-$20 = перший upgrade коли 1-rule/5-custom тиснуть; Business не виправданий.
- **Аудит-caveat:** потрібен Zone:Read токен, щоб diff'нути живу зону (поточні токени R2-scoped).

---

## 9. Gap-аналіз (бракуючі шари, пріоритезовано)

**MUST-HAVE (перед реальним трафіком):**
1. **Off-Hetzner backup-копія** (рядок §7) — топ-gap, single-account=total-loss.
2. **Prod-deploy-gate** — D5-F2 unguarded-auto-deploy на `main` CONFIRMED-LIVE → GitHub-Environment-approval / manual
   dokploy-deploy перед торканням live-DB. **Не реінтродукувати цей gap на Hetzner.**
3. **WAL-G PITR** (pgBackRest EOL) → Object-Storage.
4. **Cert-auto-renew** → CF Origin-CA (no-cron).
5. **Secrets-perms** chmod-600 (D5-F8).
6. **Supply-chain** = Trivy у CI (image+IaC+secrets one-pass).
7. **External uptime** (Uptime-Kuma на ОКРЕМІЙ інфрі / free Better-Stack) + dead-man's-switch (§2).
8. **Incident-runbook** (phase5 docs → перенацілити Hetzner; solo-on-call = external-uptime-pages-phone).
9. **Resurrect health/livez** з attic → перенацілити на pgrust/nginx.
10. **Розв'язати суперечності:** оновити ADR-0008 (SQLite→pgrust); зафіксувати `.secrets.local`-vs-«no-Supabase»;
    remote git-scrub (D5-F1: 10 unreachable blobs із ротованими ключами — OPEN gate); полагодити stale visual.yml/e2e/
    audit-sentinel targets.

**LATER (коли є трафік):** paid-WAF (Pro коли payment/PII), формальні SLO/error-budget, Hetzner-staging-parity (2-й
малий бокс той самий OpenTofu), chaos-re-runs, off-box log-shipping, DNS-failover (не варто single-box-MVP).

---

## 10. Хвилі (foundation-first, additive; кожна з RED-gate)

| Хвиля | Що | Блюпринти | Ризик |
|---|---|---|---|
| **W0** | Resurrect-from-attic + перенацілити (health/rate-limit/timeout/sentry/notify/backup/migrations) | OPS-01 | середній |
| **W1** | pgrust provision + **COMPAT-GATE** citext/pgcrypto | OPS-02 | 🔴 (весь ризик тут) |
| **W2** | Restore roles→schema→data→**RLS-fix**→row-verify | OPS-03 | 🔴 (RLS/data) |
| **W3** | App cutover + 24-72h monitor (Fly/Supabase живі) | OPS-04 | середній |
| **W4** | Моніторинг-стек (VM+VLogs+Grafana+Netdata+Gatus) + all-sources + Telegram + **dead-man's-switch** | OPS-07·08·09·10 | середній |
| **W5** | Запобіжники: circuit-breakers uniform + rate-limit un-attic + 1-cmd-rollback + health-gated-deploy | OPS-11·12·13 | середній |
| **W6** | Backups 3-2-1-1-0 (WAL-G + off-Hetzner rsync.net + Object-Lock bucket + age-multi-recipient) | OPS-14·15 | 🔴 (DR/crypto) |
| **W7** | Cloudflare: Tunnel origin-hide + firewall-lockdown + edge-hardening | OPS-16·17 | 🔴 (auth/edge) |
| **W8** | IaC (OpenTofu+cloud-init) + Dokploy + SOPS/age + **gated-prod-deploy** | OPS-18·19 | середній |
| **W9** | Latency: PgBouncer + HTTP/3 + Cache-Rules + local-first | OPS-20 | низький |
| **W10** | Drop Fly → Drop Supabase (тільки після monitor-clean) | OPS-05·06 | 🔴 незворотнє |
| **W11** | Gap-closers (Trivy/cert/chmod/external-uptime/incident-runbook) + розв'язати суперечності + ADR-0008 update | OPS-21·22 | середній |
| **W-RED** | RED proof кожної хвилі | OPS-22 | обов'язковий |

---

## 11. Найбільші ризики + чесна напруга

- **Незворотнє drop до закриття monitor-вікна** (W10) — Fly-destroy + Supabase-delete незворотні; Supabase-delete ще й
  wipes PITR миттєво. Гейт: verified row-count-matched pgrust-restore, що вже обслуговує live-трафік monitor-вікно.
- **pgrust-immediate** — весь ризик в COMPAT-GATE (W1). Якщо citext/pgcrypto непрацездатні на pgrust і app-substitute
  надто дорогий — це точка чесного re-decision (Postgres-17 fallback). Fly/Supabase живі доти = нуль-втрат.
- **Single-vendor Hetzner** — філософська напруга «нуль-зовнішніх-залежностей» vs «не-всі-яйця-в-одному-провайдері».
  Розв'язка: живі копії на Hetzner (дешево/швидко), АЛЕ DB-дамп+age-ключ = credential-isolated off-Hetzner (rsync.net).
- **Ambient-trust / незахищений deploy** — не реінтродукувати D5-F2 auto-deploy на Hetzner; gated-deploy = must.
- **bebop2-меш ще не реальний** — не планувати надійність навколо нього; хаб = реальна одиниця надійності зараз.
- **Моніторинг моніторить себе** — обов'язковий зовнішній dead-man's-switch, інакше тиша=зелено.

---

## 12. Résumé одним абзацом

Один Hetzner-хаб, недоторкане ядро (pgrust-одразу як local-first сховище, kernel decide/fold зверху). Ops-зрілість
**воскрешаємо з `attic/`**, не будуємо з нуля. **Менше точок відмови** (усувати>дублювати; #1-deploy та #2-DB —
операторські, керовані процесом; один redundancy-долар на off-Hetzner-бекап+CF-edge). **Один пульт** — VM+VLogs+Grafana,
усі джерела в одному store, Telegram-алерти окремим ботом, + зовнішній dead-man's-switch. **Запобіжники** — circuit-
breakers (resonate-never-reject, cash-survives), rate-limit (un-attic token-bucket + CF-edge), 1-cmd-rollback (symlink-
swap). **Latency** — local-first-WASM > CF-Balkans-PoPs(Тірана!) > PgBouncer > HTTP/3. **Менше DevOps-пекла** — 5 tools
(OpenTofu+Dokploy+SOPS/age+PgBouncer+WAL-G) + Cloudflare, усе rebuild-from-git за хвилини. **Дані** — verify→cutover→
monitor→THEN-drop, ніколи drop-first. **Бекапи** — 3-2-1-1-0 з off-Hetzner immutable-копією (топ-gap). **Edge** — CF-
Tunnel ховає origin, Full-Strict+WAF+Access+hardware-2FA. Найбільший ворог — незворотній крок до чистого monitor-вікна.
