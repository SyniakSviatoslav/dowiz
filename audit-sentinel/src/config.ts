import { z } from 'zod';

const envSchema = z.object({
  ENV: z.enum(['staging', 'prod']).default('staging'),
  BASE_URL: z.string().url(),
  MENU_URL: z.string().url().optional(),
  SITE_URL: z.string().url().optional(),
  TEST_TENANT: z.string().default('demo'),
  ***REDACTED***: z.string().optional(),
  TELEGRAM_CHAT_ID: z.string().optional(),
  GITHUB_TOKEN: z.string().optional(),
  GITHUB_REPO: z.string().optional(),
  AUDIT_AGENT_ENABLED: z.string().default('true'),
  CERT_EXPIRY_WARN_DAYS: z.coerce.number().default(14),
  RATE_LIMIT_BUDGET: z.coerce.number().default(50),
});

export type Env = z.infer<typeof envSchema>;

export function loadEnv(): Env {
  return envSchema.parse(process.env);
}

export const ALLOWLIST_HOSTS = [
  /^https:\/\/.*\.dowiz\.org/,
  /^https:\/\/.*\.fly\.dev/,
  /^https:\/\/api\.anthropic\.com/,
  /^https:\/\/api\.telegram\.org/,
  /^https:\/\/api\.github\.com/,
];

export function isHostAllowed(url: string): boolean {
  return ALLOWLIST_HOSTS.some((pattern) => pattern.test(url));
}
