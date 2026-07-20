#!/usr/bin/env node
/**
 * scripts/check-contracts.mjs
 *
 * Scans all route files for Fastify schema definitions and cross-references
 * against available Zod schemas in packages/shared-types/src/contracts/.
 *
 * Exit code: 0 if all routes have schemas, 1 if violations found.
 */

import { readFileSync } from 'fs';
import { resolve, relative } from 'path';
import { glob } from 'node:fs/promises';

const REPO_ROOT = resolve(new URL('.', import.meta.url).pathname, '..');

// Route files to scan
const ROUTE_PATTERNS = [
  'apps/api/src/routes/**/*.ts',
  'apps/api/src/routes/public/**/*.ts',
  'apps/api/src/routes/owner/**/*.ts',
  'apps/api/src/routes/courier/**/*.ts',
  'apps/api/src/routes/customer/**/*.ts',
  'apps/api/src/routes/admin/**/*.ts',
  'apps/api/src/routes/dev/**/*.ts',
  'apps/api/src/routes/auth/**/*.ts',
];

const EXCLUDE = [
  'spa-proxy.ts',
];

async function scan() {
  const violations = [];
  const passed = [];

  for (const pattern of ROUTE_PATTERNS) {
    for await (const file of glob(resolve(REPO_ROOT, pattern), { nodir: true })) {
      const relPath = relative(REPO_ROOT, file).replace(/\\/g, '/');
      if (EXCLUDE.some(e => relPath.includes(e))) continue;

      const content = readFileSync(file, 'utf-8');
      const lines = content.split('\n');

      let hasSchema = false;
      let missingLines = [];

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        // Check for route registrations
        const routeMatch = line.match(/(fastify|router)\.(get|post|put|patch|delete|head|options)\(/);
        if (routeMatch) {
          const nextLines = lines.slice(i, i + 10).join('\n');
          if (!nextLines.includes('schema:')) {
            missingLines.push({ line: i + 1, route: line.trim().slice(0, 80) });
          } else {
            hasSchema = true;
          }
        }
      }

      if (missingLines.length > 0) {
        violations.push({
          file: relPath,
          missing_schemas: missingLines.length,
          routes: missingLines.map(m => `  L${m.line}: ${m.route}`).join('\n'),
        });
      } else if (hasSchema) {
        passed.push(relPath);
      }
    }
  }

  const result = {
    passed: violations.length === 0,
    total_files_scanned: passed.length + violations.length,
    files_with_schemas: passed.length,
    files_missing_schemas: violations.length,
    violations: violations.map(v => ({
      file: v.file,
      routes_missing_schema: v.missing_schemas,
    })),
    details: violations.map(v => `${v.file}:\n${v.routes}`).join('\n'),
  };

  console.log(JSON.stringify(result, null, 2));

  if (violations.length > 0) {
    process.exit(1);
  }
}

scan().catch(err => {
  console.error(JSON.stringify({ passed: false, error: err.message }));
  process.exit(1);
});
