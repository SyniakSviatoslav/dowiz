import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add external_key columns
  pgm.addColumn('categories', { external_key: { type: 'text' } });
  pgm.addColumn('products', { external_key: { type: 'text' } });
  pgm.addColumn('modifier_groups', { external_key: { type: 'text' } });
  pgm.addColumn('modifiers', { external_key: { type: 'text' } });

  // Add unique indexes ensuring external_key is unique per location (if provided)
  pgm.createIndex('categories', ['location_id', 'external_key'], { unique: true, where: 'external_key IS NOT NULL', name: 'categories_loc_extkey_uniq' });
  pgm.createIndex('products', ['location_id', 'external_key'], { unique: true, where: 'external_key IS NOT NULL', name: 'products_loc_extkey_uniq' });
  pgm.createIndex('modifier_groups', ['location_id', 'external_key'], { unique: true, where: 'external_key IS NOT NULL', name: 'modifier_groups_loc_extkey_uniq' });
  // For modifiers, it's unique per group, not per location! Because modifier keys are unique within a group context usually.
  // The prompt says: "modifiers: (group_id, external_key)"
  pgm.createIndex('modifiers', ['group_id', 'external_key'], { unique: true, where: 'external_key IS NOT NULL', name: 'modifiers_group_extkey_uniq' });
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.dropIndex('modifiers', ['group_id', 'external_key'], { name: 'modifiers_group_extkey_uniq' });
  pgm.dropIndex('modifier_groups', ['location_id', 'external_key'], { name: 'modifier_groups_loc_extkey_uniq' });
  pgm.dropIndex('products', ['location_id', 'external_key'], { name: 'products_loc_extkey_uniq' });
  pgm.dropIndex('categories', ['location_id', 'external_key'], { name: 'categories_loc_extkey_uniq' });

  pgm.dropColumn('modifiers', 'external_key');
  pgm.dropColumn('modifier_groups', 'external_key');
  pgm.dropColumn('products', 'external_key');
  pgm.dropColumn('categories', 'external_key');
}
