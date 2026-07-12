import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  // Extensions
  pgm.sql('CREATE EXTENSION IF NOT EXISTS citext;');
  pgm.sql('CREATE EXTENSION IF NOT EXISTS pgcrypto;');

  // Enums
  pgm.sql(`CREATE TYPE membership_role AS ENUM ('owner', 'courier', 'admin');`);
  pgm.sql(`CREATE TYPE membership_status AS ENUM ('active', 'suspended', 'removed');`);
  pgm.sql(`CREATE TYPE order_type AS ENUM ('delivery', 'pickup', 'scheduled');`);
  pgm.sql(`CREATE TYPE order_status AS ENUM ('PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'REJECTED', 'CANCELLED', 'SCHEDULED', 'PICKED_UP');`);
  pgm.sql(`CREATE TYPE payment_method AS ENUM ('cash');`);
  pgm.sql(`CREATE TYPE payment_outcome AS ENUM ('pending', 'paid_full', 'paid_partial', 'refused_payment', 'refused_goods', 'customer_cancelled_on_door');`);
}

export async function down(): Promise<void> {
  // Never executed according to discipline, but keeping forward-only logic.
}
