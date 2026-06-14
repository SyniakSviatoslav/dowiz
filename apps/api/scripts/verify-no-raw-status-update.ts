/**
 * Guard: no raw UPDATE orders SET status outside updateOrderStatus()
 * 
 * Scans apps/api/src for direct SQL UPDATE on orders.status that bypasses
 * the canonical path (updateOrderStatus service).
 * 
 * Exceptions (non-status transitions, not customer-facing):
 * - Courier reassignment (owner/dashboard.ts: owner_reassigned reverts to READY)
 * - No-show mark (owner/signals.ts: sets CANCELLED + no_show_count)
 * - Timeout cancel (workers: auto-cancels PENDING > timeout via order.timeout queue)
 * 
 * Usage: tsx verify-no-raw-status-update.ts
 */

import { readFileSync, readdirSync } from 'fs';
import { join } from 'path';

const ROOT = join(import.meta.dirname, '../src');

const ALLOWED_PATTERNS = [
  'courier_id = NULL',     // courier reassign reverts to READY
  'no_show',               // mark-no-show sets CANCELLED + increments count
  'timeout_at',            // timeout cancel or confirm clears timeout
  'status_notes',          // no_show sets notes
];

function getAllFiles(dir: string): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.startsWith('node_modules')) result.push(...getAllFiles(full));
    else if (entry.isFile() && entry.name.endsWith('.ts')) result.push(full);
  }
  return result;
}

function main() {
  const files = getAllFiles(ROOT);
  let failures = 0;

  console.log('=== A3: Raw UPDATE orders SET status check ===\n');

  for (const file of files) {
    const content = readFileSync(file, 'utf-8');
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      // Match: UPDATE orders SET status = 'X' (raw string, not parameterized)
      const rawMatch = line.match(/UPDATE\s+orders\s+SET\s+status\s*=\s*'/i);
      if (!rawMatch) continue;

      // Check if it's in an allowed exception
      const context = lines.slice(Math.max(0, i - 2), i + 3).join('\n');
      const isAllowed = ALLOWED_PATTERNS.some(p => context.includes(p));

      // Check if it's inside updateOrderStatus (the canonical path)
      const isCanonical = file.includes('orderStatusService.ts');

      if (!isAllowed && !isCanonical) {
        const relPath = file.replace(/\\/g, '/').replace(/^.*?apps\/api\/src\//, '');
        console.log(`  ⚠️ Raw UPDATE orders SET status at ${relPath}:${i + 1}`);
        console.log(`    ${line.trim().substring(0, 80)}`);
        failures++;
      }
    }
  }

  if (failures > 0) {
    console.log(`\n❌ ${failures} raw status update(s) found — must route through updateOrderStatus()`);
    process.exit(1);
  } else {
    console.log('✅ All status changes go through canonical path (updateOrderStatus or allowed exception)');
    process.exit(0);
  }
}

main();
