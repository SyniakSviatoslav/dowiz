import { readFileSync, readdirSync, statSync } from 'fs';
import { join, sep } from 'path';
import { BASE_URL } from './config.js';
import { formatReport } from './types.js';
import { loginMockOwner } from './harness/auth.js';
import { getLocationInfo, getMenu, placeOrder, confirmOrder, rejectOrder, advanceStatus, findFirstProduct } from './harness/order.js';
import { observeHealth, observeAudit } from './harness/observe.js';
import { runFeatureRadar, type FeatureRadarOptions } from './feature.js';

/**
 * Interconnected radar: given a source file, find all flows that depend on it,
 * then probe them in dependency order.
 */
export async function runInterconnectedRadar(sourceFile: string): Promise<void> {
  console.log(`\n=== Interconnected Radar: ${sourceFile} ===\n`);

  // 1. Parse imports/exports to find consumers
  const normalized = sourceFile.replace(/^\//, '').replace(/\//g, sep);
  const apiSrcDir = normalized.substring(0, normalized.indexOf(`${sep}apps${sep}api${sep}src${sep}`) + 15);
  const allTsFiles: string[] = [];
  function walkDir(dir: string) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) walkDir(full);
      else if (entry.isFile() && entry.name.endsWith('.ts') && !entry.name.includes('node_modules')) allTsFiles.push(full);
    }
  }
  walkDir(apiSrcDir);
  const sourceContent = readFileSync(sourceFile, 'utf-8');

  // Extract exported symbols from the source file
  const exportPattern = /export (?:async )?function (\w+)|export class (\w+)/g;
  const exports: string[] = [];
  let m: RegExpExecArray | null;
  while ((m = exportPattern.exec(sourceContent)) !== null) {
    exports.push(m[1] || m[2]);
  }

  console.log(`Source exports: ${exports.join(', ') || '(none)'}`);

  // Find files that import from this source
  const consumers: string[] = [];
  const sourceRelPath = sourceFile.replace(/\\/g, '/').replace(/^.*?\/apps\/api\/src\//, '');
  for (const f of allTsFiles) {
    const content = readFileSync(f, 'utf-8');
    if (content.includes(`'./${sourceRelPath.replace(/\.ts$/, '')}`) ||
        content.includes(`'../${sourceRelPath.replace(/\.ts$/, '')}`) ||
        exports.some(ex => content.includes(ex) && !content.includes('export'))) {
      consumers.push(f);
    }
  }

  console.log(`Consumers: ${consumers.length} file(s)`);
  for (const c of consumers.slice(0, 10)) {
    console.log(`  - ${c.replace(/.*src\//, '')}`);
  }
  if (consumers.length > 10) console.log(`  ... and ${consumers.length - 10} more`);

  // 2. Determine affected flows based on consumers
  const affectedFlows: FeatureRadarOptions['flows'] = [];

  if (sourceFile.includes('orderStatusService') || sourceFile.includes('order-machine')) {
    affectedFlows.push(
      { id: 'order-create', description: 'Create a pending order', trigger: 'POST /api/orders', expectedEffects: [] },
      { id: 'order-confirm', description: 'Confirm an order', trigger: 'POST confirm endpoint', expectedEffects: [] },
      { id: 'order-reject', description: 'Reject an order', trigger: 'POST reject endpoint', expectedEffects: [] },
    );
  }

  if (sourceFile.includes('notification') || sourceFile.includes('dwell') || sourceFile.includes('event-registry')) {
    affectedFlows.push(
      { id: 'health-check', description: 'System health', trigger: '/health', expectedEffects: [] },
    );
    // Add a notification-specific audit check
    try {
      const audit = await observeAudit();
      console.log(`\n📋 Recent audit entries: ${audit.length}`);
      for (const a of audit.slice(0, 5)) {
        console.log(`  [${a.status}] ${a.event} → ${a.channel} (${a.createdAt})`);
      }
    } catch { /* audit endpoint may not be exposed */ }
  }

  if (consumers.some(c => c.includes('courier'))) {
    affectedFlows.push(
      { id: 'courier-shift-start', description: 'Start shift', trigger: 'POST /me/shift/start', expectedEffects: [] },
    );
  }

  // 3. Probe each affected flow
  if (affectedFlows.length === 0) {
    console.log('\n⚠️  No automated probes defined for these consumers.');
    return;
  }

  console.log(`\nProbing ${affectedFlows.length} affected flow(s)...\n`);
  const report = await runFeatureRadar({ flows: affectedFlows });
  console.log(formatReport(report));
}

// CLI entry point
const isMain = process.argv[1]?.replace(/\\/g, '/').endsWith('radar/interconnected.ts');
if (isMain) {
  const relPath = process.argv[2];
  if (!relPath) {
    console.error('Usage: tsx interconnected.ts <source-file-path>');
    console.error('Example: tsx interconnected.ts apps/api/src/lib/orderStatusService.ts');
    process.exit(1);
  }

  const scriptDir = new URL('.', import.meta.url).pathname.replace(/^\/[a-zA-Z]:\//, '');
  const sourceFile = join(scriptDir, '../../../../', relPath);
  console.log(`Script dir: ${scriptDir}`);
  console.log(`Resolved source: ${sourceFile}`);
  if (!statSync(sourceFile, { throwIfNoEntry: false })) {
    console.error(`Source file not found: ${sourceFile}`);
    process.exit(1);
  }

  loginMockOwner().then(() => runInterconnectedRadar(sourceFile)).catch(err => {
    console.error('Radar failed:', err);
    process.exit(1);
  });
}
