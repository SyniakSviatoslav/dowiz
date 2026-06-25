import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * MENU-AVAILABILITY · `modifier_groups.display_type` — explicit render hint (additive).
 *
 * Today the client INFERS the control from `max_select === 1` (radio) vs `> 1`
 * (checkbox) — a lossy heuristic that cannot express a single-line "quantity"
 * stepper or a compact "select" dropdown. This adds a NULLABLE enum so an owner
 * can pin the control explicitly; NULL preserves the legacy inference verbatim
 * (purely additive — every existing group keeps its current behaviour).
 *
 * Forward-only: nullable column, no default, no row rewrite, no menu_version
 * bump (a render hint is not a priced change). down() drops the column.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  // CHECK (not a pg enum) — the control set can evolve without an ALTER TYPE
  // migration, mirroring the locations.plan pattern in 1790000000054.
  pgm.sql(`
    ALTER TABLE modifier_groups
      ADD COLUMN IF NOT EXISTS display_type text
        CHECK (display_type IS NULL OR display_type IN ('radio', 'checkbox', 'select', 'quantity'));
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE modifier_groups DROP COLUMN IF EXISTS display_type;`);
}
