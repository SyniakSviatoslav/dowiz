import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Remove duplicate categories: keep the one with the most products, delete others
  pgm.sql(`
    DELETE FROM categories c1 USING (
      SELECT location_id, name, MAX(id::text)::uuid AS keep_id
      FROM (
        SELECT DISTINCT ON (c.location_id, c.name) c.id, c.location_id, c.name,
          (SELECT count(*) FROM products p WHERE p.category_id = c.id) AS cnt
        FROM categories c
        ORDER BY c.location_id, c.name, cnt DESC, c.id
      ) sub
      GROUP BY location_id, name
      HAVING count(*) > 1
    ) dup
    WHERE c1.location_id = dup.location_id
      AND c1.name = dup.name
      AND c1.id != dup.keep_id;
  `);

  // Add unique constraint
  pgm.sql(`
    CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_loc_name
    ON categories (location_id, name);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP INDEX IF EXISTS idx_categories_loc_name;`);
}
