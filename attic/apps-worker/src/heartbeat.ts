import type { Pool } from 'pg';

export class Heartbeat {
  private timer: NodeJS.Timeout | null = null;
  private readonly workerId: string;

  constructor(
    private pool: Pool,
    workerId: string = process.env.FLY_MACHINE_ID || 'local-worker'
  ) {
    this.workerId = workerId;
  }

  start(intervalMs = 20000) {
    // Initial beat
    this.beat().catch(err => console.error('Initial heartbeat failed:', err));

    this.timer = setInterval(() => {
      this.beat().catch(err => console.error('Heartbeat failed:', err));
    }, intervalMs);
    
    // Don't keep the process alive just for the heartbeat
    this.timer.unref();
  }

  private async beat() {
    await this.pool.query(
      `INSERT INTO ops_worker_heartbeat (worker_id, last_seen_at)
       VALUES ($1, now())
       ON CONFLICT (worker_id) DO UPDATE SET last_seen_at = now()`,
      [this.workerId]
    );
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
  }
}
