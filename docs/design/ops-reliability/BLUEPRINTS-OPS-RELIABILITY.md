# dowiz / bebop2 — Ops · Reliability · Migration — BLUEPRINTS

> **Дата:** 2026-07-13 · Супроводжує [OPS-RELIABILITY-PLAN.md](./OPS-RELIABILITY-PLAN.md).
> 22 одиниці OPS-01..22. Кожна: **Мета · Межа (чіпаємо/НЕ чіпаємо) · Форма · Reuse/джерело · RED · Хвиля.**
> НЕ код — блюпринт. Принцип: **воскресити з `attic/`, не будувати з нуля.** 🔴 = red-line (незворотнє/crypto/
> auth/data/money → human gate).

---

## Група A — Дані: Supabase → pgrust + дроп (W1-W3, W10)

### OPS-01 · Resurrect ops-code з attic + перенацілити
- **Мета:** повернути зрілі ops-примітиви з `attic/` у живий процес, перенацілені на Hetzner/pgrust/nginx.
- **Межа:** ЧІПАЄМО — вибіркове воскресіння + re-point. НЕ ЧІПАЄМО — логіку (переюз як-є, лише backend-adapter/env).
- **Форма:** un-attic: `health.ts` (11 перевірок → перенацілити PG→pgrust, R2→Hetzner, прибрати мертві), `rate-limit.ts`
  (token-bucket), `timeout.ts` (withTimeout/retryWithBackoff), `sentry.ts` (PII-redaction), `notifications/*` (telegram/
  email/push adapters+outbox+quiet-hours+retry), `backup-{drill,restore,verify,list}.ts` (swap R2→Hetzner, AES-GCM→age),
  node-pg-migrate 140-міграцій. Live-примітив CircuitBreaker (routing-provider) лишається. Перенацілити orphaned
  `audit-sentinel/` watchdog з мертвого Fly → Hetzner.
- **Reuse:** усе з інвентаря lane7. **RED:** воскрешений health `/livez`+`/health` відповідає на Hetzner; backup-drill
  round-trips проти бакета. **Хвиля:** W0.

### OPS-02 · pgrust provision + COMPAT-GATE
- **Мета:** підняти pgrust (рішення оператора immediate) + довести сумісність до restore.
- **Межа:** ЧІПАЄМО — pgrust-інстанс на окремому attached-volume. НЕ ЧІПАЄМО — ядро/дамп.
- **Форма:** pgrust у docker на Hetzner-volume. ★COMPAT-GATE: `CREATE EXTENSION citext + pgcrypto`. Fail → (a) порт
  contrib, (b) app-substitute (`gen_random_uuid()`→app-UUID; `citext`→`lower()`+DDL-rewrite), (c) Postgres-17 fallback
  (оператор re-decides). Оновити ADR-0008 (SQLite→pgrust, red-line-ADR).
- **Reuse:** — (pgrust external, AGPL-3.0). **RED:** обидва extensions працюють АБО substitute-план green; якщо ні —
  явний STOP/re-decision, НЕ мовчазний workaround. **Хвиля:** W1. 🔴

### OPS-03 · Restore roles→schema→data + RLS-fix + row-verify
- **Мета:** відновити дамп у pgrust правильно (не as-is).
- **Межа:** ЧІПАЄМО — restore + RLS-патч. НЕ ЧІПАЄМО — вихідну Supabase (жива як rollback).
- **Форма:** `pg_restore --list`→public-only; pre-create 4 convention-ролі; `CREATE EXTENSION`; `pg_restore --schema=
  public --no-owner --no-privileges`; дані `session_replication_role=replica`; **ФІКС 11 RLS-дір (D2 R1-R6)** — couriers/
  telegram_login_tokens HIGH; sequence-ownership confirm.
- **Reuse:** node-pg-migrate ролі/RLS міграції. **RED:** (1) NOBYPASSRLS-роль читає couriers/telegram_login_tokens → 0
  cross-tenant; (2) row-count = 168k/41k(archived-subset)/19.7k/8.5k/151/108/21, mismatch>write-drift = HARD STOP.
  **Хвиля:** W2. 🔴

### OPS-04 · App cutover + monitor-вікно
- **Мета:** перевести застосунок на pgrust зі збереженням миттєвого rollback.
- **Межа:** ЧІПАЄМО — `DATABASE_URL`→pgrust. НЕ ЧІПАЄМО — Fly/Supabase (живі паралельно).
- **Форма:** repoint DATABASE_URL, boot + read/write round-trip + E2E green; **monitor 24-72h** нуль-write-errors.
- **RED:** rollback = repoint-назад миттєво (обидва стеки живі). **Хвиля:** W3.

### OPS-05 · Fly export + destroy
- **Мета:** зняти Fly ТІЛЬКИ після clean monitor.
- **Межа:** ЧІПАЄМО — Fly-teardown. НЕ ЧІПАЄМО — pgrust (уже live).
- **Форма:** export volumes(snapshot, permanent-if-destroyed)+secrets(лише-назви, values-unretrievable)+certs → у бакет;
  `scale count 0`→`apps suspend`(reversible)→`apps destroy`(IRREVERSIBLE).
- **Reuse:** PART1-LIVE-PROD-DECOMMISSION.md runbook. **RED:** post-destroy old-URL 404; export-артефакти в бакеті ДО
  destroy. **Хвиля:** W10. 🔴 незворотнє.

### OPS-06 · Supabase project delete
- **Мета:** видалити Supabase ТІЛЬКИ після Fly-drop + monitor-clean.
- **Межа:** ЧІПАЄМО — Supabase-teardown. НЕ ЧІПАЄМО — age-дамп (last-resort).
- **Форма:** dashboard-confirm Edge-fns/Storage/Cron порожні (live-check, grep не замінить) → delete (IRREVERSIBLE:
  wipes auth/storage/realtime/PITR/API-keys; real-loss тут = лише PG-DB = в age-дампі).
- **RED:** age-дамп verified-restorable ДО delete = єдиний recovery після. **Хвиля:** W10. 🔴 незворотнє.

---

## Група B — Один моніторинг + Telegram (W4)

### OPS-07 · Monitoring stack (single pane)
- **Мета:** один real-time store+пульт, мінімум ops на 1 боксі.
- **Межа:** ЧІПАЄМО — новий observability compose-стек. НЕ ЧІПАЄМО — застосунок (лише експонує /metrics).
- **Форма:** VictoriaMetrics(single) + VictoriaLogs + Grafana (один пульт + unified-alerting=один alert-мозок, БЕЗ
  Alertmanager) + Netdata-agent(remote_write) + Gatus. Grafana за private-net/Tunnel/Tailscale — НЕ публічно.
- **Reuse:** — (greenfield). **RED:** один Grafana-URL показує host+app+DB+edge+synthetic з одного store. **Хвиля:** W4.

### OPS-08 · All-sources ingestion
- **Мета:** усі джерела в один store.
- **Межа:** ЧІПАЄМО — колектори/exporters. НЕ ЧІПАЄМО — джерела (read-only scrape/poll).
- **Форма:** host=Netdata; Rust=`metrics-exporter-prometheus` /metrics (НЕ мертвий opentelemetry-prometheus); логи=
  tracing-JSON→Alloy→VLogs; Postgres=Netdata-collector; Hetzner=hcloud_exporter/Prometheus-SD; **object-storage=
  s3_exporter (`s3_latest_file_timestamp`=backup-freshness)**; Cloudflare=GraphQL-Analytics poll→VM; cert/DNS=Gatus.
- **RED:** backup-freshness метрика жива (age об'єкта); CF-analytics у VM. **Хвиля:** W4.

### OPS-09 · Telegram alert routing + 8 pager-правил
- **Мета:** тільки critical/important → Telegram, без fatigue, окремий бот.
- **Межа:** ЧІПАЄМО — Grafana notification-policies + окремий infra-бот. НЕ ЧІПАЄМО — бізнес-бот замовлень.
- **Форма:** unified-alerting → policy by `severity` → Telegram-contact-point (ОКРЕМИЙ infra-бот). 8 правил: 5xx-rate/
  **latency-p95-SLO**/disk/cert/**backup-staleness**/DB/storage/uptime. `for:`-durations, 30-50% actionable, monthly-prune.
- **Reuse:** attic notifications/telegram adapter (як референс). **RED:** синтетичне порушення SLO → Telegram-page ≤1хв;
  low-severity → лише дашборд. **Хвиля:** W4.

### OPS-10 · External dead-man's-switch
- **Мета:** сповіщати про ТОТАЛЬНУ відмову боксу/стеку (моніторинг не моніторить себе).
- **Межа:** ЧІПАЄМО — зовнішній heartbeat поза боксом. НЕ ЧІПАЄМО — on-box стек.
- **Форма:** healthchecks.io-style / Cloudflare-Worker-cron чекає періодичний ping ВІД сервера; ping зник → Telegram
  ОКРЕМИМ шляхом. Повністю decoupled.
- **RED:** вимкнути сервер → зовнішній switch пейджить (не тиша). **Хвиля:** W4.

---

## Група C — Запобіжники (W5)

### OPS-11 · Circuit-breakers uniform
- **Мета:** degrade-closed на КОЖНОМУ зовнішньому порту.
- **Межа:** ЧІПАЄМО — обгортка портів. НЕ ЧІПАЄМО — money/order-path (ніколи не блокувати).
- **Форма:** застосувати shape `routing-provider.ts CircuitBreaker` (threshold+cooldown+half-open, resolve-never-reject):
  SMS/notify→queue+backoff; **payment→cash-fallback**; object-storage→degrade; PQ-ports→widen-retry (local write
  authoritative). Named-config, не magic-numbers. Fix sw.js-push (D5-F9).
- **Reuse:** ★CircuitBreaker вже live. **RED:** dead-payment-port → order(cash) не блокується; assert degrade-closed.
  **Хвиля:** W5.

### OPS-12 · Layered rate-limiting (un-attic)
- **Мета:** захистити OTP/auth/API, шарами.
- **Межа:** ЧІПАЄМО — винести rate-limit з attic + CF-edge-rule. НЕ ЧІПАЄМО — логіку token-bucket.
- **Форма:** Шар-1 CF-edge IP fixed-window на /api/*+OTP (Free=1-rule→OTP, throttle). Шар-2 `attic/.../rate-limit.ts`
  token-bucket (tenant+IP-SHA256, inflight-cap anti-slow-loris, presets AUTH/ORDER/STRICT/PROMO) → у живий процес.
  Rate-limit by capability-subject-key (АЛЕ дротувати roster-check на live-path першим — B4-F1).
- **Reuse:** rate-limit.ts + load/spike.js. **RED:** `load/spike.js` k6 vs Hetzner → rateLimited/429+Retry-After/5xx<1%.
  **Хвиля:** W5.

### OPS-13 · One-command rollback + health-gated deploy
- **Мета:** незалежний від CI rollback за одну команду; deploy без half-deployed-стану.
- **Межа:** ЧІПАЄМО — release-dir+symlink на боксі. НЕ ЧІПАЄМО — ядро.
- **Форма:** `releases/<sha>/` + `current`-symlink; deploy=build→health-check-local→`ln -sfn`(один rename()); **rollback
  =`ln -sfn releases/<prev> current && systemctl reload nginx`** (deploy-rollback.sh, по SSH з телефона). DB=pre-change
  snapshot. Fail-closed schema-guard (halt якщо schema≠binary).
- **Reuse:** boot-guard pattern. **RED:** deploy битого-релізу → health-fail → symlink НЕ свапнутий, прод недоторканий;
  rollback ≤1 команда. Закриває D5-F2. **Хвиля:** W5.

---

## Група D — Бекапи / DR (W6)

### OPS-14 · Backup topology 3-2-1-1-0
- **Мета:** DR-sound бекапи з off-Hetzner immutable-копією (топ-gap).
- **Межа:** ЧІПАЄМО — backup-конвеєр. НЕ ЧІПАЄМО — money (ніколи write-behind).
- **Форма:** Copy1 live-pgrust на attached-VOLUME (не boot-disk). Copy2 Hetzner-near: nightly `pg_dump`(-Fc,age)+daily
  volume-snapshot→бакет lifecycle-tiers. Copy3 off-Hetzner: age-dump→**rsync.net** daily 90д (credential-isolated).
  **Object-Lock COMPLIANCE** bucket для DB-dump (set-at-creation, не retrofit). WAL-G PITR = stretch після drill
  (pgrust-WAL неперевірений; logical-dump=baseline). Restore-verify: воскресити backup-drill monthly.
- **Reuse:** backup-{drill,restore,verify}.ts, runbooks.md, health backup-drift-check. **RED:** flip-1-byte-ciphertext →
  restore REFUSES; off-Hetzner копія існує (закриває top-gap); monthly restore-drill green. **Хвиля:** W6. 🔴

### OPS-15 · age key custody
- **Мета:** ключ = єдиний trust-root; втрата = все → надлишкова custody.
- **Межа:** ЧІПАЄМО — key-custody-процес. НЕ ЧІПАЄМО — наявний age-ключ (offline вже збережено).
- **Форма:** **age MULTI-RECIPIENT** (кожен бекап на 2 pubkeys: primary + cold-escrow окремо); ≥2 offline-копії окремої
  custody; ніколи не тримати ключ у тому ж Hetzner-blast-radius; rotate-by-risk, старий-ключ-ніколи-не-знищувати.
- **RED:** втрата primary-ключа → escrow-ключ дешифрує бекап (assert); ключ НЕ в Hetzner-бакеті. **Хвиля:** W6. 🔴

---

## Група E — Cloudflare / edge (W7)

### OPS-16 · Cloudflare Tunnel origin-hiding + firewall lockdown
- **Мета:** прибрати публічний inbound-listener origin (найбільший security-gap).
- **Межа:** ЧІПАЄМО — cloudflared + Hetzner firewall. НЕ ЧІПАЄМО — застосунок (слухає лише на loopback/tunnel).
- **Форма:** `cloudflared` systemd outbound-only; Hetzner firewall DENY-ALL inbound 80/443; DNS→CNAME cfargotunnel;
  SSH gated (tunnel-private-net/bastion-allowlist). Fallback якщо Tunnel-invasive: AOP+CF-IP-allowlist+fail2ban.
- **RED:** прямий `curl` по origin-IP:443 → connection-refused/timeout (нема listener); через CF → 200. **Хвиля:** W7. 🔴

### OPS-17 · Cloudflare edge hardening
- **Мета:** DNS/TLS/WAF/Access/2FA-хардненг на Free-tier.
- **Межа:** ЧІПАЄМО — CF-zone-налаштування. НЕ ЧІПАЄМО — origin-логіку.
- **Форма:** proxy-all-web-DNS+DNSSEC+exposed-IP-audit; Full(Strict)+Origin-CA-cert+min-TLS-1.2+HSTS-preload; AOP-mTLS;
  Free-Managed-Ruleset+Bot-Fight; 1-rate-limit→OTP; Turnstile→OTP/login/checkout; Access-gating /admin*(Zero-Trust-free);
  **hardware-key-2FA на CF-акаунт**(≥2); scoped-tokens(Zone:Read-audit, no-Global-Key); Cache-Rules+Tiered-Cache+HTTP/3+
  Early-Hints+Brotli; Argo=measure-first.
- **RED:** /admin без Access-session → deny до origin; TLS-mode Full-Strict verified; Zone:Read-token audit passes.
  **Хвиля:** W7. 🔴

---

## Група F — DevOps / IaC (W8)

### OPS-18 · IaC: OpenTofu + cloud-init
- **Мета:** сервер rebuildable-from-git за хвилини (cattle-not-pets); нуль-IaC-сьогодні = найбільший gap.
- **Межа:** ЧІПАЄМО — новий OpenTofu-модуль + cloud-init. НЕ ЧІПАЄМО — застосунок.
- **Форма:** OpenTofu (Hetzner server+firewall+volume + Cloudflare-DNS, обидва офіційні провайдери, один state, один
  apply) + cloud-init (Docker/non-root-deploy-user/SSH-harden/unattended-upgrades). Skip NixOS/Ansible-as-layer.
- **RED:** `tofu destroy`+`tofu apply` → ідентична топологія за хвилини; нуль-manual-SSH-tune. **Хвиля:** W8.

### OPS-19 · Dokploy deploy + gated-prod + SOPS/age secrets
- **Мета:** one-command deploy/rollback/health + захищений prod-deploy + secrets-in-git.
- **Межа:** ЧІПАЄМО — Dokploy + SOPS-обгортка + deploy-gate. НЕ ЧІПАЄМО — age-примітив (лишається).
- **Форма:** Dokploy (native compose-unmodified, ~350MB, push via GHA-webhook, skip-GitOps). ★GATED-PROD-DEPLOY (GitHub-
  Environment-approval / manual dokploy-step перед live-DB — НЕ реінтродукувати D5-F2). Secrets: age→**SOPS** (encrypted-
  in-git, decrypt-at-deploy); chmod-600 (D5-F8); rotation=re-encrypt+commit. Skip Infisical.
- **Reuse:** verify-secrets.ts gate. **RED:** unapproved push НЕ торкається live-DB (gate blocks); secrets-file 600.
  Закриває D5-F2/F8. **Хвиля:** W8.

---

## Група G — Latency + gap-closers (W9, W11)

### OPS-20 · Latency stack
- **Мета:** оптимізувати p95/p99 customer-HTTP (inter-node — навмисне НЕ чіпаємо, D3).
- **Межа:** ЧІПАЄМО — PgBouncer + CF-edge + WASM-render. НЕ ЧІПАЄМО — inter-node PQ-транспорт (D3 locked).
- **Форма:** ранжовано: (1) local-first-WASM-render(zero-network interaction); (2) CF-edge-PoPs Belgrade/Sofia/Skopje/
  **Тірана** (FSN1+CF латентно-здорові без Balkans-origin); (3) **PgBouncer** перед pgrust (p99↓~95% від connection-
  queuing); (4) HTTP/3+QUIC edge (mobile-tail); (5) FSN1-non-issue.
- **RED:** p99 order-placement під навантаженням (k6) падає з PgBouncer; CF-cache-hit на /assets/*. **Хвиля:** W9.

### OPS-21 · Gap-closers batch
- **Мета:** закрити must-have бракуючі шари.
- **Межа:** ЧІПАЄМО — CI-кроки + cert + external-uptime + runbook. НЕ ЧІПАЄМО — ядро.
- **Форма:** Trivy у CI (image+IaC+secrets one-pass); CF-Origin-CA-cert (no-renewal-cron); external-uptime (Uptime-Kuma
  на окремій інфрі / Better-Stack) + dead-man's-switch(OPS-10); incident-runbook (phase5→Hetzner); resurrect health/livez.
- **Reuse:** phase5 docs, audit-sentinel. **RED:** Trivy-gate red на known-CVE; external-uptime пейджить при down.
  **Хвиля:** W11.

### OPS-22 · Розв'язати суперечності + RED-proof-suite
- **Мета:** прибрати stale/суперечливе + один RED-gate на весь план.
- **Межа:** ЧІПАЄМО — docs/config-узгодження + test-крейт. НЕ ЧІПАЄМО — продакшн.
- **Форма:** оновити ADR-0008(SQLite→pgrust); зафіксувати `.secrets.local`-vs-«no-Supabase»; **remote git-scrub** (D5-F1:
  10 unreachable-blobs із ротованими ключами = OPEN gate); полагодити stale visual.yml/e2e/audit-sentinel-targets.
  RED-suite: core-untouched, RLS-cross-tenant=0, row-count-match, backup-tamper-refuse, origin-no-direct-access,
  gated-deploy-blocks-unapproved, dead-man's-switch-fires, rollback-1-command.
- **RED:** кожен рядок reachable red→green, regression-ledger-row, нуль expect(true)/skip. **Хвиля:** W11/W-RED. 🔴

---

## Зведення: блюпринт → ціль → хвиля

| BP | Назва | Ціль оператора | Хвиля | Red-line |
|---|---|---|---|---|
| OPS-01 | Resurrect-from-attic | усі (менше-роботи) | W0 | — |
| OPS-02 | pgrust + COMPAT-GATE | дані | W1 | 🔴 |
| OPS-03 | Restore+RLS-fix+verify | дані | W2 | 🔴 |
| OPS-04 | Cutover+monitor | дані | W3 | — |
| OPS-05 | Fly destroy | дроп | W10 | 🔴 незворотнє |
| OPS-06 | Supabase delete | дроп | W10 | 🔴 незворотнє |
| OPS-07 | Monitoring stack | 1 пульт | W4 | — |
| OPS-08 | All-sources ingestion | 1 пульт | W4 | — |
| OPS-09 | Telegram routing + 8 rules | алерти | W4 | — |
| OPS-10 | Dead-man's-switch | алерти | W4 | — |
| OPS-11 | Circuit-breakers uniform | запобіжники | W5 | — |
| OPS-12 | Rate-limit un-attic | запобіжники | W5 | — |
| OPS-13 | 1-cmd rollback + health-gate | запобіжники | W5 | — |
| OPS-14 | Backup 3-2-1-1-0 | менше-відмов | W6 | 🔴 |
| OPS-15 | age key custody | безпека | W6 | 🔴 |
| OPS-16 | CF Tunnel origin-hide | менше-відмов+безпека | W7 | 🔴 |
| OPS-17 | CF edge hardening | безпека | W7 | 🔴 |
| OPS-18 | OpenTofu IaC | менше-DevOps | W8 | — |
| OPS-19 | Dokploy+gated-deploy+SOPS | менше-DevOps | W8 | — |
| OPS-20 | Latency stack | latency | W9 | — |
| OPS-21 | Gap-closers | gaps | W11 | — |
| OPS-22 | Contradictions + RED-suite | усі | W11 | 🔴 |

**Інваріант усіх 22:** воскресити>будувати; менше-точок-відмови>надлишковість; degrade-closed (cash-survives); ніколи
drop-first (verify→cutover→monitor→drop); off-Hetzner immutable-копія; origin-hidden; один-пульт+dead-man's-switch;
gated-deploy (не реінтродукувати auto-deploy); pgrust-ризик у ОДНИХ COMPAT-воротах з чесним fallback.
