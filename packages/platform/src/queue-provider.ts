import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

export interface EnqueueOptions {
  singletonKey?: string;
  startAfter?: number | Date | string;
  db?: any;
}

export interface QueueProvider {
  enqueue(name: string, payload: any, opts?: EnqueueOptions): Promise<string | null>;
  work(name: string, handler: (payload: any) => Promise<void>): Promise<void>;
  start(): Promise<void>;
  stop(): Promise<void>;
  // Optional: not every caller creates queues through the provider (many still call
  // the raw `.boss.createQueue()` directly — see createQueueWithDefaults below for
  // those call sites). Optional keeps this additive/non-breaking for existing fakes.
  createQueue?(name: string, options?: QueueReliabilityOptions): Promise<void>;
}

// ── Reliability defaults for pg-boss v10 createQueue (H1/H2, 2026-07-03 audit) ──
// v10 runtime defaults: retryLimit=2, retryDelay=0s, no backoff, no deadLetter — a
// transiently-failing job is hammered twice within milliseconds, then lands
// permanently in `failed` with no salvage path. Separately, v10 only honors
// `singletonKey`-only dedup (no `singletonSeconds`) when the QUEUE's own policy is
// `short` — every queue created bare defaults to policy `standard`, which silently
// turns singletonKey-only dedup into a no-op fleet-wide.
//
// These helpers are additive/config-only: they set queue-creation options, never job
// payloads, handler signatures, or call-site retry/dedup semantics.
export interface QueueReliabilityOptions {
  retryLimit?: number;
  retryDelay?: number;
  retryBackoff?: boolean;
  expireInSeconds?: number;
  /** pg-boss v10 queue policy. 'short' makes singletonKey-only dedup on this queue
   *  actually work (blocks enqueueing a 2nd 'created' job with the same key). */
  policy?: 'standard' | 'short' | 'singleton' | 'stately';
  /** true → auto-create `${name}.dlq` and route exhausted jobs there.
   *  string → route to that (existing or about-to-be-created) queue name instead. */
  deadLetter?: boolean | string;
}

const DEFAULT_RELIABILITY_OPTIONS: Required<
  Pick<QueueReliabilityOptions, 'retryLimit' | 'retryDelay' | 'retryBackoff'>
> = {
  retryLimit: 3,
  retryDelay: 30, // seconds — v10 default is 0s (hammers a transient failure instantly)
  retryBackoff: true,
};

export function deadLetterQueueName(name: string): string {
  return `${name}.dlq`;
}

interface BossLike {
  createQueue(name: string, options?: any): Promise<void>;
}

/**
 * Creates a pg-boss v10 queue with sane retry/backoff defaults and, on request, a
 * dedicated dead-letter queue. v10 requires the DLQ target queue to already exist
 * (self-referencing FK on pgboss.queue), so it is created first when requested.
 *
 * Conservative by design: only adds retry/backoff/DLQ/policy config at queue-creation
 * time. Does not touch job payloads, handler signatures, or existing enqueue/work call
 * sites — callers keep passing whatever `singletonKey`/`startAfter` options they did
 * before.
 */
export async function createQueueWithDefaults(
  boss: BossLike,
  name: string,
  overrides: QueueReliabilityOptions = {},
): Promise<void> {
  const { deadLetter, ...rest } = overrides;
  const options: Record<string, unknown> = { ...DEFAULT_RELIABILITY_OPTIONS, ...rest };

  if (deadLetter) {
    const dlqName = typeof deadLetter === 'string' ? deadLetter : deadLetterQueueName(name);
    // Terminal sink — intentionally no retry policy of its own.
    await boss.createQueue(dlqName);
    options.deadLetter = dlqName;
  }

  await boss.createQueue(name, options);
}

export class PgBossQueueProvider implements QueueProvider {
  public boss: PgBoss;

  constructor(connectionString?: string) {
    const env = loadEnv();
    // CRITICAL: pg-boss uses session-mode connection (port 5432) for DDL + LISTEN/NOTIFY
    // Transaction pooler (port 6543) blocks both. Server.ts constructs the URL with port 5432.
    const dbUrl = connectionString || env.DATABASE_URL_OPERATIONAL;
    console.log('[PgBoss] Using database URL:', dbUrl === env.DATABASE_URL_OPERATIONAL ? 'DATABASE_URL_OPERATIONAL' : 'provided or fallback');
    this.boss = new PgBoss({
      connectionString: dbUrl,
      max: 4,
      application_name: 'pgboss',
      schema: 'pgboss',
      migrate: false,
    });

    this.boss.on('error', error => console.error('pg-boss error:', error));
  }

  async start(): Promise<void> {
    await this.boss.start();
  }

  async enqueue(name: string, payload: any, opts?: EnqueueOptions): Promise<string | null> {
    return this.boss.send(name, payload, opts as any);
  }

  async work(name: string, handler: (payload: any) => Promise<void>): Promise<void> {
    await this.boss.work(name, async (jobs) => {
      const jobArray = Array.isArray(jobs) ? jobs : [jobs];
      for (const job of jobArray) {
        await handler(job.data);
      }
    });
  }

  async createQueue(name: string, options?: QueueReliabilityOptions): Promise<void> {
    await createQueueWithDefaults(this.boss, name, options);
  }

  async stop(): Promise<void> {
    await this.boss.stop({ graceful: true, wait: true });
  }
}
