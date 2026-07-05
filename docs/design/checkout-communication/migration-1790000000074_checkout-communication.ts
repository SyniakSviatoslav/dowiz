// READY-TO-APPLY migration (operator places into packages/db/migrations/ — that path is a hard-blocked zone
// for the agent). Apply to staging FIRST (flyctl proxy 5433:5432 -a dowiz-staging-db → node-pg-migrate up)
// BEFORE the code deploy (Breaker M2: a live `signal` kind must not outrun its CHECK). Prod via release_command.
//
// Council-approved (docs/design/checkout-communication/resolution.md): expand messenger_kind to the v1 set
// (Phone first-class + Signal + SimpleX; Google Meet / MS Teams CUT), add receiver_* columns (with the
// Phase-5 anonymizer covering them in the SAME release — GDPR R6). Constraint names verified on staging via
// pg_constraint (M1): customers_messenger_kind_check · couriers_messenger_kind_check ·
// orders_customer_messenger_kind_check. ⚠️ operator: re-verify the same names on PROD (Supabase) before apply.
import type { MigrationBuilder } from 'node-pg-migrate';

const KINDS = "('phone','telegram','whatsapp','viber','signal','simplex')";

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE customers
      DROP CONSTRAINT IF EXISTS customers_messenger_kind_check,
      ADD  CONSTRAINT customers_messenger_kind_check
           CHECK (messenger_kind IS NULL OR messenger_kind IN ${KINDS});
    ALTER TABLE couriers
      DROP CONSTRAINT IF EXISTS couriers_messenger_kind_check,
      ADD  CONSTRAINT couriers_messenger_kind_check
           CHECK (messenger_kind IS NULL OR messenger_kind IN ${KINDS});
    ALTER TABLE orders
      DROP CONSTRAINT IF EXISTS orders_customer_messenger_kind_check,
      ADD  CONSTRAINT orders_customer_messenger_kind_check
           CHECK (customer_messenger_kind IS NULL OR customer_messenger_kind IN ${KINDS});

    -- "Deliver to someone else" — receiver gets their OWN communication channel. Nullable (most orders ship to
    -- the customer). 🔴 GDPR: the Phase-5 anonymizer MUST null these in the same release (non-consenting 3rd
    -- party), and a receiver-DSAR path keys on receiver_handle/receiver_phone.
    ALTER TABLE orders
      ADD COLUMN IF NOT EXISTS receiver_name           text,
      ADD COLUMN IF NOT EXISTS receiver_messenger_kind text
           CHECK (receiver_messenger_kind IS NULL OR receiver_messenger_kind IN ${KINDS}),
      ADD COLUMN IF NOT EXISTS receiver_handle         text;

    -- orders already ENABLE+FORCE RLS (tenant_isolation); new columns inherit. Re-assert belt-and-suspenders.
    ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
    ALTER TABLE orders FORCE  ROW LEVEL SECURITY;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
