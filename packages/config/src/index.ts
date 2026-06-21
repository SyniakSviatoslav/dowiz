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
  // Shared secret gating the /dev and /api/dev test-only endpoints (mock-auth, etc).
  // When unset/empty, those endpoints are fully disabled (404) — the safe prod default.
  DEV_AUTH_SECRET: z.string().optional(),
  TELEGRAM_BOT_TOKEN: z.string().optional(),
  TELEGRAM_BOT_SECRET: z.string().optional(),
  TELEGRAM_BOT_USERNAME: z.string().optional(),
  // WhatsApp (Baileys) notification channel
  WHATSAPP_ENABLED: z.enum(['true', 'false']).default('false'),
  WHATSAPP_AUTH_DIR: z.string().optional(),
  // Phone OTP verification — globally disabled until a real SMS gateway is wired
  // (current OTP send is a console.log scaffold). Flip to 'true' to re-enable;
  // per-location `require_phone_otp` only takes effect when this is 'true'.
  OTP_ENABLED: z.enum(['true', 'false']).default('false'),
  // Backup Configuration
  BACKUP_ENCRYPTION_KEY: z.string().optional(), // 32 bytes base64 (required if BACKUP_ENABLED=true)
  R2_ACCESS_KEY_ID: z.string().optional(),
  R2_SECRET_ACCESS_KEY: z.string().optional(),
  R2_ENDPOINT: z.string().optional(),
  R2_BUCKET: z.string().optional(),
  R2_PUBLIC_URL: z.string().optional(),
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
  WORKER_CRITICAL_LIST: z.string().default('dispatcher,settlement-cron,dwell-monitor,anonymizer-retention'),
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
  COURIER_DISPATCH_RETRY_MS: z.string().optional(),
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
  return result.data;
}
