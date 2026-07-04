# REBUILD-MAP — Master Traceability & Execution Plan (Rust + Astro/Svelte)

- **Date:** 2026-07-04 · **Status:** SYNTHESIS of 6 inventory lanes (all complete, zero-delta reconciled censuses)
- **Program:** complete rebuild per operator decision (`06-complete-rebuild-stack.md`); DB schema UNCHANGED,
  data never migrates; strangler-by-surface; **the E2E net is the parity oracle**.
- **Inventory sources (the map's ground truth — this doc indexes, never duplicates):**
  `inventory/10-api-realtime-jobs.md` (3,125 lines) · `inventory/11-frontend-surface.md` (492) ·
  `inventory/12-data-layer.md` · `inventory/13-scripts-ops-guardrails.md` · `inventory/14-crosscutting-proofnet.md` (633) ·
  `docs/research/2026-07-04-customer-distribution-channels.md` (channel-hub, 742)
- **Prime rule:** every census row below exists in `traceability.csv` (§4) with a target + proof + status;
  the map-coverage gate (§5) makes completeness a CI property, not a claim.

---

## 1. The complete functional surface (consolidated counts — each machine-re-derivable)

| Namespace | Count | Source § | Rebuild target family |
|---|---|---|---|
| HTTP routes | **236** (+2 non-route handlers) | 10 §1 | axum routers per surface crate; utoipa → OpenAPI SSOT |
| WS inbound / outbound / rooms | **5 / 24 / 5** (33 publish sites; 34 internal bus topics kept separate) | 10 §2 | axum-WS + PgListener; ONE Rust event enum (kills the 19 silently-unhandled types) |
| Job queues / crons | **30 live** (+3 dead) / **23 UTC crons**; 1 transactional-enqueue site | 10 §3 | hand-rolled SKIP LOCKED + PgListener worker (verdict 10 §5); enqueue-in-txn preserved |
| Server integrations | **16 areas** (13 active, 3 dead; mem0ai = drop) | 10 §4 | per-integration modules; media (libvips FFI / tesseract sidecar / pdfium) → `media-worker` image |
| Boot/env/flags (server) | **80 EnvSchema + 48 raw reads** (~20 shadow) · **35 flags** | 10 §5, 14 §1-2 | ONE Rust config struct, preflight-validated; raw-read drift eliminated by construction |
| FE routes / pages | **27 / 35** | 11 §1 | Astro route groups `/s/[slug]` · `/admin` · `/courier` |
| Components | **67** (56 ui + 11 web; 8 dead-candidates) | 11 §3 | **27 Svelte islands** + Astro-static; framer-motion → Svelte transitions |
| i18n keys | **1,445 × 3 locales** (13 dynamic families) | 11 §5 | Paraglide-JS 2 (verdict 11 §5; 61.4 kB gz catalog → per-message tree-shaking) |
| Client flags | **19 VITE_*** (2 dead seams) | 11 §6.3, 14 §1 | Astro env schema; every flag declared + CI build-arg'd (closes the 0-build-args gap) |
| Tables | **86 live** (84 from 157 migrations + 2 out-of-band) · 62 tenant-scoped · 16 money | 12 §1 | UNCHANGED; sqlx query_as + newtypes (`Lek(i64)`); `ci-schema-drift` live diff = Phase-0 gate |
| RLS policies | **103** (FORCE 56 / ENABLE 18 / NONE 11; 46 bare current_setting; 3 FORCE+zero-policy; 3 throw-on-unset) | 12 §2 | honored via txn-scoped GUC; initplan wrapping during transition (council); B3 flip test matrix |
| DB functions / triggers / indexes / enums | **45 / 13 / 114 / 10** (35 DEFINER, 5 unpinned search_path incl `read_public_menu`) | 12 §3-5 | unchanged; unpinned-search_path fix = council row; order_status enum → exhaustive Rust enum 🔴 |
| Root scripts / script files / guardrails / gates / lint rules | **70 / 93 / 11 / 25 / 26** | 13 §1-2 | 23+14 PORT / 23+49 KEEP / 24+15 RETIRE; **0 unmappable intents** (3 land grep-strength until dylint) |
| Error codes | **68 envelope + 8 preflight + 51 ad-hoc sites** | 14 §3 | ONE Rust error enum + IntoResponse; ad-hoc shapes become unrepresentable |
| Auth flows | **12** (2 undocumented; 5 gaps carried as explicit decisions) | 14 §4 | 🔴 council; axum claims-extractor type-state |
| Cross-cutting integrations | **17** (10 live / 4 dark / 3 inert) · 35 outbound hosts | 14 §5 | per-row disposition in matrix |
| Test estate | **174 E2E** (151 PARITY / 23 REBASE) · **181 unit files → 13 invariant clusters** · 162 visual comparisons (0 committed baselines) | 14 §7 | E2E runs as-is per surface; 13 clusters = the Rust unit-test backlog; visual re-baselined once |

**🔴 Council register: ~144 rows → 8 councils** (10 §6 + 11 §8 + 12): money/state-machine ·
auth · RLS/tenancy · WS-authz · payment webhooks · GDPR · backup/DR · dispatch. Each council covers its
surface's rows as a batch, pre-port; no 🔴 row is built without its council APPROVED.

## 2. Decision register (all lane verdicts, one place)

| Decision | Verdict | Source |
|---|---|---|
| Job queue | **Hand-rolled SKIP LOCKED + PgListener** (apalis-postgres RC-only, underway/sqlxmq stale; fallback graphile_worker_rs). 7 requirements: txn enqueue, retry/backoff, DLQ, cron, singleton, Supavisor-safe, embedded | 10 §5 |
| Imaging / OCR / PDF | libvips FFI · Tesseract **sidecar** · pdfium-render → **two images**: scratch `api` (~15-25 MB) + Debian-slim `media-worker` | 10 §5 |
| Migrations | **sqlx::migrate embedded**; own `_sqlx_migrations` coexists with frozen `pgmigrations` (157 stay applied); Fly `release_command = "/app/server migrate"`; boot guard + rowcount tripwire | 12 §8 |
| i18n | **Paraglide-JS 2** (per-message tree-shaking; typed dynamic-key maps; parity gate re-targets generated messages; fallback recorded) | 11 §5 |
| CI/CD | 5-stage (13 §CI): warm PR <10 min (fmt/clippy -D/test/sqlx-prepare-check ∥ cargo-deny ∥ astro+svelte-check+size-limit ∥ openapi-diff ∥ static gates) → fresh-provision+smoke → cargo-chef/musl/scratch → full 174-spec vs staging → manual prod gate. Cache: rust-cache + cargo-chef + committed `.sqlx/` | 13 |
| Connection topology | 3-pool per mode; **Supavisor decision (cache-off vs all-:5432) = Phase-A spike exit question** | 12 §7 |
| Fix-vs-carry rule | Pre-existing defects found by lanes: **CARRY-VERBATIM by default** (parity oracle sees identical behavior), **FIX-IN-PORT only for 🔴 security/correctness with council + documented E2E delta.** Registers: 7 backend bugs (10 §bonus: Telegram webhook fail-open, 24/30 bare-retry queues, refunds mount anomaly, dual tenant GUC), 6 latent GUC classes (12 risk-2 — NEVER copy verbatim: these are the cross-tenant leak class), 3 FE defects (11 §8: cashCollected untransmitted, atob 'c1' fallback, client-side OSRM), 5 auth gaps + error-shape divergence + 19 unhandled WS types (14) | all |
| RETIRE register | 14 dead API surfaces · 8 dead components · 2 dead client flags · 3 dead queue constants · 13 dead envs · 24+15 RETIRE scripts · mem0ai · dead deps (three, pdfjs-dist) — each RETIRE is a matrix row with proof-of-deadness, not a silent omission | all |

## 3. Phase plan

**Phase 0 — foundations (before any Rust code):**
1. `traceability.csv` seeded from the 6 inventory docs (every namespace row; status=MAPPED).
2. Map-coverage extractors running against the CURRENT codebase (proves the extractor pairs work).
3. `ci-schema-drift`: live-DB catalog diff vs migrations-derived census (catches further out-of-band objects — 2 already found).
4. Lane E pre-A blockers: lock 162 visual baselines · CI-arm `test:unit` + size-limit + lhci · triage 158 E2E skips (frozen skip-list).
5. The 8 council packets drafted from the 🔴 register (councils run per-surface, just-in-time before each port).

**Phase A — contract + spike (go/no-go gate):**
OpenAPI SSOT extracted from the 236-route census · Rust workspace scaffold (crates: api, domain, media-worker; two-image build) · storefront-read surface (menu read + Astro `/s/[slug]` SSR) live behind the existing proxy · sqlx GUC pattern + Supavisor answer · Paraglide spike (≤8 kB gz overhead check) · parity: storefront E2E slice green + RSS/p99 measured vs Node. **Go fallback trigger evaluated here (06 §2).**

**Phase B — strangler surfaces (each = council(🔴) → port → cutover DoD → flag flip):**
S1 storefront-read (done in A) → S2 auth 🔴 → S3 catalog/admin CRUD → S4 media pipeline (media-worker) → S5 orders/money 🔴 (+`sales_channel` entity + channel attribution) → S6 realtime WS 🔴 → S7 courier/dispatch 🔴 → S8 jobs/notifications → S9 GDPR/compliance 🔴 → S10 platform-admin/provisioning.
**Cutover DoD per surface (14 §8):** E2E slice green (as-is specs) + openapi-diff empty + invariant-cluster tests red→green + map-coverage zero-diff for the surface's namespaces + a11y/size budgets + council sign-off & rollback plan for 🔴.

**Phase C — channel-hub heads (post-S5, attach as surfaces stabilize):**
Feed heads (schema.org JSON-LD, GBP/Apple links, Meta feed) → MCP/`.well-known/ucp` read-only stub → TMA wrap → conversational heads per G7-survey demand. Governed by §6 invariants.

**Phase D — decommission + final acceptance:**
Node removed · full 174-spec run (frozen skip-list) · 162-shot re-baseline human-reviewed · 48h staging soak · reliability-gate L0–L11 GO · RLS adversarial + B3-flip matrix green · **all 67 regression-ledger rows re-proven against the new stack** · map-coverage: zero UNMAPPED/UNBUILT/ORPHAN.

## 4. traceability.csv (the matrix — data, not prose)

Schema: `id, namespace, artifact(file:line), behavior, target, proof, phase, redline, status, disposition`
- `status` ratchet (monotonic): MAPPED → COUNCIL-APPROVED(🔴 only) → BUILT → PROVEN → CUTOVER; RETIRED is terminal-with-proof.
- `disposition`: PORT | CARRY-VERBATIM | FIX-IN-PORT(council) | RETIRE(proof) | KEEP(unchanged, e.g. DB objects).
- Seeded in Phase 0 from the inventory docs; owned by the map-coverage gate thereafter.

## 5. Map-coverage gate (what makes "nothing missed" a CI property)

Per namespace, a dumb extractor pair — one against the Node/React tree, one against the Rust/Astro tree
(routes, WS types, queues, pages, islands, flags, envs, error codes, i18n keys, tables consumed, scripts).
CI fails on any of three set-diffs:
- **UNMAPPED** — exists in old code, no matrix row (census rot)
- **UNBUILT** — matrix row past its phase with no new-code counterpart
- **ORPHAN** — exists in new code with no matrix row (scope creep / untracked surface)
The gate runs from Phase 0 (old-side only) so extractor bugs surface before they can hide a port gap.

## 6. Channel-hub: architecture, invariants, failure modes

**Model:** one commerce core (the Rust hub being built) + 4 thin head families — render (Astro storefront,
kiosk, TMA) · conversational (ONE dialogue FSM + per-platform transports) · feed/discovery (JSON-LD, GBP/
Apple, Meta feed) · agentic (MCP/UCP stub; industry converged on merchant-checkout link-out — Instant
Checkout discontinued 2026-03, Google UCP names local food delivery as the next Universal Cart vertical).
`sales_channel` = first-class entity; orders carry channel attribution (lands with S5).

**Non-negotiable invariants (breaker-reviewed):**
1. **Single money surface** — every channel terminates in a signed, TTL'd, idempotent **cart token** →
   the ONE web checkout. Channels build carts; only checkout touches money. In-chat payments = per-channel 🔴 council, later.
2. **Thin heads, machine-enforced** — heads contain translation only; boundary rule (heads import the
   client contract, never the domain) enforced by dependency-cruiser/module visibility; every head gets its own authz council row (heads authenticate as channel principals, minimal scope, never as the tenant).
3. **Strong center** — read/write split (catalog cacheable → render/feed heads degrade gracefully on hub
   outage; only order intake requires the live hub); inbound webhooks → durable queue, never synchronous; per-head idempotency keys; per-channel rate quotas + kill-switch flags (default-off).
4. **Channel count = ops budget** — a channel goes live only with demand evidence + a monitoring line
   (feed freshness, webhook lag, injection success — surfaced in the vendor's admin) + a kill switch.

**Failure modes accepted with eyes open:** hub = concentrated blast radius (priced: order intake is
all-or-nothing by design); LCD trap (chat sells simple items; cart-token handoff is the fidelity escape);
platform churn (heads stay ≤~500-line adapters so channel death is a flag flip); regression coupling
(additive-only contract + openapi-diff gate).

## 7. Open items (named, not hidden)

1. Lane A §6.4: customer-JWT-mint↔WS-authz linkage needs one focused pass (fold into WS-authz council packet).
2. Lane C: live-DB catalog diff (Phase-0 item 3) — census is migrations-derived until then.
3. Lane B: voice-FE rows must reconcile with `docs/design/voice-fe-mount/` before porting (currently uncommitted working-tree unit).
4. EnvSchema census (80) was manual — the Phase-0 extractor must parse the Zod schema programmatically.
5. The current-prod backlog is NOT suspended by the rebuild: operator-gated SQL drafts (085 watermark **2026-07-10**), secrets rotation, ADR-020 gates all proceed on the Node stack until surfaces cut over.

## 8. Red-line integrations deep-census addenda (OTP / Plisio / GDPR / Backup-DR sub-lane)

**Stays-in-Postgres register (never reimplement in Rust):** 3 SECURITY DEFINER functions are structural
safety boundaries — `payment_location_by_provider_ref` (tenant resolution for the unauthenticated
payment webhook), `erase_shadow_tenant` (cross-tenant DELETE the app role has no grants for), and the
staged `gdpr_erase_customer` draft 088 (RLS-independence by design). The Rust port CALLS them; matrix
disposition = KEEP.

**Greenfield gaps discovered (absent features, not port targets — need their own product decisions):**
- **GDPR Art.15 export endpoint does not exist** (grep-verified) — erase-only today. New `gdpr::export` scope in Rust; 🔴 council.
- **No automated restore-to-production** — `scripts/backup-restore.ts` deliberately aborts and prints manual `pg_restore` instructions; DR is a documented manual procedure. The Rust ops binary should finally implement a confirmation-gated real restore (backup/DR council).

**Unproven-but-shaped (test debt to open before any flag flips):**
- Plisio webhook HMAC is correctly shaped (HMAC-SHA1 over PHP-serialize, `timingSafeEqual`, fail-closed 401) but has **zero tests** and the PHP-serialize key-order has never been validated against a real Plisio callback (self-flagged in source) — byte-fidelity proof + HMAC harness are launch prerequisites.
- OTP verify-lockout and expiry(410) branches untested; the SMS send path is a `console.log` scaffold (no carrier) — Rust target: `SmsProvider` trait with log-stub default.
- Order-path anonymization (`anonymizeOrder`) has no assertion; add to the GDPR invariant cluster.

**Architecture note for the port:** backup pipeline (dump/encrypt/upload/verify drills) becomes a
**sidecar ops binary** (`tokio::process` pg_dump/pg_restore + aes-gcm + aws-sdk-s3), never the API
request path; only the 3 platform-admin trigger endpoints stay in axum. OTP + Plisio port dark
(flags default-off preserved); GDPR-erase + Backup port as always-on.
