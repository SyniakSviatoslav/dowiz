import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Customers
  pgm.sql(`
    CREATE TABLE customers (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      phone text NOT NULL,
      name text,
      no_show_count int NOT NULL DEFAULT 0,
      created_at timestamptz NOT NULL DEFAULT now(),
      UNIQUE (location_id, phone)
    );
  `);

  // 2. Orders
  pgm.sql(`
    CREATE TABLE orders (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      customer_id uuid REFERENCES customers(id),
      courier_id uuid,
      type order_type NOT NULL DEFAULT 'delivery',
      status order_status NOT NULL DEFAULT 'PENDING',
      rejection_reason text,
      delivery_address text,
      delivery_lat double precision,
      delivery_lng double precision,
      subtotal integer NOT NULL CHECK (subtotal >= 0),
      total integer NOT NULL CHECK (total >= 0),
      payment_method payment_method NOT NULL DEFAULT 'cash',
      payment_outcome payment_outcome NOT NULL DEFAULT 'pending',
      cash_pay_with integer,
      preferences jsonb NOT NULL DEFAULT '{}',
      timeout_at timestamptz,
      scheduled_at timestamptz,
      pickup_code text,
      created_at timestamptz NOT NULL DEFAULT now(),
      confirmed_at timestamptz
    );
    CREATE INDEX orders_location_status_idx ON orders(location_id, status);
    CREATE INDEX orders_location_created_idx ON orders(location_id, created_at DESC);
  `);

  // 3. Order Items
  pgm.sql(`
    CREATE TABLE order_items (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      product_id uuid REFERENCES products(id),
      name_snapshot text NOT NULL,
      price_snapshot integer NOT NULL CHECK (price_snapshot >= 0),
      quantity integer NOT NULL CHECK (quantity > 0)
    );
  `);

  // 4. Idempotency Keys
  pgm.sql(`
    CREATE TABLE idempotency_keys (
      key text PRIMARY KEY,
      location_id uuid NOT NULL,
      request_hash text NOT NULL,
      order_id uuid REFERENCES orders(id),
      response_code int,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 5. RLS
  pgm.sql(`
    ALTER TABLE customers ENABLE ROW LEVEL SECURITY;
    ALTER TABLE customers FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON customers
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
    ALTER TABLE orders FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON orders
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    ALTER TABLE order_items ENABLE ROW LEVEL SECURITY;
    ALTER TABLE order_items FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON order_items
      USING ( EXISTS (SELECT 1 FROM orders o WHERE o.id = order_items.order_id AND o.location_id IN (SELECT app_member_location_ids())) );
  `);

  pgm.sql(`
    ALTER TABLE idempotency_keys ENABLE ROW LEVEL SECURITY;
    ALTER TABLE idempotency_keys FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON idempotency_keys
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
