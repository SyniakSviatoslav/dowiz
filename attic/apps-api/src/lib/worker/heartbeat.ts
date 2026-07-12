// @ts-nocheck
import type { Pool } from 'pg';

export interface HeartbeatOptions {
  workerId: string;
  instanceId?: string;
  jobName?: string;
  intervalMs?: number;
}

export class WorkerHeartbeat {
  private timer: NodeJS.Timeout | null = null;
  private readonly workerId: string;
  private readonly instanceId: string;
  private readonly jobName: string;
  private readonly intervalMs: number;

  constructor(
    private pool: Pool,
    opts: HeartbeatOptions,
  ) {
    this.workerId = opts.workerId;
    this.instanceId = opts.instanceId || process.env.FLY_MACHINE_ID || process.env.HOSTNAME || 'local';
    this.jobName = opts.jobName || 'unknown';
    this.intervalMs = opts.intervalMs || parseInt(process.env.WORKER_HEARTBEAT_INTERVAL_MS || '15000', 10);
  }

  start() {
    this.beat('healthy').catch(err => {
      console.error(`[Heartbeat:${this.workerId}] Initial beat failed:`, err);
    });
    this.timer = setInterval(() => {
      this.beat('healthy').catch(err => {
        console.error(`[Heartbeat:${this.workerId}] Beat failed:`, err);
      });
    }, this.intervalMs);
    this.timer.unref();
  }

  async beat(status: 'healthy' | 'degraded' = 'healthy', lastJobAt?: Date) {
    await this.pool.query(
      `INSERT INTO ops_worker_heartbeat (worker_id, instance_id, job_name, status, last_seen_at, last_job_at)
       VALUES ($1, $2, $3, $4, now(), COALESCE($5, now()))
       ON CONFLICT (worker_id) DO UPDATE SET
         instance_id = EXCLUDED.instance_id,
         job_name = EXCLUDED.job_name,
         status = EXCLUDED.status,
         last_seen_at = now(),
         last_job_at = COALESCE(EXCLUDED.last_job_at, ops_worker_heartbeat.last_job_at)`,
      [this.workerId, this.instanceId, this.jobName, status, lastJobAt || null],
    );
  }

  async markDegraded() {
    await this.beat('degraded');
  }

  stop() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }
}
