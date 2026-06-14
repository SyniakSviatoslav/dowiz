const { readFileSync, existsSync } = require('fs');
const { Pool } = require('pg');

const pool = new Pool({
  connectionString: process.env.***REDACTED*** || process.env.DATABASE_URL,
});

async function run() {
  const client = await pool.connect();
  try {
    console.log('Connected, running migration...');
    
    // Create migration tracking table if not exists
    await client.query(`
      CREATE TABLE IF NOT EXISTS migrations (
        id SERIAL PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
      )
    `);
    
    const migName = '1790000000017_promotions_full';
    
    // Check if already applied
    const existing = await client.query('SELECT id FROM migrations WHERE name = $1', [migName]);
    if (existing.rows.length > 0) {
      console.log('Migration already applied, skipping');
      return;
    }
    
    // Drop old placeholder
    await client.query(`DROP TABLE IF EXISTS promotions CASCADE`);
    
    // Create trigger function
    await client.query(`
      CREATE OR REPLACE FUNCTION update_updated_at_column()
      RETURNS TRIGGER AS $$
      BEGIN
        NEW.updated_at = now();
        RETURN NEW;
      END;
      $$ LANGUAGE plpgsql
    `);
    
    // Create promotions table
    await client.query(`
      CREATE TABLE promotions (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
        code text NOT NULL,
        type text NOT NULL CHECK (type IN ('percentage', 'fixed', 'free_delivery')),
        discount_value integer NOT NULL CHECK (discount_value > 0),
        min_order_amount integer DEFAULT 0 CHECK (min_order_amount >= 0),
        max_uses integer DEFAULT NULL,
        current_uses integer NOT NULL DEFAULT 0 CHECK (current_uses >= 0),
        max_uses_per_customer integer DEFAULT 1,
        valid_from timestamptz NOT NULL DEFAULT now(),
        valid_until timestamptz,
        is_active boolean NOT NULL DEFAULT true,
        applicable_product_ids uuid[] DEFAULT '{}',
        description text,
        created_at timestamptz NOT NULL DEFAULT now(),
        updated_at timestamptz NOT NULL DEFAULT now()
      )
    `);
    
    // Indexes
    await client.query(`CREATE UNIQUE INDEX idx_promotions_location_code ON promotions(location_id, code)`);
    await client.query(`CREATE INDEX idx_promotions_location_id ON promotions(location_id)`);
    
    // RLS
    await client.query(`ALTER TABLE promotions ENABLE ROW LEVEL SECURITY`);
    await client.query(`ALTER TABLE promotions FORCE ROW LEVEL SECURITY`);
    
    // Tenant isolation policy
    await client.query(`
      CREATE POLICY tenant_isolation ON promotions
        USING (location_id = (SELECT id FROM locations WHERE id = location_id AND location_id = current_setting('app.location_id')::uuid))
    `);
    
    // Trigger
    await client.query(`
      CREATE TRIGGER update_promotions_updated_at
        BEFORE UPDATE ON promotions
        FOR EACH ROW
        EXECUTE FUNCTION update_updated_at_column()
    `);
    
    // Record migration
    await client.query('INSERT INTO migrations (name) VALUES ($1)', [migName]);
    
    console.log('Migration applied successfully!');
  } catch (err) {
    console.error('Migration failed:', err);
    process.exit(1);
  } finally {
    client.release();
    await pool.end();
  }
}

run();
