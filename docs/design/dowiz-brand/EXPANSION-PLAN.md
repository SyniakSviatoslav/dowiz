# DOWIZ EXPANSION PLAN — The Sovereign Multi-Layered Hub
> Authoritative living plan. Grounds: user vision (2026-07-07) · GRAND-PLAN (sovereign core) ·
> deep research + reasoning · BRAND-BIBLE.md. Frame: **Dowiz is a hybrid, multi-layered hub for many
> environments.** The kernel is the cathedral; every entry point is a write-only door that carries
> commands to it. "Hybrid is a feature, not a bug."

## Layer model
- **Layer 0 — Foundation / release-gate** (hardening; nothing public until done)
- **Layer 1 — The Hub Core** (brand + entry points + telemetry + auth — the visible expansion)
- **Layer 2 — Scale** (voice, translations, observability, smart devices — behind existing seams)

Decision rule for every entry point: *does this door decide anything, or only carry?* Doors carry a
`Command` to `kernel::decide` and receive events/refusal. Doors NEVER price, transition state, or
invent money. Coherence by construction, not vigilance.

---

## LAYER 0 — FOUNDATION (must precede anything public)

| # | Item | Gate / proof | Red-line |
|---|---|---|---|
| 0.1 | **Secrets history scrub** (Stripe token etc. in git history) — force-push | `git log --all` clean; rotate creds after | operator, BLOCKING |
| 0.2 | **License flip** Apache-2.0 → AGPLv3 (+ TRADEMARK.md, +DCO) | LICENSE = Affero; TRADEMARK.md exists & linked | counsel |
| 0.3 | **README / SECURITY.md truth-pass** (stack is Rust core + React web) | zero dead links; zero marketing; agent-readable arch | no |
| 0.4 | **Design-system unification** — collapse stray `tokens.css` into one; bebop skin canonical | single source; no secondary token files | no |
| 0.5 | **Security scanners in CI** (see §Security review) | semgrep/trivy/gitleaks green; RED proof each | AppSec |
| 0.6 | **rsa / num-bigint RUSTSEC** remediation (auth/crypto chain) | advisory cleared or documented accept | AppSec |

---

## LAYER 1 — THE HUB CORE

### 1.A Brand (Warm Cosmo-Noir) — IN PROGRESS
- ✅ Pass 1: `[data-skin="bebop"]` tokens + BRAND-BIBLE.md (Paper/Moebius retired).
- ✅ Pass 2 (landing): cinematic entry page — Nomadic skeleton + Horizon Drift + city-pop easter egg
  + dry-wit trilingual copy. Live at `/`.
- ⏳ Pass 2 (rest): flip `paperSkinAttr()`→bebop, delete Paper, migrate admin/courier + daylight
  toggle, add fonts to allowlist, visual-regression proof.
- **Voice is project-wide canon** (see BRAND-BIBLE §9): dry humanized "texting" narration on EVERY
  surface incl. **sales & marketing**; money/auth/security copy stays plain. All strings sq/en/uk.

### 1.B Auth — Better Auth + Telegram  ← (operator 2026-07-07)
- **Model:** two first-class paths, both landing on the same session/identity.
  - **Better Auth** (`better-auth`) as the primary framework — email/password + OAuth + session
    management, type-safe, self-hostable (no third-party identity custodian → matches sovereignty).
    Adapter to the existing Postgres; sessions server-side.
  - **Telegram login** — Telegram Login Widget / Mini-App `initData` HMAC verification → mints the
    same Better Auth session. Already partially scaffolded (`lib/tma.ts`, admin Telegram login).
- **Doctrine:** auth is a *door* — it authenticates and attributes, never prices/decides. Owner,
  courier, and customer identities are RLS-scoped (customers already own their data, Phase 2.3).
- **DoD / gates:** Telegram `initData` HMAC verified server-side (RED: tampered hash → rejected);
  Better Auth session cookie httpOnly+secure+sameSite; NOBYPASSRLS behavioral test still green;
  session fixation + CSRF tests. **Red-line (auth)** → security review + council before merge.
- **Skills/best-practice:** `better-auth-best-practices` skill. Deps + config are Layer-1 work.

### 1.C Entry points (write-only doors → one hub)
| Door | Status | Notes |
|---|---|---|
| Web direct `/s/:slug` | exists | reskin later; carry only |
| **Landing `/`** | ✅ built | Bebop entry, CTAs → /claim, /start |
| QR | plan | payload = items only, no money; `x-channel: qr` |
| Telegram Mini App | scaffolded | wire; never compute price in TG |
| Instagram / Facebook | plan | embed → redirect to web checkout |
| **WhatsApp** ← (operator) | plan | **VERDICT (research): use WhatsApp Cloud API (official), NOT open-wa in production.** open-wa/@open-wa/wa-automate is unofficial (reverse-engineered) → 2–8wk ban timelines, permanent number loss, no appeal; and ~4GB RAM per headless session = >50% of the 7.6GB box. Order intake is money-adjacent → a ban mid-service is red-line-caliber. Cloud API: free platform, per-template billing (use **utility** category for confirmations, ~80–90% cheaper than marketing; free-form replies free inside 24h window), Business Verification 2–5 days (prototype free on Meta test number meanwhile). Door: webhook → verify `X-Hub-Signature-256` (HMAC-SHA256, App Secret, timing-safe) → interactive list/button `id`→SKU (avoid free-text) → `x-channel: whatsapp`. open-wa allowed ONLY as a throwaway prototype. |
| SMS / async | later | Notify/OTP only, not intake. ~$0.08–0.11/msg (Albania: Plivo cheapest, Twilio best DX). Verify provider signature on any inbound; SMS never an identity source beyond OTP. |
| Voice | Layer 2 | local-only; ConfirmationGate gated |
| **City-pop radio** | ✅ easter egg | brand delight on landing; not an order path |

**Entry-point sequencing (research verdict):** **QR first** (zero new infra — it IS web-direct + `x-channel:qr`; dynamic QR w/ TTL nonce, kernel re-prices from SKU, ignore client price) → **Telegram Mini App second** (highest leverage in Albania: free, official, ~0 RAM stateless webhook; verify `initData` HMAC-SHA256 + `auth_date` freshness on EVERY write, + `X-Telegram-Bot-Api-Secret-Token` on the bot webhook) → **WhatsApp Cloud API third**. **Instagram/Facebook: no bespoke door** — Meta retired in-app Shops checkout (Sep 2025); wire as a `web-direct` attribution link (`?x-channel=instagram`); Messaging API only later for post-purchase pings (200 DM/hr cap, 24h window). Universal door rule: HMAC/signature verify, rate-limit, recompute price server-side, never auto-execute from free text — require an explicit confirm tap.

### 1.D Telemetry → Telegram (operator-facing only)
- Deterministic reduce over the **event log** (money plane) + kernel refusals (CorridorBreach, RLS
  denials, invariant violations) + infra (RAM/disk/latency). One alerter worker → your Telegram.
- Falsifiable per alert (ship the RED case). No diner surveillance — operator-facing system health only.

---

## LAYER 2 — SCALE (behind existing seams)
Voice UI (local transformers.js; ConfirmationGate + dietary-denylist; 3 RED proofs: local-custody,
refuses-unsafe, kernel-routed) · live translations (local, read-only) · Langfuse (LLM traces, when
voice/agents ship) · SkillOpt (advisory self-improvement) · canvas/intent UI · smart-device adapters ·
aggregator read-only ingestion. **Skip Temporal** (event-sourced sagas already cover it).

---

## SECURITY REVIEW (standing track — operator 2026-07-07)  ← don't skip
A dedicated security review gates every red-line surface (auth, money, RLS, migrations, new ingress).

**Static / supply-chain (CI + pre-commit, on-demand — no resident daemon on the 7.6GB box):**
- **gitleaks** — secret scanning (also guards the OSS publish).
- **semgrep** — SAST; custom rules banning crypto outside kernel, raw SQL, cookie writes, money math in doors.
- **trivy** — deps + Docker image + IaC + secret scan (complements `cargo-deny`).
- **cargo-deny** — Rust bans/advisories (already catches RUSTSEC).

**Dynamic / adversarial (on-demand vs staging):**
- **OWASP ZAP (zaproxy)** — DAST on `/s/:slug`, `/admin/*`, `/api/*`, and every new door (WhatsApp,
  Telegram, QR ingress).
- **garak** — only once an LLM surface ships (voice/translation/agents) — prompt-injection/jailbreak.

**Review cadence & scope:**
1. **Per red-line PR:** threat-model the change (auth/money/RLS/ingress), secure-code review, RED proof.
2. **Pre-publish (OSS) full sweep:** gitleaks history-clean + semgrep + trivy + ZAP baseline + license/deps audit → written report in `docs/security/`.
3. **New entry point = new attack surface:** each door gets an ingress review (authn/z, rate-limit,
   input validation, HMAC/signature verification, abuse/DoS) before it goes live.
4. Skills: `owasp-security`, `security-review`, `Application Security Engineer` agent for deep passes.
- **Falsifiable rule (VbM):** every control ships with the input that makes it go RED.

---

## RED-LINE DECISIONS (operator approval)
Secrets force-push · AGPL flip · auth (Better Auth/Telegram) · any migration · WhatsApp/open-wa ingress
· voice-to-prod (3 RED proofs) · **never** diner surveillance.

## TOOL TRIAGE
Adopt now: gitleaks·semgrep·trivy·cargo-deny (security), Better Auth (auth). Layer-2: ZAP·garak,
Langfuse, SkillOpt. Evaluate: open-wa vs WhatsApp Cloud API. Skip: Temporal, Astryx (extend own
tokens), Stix, Gitghost.

## TIMELINE (indicative)
Wk1–3 Layer 0 (secrets·license·docs·design-unify·scanners·rsa) · Wk4–9 Layer 1 (brand finish·auth·
entry points·telemetry) · Wk10+ Layer 2. Each ships as a small commit with a falsifiable proof.

---

## RESEARCH FINDINGS (distilled — being folded in as the fleet lands)

### Telemetry / monitoring (verdict: extend what exists, add NO heavy daemon)
Dowiz already has the plumbing — **don't parallel-build**: `apps/api/src/lib/metrics.ts` (zero-dep Prometheus `/metrics`, token-dark), `routes/health.ts` (pg/worker-heartbeat/resource snapshots), `free_tier_snapshots` table (the snapshot pattern to extend), and an **operator-only Telegram channel already exists** (`TELEGRAM_OPS_CHAT_ID` + `scripts/automation/notify.sh`, used by tier1-3 LLM crons).
- **Build:** a new **deterministic** (non-LLM) digest job (stdlib SQL+arithmetic) posting a `#sys key=value` digest to `TELEGRAM_OPS_CHAT_ID`, tagged apart from the `#agent` LLM reports. Live "dashboard" = **pin one message + `editMessageText`** every 15–60s (well inside 30 msg/s + ~20 edits/min limits), not per-request spam.
- **Use a SEPARATE ops bot token** from the customer bot → operator/diner boundary is structural, not convention.
- **Metrics — critical (page):** payment-success drop / refund spike; kernel `CorridorBreach` + RLS-denial + invariant rate (should be ~0 always); pg down; worker heartbeat stale; 5xx/p95 spike; RSS near 512MB(web)/256MB(worker) ceiling; free-tier limits. **Valuable (trend):** orders/hr, latency p50/p95, WS conns, pg-boss backlog.
- **OpenTelemetry: DEFER/skip self-hosting.** No local OTel Collector / Prometheus / Grafana / Netdata (all too heavy for 512MB VM). **Fly already runs managed Prometheus+Grafana at `fly-metrics.net`** off-box for free — add `[metrics]` to fly.toml (resolve the token-vs-6PN-scrape question). If distributed tracing later: OTLP/HTTP direct to a managed backend, no local collector. Optional external prober: Uptime Kuma / Beszel on its OWN tiny Fly machine.
- **Privacy:** allowlist-by-construction payloads (fixed counter schema, never raw bodies/PII); route IDs not raw URLs; ship the RED case per alert (fires `#critical` when CorridorBreach>0, green when 0).

### Marketing / demo tooling (verdicts)
- **Remotion — ADOPT** (free ≤3 people / local render free forever; reuses React/TS). One prop-driven composition → per-venue demo videos w/ burned-in subtitles. (Ignore SEO-farm stats; official docs solid.)
- **ai-website-cloner-template — ADOPT (pilot, extraction only)** — point at a prospect's site/IG/Google listing to extract their real menu/photos/colors → feed into Dowiz's own `/s/:slug` demo shell (the "poisoned chalice"). Keep assets in a private per-prospect demo link (matches existing SHADOW/owner_id-NULL pattern). Don't ship its Next.js scaffold.
- **Meetily — ADOPT** (MIT, 100%-local Whisper+LLM, sovereignty-fit) for sales-call transcripts → mine objections. Caveat: desktop-first, verify Linux build. The objections-tracker table doesn't exist yet — small addition.
- **InfiniteTalk — DEFER** (real, Apache-2.0, but needs 24GB-VRAM GPU; disproportionate pre-revenue).
- **google-labs-code/design.md — DEFER** (Dowiz already has mature `tokens.css` SoT; the real gap is writing `mood.md`/`voice.md`/`tokens.md` for the critique skills — zero new dep. Now largely covered by BRAND-BIBLE.md).
- **cb-userhunter — SKIP** (username OSINT, not lead-gen; conflicts with data-sovereignty brand ethics).
- **Demo pipeline (≈$0 net infra):** reuse existing per-slug `?ch=` attribution (`lib/channel.ts`) → mint `/s/<demo>?ch=outreach&lead=<opaque-id>`; extend the EXISTING `POST /api/telemetry` (`routes/public/telemetry.ts`, already SHA-256 IP-hashed, privacy-first) with `demo_open/scroll/add_to_cart/dwell_30s` events tagged `lead_id`; watch `analytics_events` (LISTEN/NOTIFY or cron) → alert via existing Telegram bot infra → log `telegram_alert_sent` to close the loop. Only new artifact: an `objections` table.

### Auth (verdict: Dowiz already has mature bespoke auth — pilot Better Auth on OWNER login only)
Existing (found): RS256 JWT (`jose`, dev/prod kid-segregated), **rotating refresh tokens w/ reuse-detection + family revocation** (already best-practice), Google OAuth+PKCE, local argon2, Telegram bot-deep-link+poll, `courier_sessions` (hard revoke), same-origin Fastify (SPA+API, CORS default-deny), RLS via `set_config('app.user_id')` at ~40 sites.
- **Better Auth = pilot on owner web login ONLY** (built-in Kysely/`pg` adapter, map onto existing `users` table, cookie sessions `httpOnly`+`secure`+`sameSite:lax`, origin-check CSRF — clean since same-origin). Hardens the current `localStorage` (XSS-exposed) owner tokens. **Do NOT migrate courier/customer** (bearer-by-design: SMS links, mobile). **Red-line → human-gated**; keep refresh-rotation+reuse-detection (Better Auth's default session is weaker).
- **Ship first (additive, low-risk):** `POST /auth/telegram/tma` verifying Mini-App `initData` HMAC (`secret=HMAC_SHA256("WebAppData", bot_token)`; check `auth_date` freshness; Redis-cache consumed hashes for replay defense) → reuse existing token-mint; removes the poll round-trip inside the TMA. Login Widget uses a DIFFERENT key (`SHA256(bot_token)`) — don't conflate.
- Community `better-auth-telegram` plugin exists → **read its HMAC source before trusting** (red-line). RLS is decoupled from auth mechanism (swap `verifyAuthToken`→`getSession`, `set_config` sites unchanged). RED proof: tampered `hash`/stale `auth_date` → 401.

### OSS release hardening (verdict + exact order)
Repo is **half-relicensed**: `LICENSE`=Apache-2.0 but `Cargo.toml`=`AGPL-3.0-or-later` and `CONTRIBUTING.md`=`AGPL-3.0-only` → **reconcile to ONE SPDX (recommend `-or-later`)**. Single copyright holder → clean relicense. Pinned "stripe" secret in `.gitleaksignore` is a 19-char **placeholder in a mockup** (not real key format) — scrub anyway; a SEPARATE already-rotated real cred still needs the history scrub.
- **Order:** (1) `cargo update -p num-bigint` clears the yanked-crate gate today (unrelated to rsa). (2) rsa RUSTSEC-2023-0071 is **unpatched upstream** — best fix = force `jwt-simple` `optimal`(BoringSSL) feature to drop the dead RSA path (VAPID only uses ES256); **test build RAM on the 7.6GB/no-swap box** (BoringSSL C build); else a dated/justified `deny.toml` exception. (3) **git-filter-repo** `--replace-text` → `push --force --all/--tags` → **contact GitHub Support to purge caches/forks** → verify (`grep` + `gitleaks --all`). (4) swap `LICENSE`→AGPLv3 full text + SPDX headers + `license` field in package.json's. (5) DCO via **`dco-check-action` in `ci.yml`** (not the app) — verify it gates a throwaway PR. (6) add `SECURITY.md` + GitHub **Private Vulnerability Reporting** + `/.well-known/security.txt`; disclose the rsa advisory proactively in README "known limitations". THEN flip public.

### AI infra / self-improvement (verdicts)
**SkillOpt — ADOPT (pilot, held-out eval)** — formalizes the existing expensive→cheap-checklist practice, model-agnostic/on-box. **Langfuse — DEFER** (ClickHouse+PG+Redis ~16GB; only once an LLM feature ships, on separate hardware). **SKIP:** LangSmith (enterprise/k8s), Braintrust (closed, phones home), 9router (arbitrages 3rd-party API tiers = red-line), rtrvr.ai (cloud+Gemini = red-line), qwen-agentworld (35B+ RL world-model, wrong tool), transmute (not an AI tool). **DEFER/monitor:** page-agent (in-page product copilot idea, not a test driver), openspace (HKUDS self-evolving skill cache, immature, overlaps own work), aisoc (heavy; only if a security-hardening arc opens).

### Codebase map (load-bearing facts for every layer above)
**TWO stacks:** **LIVE = Node/TS** (Fastify `apps/api`, React SPA `apps/web`, pg-boss `apps/worker`) — this is what's deployed and what the Bebop landing/skin ship into. **REBUILD = Rust+Astro** (`rebuild/crates/domain` kernel is the target core; the Node stack is the **oracle** it's verified against; only storefront-read is live). Kernel `decide` = the one door (`kernel.rs:306`, events-not-state, compiler-enforced `fold` exhaustiveness). **`order_events` append-only shadow log** (`pg.rs:868`, dual-write for replay-parity — the seam to become authoritative). Money path pure integer `Lek`; `charged_tax`=the LC1 term; **`discount_total` is a reserved ZERO seam** (Promotions CRM built, no redemption runtime — the money-path expansion point). RLS = **dual-GUC in `db.rs`** (`app.current_tenant` + `app.user_id`) + phased additive migrations (NOBYPASSRLS flip staged; courier + customer-ownership policies landed). Entry points follow **"typed-contract-then-wire-transport"** (channel allowlist write-only; `analytics.ts` PostHog transport NOT wired; `tma.ts` doesn't load `telegram-web-app.js` — **CSP not whitelisted**, the exact blocker for the TMA + city-pop-radio YouTube embed). Voice safety core built (classify→gate→handlers, dietary denylist, transcription providers present, data-only). i18n `sq/en/uk` single-source, parity-enforced. **Extension seams:** kernel event alphabet (compiler-forced), `order_events` log, dual-GUC + additive-policy pattern, wire-one-transport scaffolds.

### Voice + translations (verdict: all inference on the DINER's device → zero server RAM; license is the trap)
- **ASR:** keep the scaffolded **transformers.js Whisper** (`Xenova/whisper-base`, q8, WebGPU→WASM→CPU) as the ONLY path. **Web Speech API = skip** (cloud by default; on-device mode lacks sq/uk). **Albanian base Whisper is weak (~50%+ WER)** → swap a per-locale fine-tune (`whisper-medium-sq` ~5.8% WER) behind the existing `TransformersTranscriberOptions.model` field; uk OK on base. **pipecat = skip** (server-centric; audio leaves device).
- **TTS:** kittentts/luxtts are **English-only** (not viable sq/uk). **Piper** = only option with native **sq+uk** voices + commercially licensable (pin archived MIT engine; audit each voice's tag) + community WASM ports. **MMS-TTS + Coqui XTTS = CC-BY-NC → EXCLUDE.** `transkriptionsuite` = **does not exist** (non-finding).
- **Translation:** **Bergamot / Firefox-Translations** (`@browsermt/bergamot-translator`, MPL-2.0, sq+uk, ~15-40MB/dir, in-browser) = primary; **Opus-MT** (CC-BY-4.0, `Xenova/opus-mt-*`) = fallback. **NLLB-200 + MMS = CC-BY-NC → EXCLUDE** (the habitual wrong pick). Menu text is public → **batch-translate at ingest** (one-shot job, no live service, no RAM cost); voice stays in-browser.
- **Ship gate (falsifiable):** Playwright network-cut RED test (zero requests post-warmup); ConfirmationGate fail-closed (money/checkout/dietary have NO `IntentKind` → REJECT by construction); STATEFUL writes need a real human `confirm()` tap; voice handler === the touch handler (routes through `kernel::decide`, zero write capability in the engine); deterministic WER gate + RED fixture; **CI license-audit** (no CC-BY-NC model in the bundle).
- **CSP note:** `connect-src` must be scoped to same-origin + own model-CDN — this is ALSO the blocker to fix for the Telegram TMA (`telegram-web-app.js`) and the city-pop-radio YouTube embed.

### Security scanners / CI wiring (verdict + a critical existing false-green)
- **🔴 FIX FIRST — existing false-green:** `pnpm verify:secrets` in `.github/workflows/ci.yml` is a **no-op** — `scripts/verify-secrets.ts` only runs gitleaks if a binary is on PATH, which CI never installs. Every CI run silently skips real secret scanning (only the custom `.env`/JWT regex runs). Classic VbM false-positive-green. One-line install step fixes it; highest value, zero risk.
- **gitleaks:** run the **raw binary** (the `gitleaks-action` wrapper needs a PAID license for org private repos). CI: full-history scan (`--log-opts="--all --full-history"`, `fetch-depth:0`) → SARIF. Pre-commit: `gitleaks protect --staged`. Require a `# reason+reviewer` comment on every `.gitleaksignore` entry.
- **semgrep:** Rust is **GA** (custom Rust rules free). Use `semgrep ci --no-suppress-errors` (**it fails OPEN on scan errors otherwise** = silent green). 4 custom rules: domain-crate import bans, raw SQL, cookie-setting, money-arith (**WARNING-only**, low precision). Run ALONGSIDE existing bespoke guardrails (`guardrail-no-set-cookie.mjs` etc.) for a cycle before retiring any.
- **trivy:** **🔴 PIN the action to a full commit SHA** — `trivy-action` was supply-chain-compromised March 2026 (76/77 tags force-pushed malicious). Image + fs SCA, config on Dockerfile only (**fly.toml N/A**), gate CRITICAL/HIGH, `cache:true` (disk-pressure history). Complements cargo-deny (which stays authoritative for Rust crate CVEs — avoid double-reporting).
- **cargo-deny:** `[[bans.deny]]` is currently **EMPTY** — add tokio/sqlx/axum/rand/chrono with `wrappers=["api"]` + a second zero-exception invocation scoped to `crates/domain` (catches a future member that forgets a `module.toml`, which `module-integrity.mjs` can't see).
- **OWASP ZAP** (now "ZAP by Checkmarx", still Apache-2.0): `action-baseline` (passive) on-demand vs staging; `action-full-scan` is **dangerous** (submits forms → bogus orders/notifications) → `workflow_dispatch`-only, exclude `checkout`/`orders` paths, inject bearer JWT via Replacer (reuse the Playwright staging-login pattern).
- **garak:** DEFERRED — nothing to point at (no product LLM surface). Trigger = first feature routing external-user input into an LLM call. NOT the same as SkillSpector (already wired in `skill-security.yml`).
- **Rollout order (each its own reviewed commit + RED proof):** fix gitleaks no-op → cargo-deny bans → trivy (SHA-pinned) → semgrep → ZAP baseline → garak deferred.

> ✅ ALL 9 research reports distilled into this section (2026-07-07). Full per-agent reports persist under the session `tasks/*.output` files if deeper detail is needed.
