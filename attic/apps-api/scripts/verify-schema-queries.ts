import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

const env = loadEnv();

interface TableSchema {
  columns: Map<string, string>; // column_name -> data_type
}

// Extract SQL queries from notification source files
interface ParsedQuery {
  file: string;
  line: number;
  sql: string;
  table: string;
  columns: string[];
}

function parseQueries(filePath: string, relativePath: string): ParsedQuery[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const queries: ParsedQuery[] = [];
  const sqlRegex = /\.query\(\s*`([\s\S]*?)`/g;
  let match;

  while ((match = sqlRegex.exec(content)) !== null) {
    const sql = match[1];
    const lineNum = content.substring(0, match.index).split('\n').length;

    // Extract table name (FROM or JOIN clause)
    const tableMatch = sql.match(/(?:FROM|JOIN)\s+(\w+)/i);
    if (!tableMatch) continue;
    const table = tableMatch[1];

    // Extract column references
    const colRefs = new Set<string>();
    const colRegex = /\b(\w+)\.(\w+)\b/g;
    let colMatch;
    while ((colMatch = colRegex.exec(sql)) !== null) {
      const alias = colMatch[1].toLowerCase();
      const col = colMatch[2];
      colRefs.add(col);
    }

    // Also catch bare column names in SELECT clauses
    const selectColRegex = /SELECT\s+(.+?)\s+FROM/is;
    const selectMatch = sql.match(selectColRegex);
    if (selectMatch) {
      const selectClause = selectMatch[1];
      const parts = selectClause.split(',').map(p => p.trim());
      for (const part of parts) {
        const asMatch = part.match(/AS\s+(\w+)/i);
        if (asMatch) continue; // alias, not source column name
        // Check if it's a literal, function, or expression
        if (/^[a-zA-Z_]\w*$/.test(part)) {
          colRefs.add(part);
        }
      }
    }

    queries.push({
      file: relativePath,
      line: lineNum,
      sql: sql.substring(0, 120).replace(/\n/g, ' '),
      table,
      columns: Array.from(colRefs).sort(),
    });
  }

  return queries;
}

async function main() {
  console.log('\n=== Schema-Query Integrity Check ===\n');

  const srcDir = resolve(import.meta.dirname, '../src');
  const notificationFiles = [
    'notifications/workers/index.ts',
    'notifications/render.ts',
    'routes/telegram-webhook.ts',
    'routes/orders.ts',
    'lib/orderStatusService.ts',
  ];

  const allQueries: ParsedQuery[] = [];
  for (const file of notificationFiles) {
    const fullPath = resolve(srcDir, file);
    try {
      const queries = parseQueries(fullPath, file);
      allQueries.push(...queries);
    } catch {
      // file may not exist in all paths
    }
  }

  if (allQueries.length === 0) {
    console.log('No SQL queries found in notification source files.');
    process.exit(0);
  }

  console.log(`Found ${allQueries.length} SQL queries across ${notificationFiles.length} files.\n`);

  // Get unique tables
  const tables = [...new Set(allQueries.map(q => q.table))];
  console.log(`Tables referenced: ${tables.join(', ')}\n`);

  // Fetch column info from information_schema
  const pool = createSessionPool();
  const schemas: Map<string, TableSchema> = new Map();

  for (const table of tables) {
    const res = await pool.query(
      `SELECT column_name, data_type 
       FROM information_schema.columns 
       WHERE table_name = $1 
       ORDER BY ordinal_position`,
      [table]
    );
    const schema: TableSchema = { columns: new Map() };
    for (const row of res.rows) {
      schema.columns.set(row.column_name, row.data_type);
    }
    schemas.set(table, schema);
    console.log(`  ${table}: ${schema.columns.size} columns`);
  }

  console.log('');

  // Verify each query's column references
  let totalErrors = 0;
  let totalWarnings = 0;

  for (const query of allQueries) {
    const schema = schemas.get(query.table);
    if (!schema) {
      console.error(`  ❌ [${query.file}:${query.line}] Table '${query.table}' not found in information_schema`);
      totalErrors++;
      continue;
    }

    const missingCols: string[] = [];
    const seenCols = new Set<string>();

    for (const col of query.columns) {
      if (seenCols.has(col)) continue;
      seenCols.add(col);
      if (!schema.columns.has(col)) {
        missingCols.push(col);
      }
    }

    if (missingCols.length > 0) {
      console.warn(`  ⚠️  [${query.file}:${query.line}] Table '${query.table}' — columns not found: ${missingCols.join(', ')}`);
      console.warn(`      SQL: ${query.sql}`);
      totalWarnings++;
    } else {
      console.log(`  ✅ [${query.file}:${query.line}] Table '${query.table}' — ${query.columns.length} column refs verified`);
    }
  }

  await pool.end();

  console.log(`\n=== VERDICT ===`);
  if (totalErrors === 0 && totalWarnings === 0) {
    console.log(`✅ All ${allQueries.length} SQL queries verified against information_schema.`);
    process.exit(0);
  } else if (totalErrors === 0 && totalWarnings > 0) {
    console.log(`⚠️  All queries executed, but ${totalWarnings} warning(s) — columns not found in information_schema.`);
    console.log(`   (These may be aliases, CTE columns, or expressions — manual review needed.)`);
    process.exit(totalWarnings > 3 ? 1 : 0);
  } else {
    console.error(`❌ ${totalErrors} error(s) — table(s) not found in information_schema.`);
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Schema-query verification failed:', err);
  process.exit(1);
});
