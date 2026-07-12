import { readFileSync, readdirSync } from 'fs';
import { join } from 'path';
import { BUS_CHANNELS, QUEUE_NAMES } from '../src/lib/registry.js';

const ALL_CHANNEL_VALUES = new Set(Object.values(BUS_CHANNELS));
const ALL_QUEUE_VALUES = new Set(Object.values(QUEUE_NAMES));

function getAllFiles(dir: string): string[] {
  const result: string[] = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules') {
      result.push(...getAllFiles(full));
    } else if (entry.isFile() && (entry.name.endsWith('.ts') || entry.name.endsWith('.tsx'))) {
      result.push(full);
    }
  }
  return result;
}

const QUEUES_WITH_SELF_REGISTERED_WORKERS = new Set([
  'dwell.monitor', 'dwell.escalate', 'liveness.check', 'gps.purge',
  'courier.stale_check', 'signal.raiser', 'settlement.generate',
  'backup.hourly', 'backup.daily', 'backup.weekly', 'backup.monthly',
  'backup.verify.restore', 'backup.verify.r2',
]);

const CHANNELS_WITH_SELF_REGISTERED_SUBSCRIBERS = new Set([
  'order.created', 'order.courier_accepted', 'order.dispatch_failed',
  'order.cancelled.customer_after_dispatch', 'backup.completed',
  'dwell.monitor.failed', 'dwell.alert_resolved',
  'customer.anonymized', 'order.anonymized',
  'customer.no_show_incremented', 'menu.import.previewed', 'menu.imported', 'menu.translated',
  'otp.sent', 'otp.verified', 'worker.stale', 'worker.batch_stale', 'worker.recovered',
  'worker.failed', 'alert.worker_liveness', 'liveness.check.failed',
  'anonymizer.gdpr.failed', 'gdpr.erasure_completed',
  'signal.created', 'signal.acknowledged', 'signal.dismissed',
  'alert.acknowledged', 'alert.resolved_automatically', 'contact.revealed',
  'settlement.approved', 'order.status',
]);

function main() {
  const apiFiles = getAllFiles(join(import.meta.dirname, '../src'));
  const webFiles = getAllFiles(join(import.meta.dirname, '../../../apps/web/src'));
  const workerFiles = getAllFiles(join(import.meta.dirname, '../../../apps/worker/src'));
  const allFiles = [...apiFiles, ...webFiles, ...workerFiles];

  let failures = 0;

  const fileContents = new Map<string, string>();
  for (const f of allFiles) {
    try { fileContents.set(f, readFileSync(f, 'utf-8')); } catch { /* skip */ }
  }

  // Check 1: Raw string leaks (channels)
  console.log('=== Raw String Leak Check (channels) ===');
  for (const [file, content] of fileContents) {
    const rawSubPattern = /\.subscribe\(['"]([\w.-]+)['"]/g;
    let m: RegExpExecArray | null;
    while ((m = rawSubPattern.exec(content)) !== null) {
      if (ALL_CHANNEL_VALUES.has(m[1])) {
        console.log(`  ⚠️ Raw channel "${m[1]}" in .subscribe() at ${rel(file)}`);
        failures++;
      }
    }
    const rawPubPattern = /\.publish\(['"]([\w.-]+)['"]/g;
    while ((m = rawPubPattern.exec(content)) !== null) {
      if (ALL_CHANNEL_VALUES.has(m[1])) {
        console.log(`  ⚠️ Raw channel "${m[1]}" in .publish() at ${rel(file)}`);
        failures++;
      }
    }
    const rawWorkPattern = /\.work\(['"]([\w.-]+)['"]/g;
    while ((m = rawWorkPattern.exec(content)) !== null) {
      if (ALL_QUEUE_VALUES.has(m[1])) {
        console.log(`  ⚠️ Raw queue "${m[1]}" in .work() at ${rel(file)}`);
        failures++;
      }
    }
    const rawSendPattern = /\.(send|enqueue)\(['"]([\w.-]+)['"]/g;
    while ((m = rawSendPattern.exec(content)) !== null) {
      if (ALL_QUEUE_VALUES.has(m[1])) {
        console.log(`  ⚠️ Raw queue "${m[1]}" in .${m[1]}() at ${rel(file)}`);
        failures++;
      }
    }
  }
  if (failures === 0) console.log('  ✅ No raw string leaks');

  // Check 2: Only critical queue orphans (jobs sent but no worker exists)
  console.log('\n=== Critical Queue Orphans (dropped jobs) ===');
  for (const [key, queueName] of Object.entries(QUEUE_NAMES)) {
    if (QUEUES_WITH_SELF_REGISTERED_WORKERS.has(queueName)) continue;

    let hasEnqueue = false;
    let hasWorker = false;

    for (const [, content] of fileContents) {
      const ref = `QUEUE_NAMES.${key}`;
      if (content.includes(ref)) {
        const enqPattern = new RegExp(`\\.(send|enqueue)\\(${ref.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`);
        const workPattern = new RegExp(`\\.work\\(${ref.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`);
        if (enqPattern.test(content)) hasEnqueue = true;
        if (workPattern.test(content)) hasWorker = true;
      }
    }

    if (hasEnqueue && !hasWorker) {
      console.log(`  ⚠️ Queue "${queueName}" — jobs sent but no worker found`);
      failures++;
    }
    if (!hasEnqueue && hasWorker) {
      console.log(`  ℹ️ Queue "${queueName}" — worker registered but no sender (dead worker, OK for H-1)`);
    }
  }
  if (failures === 0) console.log('  ✅ No critical queue orphans');

  // Check A2: silent returns in notification workers (return without audit/log)
  console.log('\n=== A2: Silent Return Check (notification workers) ===');
  const NOTIF_FILES = ['notifications/workers/index.ts', 'workers/dwell-monitor.ts'];
  let silentReturns = 0;
  for (const [file, content] of fileContents) {
    if (!NOTIF_FILES.some(nf => file.replace(/\\/g, '/').includes(nf))) continue;
    // Find bare "return;" or "return;" that isn't preceded by writeAudit/console.error
    const lines = content.split('\n');
    for (let i = 0; i < lines.length; i++) {
      const trimmed = lines[i].trim();
      if (trimmed === 'return;' || trimmed === 'return' || trimmed.match(/^\s*return\s*$/)) {
        // Check 5 lines above for writeAudit or console
        const context = lines.slice(Math.max(0, i - 5), i + 1).join('\n');
        if (!context.includes('writeAudit') && !context.includes('console.error') && !context.includes('console.log')) {
          console.log(`  ⚠️ Silent return at ${file}:${i + 1}`);
          silentReturns++;
        }
      }
    }
  }
  if (silentReturns > 0) {
    console.log(`  ❌ ${silentReturns} silent return(s) found — each dispatch drop must leave audit trail`);
    failures += silentReturns;
  } else {
    console.log('  ✅ No silent returns in notification workers');
  }

  if (failures > 0) {
    console.log(`\n❌ ${failures} issue(s) found`);
    process.exit(1);
  } else {
    console.log(`\n✅ NX-1 stop-bleeding checks pass. Zero raw string leaks. Zero critical queue orphans.`);
    process.exit(0);
  }

  function rel(f: string) {
    const base = join(import.meta.dirname, '../../..');
    return f.startsWith(base) ? f.slice(base.length + 1) : f;
  }
}

main();
