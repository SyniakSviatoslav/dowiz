import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add is_auto flag and audit fields to existing translation tables
  pgm.addColumn('product_translations', {
    is_auto: { type: 'boolean', notNull: true, default: false },
    last_edited_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    last_edited_by: { type: 'uuid', notNull: false, references: 'users(id)', onDelete: 'SET NULL' }
  });

  pgm.addColumn('category_translations', {
    is_auto: { type: 'boolean', notNull: true, default: false },
    last_edited_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    last_edited_by: { type: 'uuid', notNull: false, references: 'users(id)', onDelete: 'SET NULL' }
  });

  // Create modifier_translations
  pgm.createTable('modifier_translations', {
    modifier_id: { type: 'uuid', notNull: true, references: 'modifiers(id)', onDelete: 'CASCADE' },
    locale: { type: 'text', notNull: true },
    name: { type: 'text', notNull: true },
    is_auto: { type: 'boolean', notNull: true, default: false },
    last_edited_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    last_edited_by: { type: 'uuid', notNull: false, references: 'users(id)', onDelete: 'SET NULL' }
  });
  pgm.addConstraint('modifier_translations', 'modifier_translations_pkey', {
    primaryKey: ['modifier_id', 'locale']
  });

  // Create modifier_group_translations
  pgm.createTable('modifier_group_translations', {
    group_id: { type: 'uuid', notNull: true, references: 'modifier_groups(id)', onDelete: 'CASCADE' },
    locale: { type: 'text', notNull: true },
    name: { type: 'text', notNull: true },
    is_auto: { type: 'boolean', notNull: true, default: false },
    last_edited_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    last_edited_by: { type: 'uuid', notNull: false, references: 'users(id)', onDelete: 'SET NULL' }
  });
  pgm.addConstraint('modifier_group_translations', 'modifier_group_translations_pkey', {
    primaryKey: ['group_id', 'locale']
  });

  // RLS for modifier_translations
  pgm.alterTable('modifier_translations', { levelSecurity: 'ENABLE' });
  pgm.sql(`ALTER TABLE modifier_translations FORCE ROW LEVEL SECURITY;`);
  pgm.createPolicy('modifier_translations', 'tenant_isolation', {
    command: 'ALL',
    using: `modifier_id IN (SELECT id FROM modifiers WHERE location_id IN (SELECT app_member_location_ids()))`
  });

  // RLS for modifier_group_translations
  pgm.alterTable('modifier_group_translations', { levelSecurity: 'ENABLE' });
  pgm.sql(`ALTER TABLE modifier_group_translations FORCE ROW LEVEL SECURITY;`);
  pgm.createPolicy('modifier_group_translations', 'tenant_isolation', {
    command: 'ALL',
    using: `group_id IN (SELECT id FROM modifier_groups WHERE location_id IN (SELECT app_member_location_ids()))`
  });
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.dropPolicy('modifier_group_translations', 'tenant_isolation');
  pgm.dropTable('modifier_group_translations');

  pgm.dropPolicy('modifier_translations', 'tenant_isolation');
  pgm.dropTable('modifier_translations');

  pgm.dropColumn('category_translations', ['is_auto', 'last_edited_at', 'last_edited_by']);
  pgm.dropColumn('product_translations', ['is_auto', 'last_edited_at', 'last_edited_by']);
}
