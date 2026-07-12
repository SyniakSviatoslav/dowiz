import type { MigrationBuilder } from 'node-pg-migrate';

// UX-2 messenger deep-link. Optional secondary contact channel (Telegram/
// WhatsApp/Viber) for client<->courier, parallel to phone. Customers + couriers
// carry their own handle; orders snapshot the customer's at checkout. Nullable;
// inherit each table's tenant_isolation RLS. Handle is a shareable contact, kept
// plaintext like customers.phone (Phase-5 anonymizer covers it later).
const KINDS = "('telegram','whatsapp','viber')";

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE customers
      ADD COLUMN IF NOT EXISTS messenger_kind   text CHECK (messenger_kind IN ${KINDS}),
      ADD COLUMN IF NOT EXISTS messenger_handle text;
    ALTER TABLE couriers
      ADD COLUMN IF NOT EXISTS messenger_kind   text CHECK (messenger_kind IN ${KINDS}),
      ADD COLUMN IF NOT EXISTS messenger_handle text;
    ALTER TABLE orders
      ADD COLUMN IF NOT EXISTS customer_messenger_kind   text CHECK (customer_messenger_kind IN ${KINDS}),
      ADD COLUMN IF NOT EXISTS customer_messenger_handle text;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
