import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.createTable('import_sessions', {
    id: { type: 'uuid', primaryKey: true, default: pgm.func('gen_random_uuid()') },
    location_id: { type: 'uuid', notNull: true, references: 'locations(id)', onDelete: 'CASCADE' },
    owner_id: { type: 'uuid', notNull: true, references: 'users(id)', onDelete: 'CASCADE' },
    status: { type: 'text', notNull: true, check: "status IN ('previewed', 'committed', 'expired', 'failed')" },
    mode: { type: 'text', notNull: true, check: "mode IN ('replace', 'add_only', 'merge')" },
    draft_json: { type: 'jsonb', notNull: true },
    issues_json: { type: 'jsonb', notNull: true, default: '[]' },
    summary_json: { type: 'jsonb', notNull: true },
    commit_token: { type: 'uuid', notNull: false },
    idempotency_key: { type: 'uuid', notNull: false },
    created_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    expires_at: { type: 'timestamptz', notNull: true },
    committed_at: { type: 'timestamptz', notNull: false },
  });

  pgm.createIndex('import_sessions', ['commit_token'], { unique: true, where: 'commit_token IS NOT NULL', name: 'import_sessions_commit_token_uniq' });
  pgm.createIndex('import_sessions', ['location_id', 'idempotency_key'], { unique: true, where: 'idempotency_key IS NOT NULL', name: 'import_sessions_idem_key_uniq' });

  // RLS
  pgm.alterTable('import_sessions', { levelSecurity: 'ENABLE' });
  
  // Policies: Only owners can read/write their location's import sessions
  pgm.createPolicy('import_sessions', 'import_sessions_owner_select', {
    command: 'SELECT',
    using: "EXISTS (SELECT 1 FROM memberships ol WHERE ol.user_id = current_setting('app.user_id')::uuid AND ol.location_id = import_sessions.location_id)",
  });
  pgm.createPolicy('import_sessions', 'import_sessions_owner_insert', {
    command: 'INSERT',
    withCheck: "EXISTS (SELECT 1 FROM memberships ol WHERE ol.user_id = current_setting('app.user_id')::uuid AND ol.location_id = import_sessions.location_id)",
  });
  pgm.createPolicy('import_sessions', 'import_sessions_owner_update', {
    command: 'UPDATE',
    using: "EXISTS (SELECT 1 FROM memberships ol WHERE ol.user_id = current_setting('app.user_id')::uuid AND ol.location_id = import_sessions.location_id)",
  });
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.dropPolicy('import_sessions', 'import_sessions_owner_update');
  pgm.dropPolicy('import_sessions', 'import_sessions_owner_insert');
  pgm.dropPolicy('import_sessions', 'import_sessions_owner_select');
  pgm.dropTable('import_sessions');
}
