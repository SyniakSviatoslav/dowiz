import type { Pool } from 'pg';
import type Boss from 'pg-boss';

const DEBOUNCE_WINDOW_MS = 5000; // flush every 5s

interface VelocityQueueItem {
  locationId: string;
  phoneHash?: string;
  clientIpHash?: string;
  kind: 'order_placed' | 'order_cancelled';
  timestamp: string;
}

export class VelocityIncrementer {
  private buffer: VelocityQueueItem[] = [];
  private flushTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private pool: Pool,
    private boss: Boss,
  ) {}

  async increment(
    locationId: string,
    opts: { phoneHash?: string; clientIpHash?: string },
    kind: 'order_placed' | 'order_cancelled' = 'order_placed',
  ): Promise<void> {
    this.buffer.push({
      locationId,
      phoneHash: opts.phoneHash,
      clientIpHash: opts.clientIpHash,
      kind,
      timestamp: new Date().toISOString(),
    });

    this.scheduleFlush();
  }

  private scheduleFlush() {
    if (this.flushTimer) return;
    this.flushTimer = setTimeout(() => {
      this.flushTimer = null;
      this.flush();
    }, DEBOUNCE_WINDOW_MS);
  }

  private async flush() {
    const items = this.buffer.splice(0, this.buffer.length);
    if (items.length === 0) return;

    try {
      // Use pg-boss for async durable flush
      await this.boss.send('velocity.flush', { items }, { singletonKey: 'velocity.flush', retryLimit: 3 });
    } catch (err) {
      console.error('[VelocityIncrementer] Failed to schedule flush:', err);
    }
  }

  async handleFlush(job: any) {
    const { items } = job.data as { items: VelocityQueueItem[] };
    const client = await this.pool.connect();
    try {
      for (const item of items) {
        await client.query(
          `INSERT INTO velocity_events (location_id, phone_hash, client_ip_hash, kind, window_started_at)
           VALUES ($1, $2, $3, $4, $5)`,
          [item.locationId, item.phoneHash || null, item.clientIpHash || null, item.kind, item.timestamp],
        );
      }
    } catch (err) {
      console.error('[VelocityIncrementer] Flush INSERT error:', err);
      throw err;
    } finally {
      client.release();
    }
  }
}
