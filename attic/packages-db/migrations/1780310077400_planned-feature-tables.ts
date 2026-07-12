import { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // webhook_endpoints
  pgm.sql(`
    CREATE TABLE webhook_endpoints (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE webhook_endpoints ENABLE ROW LEVEL SECURITY;
    ALTER TABLE webhook_endpoints FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON webhook_endpoints
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // api_keys
  pgm.sql(`
    CREATE TABLE api_keys (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;
    ALTER TABLE api_keys FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON api_keys
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // domain_verifications
  pgm.sql(`
    CREATE TABLE domain_verifications (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE domain_verifications ENABLE ROW LEVEL SECURITY;
    ALTER TABLE domain_verifications FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON domain_verifications
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // recurring_orders
  pgm.sql(`
    CREATE TABLE recurring_orders (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE recurring_orders ENABLE ROW LEVEL SECURITY;
    ALTER TABLE recurring_orders FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON recurring_orders
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // promotions
  pgm.sql(`
    CREATE TABLE promotions (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE promotions ENABLE ROW LEVEL SECURITY;
    ALTER TABLE promotions FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON promotions
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // location_alerts
  pgm.sql(`
    CREATE TABLE location_alerts (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      placeholder_data text
    );
  `);
  pgm.sql(`
    ALTER TABLE location_alerts ENABLE ROW LEVEL SECURITY;
    ALTER TABLE location_alerts FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON location_alerts
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
