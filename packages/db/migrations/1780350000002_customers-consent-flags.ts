import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add consent flags to customers table for phase 2.3
  pgm.addColumns('customers', {
    consented_to_terms: {
      type: 'boolean',
      notNull: true,
      default: false,
    },
    consented_to_marketing: {
      type: 'boolean',
      notNull: true,
      default: false,
    },
    updated_at: {
      type: 'timestamptz',
      notNull: true,
      default: pgm.func('now()'),
    },
  });

  // Create index for searching customers by phone
  pgm.createIndex('customers', ['location_id', 'phone']);

  // Create index for sorting by created_at
  pgm.createIndex('customers', ['location_id', 'created_at'], { reverse: true });
}

export async function down(): Promise<void> {}
