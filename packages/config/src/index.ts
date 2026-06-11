import { z } from "zod";

const EnvSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]),
  PORT: z.coerce.number().int().positive().default(8080),
  APP_BASE_URL: z.string().url(),
  ***REDACTED***: z.string().url(),   // transaction pooler :6543
  ***REDACTED***: z.string().url(),       // session pooler :5432
  ***REDACTED***: z.string().url(),    // session pooler :5432
  REDIS_URL: z.string().url(),                  // upstash, pub/sub only
  ***REDACTED***: z.string().min(1),
  ***REDACTED***: z.string().min(1),
  JWT_KID: z.string().min(1),
  ***REDACTED***: z.string().min(1),
  ***REDACTED***: z.string().min(1),
  ***REDACTED***: z.string().optional(),
  ***REDACTED***: z.string().optional(),
  TELEGRAM_BOT_USERNAME: z.string().optional(),
  // Backup Configuration
  BACKUP_ENCRYPTION_KEY: z.string().optional(), // 32 bytes base64 (required if BACKUP_ENABLED=true)
  R2_ACCESS_KEY_ID: z.string().optional(),
  R2_SECRET_ACCESS_KEY: z.string().optional(),
  R2_ENDPOINT: z.string().optional(),
  R2_BUCKET: z.string().optional(),
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
  // AI OCR — Groq
  GROQ_API_KEY: z.string().optional(),
  GROQ_MODEL: z.string().optional(),
  // AI OCR — OpenAI
  OPENAI_API_KEY: z.string().optional(),
  OPENAI_MODEL: z.string().optional(),
  LLM_ADAPTER: z.string().optional(),
  LLM_PROVIDER: z.string().optional(),
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
