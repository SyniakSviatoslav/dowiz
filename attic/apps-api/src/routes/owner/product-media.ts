/**
 * Owner rich-media CRUD for the cinematic product-media seam (ADR-0002).
 *
 * Flow: presign → client PUTs straight to R2 → confirm (server re-validates the
 * actual bytes, then writes ONE product_media row through the operational pool
 * via withTenant — never a BYPASSRLS write, RC1). Plus set-primary / reorder /
 * available-toggle. All gated to the owner role; location_id is resolved
 * server-side from membership, never trusted from the client.
 *
 * R2 is absent on staging, so the security-critical logic (magic-byte sniff,
 * per-location budget, frame-count bounds, mime allow-list) lives in
 * lib/product-media-validation.ts and is unit-tested there.
 */
import type { FastifyInstance } from 'fastify';
import crypto from 'node:crypto';
import { withTenant } from '@deliveryos/platform';
import {
  isAllowedMime,
  isAllowedPosterMime,
  extForMime,
  maxBytesForMime,
  checkBudget,
  checkFrameCount,
  sumIncomingBytes,
  LOCATION_BUDGET_BYTES,
  type ProductMediaKind,
} from '../../lib/product-media-validation.js';

const PRESIGN_TTL_SECONDS = 300; // ≤5 min — short-lived PUT window (contract).
const KINDS: ProductMediaKind[] = ['image', 'video', 'spin', 'model'];

interface PresignItem {
  mimeType: string;
  bytes: number;
  sha256: string;
  poster?: boolean; // raster poster for a video/spin — webp/jpeg only, never svg
}

export default async function ownerProductMediaRoutes(
  fastify: FastifyInstance,
  opts: { db: any; storage?: any },
) {
  const { db, storage } = opts;

  // Resolve the owner's location from membership (server-side; never client-supplied).
  async function getOwnerLocation(request: any): Promise<{ locId: string; userId: string } | null> {
    const user = request.user;
    if (!user?.userId) return null;
    // P-d (ADR-0004): verify the baked activeLocationId against a LIVE active owner membership —
    // a removed/downgraded owner's ≤24h token must not write media into a tenant it left.
    if (user.activeLocationId) {
      const ok = await db.query(
        `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active' LIMIT 1`,
        [user.userId, user.activeLocationId],
      );
      return (ok.rowCount ?? 0) > 0 ? { locId: user.activeLocationId, userId: user.userId } : null;
    }
    const res = await db.query(
      `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' AND status = 'active' LIMIT 1`,
      [user.userId],
    );
    if (res.rows.length === 0) return null;
    return { locId: res.rows[0].location_id, userId: user.userId };
  }

  // Confirm the product belongs to the owner's location (defence-in-depth on top of RLS).
  async function productInLocation(productId: string, locId: string): Promise<boolean> {
    const r = await db.query(`SELECT 1 FROM products WHERE id = $1 AND location_id = $2`, [productId, locId]);
    return r.rowCount > 0;
  }

  // SUM(bytes) already stored for a location.
  async function locationUsedBytes(locId: string): Promise<number> {
    const r = await db.query(
      `SELECT COALESCE(SUM(bytes), 0)::bigint AS used FROM product_media WHERE location_id = $1`,
      [locId],
    );
    return Number(r.rows[0]?.used || 0);
  }

  // ─── POST /menu/products/:productId/media/presign ──────────────────────
  fastify.post(
    '/menu/products/:productId/media/presign',
    {
      preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])],
      config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
    },
    async (request: any, reply: any) => {
      const ctx = await getOwnerLocation(request);
      if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
      const productId = request.params.productId as string;
      if (!(await productInLocation(productId, ctx.locId))) {
        return reply.status(404).send({ error: 'Product not found' });
      }

      const body = (request.body || {}) as { kind?: string; items?: PresignItem[] };
      const kind = body.kind as ProductMediaKind;
      if (!KINDS.includes(kind)) return reply.status(400).send({ error: 'Invalid media kind' });
      const items = Array.isArray(body.items) ? body.items : [];
      if (items.length === 0) return reply.status(400).send({ error: 'No items' });

      // Spin frame-count bounds (each item is a frame).
      if (kind === 'spin') {
        const fc = checkFrameCount(items.length);
        if (!fc.ok) return reply.status(400).send({ error: fc.reason });
      }

      // Per-item: mime allow-list + per-file size ceiling. Poster items are
      // raster-only (webp/jpeg, never svg).
      for (const it of items) {
        const mime = String(it.mimeType || '');
        const bytes = Number(it.bytes || 0);
        const sha = String(it.sha256 || '');
        if (!/^[0-9a-f]{64}$/i.test(sha)) return reply.status(400).send({ error: 'Invalid sha256' });
        const allowed = it.poster ? isAllowedPosterMime(mime) : isAllowedMime(mime);
        if (!allowed) return reply.status(400).send({ error: `Disallowed mime: ${mime}` });
        if (!(bytes > 0)) return reply.status(400).send({ error: 'Invalid bytes' });
        if (bytes > maxBytesForMime(mime)) {
          return reply.status(413).send({ error: `File exceeds size limit for ${mime}` });
        }
      }

      // Per-location budget: SUM(existing) + incoming ≤ 150MB → else 413.
      const used = await locationUsedBytes(ctx.locId);
      const incoming = sumIncomingBytes(items);
      const budget = checkBudget(used, incoming, LOCATION_BUDGET_BYTES);
      if (!budget.ok) {
        return reply
          .status(413)
          .send({ error: 'Storage budget exceeded', used: budget.used, incoming: budget.incoming, limit: budget.limit });
      }

      // Build content-addressed, tenant-scoped keys and short-TTL presigned PUTs.
      // Key prefix is server-built from membership locId → the client can never
      // sign a write into another tenant's prefix.
      const bucket = process.env.R2_BUCKET;
      const endpoint = process.env.R2_ENDPOINT;
      if (!bucket || !endpoint) {
        return reply.status(503).send({ error: 'Object storage not configured' });
      }

      let getSignedUrl: any, S3Client: any, PutObjectCommand: any;
      try {
        ({ getSignedUrl } = await import('@aws-sdk/s3-request-presigner'));
        ({ S3Client, PutObjectCommand } = await import('@aws-sdk/client-s3'));
      } catch (e: any) {
        request.log?.error?.({ err: e?.message }, '[product-media] presigner import failed');
        return reply.status(503).send({ error: 'Presign unavailable' });
      }

      const client = new S3Client({
        endpoint,
        region: 'auto',
        credentials: {
          accessKeyId: process.env.R2_ACCESS_KEY_ID || '',
          secretAccessKey: process.env.R2_SECRET_ACCESS_KEY || '',
        },
      });

      const uploads = [];
      for (const it of items) {
        const ext = extForMime(it.mimeType)!;
        // poster shares the spin/video prefix but is tagged so confirm can map it.
        const subKind = it.poster ? `${kind}-poster` : kind;
        const key = `${ctx.locId}/${productId}/${subKind}/${it.sha256.slice(0, 12)}.${ext}`;
        const cmd = new PutObjectCommand({ Bucket: bucket, Key: key, ContentType: it.mimeType });
        const url = await getSignedUrl(client, cmd, { expiresIn: PRESIGN_TTL_SECONDS });
        uploads.push({ key, url, sha256: it.sha256, poster: !!it.poster });
      }

      return reply.send({ uploads, expiresIn: PRESIGN_TTL_SECONDS });
    },
  );

  // ─── POST /menu/products/:productId/media/confirm ──────────────────────
  // Re-validate the actual stored bytes (magic-byte sniff) before persisting,
  // then write ONE product_media row through the operational pool (withTenant).
  fastify.post(
    '/menu/products/:productId/media/confirm',
    { preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])] },
    async (request: any, reply: any) => {
      const ctx = await getOwnerLocation(request);
      if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
      const productId = request.params.productId as string;
      if (!(await productInLocation(productId, ctx.locId))) {
        return reply.status(404).send({ error: 'Product not found' });
      }

      const body = (request.body || {}) as {
        kind?: string;
        storageKey?: string;
        mimeType?: string;
        bytes?: number;
        width?: number;
        height?: number;
        durationMs?: number;
        posterKey?: string;
        alt?: string;
        frameKeys?: string[];
      };
      const kind = body.kind as ProductMediaKind;
      if (!KINDS.includes(kind)) return reply.status(400).send({ error: 'Invalid media kind' });
      const storageKey = String(body.storageKey || '');
      const mimeType = String(body.mimeType || '');
      if (!storageKey) return reply.status(400).send({ error: 'Missing storageKey' });
      if (!isAllowedMime(mimeType)) return reply.status(400).send({ error: `Disallowed mime: ${mimeType}` });

      // The key must live under this tenant's product prefix — reject anything else.
      if (!storageKey.startsWith(`${ctx.locId}/${productId}/`)) {
        return reply.status(400).send({ error: 'Key outside tenant prefix' });
      }

      // Spin frame-count.
      const frameKeys = Array.isArray(body.frameKeys) ? body.frameKeys : [];
      if (kind === 'spin') {
        const fc = checkFrameCount(frameKeys.length);
        if (!fc.ok) return reply.status(400).send({ error: fc.reason });
      }

      // Re-validate the stored bytes by sniffing the magic number. R2 is absent on
      // staging, so this is a no-op there (storage.get → null); the sniff logic
      // itself is unit-tested. Lazily import to avoid loading the SDK when unused.
      const { sniffMime } = await import('../../lib/product-media-validation.js');
      if (storage) {
        try {
          const buf = await storage.get(storageKey);
          if (buf && sniffMime(buf) !== mimeType) {
            return reply.status(400).send({ error: 'Stored bytes do not match declared type' });
          }
        } catch (e: any) {
          request.log?.warn?.({ err: e?.message }, '[product-media] magic-byte recheck failed');
        }
      }

      const meta: Record<string, any> = {};
      if (kind === 'spin') {
        meta.frameKeys = frameKeys;
        meta.frameCount = frameKeys.length;
      }

      const id = crypto.randomUUID();
      const row = await withTenant(db, ctx.userId, async (client) => {
        // Append at the end of the sort order.
        const ord = await client.query(
          `SELECT COALESCE(MAX(sort_order), -1) + 1 AS next FROM product_media WHERE product_id = $1`,
          [productId],
        );
        const sortOrder = ord.rows[0]?.next ?? 0;
        const res = await client.query(
          `INSERT INTO product_media
             (id, location_id, product_id, kind, storage_key, mime_type, bytes, width, height,
              duration_ms, poster_key, alt, sort_order, available, meta)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,true,$14)
           RETURNING id, sort_order`,
          [
            id, ctx.locId, productId, kind, storageKey, mimeType, Number(body.bytes || 0),
            body.width ?? null, body.height ?? null, body.durationMs ?? null,
            body.posterKey ?? null, body.alt ?? null, sortOrder, JSON.stringify(meta),
          ],
        );
        return res.rows[0];
      });

      return reply.status(201).send({ id: row.id, sortOrder: row.sort_order });
    },
  );

  // ─── POST /menu/products/:productId/media/:mediaId/set-primary ─────────
  // Read-before-write: skip the no-op (already primary) so we don't bump the
  // menu version for nothing.
  fastify.post(
    '/menu/products/:productId/media/:mediaId/set-primary',
    { preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])] },
    async (request: any, reply: any) => {
      const ctx = await getOwnerLocation(request);
      if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
      const { productId, mediaId } = request.params as { productId: string; mediaId: string };

      const result = await withTenant(db, ctx.userId, async (client) => {
        const cur = await client.query(
          `SELECT primary_media_id FROM products WHERE id = $1 AND location_id = $2`,
          [productId, ctx.locId],
        );
        if (cur.rowCount === 0) return { status: 404 as const };
        // The media must belong to this product (guards cross-product set).
        const owns = await client.query(
          `SELECT 1 FROM product_media WHERE id = $1 AND product_id = $2 AND location_id = $3`,
          [mediaId, productId, ctx.locId],
        );
        if (owns.rowCount === 0) return { status: 404 as const };
        if (cur.rows[0].primary_media_id === mediaId) return { status: 200 as const, changed: false };
        await client.query(`UPDATE products SET primary_media_id = $1 WHERE id = $2`, [mediaId, productId]);
        return { status: 200 as const, changed: true };
      });

      if (result.status === 404) return reply.status(404).send({ error: 'Not found' });
      return reply.send({ ok: true, changed: result.changed });
    },
  );

  // ─── POST /menu/products/:productId/media/reorder ──────────────────────
  // Single transaction; no menu-version bump (ordering isn't a published change).
  fastify.post(
    '/menu/products/:productId/media/reorder',
    { preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])] },
    async (request: any, reply: any) => {
      const ctx = await getOwnerLocation(request);
      if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
      const productId = request.params.productId as string;
      const order = (request.body as any)?.order;
      if (!Array.isArray(order) || order.some((x) => typeof x !== 'string')) {
        return reply.status(400).send({ error: 'order must be an array of media ids' });
      }

      await withTenant(db, ctx.userId, async (client) => {
        for (let i = 0; i < order.length; i++) {
          await client.query(
            `UPDATE product_media SET sort_order = $1
              WHERE id = $2 AND product_id = $3 AND location_id = $4`,
            [i, order[i], productId, ctx.locId],
          );
        }
      });

      return reply.send({ ok: true });
    },
  );

  // ─── PATCH /menu/products/:productId/media/:mediaId — available toggle ──
  fastify.patch(
    '/menu/products/:productId/media/:mediaId',
    { preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])] },
    async (request: any, reply: any) => {
      const ctx = await getOwnerLocation(request);
      if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
      const { productId, mediaId } = request.params as { productId: string; mediaId: string };
      const available = (request.body as any)?.available;
      if (typeof available !== 'boolean') {
        return reply.status(400).send({ error: 'available must be a boolean' });
      }

      const updated = await withTenant(db, ctx.userId, async (client) => {
        const res = await client.query(
          `UPDATE product_media SET available = $1
            WHERE id = $2 AND product_id = $3 AND location_id = $4
            RETURNING id, available`,
          [available, mediaId, productId, ctx.locId],
        );
        return res.rows[0];
      });

      if (!updated) return reply.status(404).send({ error: 'Not found' });
      return reply.send({ id: updated.id, available: updated.available });
    },
  );
}
