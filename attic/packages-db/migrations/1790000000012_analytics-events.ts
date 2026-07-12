import type { MigrationBuilder } from 'node-pg-migrate';

// WHITELIST: analytics tables are intentionally non-tenant-scoped.
// These tables store anonymous telemetry (no PII), analytics events,
// abuse logs, and CWV metrics. They are queried per-location but
// aggregated across tenants for abuse detection and platform health.
// location_id is stored for per-location breakdowns but RLS is NOT
// enforced — the telemetry endpoint inserts without tenant context.
// Do NOT add FORCE ROW LEVEL SECURITY to these tables without
// changing the telemetry insert path.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE analytics_events (
      id            BIGSERIAL,
      ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
      event         TEXT NOT NULL,
      location_id   UUID,
      surface       TEXT,
      lang          TEXT,
      anon_id       TEXT,
      session_id    TEXT,
      version       TEXT,
      is_bot        BOOLEAN NOT NULL DEFAULT false,
      ua_class      TEXT,
      props         JSONB DEFAULT '{}'::jsonb,
      ip_hash       TEXT
    );
  `);

  pgm.sql(`CREATE INDEX idx_analytics_events_ts ON analytics_events (ts DESC);`);
  pgm.sql(`CREATE INDEX idx_analytics_events_event ON analytics_events (event, ts DESC);`);
  pgm.sql(`CREATE INDEX idx_analytics_events_location ON analytics_events (location_id, ts DESC);`);
  pgm.sql(`CREATE INDEX idx_analytics_events_session ON analytics_events (session_id, ts DESC);`);

  pgm.sql(`
    CREATE TABLE analytics_abuse_log (
      id            BIGSERIAL,
      ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
      event         TEXT NOT NULL,
      location_id   UUID,
      kind          TEXT NOT NULL,
      severity      TEXT NOT NULL,
      reason        TEXT,
      ip_hash       TEXT,
      anon_id       TEXT,
      metadata      JSONB DEFAULT '{}'::jsonb
    );
  `);

  pgm.sql(`CREATE INDEX idx_analytics_abuse_log_ts ON analytics_abuse_log (ts DESC);`);
  pgm.sql(`CREATE INDEX idx_analytics_abuse_log_kind ON analytics_abuse_log (kind, ts DESC);`);

  pgm.sql(`
    CREATE TABLE analytics_cwv (
      id            BIGSERIAL,
      ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
      location_id   UUID,
      surface       TEXT,
      anon_id       TEXT,
      metric        TEXT NOT NULL,
      value         DOUBLE PRECISION NOT NULL,
      rating        TEXT,
      props         JSONB DEFAULT '{}'::jsonb
    );
  `);

  pgm.sql(`CREATE INDEX idx_analytics_cwv_ts ON analytics_cwv (ts DESC);`);
  pgm.sql(`CREATE INDEX idx_analytics_cwv_metric ON analytics_cwv (metric, ts DESC);`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS analytics_cwv;`);
  pgm.sql(`DROP TABLE IF EXISTS analytics_abuse_log;`);
  pgm.sql(`DROP TABLE IF EXISTS analytics_events;`);
}
