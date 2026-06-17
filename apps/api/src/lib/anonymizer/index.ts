// @ts-nocheck
import type { Pool, PoolClient } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import type { StorageProvider } from '../../ports.js';
import { BUS_CHANNELS } from '../registry.js';

export type AnonymizationScope = 'retention' | 'gdpr';
export type ActorKind = 'system' | 'owner' | 'customer';
export type SubjectKind = 'customer' | 'order';

export interface AnonymizeOptions {
  scope: AnonymizationScope;
  subject?: {
    customerId?: string;
    orderId?: string;
    locationId?: string;
  };
  batchSize?: number;
  dryRun?: boolean;
  actorId?: string;
}

export interface AnonymizeResult {
  customersAnonymized: number;
  ordersAnonymized: number;
  storagePurged: number;
  r2Marked: number;
  skipped: number;
  durationMs: number;
  dryRun: boolean;
}

interface AnonymizeSubResult {
  anon: boolean;
  skipped: boolean;
  storagePurged: number;
}

export class AnonymizerService {
  constructor(
    private pool: Pool,
    private messageBus: MessageBus,
    private storage?: StorageProvider,
  ) {}

  async anonymize(options: AnonymizeOptions): Promise<AnonymizeResult> {
    const start = Date.now();
    const batchSize = options.batchSize || 100;
    let customersAnonymized = 0;
    let ordersAnonymized = 0;
    let storagePurged = 0;
    let r2Marked = 0;
    let skipped = 0;

    if (options.dryRun) {
      if (options.subject?.customerId) {
        const res = await this.pool.query(
          `SELECT anonymized_at IS NOT NULL AS done FROM customers WHERE id = $1`,
          [options.subject.customerId],
        );
        if (res.rows.length > 0 && !res.rows[0].done) customersAnonymized = 1;
      }
      if (options.subject?.orderId) {
        const res = await this.pool.query(
          `SELECT anonymized_at IS NOT NULL AS done FROM orders WHERE id = $1`,
          [options.subject.orderId],
        );
        if (res.rows.length > 0 && !res.rows[0].done) ordersAnonymized = 1;
      }
      return { customersAnonymized, ordersAnonymized, storagePurged, r2Marked, skipped, durationMs: Date.now() - start, dryRun: true };
    }

    if (options.subject?.customerId) {
      const result = await this.anonymizeCustomer(options.subject.customerId, options);
      customersAnonymized += result.anon ? 1 : 0;
      skipped += result.skipped ? 1 : 0;
      storagePurged += result.storagePurged;
    }

    if (options.subject?.orderId) {
      const result = await this.anonymizeOrder(options.subject.orderId, options);
      ordersAnonymized += result.anon ? 1 : 0;
      skipped += result.skipped ? 1 : 0;
    }

    if (!options.subject && options.scope === 'retention') {
      const customers = await this.findExpiredCustomers(options.subject?.locationId, batchSize);
      for (const c of customers) {
        const result = await this.anonymizeCustomer(c.id, { ...options, subject: { customerId: c.id, locationId: c.location_id } });
        if (result.anon) customersAnonymized++;
        else skipped++;
        storagePurged += result.storagePurged;
      }

      const orders = await this.findExpiredOrders(options.subject?.locationId, batchSize);
      for (const o of orders) {
        const result = await this.anonymizeOrder(o.id, { ...options, subject: { orderId: o.id, locationId: o.location_id } });
        if (result.anon) ordersAnonymized++;
        else skipped++;
      }
    }

    return {
      customersAnonymized,
      ordersAnonymized,
      storagePurged,
      r2Marked,
      skipped,
      durationMs: Date.now() - start,
      dryRun: false,
    };
  }

  private async anonymizeCustomer(customerId: string, options: AnonymizeOptions): Promise<AnonymizeSubResult> {
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');
      const lockRes = await client.query(
        `SELECT anonymized_at, location_id FROM customers WHERE id = $1 FOR UPDATE`,
        [customerId],
      );
      if (lockRes.rows.length === 0) {
        await client.query('ROLLBACK');
        return { anon: false, skipped: true, storagePurged: 0 };
      }
      const row = lockRes.rows[0];
      if (row.anonymized_at) {
        await client.query('ROLLBACK');
        return { anon: false, skipped: true, storagePurged: 0 };
      }
      const locationId = options.subject?.locationId || row.location_id;

      await client.query(
        `UPDATE customers
         SET phone = 'anon_' || gen_random_uuid()::text,
             name = NULL,
             marketing_opt_in = false,
             anonymized_at = now()
         WHERE id = $1`,
        [customerId],
      );

      let storagePurged = 0;
      if (this.storage) {
        const hasAvatarKey = await this.columnExists(client, 'customers', 'avatar_key');
        if (hasAvatarKey) {
          const avatarRes = await client.query(
            `SELECT avatar_key FROM customers WHERE id = $1`,
            [customerId],
          );
          const avatarKey = avatarRes.rows[0]?.avatar_key;
          if (avatarKey) {
            try {
              await this.storage.delete(avatarKey);
              storagePurged = 1;
            } catch (err) {
              console.error('[Anonymizer] Failed to purge storage:', err);
            }
          }
        }
      }

      await this.insertAuditLog(client, {
        scope: options.scope,
        subjectKind: 'customer',
        subjectId: customerId,
        locationId,
        actorKind: options.actorId ? 'owner' : 'system',
        actorId: options.actorId || null,
        metadata: { counts: { customersAnonymized: 1, ordersAnonymized: 0 } },
      });

      await client.query('COMMIT');

      await this.messageBus.publish(BUS_CHANNELS.CUSTOMER_ANONYMIZED, {
        customerId,
        locationId,
        scope: options.scope,
        timestamp: new Date().toISOString(),
      });

      return { anon: true, skipped: false, storagePurged };
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  }

  private async anonymizeOrder(orderId: string, options: AnonymizeOptions): Promise<AnonymizeSubResult> {
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');
      const lockRes = await client.query(
        `SELECT anonymized_at, location_id FROM orders WHERE id = $1 FOR UPDATE`,
        [orderId],
      );
      if (lockRes.rows.length === 0) {
        await client.query('ROLLBACK');
        return { anon: false, skipped: true, storagePurged: 0 };
      }
      const row = lockRes.rows[0];
      if (row.anonymized_at) {
        await client.query('ROLLBACK');
        return { anon: false, skipped: true, storagePurged: 0 };
      }
      const locationId = options.subject?.locationId || row.location_id;

      await client.query(
        `UPDATE orders
         SET client_ip_hash = NULL,
             delivery_address = NULL,
             anonymized_at = now()
         WHERE id = $1`,
        [orderId],
      );

      // Clear rating comment (PII) while preserving stars for analytics.
      // Guard: ignore if migration hasn't run yet on this environment.
      try {
        await client.query(
          `UPDATE order_ratings SET comment = NULL WHERE order_id = $1 AND comment IS NOT NULL`,
          [orderId],
        );
      } catch (err: any) {
        if (err.code !== '42P01') throw err; // 42P01 = undefined_table
      }

      await this.insertAuditLog(client, {
        scope: options.scope,
        subjectKind: 'order',
        subjectId: orderId,
        locationId,
        actorKind: options.actorId ? 'owner' : 'system',
        actorId: options.actorId || null,
        metadata: { counts: { customersAnonymized: 0, ordersAnonymized: 1 } },
      });

      await client.query('COMMIT');

      await this.messageBus.publish(BUS_CHANNELS.ORDER_ANONYMIZED, {
        orderId,
        locationId,
        scope: options.scope,
        timestamp: new Date().toISOString(),
      });

      return { anon: true, skipped: false, storagePurged: 0 };
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  }

  private async findExpiredCustomers(locationId: string | undefined, limit: number): Promise<Array<{ id: string; location_id: string }>> {
    const res = await this.pool.query(
      `SELECT c.id, c.location_id FROM customers c
       WHERE c.anonymized_at IS NULL
         AND c.created_at < now() - (SELECT retention_days FROM locations WHERE id = c.location_id) * interval '1 day'
         AND ($1::uuid IS NULL OR c.location_id = $1)
       ORDER BY c.created_at ASC
       LIMIT $2`,
      [locationId || null, limit],
    );
    return res.rows;
  }

  private async findExpiredOrders(locationId: string | undefined, limit: number): Promise<Array<{ id: string; location_id: string }>> {
    const res = await this.pool.query(
      `SELECT o.id, o.location_id FROM orders o
       WHERE o.anonymized_at IS NULL
         AND o.created_at < now() - (SELECT retention_days FROM locations WHERE id = o.location_id) * interval '1 day'
         AND ($1::uuid IS NULL OR o.location_id = $1)
       ORDER BY o.created_at ASC
       LIMIT $2`,
      [locationId || null, limit],
    );
    return res.rows;
  }

  private async insertAuditLog(
    client: PoolClient,
    params: {
      scope: AnonymizationScope;
      subjectKind: SubjectKind;
      subjectId: string;
      locationId: string;
      actorKind: ActorKind;
      actorId: string | null;
      metadata: Record<string, unknown>;
    },
  ): Promise<void> {
    await client.query(
      `INSERT INTO anonymization_audit_log (scope, subject_kind, subject_id, location_id, actor_kind, actor_id, metadata)
       VALUES ($1, $2, $3, $4, $5, $6, $7)`,
      [params.scope, params.subjectKind, params.subjectId, params.locationId, params.actorKind, params.actorId, JSON.stringify(params.metadata)],
    );
  }

  private async columnExists(client: PoolClient, table: string, column: string): Promise<boolean> {
    const res = await client.query(
      `SELECT TRUE FROM pg_attribute
       WHERE attrelid = $1::regclass
         AND attname = $2
         AND NOT attisdropped`,
      [table, column],
    );
    return res.rowCount !== null && res.rowCount > 0;
  }
}
