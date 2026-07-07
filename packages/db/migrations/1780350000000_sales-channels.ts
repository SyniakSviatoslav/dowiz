  import type { MigrationBuilder } from 'node-pg-migrate';

  export async function up(pgm: MigrationBuilder): Promise<void> {
    pgm.sql(`
      CREATE TABLE sales_channels (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        location_id uuid NOT NULL REFERENCES locations(id),
        kind text NOT NULL CHECK (kind IN (
          'web-direct', 'qr', 'nfc', 'gbp', 'apple-maps',
          'instagram', 'facebook', 'whatsapp', 'telegram-tma',
          'kiosk', 'widget', 'agent', 'other'
        )),
        name text NOT NULL,
        token text NOT NULL UNIQUE,
        active boolean NOT NULL DEFAULT true,
        created_at timestamptz NOT NULL DEFAULT now()
      );
      CREATE INDEX sales_channels_location_idx ON sales_channels(location_id);
      CREATE INDEX sales_channels_kind_idx ON sales_channels(kind);
    `);

    pgm.sql(`
      ALTER TABLE sales_channels ENABLE ROW LEVEL SECURITY;
      ALTER TABLE sales_channels FORCE ROW LEVEL SECURITY;
      CREATE POLICY tenant_isolation ON sales_channels
        USING ( location_id IN (SELECT app_member_location_ids()) );
    `);
  }

  export async function down(): Promise<void> {}
