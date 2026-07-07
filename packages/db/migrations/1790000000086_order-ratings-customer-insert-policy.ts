import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add customer_insert RLS policy to order_ratings
  // Allows authenticated customers to INSERT ratings for their own orders
  // Required for POST /api/orders/:orderId/rating when NOBYPASSRLS is enforced
  pgm.sql(`
    DROP POLICY IF EXISTS customer_insert ON order_ratings;
    CREATE POLICY customer_insert ON order_ratings FOR INSERT
      WITH CHECK (order_id IN (SELECT id FROM orders WHERE customer_id = app_current_user()));
  `);
}

export async function down(): Promise<void> {
  // Never executed according to discipline, but keeping forward-only logic.
}
