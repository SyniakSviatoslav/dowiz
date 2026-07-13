# Ops / Reliability Plan — RESEARCH CONSPECT (7 lanes)

> Дата: 2026-07-13 · 7 паралельних дослідницьких смуг + інвентар, зведені для
> [OPS-RELIABILITY-PLAN.md](./OPS-RELIABILITY-PLAN.md) + [BLUEPRINTS](./BLUEPRINTS-OPS-RELIABILITY.md).
> Операторські рішення: **pgrust одразу** · дроп Fly+Supabase · Cloudflare=edge/hosting-only.

## Lane 1 — Supabase→pgrust + Fly/Supabase drop
pgrust ([malisper/pgrust], AGPL-3.0) = REAL але EXPERIMENTAL (passes regression, disk-compat PG18.3, BUT extensions
pgcrypto/PostGIS/pg_cron "not generally compatible yet", not-perf-tuned). Дамп потребує ТІЛЬКИ citext+pgcrypto (обидва
stock contrib = саме ризикова поверхня pgrust). НЕМАЄ реальної Supabase Auth/Storage/Realtime/Edge залежності (нуль
@supabase/supabase-js; ролі=RLS-convention-mirror). 76 таблиць, 140 міграцій, 11 RLS-дір (couriers/telegram_login_tokens
HIGH per D2 red-team). Restore: --no-owner --no-privileges, pre-create-ролі, CREATE EXTENSION, session_replication_role=
replica, FIX-RLS-not-restore, row-verify. Drop: verify→cutover→24-72h-monitor→Fly-export+destroy→Supabase-delete
(незворотні; PART1-DECOMMISSION runbook exists). Decommission-runbook + D2-RLS-audit already in-repo (today).

## Lane 2 — Single-pane monitoring + Telegram
★VictoriaMetrics+VictoriaLogs+Grafana (unified-alerting=один-мозок, no-Alertmanager) + Netdata(zero-config remote_write)
+ Gatus(synthetic/cert Prometheus-native). Rust=metrics-exporter-prometheus (NOT dead opentelemetry-prometheus). All-
sources→one-store (host/app/PG/Hetzner-hcloud_exporter/object-storage-s3_exporter=backup-freshness/CF-GraphQL-analytics/
Gatus-cert). Telegram=Grafana-policies-by-severity→SEPARATE-infra-bot (not business-bot). 8-pager-rules(5xx/latency-p95/
disk/cert/backup-staleness/DB/storage/uptime). ★BIGGEST-TRAP=monitoring-monitors-itself→external DEAD-MAN'S-SWITCH
(healthchecks.io/CF-Worker-cron). Business-Telegram-pipeline exists (telegram-notifications-actions) = keep separate.

## Lane 3 — Reliability: FMEA/breakers/rollbacks/rate-limit
"single Hetzner"=owner's-own-node (blast-radius contained). bebop2-mesh NOT-real (B4: stub-transport, unwired-auth, zero-
delivery-domain)→don't-plan-around. FMEA: #1 deploy-breaks-prod + #2 DB-corruption DOMINATE (operator-caused=process-
controllable); #7/#8 CF/region-outage=consolidation-tax(rare-total)=the-1-redundancy-dollar. Reduce-failure-mode>redundify.
Now: CF-orange+Tunnel + CF-Always-Online-static-floor + Litestream/WAL-G + Hetzner-snapshot + cold-boot-runbook-as-script.
NOT-now: 2nd-node/read-replica/mesh-redundancy. ★CircuitBreaker LIVE (routing-provider.ts, degrade-closed haversine-
fallback)→apply-uniform (SMS→queue, payment→cash, storage→degrade, resolve-never-reject). Rollback=releases/<sha>+symlink-
atomic-swap (ln -sfn, 1-cmd, SSH-from-phone, closes-D5-F2). Rate-limit=CF-edge + attic-token-bucket-un-attic (tenant+IP+
inflight, AUTH/ORDER presets), validate-k6-spike.js.

## Lane 4 — Backups/snapshots/DR
3-2-1-1-0 (3-copies/2-media/1-offsite/1-IMMUTABLE/0-verified-errors). Single-Hetzner FAILS "1-offsite" (compromised-account=
total-loss). Copy1 live-PG-on-attached-VOLUME(not-boot-disk—Hetzner-snapshot-doesn't-cover-volumes!). Copy2 Hetzner-near
(nightly pg_dump-age + volume-snapshot, lifecycle). Copy3 OFF-Hetzner ★rsync.net(zero-egress/SSH-only/credential-isolated)
OR Backblaze-B2. Object-Lock COMPLIANCE (set-at-creation NOT-retrofit, even-API-key-holder-can't-delete=beats-ransomware+
stolen-key). pgBackRest DEAD(archived-Apr2026)→WAL-G for PITR(stretch, pgrust-WAL-unverified→logical-dump=baseline). Config-
as-code=real-resilience(Terraform+cloud-init, snapshots-only-stateful-data). age: multi-recipient(primary+cold-escrow),
never-destroy-old-key, never-colocate-key-with-backups. ★TOP-GAP=no-off-Hetzner-copy-today.

## Lane 5 — Cloudflare hardening
★Origin-hiding=Cloudflare TUNNEL(cloudflared outbound-only, firewall-deny-all-inbound, FREE)>>IP-allowlist(shared-spoofable).
DNS: proxy-all+DNSSEC+exposed-IP-audit(old-Fly/R2-records-leak)+delete-stale-subdomains. TLS: Full(Strict)+Origin-CA-cert-
15yr+min-1.2+HSTS-preload(one-way-door)+AOP-mTLS. WAF: Free-Managed-Ruleset+Bot-Fight+1-rate-limit→OTP+Turnstile-forms+
custom-/admin; no-broad-geo-block(Balkans+diaspora). Admin: Access-ZeroTrust-free-gating-/admin + hardware-key-2FA-on-CF-
account + scoped-per-job-tokens(add-Zone:Read, never-Global-Key). Cache: Cache-Rules(cache-/assets/*-bypass-/api/*)+Tiered-
Cache(FSN1-hint)+HTTP/3+Early-Hints. Argo=measure-first. FREE-sufficient-all-must-do, Pro-$20-when-constrained. Audit-
caveat: need-Zone:Read-token(current=R2-only).

## Lane 6 — DevOps + latency + gaps
5-tool: OpenTofu(provision-Hetzner+CF-DNS-one-apply) + Dokploy(compose-unmodified,~350MB,push-GHA-webhook,skip-GitOps/Argo)
+ SOPS+age(encrypted-in-git,skip-Infisical) + PgBouncer(p99↓~95%) + WAL-G(pgBackRest-EOL). Config-as-code=largest-gap(zero-
IaC-today). Latency(2-separate): (A)customer-HTTP: local-first-WASM(zero-network)>CF-edge-PoPs-Belgrade/Sofia/Skopje/TIRANA
(FSN1+CF-latency-sound-no-Balkans-origin)>PgBouncer>HTTP/3>FSN1-non-issue; (B)inter-node-PQ=DELIBERATELY-not-optimized(D3
DTN/BPv7-reliability-over-latency). GAPS-must-now: WAL-G-PITR, gated-prod-deploy(D5-F2-live), CF-Origin-CA-cert, chmod-600
(D5-F8), Trivy-CI, external-uptime. Later: paid-WAF, SLO, staging-parity, chaos, log-shipping, DNS-failover.

## Lane 7 — Inventory: CURRENT OPS REALITY
★D1(2026-07-12 AUTHORITATIVE)=centralized-server DROPPED("no server/central-DB/Supabase/Fly")→apps/api+worker+packages/db
QUARANTINED-to-attic, deploy-job-stripped. LIVE=static-SPA-ONLY(Dockerfile→nginx, 1-CI-gate, NO deploy/server/rate-limit/
live-health/wired-alerting). BUT main-prod STILL-Fly + .secrets.local(today)-has-live-Supabase/R2/CF-creds=CONTRADICTION.
★REUSABLE-FROM-ATTIC(resurrect-not-rebuild): health.ts(11-checks), rate-limit.ts(token-bucket), timeout.ts, sentry.ts(PII-
redaction), notifications/*(telegram/email/push+outbox+quiet-hours+retry), backup-{drill,restore,verify,list}.ts, node-pg-
migrate-140-migrations, boot-guard-pattern, k6-spike.js. ONE-live-primitive=CircuitBreaker(routing-provider). Orphaned=
audit-sentinel-watchdog(Telegram-escalate, dead-Fly-target→repoint)+stale-visual.yml/e2e/spike.js. bebop2=strong-crypto
ZERO-reliability(no-retransmit/custody, plaintext-WSS-transport, unenforced-authz per-B3/B2)+ZERO-delivery-domain. 10-
unreachable-git-blobs-still-have-rotated-keys(remote-scrub OPEN-gate D5-F1). Config=Zod-EnvSchema-~150-keys(targets-retired-
server). GAPS: no-live-health/rate-limit/OTel/deploy-pipeline/enforced-backup-cadence/bebop2-reliability.

---
Ground-truth anchors: MANIFESTO.md, DECISIONS.md D1/D3, docs/adr/0008-local-sqlite-pq-at-rest.md, docs/MIGRATION-PLAN.md,
docs/red-team/2026-07-13/{PART1-DECOMMISSION,D2-rls,D5-reliability-ops,MASTER-SYNTHESIS}.md, packages/platform/routing-
provider.ts (CircuitBreaker LIVE), attic/apps-api/{routes/health.ts,lib/resilience/{rate-limit,timeout}.ts,lib/sentry.ts,
notifications/*}, attic/packages-db/migrations/* (140, extensions-and-enums=citext+pgcrypto, create-supabase-roles, force-
rls), scripts/backup-{drill,restore,verify,list}.ts, load/spike.js, audit-sentinel/, .secrets.local (today), bebop2 red-
team B2/B3/B4. External: victoriametrics/grafana/netdata/gatus, rsync.net/backblaze-b2, hetzner object-lock/snapshot docs,
wal-g, pgbackrest-EOL, cloudflare tunnel/full-strict/AOP/access/2fa/tiered-cache/http3 docs, dokploy/opentofu/sops-age,
pgbouncer, cloudflare-belgrade/tirana-PoPs, malisper/pgrust.
