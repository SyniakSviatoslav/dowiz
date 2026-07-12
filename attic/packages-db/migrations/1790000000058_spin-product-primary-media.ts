import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * Phase-2 demo-fixture fix. Seeds 056/057 split media across statements: the per-statement
 * `NOT EXISTS (product_media …)` guard meant the `spin` row landed on a DIFFERENT product than
 * the image galleries, leaving the spin product with primary_media_id = NULL. The storefront
 * only lazy-fetches media when primary_media_id is set, so that spin never opened in the modal.
 *
 * Fix: point any spin-only product's primary_media_id at its spin row, so it renders via the
 * SpinViewer. Idempotent (only where still NULL), forward-only, scoped to products that
 * actually have a spin. No effect on products that already have an image primary.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE products p
    SET primary_media_id = s.id
    FROM (
      SELECT DISTINCT ON (pm.product_id) pm.product_id, pm.id
      FROM product_media pm
      WHERE pm.kind = 'spin'
      ORDER BY pm.product_id, pm.sort_order, pm.created_at
    ) AS s
    WHERE p.id = s.product_id
      AND p.primary_media_id IS NULL;
  `);
}

export async function down(): Promise<void> {
  // Forward-only demo fixture; nothing to reverse.
}
