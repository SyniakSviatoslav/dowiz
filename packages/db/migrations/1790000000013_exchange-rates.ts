import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE exchange_rates (
      id              BIGSERIAL,
      base_currency   TEXT NOT NULL,
      target_currency TEXT NOT NULL,
      rate            DECIMAL(18,8) NOT NULL,
      source          TEXT,
      fetched_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
      PRIMARY KEY (base_currency, target_currency)
    );
  `);

  pgm.sql(`CREATE INDEX idx_exchange_rates_fetched ON exchange_rates (fetched_at DESC);`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS exchange_rates;`);
}
