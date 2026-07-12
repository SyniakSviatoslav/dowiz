import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Users
  pgm.sql(`
    CREATE TABLE users (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      email citext UNIQUE,
      google_sub text UNIQUE,
      display_name text,
      phone text,
      totp_secret_enc bytea,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 2. Organizations
  pgm.sql(`
    CREATE TABLE organizations (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      name text NOT NULL,
      owner_id uuid REFERENCES users(id),
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 3. Locations
  pgm.sql(`
    CREATE TABLE locations (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      org_id uuid NOT NULL REFERENCES organizations(id),
      slug text NOT NULL UNIQUE,
      name text NOT NULL,
      phone text NOT NULL,
      status text NOT NULL DEFAULT 'closed',
      busy_mode boolean NOT NULL DEFAULT false,
      confirm_timeout_min int NOT NULL DEFAULT 10,
      delivery_radius_km numeric,
      lat double precision,
      lng double precision,
      closed_message text,
      menu_version int NOT NULL DEFAULT 1,
      custom_domain varchar(253),
      domain_verified_at timestamptz,
      widget_enabled boolean NOT NULL DEFAULT false,
      customer_otp_required boolean NOT NULL DEFAULT false,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 4. Memberships
  pgm.sql(`
    CREATE TABLE memberships (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      role membership_role NOT NULL,
      status membership_status NOT NULL DEFAULT 'active',
      created_at timestamptz NOT NULL DEFAULT now(),
      UNIQUE (user_id, location_id, role)
    );
    CREATE INDEX memberships_user_id_active_idx ON memberships (user_id) WHERE status = 'active';
    CREATE INDEX memberships_location_role_active_idx ON memberships (location_id, role) WHERE status = 'active';
  `);

  // 5. RLS Helpers
  pgm.sql(`
    CREATE FUNCTION app_current_user() RETURNS uuid
      LANGUAGE sql STABLE AS
      $$ SELECT NULLIF(current_setting('app.user_id', true), '')::uuid $$;
  `);

  pgm.sql(`
    CREATE FUNCTION app_member_location_ids() RETURNS SETOF uuid
      LANGUAGE sql STABLE SECURITY DEFINER AS
      $$ SELECT location_id FROM memberships
         WHERE user_id = app_current_user() AND status = 'active' $$;
  `);

  // 6. RLS Policies
  pgm.sql(`
    ALTER TABLE locations ENABLE ROW LEVEL SECURITY;
    ALTER TABLE locations FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON locations
      USING ( id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    ALTER TABLE memberships ENABLE ROW LEVEL SECURITY;
    ALTER TABLE memberships FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON memberships
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
    ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON organizations
      USING ( id IN (SELECT org_id FROM locations WHERE id IN (SELECT app_member_location_ids())) );
  `);
}

export async function down(): Promise<void> {}
