import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Allow anonymous INSERT into orders (for POST /orders — customer has no session)
  pgm.sql(`
    CREATE POLICY anonymous_insert ON orders FOR INSERT
      WITH CHECK (app_current_user() IS NULL);
  `);

  pgm.sql(`
    CREATE POLICY anonymous_insert ON order_items FOR INSERT
      WITH CHECK (EXISTS (SELECT 1 FROM orders o WHERE o.id = order_items.order_id));
  `);

  pgm.sql(`
    CREATE POLICY anonymous_insert ON customers FOR INSERT
      WITH CHECK (app_current_user() IS NULL);
  `);

  pgm.sql(`
    CREATE POLICY anonymous_insert ON idempotency_keys FOR INSERT
      WITH CHECK (app_current_user() IS NULL);
  `);

  // Allow anyone to SELECT idempotency_keys by key (idempotency check)
  pgm.sql(`
    CREATE POLICY anonymous_select ON idempotency_keys FOR SELECT
      USING (key IS NOT NULL);
  `);
}

export async function down(): Promise<void> {}
