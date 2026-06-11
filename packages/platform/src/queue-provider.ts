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
}

export class PgBossQueueProvider implements QueueProvider {
  private boss: PgBoss;

  constructor() {
    const env = loadEnv();
    this.boss = new PgBoss({
      connectionString: env.DATABASE_URL_OPERATIONAL,
      max: 4,
      application_name: 'pgboss',
      schema: 'public',
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

  async stop(): Promise<void> {
    await this.boss.stop({ graceful: true, wait: true });
  }
}
