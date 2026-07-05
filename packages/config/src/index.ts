import { z } from "zod";

const EnvSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]),
  PORT: z.coerce.number().int().positive().default(8080),
  APP_BASE_URL: z.string().url(),
  DATABASE_URL_OPERATIONAL: z.string().url(),   // transaction pooler :6543
  DATABASE_URL_SESSION: z.string().url(),       // session pooler :5432
  DATABASE_URL_MIGRATIONS: z.string().url(),    // session pooler :5432
  REDIS_URL: z.string().url(),                  // upstash, pub/sub only
  JWT_PRIVATE_KEY: z.string().min(1),
  JWT_PUBLIC_KEY: z.string().min(1),
  JWT_KID: z.string().min(1),
  GOOGLE_CLIENT_ID: z.string().min(1),
  GOOGLE_CLIENT_SECRET: z.string().min(1),
  // Google OAuth is launch-gated OFF by default: when 'false' BOTH the FE button
  // (VITE_GOOGLE_OAUTH_ENABLED) and the backend /api/auth/google routes are closed, so
  // "hidden" actually means "disabled" (not just a hidden-but-reachable auth path).
  GOOGLE_OAUTH_ENABLED: z.enum(['true', 'false']).default('false'),
  // Shared secret gating the /dev and /api/dev test-only endpoints (mock-auth, etc).
  // When unset/empty, those endpoints are fully disabled (404) — the safe prod default.
  DEV_AUTH_SECRET: z.string().optional(),
  // ── Dev-login hardening (ADR-0003) ──
  // Master runtime gate for ALL dev/test auth bypasses (the /auth/local dev branch AND
  // the /dev/* mock-auth minters). The secret alone is NOT enough — both this flag AND
  // DEV_AUTH_SECRET must be set for any bypass to activate. Default false → prod fails
  // closed even if DEV_AUTH_SECRET leaks again (the root cause of the live incident).
  // loadEnv() below FATAL-throws if this (or the secret / dev kid) is set on a prod box.
  ALLOW_DEV_LOGIN: z.enum(['true', 'false']).default('false'),
  // The dev-login account credentials, sourced from env so no credential literal ships
  // in code (ADR-0003 #7). Inert unless ALLOW_DEV_LOGIN+DEV_AUTH_SECRET are also set.
  // Set on staging/CI/local only (e.g. test@dowiz.com / test123456); absent on prod.
  DEV_LOGIN_EMAIL: z.string().optional(),
  DEV_LOGIN_PASSWORD: z.string().optional(),
  // Dev-token kid segregation: dev/mock tokens are signed under JWT_DEV_KID with the dev
  // keypair so a prod verifier (which has neither the dev keypair nor accepts the dev kid)
  // cryptographically rejects them. Present on staging/CI/local only — NEVER on prod.
  JWT_DEV_KID: z.string().optional(),
  JWT_DEV_PRIVATE_KEY: z.string().optional(),
  JWT_DEV_PUBLIC_KEY: z.string().optional(),
  TELEGRAM_BOT_TOKEN: z.string().optional(),
  TELEGRAM_BOT_SECRET: z.string().optional(),
  TELEGRAM_BOT_USERNAME: z.string().optional(),
  // WhatsApp/Baileys channel removed (P0-2, ADR-p0-privacy-hardening).
  // Phone OTP verification — globally disabled until a real SMS gateway is wired
  // (current OTP send is a console.log scaffold). Flip to 'true' to re-enable;
  // per-location `require_phone_otp` only takes effect when this is 'true'.
  OTP_ENABLED: z.enum(['true', 'false']).default('false'),
  // Rich product media (ADR-0002 cinematic-product-media seam) — global kill-switch,
  // default OFF. Phase 1 ships only the inert schema; while this is 'false' the storefront
  // is byte-identical to today and product_media / primary_media_id are unread. The lazy
  // media endpoint + renderers (Phase 2) gate on this AND locations.plan='business'.
  MEDIA_RICH_ENABLED: z.enum(['true', 'false']).default('false'),
  // SENSOR-BUS §1.3 funnel ingest kill-switch (ADR-0009). Default ON — the public ingest is
  // non-blocking, per-IP rate-limited and best-effort, so capture is safe by default; this flag
  // lets ops silence a misbehaving/abused funnel sensor WITHOUT a deploy (set 'false' as a secret).
  FUNNEL_INGEST_ENABLED: z.enum(['true', 'false']).default('true'),
  // ── Reversible cutover harness (ADR-0022, rebuild strangler) ──
  // Internal-only Rust upstream base URL (e.g. http://dowiz-rust-staging.flycast:8080).
  // UNSET (the default everywhere) leaves the front-door fully inert: no hook, no flag
  // poll, no health probes — byte-identical request handling. Set as a secret per-env
  // only when that env's Rust twin is deployed dark.
  CUTOVER_RUST_UPSTREAM: z.string().url().optional(),
  // Q6 Astro sub-target: internal Astro SSR app serving the S1 storefront page when S1
  // is flipped (e.g. http://dowiz-astro-staging.flycast). Unset = S1 HTML stays on Node.
  CUTOVER_ASTRO_UPSTREAM: z.string().url().optional(),
  // Break-glass (ADR-0022 §4): forces every surface to Node WITHOUT consulting the
  // cutover_flags store — for when the flag store itself is impaired. Runtime env, no DB.
  CUTOVER_FORCE_ALL_NODE: z.enum(['true', 'false']).default('false'),
  // REV-C3: flag propagation is a bounded-TTL poll (LISTEN/NOTIFY is blocked on the
  // transaction pooler); this TTL is the REAL split-brain window per flip — keep small.
  CUTOVER_FLAGS_TTL_MS: z.coerce.number().int().positive().default(2000),
  CUTOVER_HEALTH_INTERVAL_MS: z.coerce.number().int().positive().default(5000),
  // Closed-venue order gate. The audit found POST /orders accepts orders even when the storefront
  // shows "closed". When 'true', the create handler mirrors the storefront's EXACT open/closed
  // computation (apps/api/src/lib/venue-open.ts ← public/menu.ts) and refuses a closed venue with
  // 409 VENUE_CLOSED. Default OFF / reversible (ship-discipline): deploy dark, flip on after
  // staging validation; flip back instantly if it misfires — a false-"closed" would drop real
  // orders, so the safe default is to accept.
  ENFORCE_VENUE_HOURS: z.enum(['true', 'false']).default('false'),
  // Menu-import LLM (ai-ocr-parser). OpenRouter is OpenAI-wire compatible; when its key is
  // set it serves PDF/image menu extraction. Without any provider the parser falls back to a
  // zero-dependency heuristic structurer. All optional — read via process.env in the parser.
  OPENROUTER_API_KEY: z.string().optional(),
  OPENROUTER_MODEL: z.string().optional(), // default 'openai/gpt-4o-mini'
  OPENROUTER_ENDPOINT: z.string().optional(),
  // OpenCode Zen — OpenAI-wire compatible gateway with free models. Preferred over OpenRouter
  // when its key is set (OpenRouter account ran out of credits). Free-model fallback chain lives
  // in the parser; all optional, read via process.env there.
  OPENCODE_ZEN_API_KEY: z.string().optional(),
  OPENCODE_ZEN_MODEL: z.string().optional(), // default 'deepseek-v4-flash-free'
  OPENCODE_ZEN_ENDPOINT: z.string().optional(),
  // Backup Configuration
  BACKUP_ENCRYPTION_KEY: z.string().optional(), // 32 bytes base64 (required if BACKUP_ENABLED=true)
  R2_ACCESS_KEY_ID: z.string().optional(),
  R2_SECRET_ACCESS_KEY: z.string().optional(),
  R2_ENDPOINT: z.string().optional(),
  R2_BUCKET: z.string().optional(),
  R2_PUBLIC_URL: z.string().optional(),
  // Operational (hot-path) pool size per API instance. Runs through Supavisor's transaction
  // pooler (:6543), which multiplexes, so this can safely exceed the old hardcoded 8. Raised
  // to give public-storefront bursts headroom over owner/courier/order writes (see the menu
  // pool-starvation fix). Tune per Supabase plan / instance count via env.
  OPERATIONAL_POOL_SIZE: z.coerce.number().int().positive().default(20),
  BACKUP_ENABLED: z.enum(['true', 'false']).default('false'),
  BACKUP_POOL_SIZE: z.coerce.number().int().positive().default(2),
  BACKUP_HOURLY_CRON: z.string().default('0 * * * *'),
  BACKUP_DAILY_CRON: z.string().default('0 3 * * *'),
  BACKUP_WEEKLY_CRON: z.string().default('0 4 * * 0'),
  BACKUP_MONTHLY_CRON: z.string().default('0 5 1 * *'),
  BACKUP_HOURLY_RETENTION_HOURS: z.coerce.number().int().positive().default(24),
  BACKUP_DAILY_RETENTION_DAYS: z.coerce.number().int().positive().default(30),
  BACKUP_MONTHLY_RETENTION_YEARS: z.coerce.number().int().positive().default(7),
  BACKUP_PII_FIELDS: z.string().default('email_encrypted,phone_encrypted,full_name_encrypted,address_encrypted,customer_phone,customer_address,customer_name'),
  // Dwell Monitor
  DWELL_CRON: z.string().default('* * * * *'),
  DWELL_TIER2_DELAY_MS: z.coerce.number().int().positive().default(30000),
  DWELL_TIER3_DELAY_MS: z.coerce.number().int().positive().default(90000),
  DWELL_TIER3_ENABLED: z.enum(['true', 'false']).default('false'),
  DWELL_BATCH_THRESHOLD: z.coerce.number().int().positive().default(10),
  // P26 — Anti-Fake Signals
  SIGNAL_RAISE_CRON: z.string().default('*/5 * * * *'),
  OTP_SEND_RATE_LIMIT: z.coerce.number().int().positive().default(3),
  OTP_VERIFY_RATE_LIMIT: z.coerce.number().int().positive().default(5),
  OTP_TTL_MS: z.coerce.number().int().positive().default(300000),
  VELOCITY_WINDOW_1H_S: z.coerce.number().int().positive().default(3600),
  VELOCITY_WINDOW_24H_S: z.coerce.number().int().positive().default(86400),
  VELOCITY_THRESHOLD_1H: z.coerce.number().int().positive().default(3),
  VELOCITY_THRESHOLD_24H: z.coerce.number().int().positive().default(10),
  // P28 — Native Web Push (VAPID)
  VAPID_PUBLIC_KEY: z.string().min(1),
  VAPID_PRIVATE_KEY: z.string().min(1),
  VAPID_SUBJECT: z.string().email().default('push@deliveryos.app'),
  // P26 — Anonymizer Retention (GDPR right-to-erasure)
  ANONYMIZER_RETENTION_CRON: z.string().default('0 3 * * *'),
  ANONYMIZER_RETENTION_BATCH_SIZE: z.coerce.number().int().positive().default(100),
  R2_RETENTION_OVERRIDE_DAYS: z.coerce.number().int().positive().optional(),
  // P31 — Observability & Worker Liveness
  SENTRY_DSN: z.string().optional(),
  LOG_LEVEL: z.enum(['trace', 'debug', 'info', 'warn', 'error', 'fatal']).default('info'),
  WORKER_HEARTBEAT_INTERVAL_MS: z.coerce.number().int().positive().default(15000),
  WORKER_LIVENESS_CHECK_MS: z.coerce.number().int().positive().default(60000),
  WORKER_LIVENESS_STALE_MS: z.coerce.number().int().positive().default(60000),
  // ADR-dispatch-recovery R3′: backup-hourly (data-recovery red-line) gains the live 60s
  // LivenessChecker path in addition to nightly Recon A6.
  WORKER_CRITICAL_LIST: z.string().default('dispatcher,settlement-cron,dwell-monitor,anonymizer-retention,backup-hourly'),
  GIT_SHA: z.string().optional(),
  // P32 — Backup Verification
  DATABASE_URL_ADMIN: z.string().url().optional(),
  RESTORE_VERIFY_CRON: z.string().default('0 4 * * *'),
  RESTORE_VERIFY_FULL_HASH: z.enum(['true', 'false']).default('false'),
  RESTORE_POOL_SIZE: z.coerce.number().int().positive().default(2),
  // Currency Rates Refresh
  RATES_CRON: z.string().default('0 * * * *'),
  // AI OCR — Groq
  GROQ_API_KEY: z.string().optional(),
  GROQ_MODEL: z.string().optional(),
  // AI OCR — OpenAI
  OPENAI_API_KEY: z.string().optional(),
  OPENAI_MODEL: z.string().optional(),
  LLM_ADAPTER: z.string().optional(),
  LLM_PROVIDER: z.string().optional(),
  // Missing from earlier schema — added 2026-06-13 audit
  COURIER_PII_ENCRYPTION_KEY: z.string().optional(),
  COURIER_ACCEPT_WINDOW_MS: z.string().optional(),
  CANCEL_AFTER_DISPATCH_WINDOW_MS: z.string().optional(),
  COURIER_DISPATCH_MAX_ATTEMPTS: z.string().optional(),
  // COURIER_DISPATCH_RETRY_MS retired (ADR-dispatch-recovery): the in-worker 30s self-retry was
  // deleted — the 60s CourierOfferSweep pump is the single dispatch retry cadence.
  // 'assigned' acceptance timeout (ms; default 300000 = 5 min in the sweep). R-OPEN-1: must
  // comfortably exceed the courier FE accept window (COURIER_ACCEPT_WINDOW_MS, 30s default).
  COURIER_ASSIGN_ACCEPT_TIMEOUT_MS: z.string().optional(),
  // ── Dispatch-exhaustion grace window (ADR-dispatch-recovery, R-NEEDS-HUMAN-1) ──
  // 🔴 Ships FLAG-OFF until the operator ratifies at STOP-ETHICS: when 'true', an order whose
  // dispatch_exhausted_at is older than DISPATCH_OWNER_GRACE_MS (default 900000 = 15 min) with
  // no owner action auto-transitions to the customer-honest terminal CANCELLED + honest push.
  DISPATCH_OWNER_GRACE_ENABLED: z.enum(['true', 'false']).default('false'),
  DISPATCH_OWNER_GRACE_MS: z.string().optional(),
  COURIER_GPS_MAX_DIST_KM: z.string().optional(),
  FLY_MACHINE_ID: z.string().optional(),
  GROQ_ENDPOINT: z.string().optional(),
  HOSTNAME: z.string().optional(),
  IP_HASH_SALT: z.string().min(1, 'Required for deterministic PII hashing — set any value in dev'),
  LLM_ENDPOINT: z.string().optional(),
  MEM0_EMBED_MODEL: z.string().optional(),
  MEM0_LLM_MODEL: z.string().optional(),
  MEM0_OLLAMA_URL: z.string().optional(),
  OPENAI_ENDPOINT: z.string().optional(),
  RENDER_GIT_COMMIT: z.string().optional(),
  TRANSLATION_ENDPOINT: z.string().optional(),
  TRANSLATION_PROVIDER: z.string().optional(),
  // Geo-seams — RoutingProvider (per-leg road routing). Provider is chosen by env,
  // never by code. All three are safe defaults so backend boot never depends on a
  // new required var; the routing seam degrades to haversine when unconfigured.
  // verify:env still fails fast on an INVALID provider value or a malformed URL.
  ROUTING_PROVIDER: z.enum(['ors', 'self', 'haversine']).default('ors'),
  ROUTING_BASE_URL: z.string().url().default('https://api.openrouteservice.org'),
  ROUTING_API_KEY: z.string().optional(),

  // ── Soft access gate (ADR-soft-access-gate) ──
  // STOP-1 enforcer: gates BACKEND route registration (POST /api/access-requests 404s
  // while off) AND frontend CTA render. Default false → feature is dark until the
  // owner-onboarding-invite-gating prerequisite ships and this is flipped (R3-4).
  ACCESS_GATE_PUBLIC_ENABLED: z.enum(['true', 'false']).default('false'),
  // Companion flag for the CI banned-strings test: scarcity copy is only permitted once
  // invite-gating has shipped and this is set (R2-10).
  ACCESS_GATE_INVITE_GATING_SHIPPED: z.enum(['true', 'false']).default('false'),
  // Operator notification for new access requests (best-effort, via Resend).
  RESEND_API_KEY: z.string().optional(),
  WAITLIST_NOTIFY_EMAIL: z.string().optional(),
  // Privacy notice version stamped on every consented row; the CI content-hash test
  // (R2-6) fails the build if the /privacy prose changes without bumping this.
  PRIVACY_NOTICE_VERSION: z.string().default('2026-06-20'),
  // 12-month retention auto-erase (STOP-2). Window must equal the number stated in /privacy.
  ACCESS_REQUEST_RETENTION: z.string().default('12 months'),
  ACCESS_REQUEST_RETENTION_CRON: z.string().default('0 3 * * *'),
  // Notify-gap reconciliation sweep cadence (B3 / R2-4).
  ACCESS_REQUEST_RECONCILE_CRON: z.string().default('*/15 * * * *'),
  // Bounded reconcile re-feed guard (R2-9): rows past this many cumulative notify
  // attempts stop being re-enqueued and surface in the aggregated alert.
  ACCESS_REQUEST_NOTIFY_MAX_ATTEMPTS: z.coerce.number().int().positive().default(10),
});

export type Env = z.infer<typeof EnvSchema>;

export function loadEnv(): Env {
  const result = EnvSchema.safeParse(process.env);
  if (!result.success) {
    const issues = result.error.issues
      .map((issue) => `- ${issue.path.join('.')}: ${issue.message}`)
      .join('\n');
    throw new Error(`Invalid environment variables:\n${issues}`);
  }
  const env = result.data;
  assertDevAuthDisabledInProd(env);
  return env;
}

/**
 * Boot-guard D (ADR-0003) — fail-fast so a production box can NEVER carry a dev-auth
 * surface. A dev bypass on prod was a live CRITICAL; this turns the next misconfig into
 * an aborted boot instead of a silent backdoor. Fires only on the DANGEROUS direction
 * (NODE_ENV=production with any dev-auth knob set); the inverse (prod NODE_ENV not
 * 'production') is caught pre-traffic by the release_command guard, not here.
 */
export function assertDevAuthDisabledInProd(env: Env): void {
  if (env.NODE_ENV !== 'production') return;
  const offenders: string[] = [];
  if (env.ALLOW_DEV_LOGIN === 'true') offenders.push('ALLOW_DEV_LOGIN');
  if (env.DEV_AUTH_SECRET) offenders.push('DEV_AUTH_SECRET');
  if (env.JWT_DEV_KID) offenders.push('JWT_DEV_KID');
  if (env.JWT_DEV_PRIVATE_KEY) offenders.push('JWT_DEV_PRIVATE_KEY');
  if (env.JWT_DEV_PUBLIC_KEY) offenders.push('JWT_DEV_PUBLIC_KEY');
  if (offenders.length > 0) {
    throw new Error(
      `FATAL: dev-auth surface present on a production box (NODE_ENV=production): ` +
        `${offenders.join(', ')} must be unset in production. Refusing to boot.`,
    );
  }
}
