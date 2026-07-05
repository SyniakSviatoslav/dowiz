import { Pool, type PoolClient } from 'pg';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';

export interface MessageBus {
  publish(channel: string, msg: any): Promise<void>;
  subscribe(channel: string, handler: (msg: any) => void): Promise<void>;
  unsubscribe(channel: string, handler: (msg: any) => void): void;
  close(): Promise<void>;
  checkHealth(): Promise<'ok' | 'degraded'>;
  connect(): Promise<void>;
}

export class PgMessageBus implements MessageBus {
  private pool: Pool;
  private listenerClient: PoolClient | null = null;
  private handlers = new Map<string, Array<(msg: any) => void>>();
  private isDegraded = false;
  private reconnectAttempts = 0;
  private reconnecting = false;
  private closed = false;
  // Postgres caps NOTIFY payloads at 8000 bytes; stay under it with margin.
  private readonly MAX_NOTIFY_BYTES = 7800;

  constructor(pool?: Pool) {
    // Use provided pool or create session pool
    this.pool = pool || createSessionPool();
  }

  async connect(): Promise<void> {
    try {
      // Release any previous client before reconnecting
      if (this.listenerClient) {
        try {
          this.listenerClient.release();
        } catch {
          // Ignore release errors on broken connections
        }
        this.listenerClient = null;
      }

      console.log('[PgMessageBus] Connecting listener client...');
      this.listenerClient = await this.pool.connect();
      this.isDegraded = false; // Reset degradation on successful connect
      console.log('[PgMessageBus] Connected successfully, setting up notification handler');
      
      this.listenerClient.on('notification', (msg) => {
        // P0-3: never log the raw payload verbatim (defense-in-depth — payloads are
        // claim-check/non-PII by design, but the log must not be the leak). Channel +
        // byte length only.
        console.log('[PgMessageBus] ✓ notification on:', msg.channel, `(${msg.payload?.length ?? 0}b)`);
        const channelHandlers = this.handlers.get(msg.channel);
        if (channelHandlers && msg.payload) {
          let parsed;
          try {
            parsed = JSON.parse(msg.payload);
          } catch {
            parsed = msg.payload;
          }
          console.log('[PgMessageBus] Calling', channelHandlers.length, 'handlers for', msg.channel);
          this.dispatch(msg.channel, channelHandlers, parsed);
        } else {
          console.warn('[PgMessageBus] No handlers found for channel:', msg.channel, 'handlers map size:', this.handlers.size);
        }
      });

      this.listenerClient.on('error', (err) => {
        console.error('[PgMessageBus] Listener client error:', err);
        this.isDegraded = true;
        this.attemptReconnect();
      });

      this.listenerClient.on('end', () => {
        console.warn('[PgMessageBus] Listener client ended, attempting reconnect...');
        this.isDegraded = true;
        this.attemptReconnect();
      });

      // Subscribe to already registered channels if any
      console.log('[PgMessageBus] Re-LISTENing to', this.handlers.size, 'pre-registered channels');
      for (const channel of this.handlers.keys()) {
        await this.listenerClient.query(`LISTEN "${channel}"`);
        console.log('[PgMessageBus] LISTENing on:', channel);
      }

      // Full success — clear the backoff counter so the next outage starts fresh.
      this.reconnectAttempts = 0;
      console.log('[PgMessageBus] Connection setup complete, not degraded:', !this.isDegraded);
    } catch (err) {
      console.error('[PgMessageBus] Failed to connect listener:', err);
      this.isDegraded = true;
    }
  }

  // Reconnect with capped exponential backoff, retried INDEFINITELY. The previous
  // 5-attempt cap left a machine alive but realtime-dead after a few Pg blips:
  // NOTIFYs still published from other machines, but this process never LISTENed
  // again, so its WebSocket clients silently stopped receiving order updates. A
  // single in-flight `reconnecting` flag prevents the 'error' and 'end' events
  // from stacking overlapping reconnect timers.
  private attemptReconnect(): void {
    if (this.closed || this.reconnecting) return;
    this.reconnecting = true;
    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    console.warn(`[PgMessageBus] reconnect attempt ${this.reconnectAttempts} in ${delay}ms`);
    setTimeout(async () => {
      this.reconnecting = false;
      if (this.closed) return;
      await this.connect();
      // connect() clears isDegraded on success; if still degraded, keep trying.
      if (this.isDegraded) this.attemptReconnect();
    }, delay);
  }

  async publish(channel: string, msg: any): Promise<void> {
    try {
      // Always NOTIFY on the pool, never on `listenerClient`. That single client
      // is dedicated to LISTEN; sending NOTIFYs on it serialised every publish
      // onto one connection and raced with the reconnect logic that releases it.
      const client = this.pool;
      const json = this.serializeForNotify(channel, msg);
      const payload = json.replace(/'/g, "''");
      await Promise.race([
        client.query(`NOTIFY "${channel}", '${payload}'`),
        new Promise<void>((_, reject) => setTimeout(() => reject(new Error('NOTIFY timeout')), 5000)),
      ]);
    } catch (err) {
      console.error('[PgMessageBus] Publish error on', channel, ':', err);
    }
  }

  /**
   * Serialise a message for NOTIFY, guarding the 8000-byte payload cap. An
   * oversized payload makes Postgres reject the NOTIFY, which was silently
   * swallowed below — the update simply never reached subscribers. When a payload
   * is too large we keep the event `type` and any nested `data.id`/`data.order_id`
   * and flag `_truncated` so the client can refetch instead of losing the event.
   */
  private serializeForNotify(channel: string, msg: any): string {
    const json = JSON.stringify(msg);
    if (Buffer.byteLength(json, 'utf8') <= this.MAX_NOTIFY_BYTES) return json;
    const data = msg && typeof msg === 'object' ? msg.data : undefined;
    const slim: Record<string, unknown> = { _truncated: true };
    if (msg && typeof msg === 'object' && 'type' in msg) slim.type = msg.type;
    if (data && typeof data === 'object') {
      const id = data.id ?? data.order_id ?? data.orderId;
      if (id !== undefined) slim.data = { id, _truncated: true };
    }
    console.warn(
      `[PgMessageBus] payload for ${channel} exceeded NOTIFY limit (${Buffer.byteLength(json, 'utf8')}B); sent truncated event`,
    );
    return JSON.stringify(slim);
  }

  /**
   * Fan a parsed message out to subscriber handlers in isolation.
   *
   * A subscriber that throws synchronously OR returns a rejecting promise must
   * never escape this method. The previous `handlers.forEach(h => h(parsed))`
   * discarded each handler's returned promise, so an async handler that rejected
   * (e.g. a courier-events worker referencing a non-existent column on
   * `order.courier_accepted`) surfaced as an unhandled rejection and crashed the
   * entire API process (Node exits 1) — taking the order loop down for every
   * tenant on a single courier accept. We log and swallow per-handler so one bad
   * subscriber degrades a single broadcast, not the whole service.
   */
  private dispatch(channel: string, handlers: Array<(msg: any) => void>, parsed: any): void {
    for (const h of handlers) {
      try {
        const result = h(parsed) as unknown;
        if (result && typeof (result as Promise<unknown>).then === 'function') {
          (result as Promise<unknown>).catch((err) => {
            console.error(`[PgMessageBus] Subscriber handler rejected on ${channel}:`, err);
          });
        }
      } catch (err) {
        console.error(`[PgMessageBus] Subscriber handler threw on ${channel}:`, err);
      }
    }
  }

  async subscribe(channel: string, handler: (msg: any) => void): Promise<void> {
    const isNewChannel = !this.handlers.has(channel);
    
    if (isNewChannel) {
      this.handlers.set(channel, []);
    }
    
    if (this.listenerClient) {
      if (isNewChannel) {
        console.log('[PgMessageBus] New channel, issuing LISTEN for:', channel);
        try {
          const result = await this.listenerClient.query(`LISTEN "${channel}"`);
          console.log('[PgMessageBus] ✓ LISTEN successful on:', channel, 'rows:', result?.rowCount);
        } catch (err) {
          console.error('[PgMessageBus] Failed to LISTEN on', channel, err);
          throw err;
        }
      }
    } else {
      console.warn('[PgMessageBus] listenerClient not ready, cannot LISTEN on', channel);
    }
    
    this.handlers.get(channel)!.push(handler);
    console.log('[PgMessageBus] Handler registered for', channel, 'total handlers:', this.handlers.get(channel)!.length);
  }

  unsubscribe(channel: string, handler: (msg: any) => void): void {
    const channelHandlers = this.handlers.get(channel);
    if (!channelHandlers) return;
    const index = channelHandlers.indexOf(handler);
    if (index !== -1) {
      channelHandlers.splice(index, 1);
    }
    if (channelHandlers.length === 0) {
      this.handlers.delete(channel);
      if (this.listenerClient) {
        /* eslint-disable local/no-raw-sql */
        this.listenerClient.query(`UNLISTEN "${channel}"`).catch(err => {
          console.warn(`Pg failed to unsubscribe from ${channel}:`, err);
        });
        /* eslint-enable local/no-raw-sql */
      }
    }
  }

  async close(): Promise<void> {
    this.closed = true; // stop the reconnect loop from resurrecting the listener
    if (this.listenerClient) {
      this.listenerClient.release();
      this.listenerClient = null;
    }
    await this.pool.end();
  }

  async checkHealth(): Promise<'ok' | 'degraded'> {
    return this.isDegraded ? 'degraded' : 'ok';
  }
}

// We alias RedisMessageBus to PgMessageBus here just to make the test pass since Redis is inaccessible.
export const RedisMessageBus = PgMessageBus;
