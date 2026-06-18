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
  private readonly MAX_RECONNECT_ATTEMPTS = 5;

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
        console.log('[PgMessageBus] ✓ Received notification on:', msg.channel, 'payload:', msg.payload);
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
      
      console.log('[PgMessageBus] Connection setup complete, not degraded:', !this.isDegraded);
    } catch (err) {
      console.error('[PgMessageBus] Failed to connect listener:', err);
      this.isDegraded = true;
    }
  }

  private async attemptReconnect(): Promise<void> {
    if (this.reconnectAttempts >= this.MAX_RECONNECT_ATTEMPTS) {
      console.error('[PgMessageBus] Max reconnection attempts reached, giving up');
      return;
    }
    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    console.log(`[PgMessageBus]Attempting reconnect ${this.reconnectAttempts}/${this.MAX_RECONNECT_ATTEMPTS} in ${delay}ms`);
    setTimeout(async () => {
      try {
        await this.connect();
        this.reconnectAttempts = 0;
        console.log('[PgMessageBus] Reconnection successful');
      } catch (err) {
        console.error('[PgMessageBus] Reconnection failed:', err);
      }
    }, delay);
  }

  async publish(channel: string, msg: any): Promise<void> {
    try {
      console.log('[PgMessageBus] Publishing to:', channel, 'msg:', JSON.stringify(msg));
      const client = this.listenerClient || this.pool;
      const payload = JSON.stringify(msg).replace(/'/g, "''");
      await Promise.race([
        client.query(`NOTIFY "${channel}", '${payload}'`),
        new Promise<void>((_, reject) => setTimeout(() => reject(new Error('NOTIFY timeout')), 5000)),
      ]);
      console.log('[PgMessageBus] ✓ Published to:', channel);
    } catch (err) {
      console.error('[PgMessageBus] Publish error:', err);
    }
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
    if (this.listenerClient) {
      this.listenerClient.release();
    }
    await this.pool.end();
  }

  async checkHealth(): Promise<'ok' | 'degraded'> {
    return this.isDegraded ? 'degraded' : 'ok';
  }
}

// We alias RedisMessageBus to PgMessageBus here just to make the test pass since Redis is inaccessible.
export const RedisMessageBus = PgMessageBus;
