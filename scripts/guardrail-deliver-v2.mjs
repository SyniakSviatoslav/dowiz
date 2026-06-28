#!/usr/bin/env node
// Guardrail (ADR-deliver-v2-cash-as-proof) — two deterministic invariants the council required:
//
//  (R2-1 completion-parity) The cash-as-proof 'hold' + delivery_trace crumb are written ONLY by the single
//  primitive lib/deliveryCompletion.ts::completeDelivery. A new completion path that writes delivery_trace or
//  a courier_cash_ledger 'hold' inline (instead of calling completeDelivery) re-opens the R2-1 silent-debt
//  gap (the owner-proxy path used to write NONE). So: any `INSERT INTO delivery_trace` / `INSERT INTO
//  courier_cash_ledger` outside deliveryCompletion.ts is RED.
//
//  (R3-3 no-new-raw-cancel) No NEW raw `UPDATE orders SET status … 'CANCELLED'` outside the central
//  updateOrderStatus (which folds in the assignment-terminalize so no order leaves IN_DELIVERY stranded).
//  Allowlisted existing sites: orderStatusService.ts (the central fold itself), customer/orders.ts (the
//  grandfathered cash-reversal-coupled site, R-16/R3-3), order-timeout-sweep.ts (PENDING→CANCELLED timeout,
//  never IN_DELIVERY). Any new occurrence elsewhere is RED → forced through the fold.
//
// Run: node scripts/guardrail-deliver-v2.mjs   (exit 1 on violation). Escape hatch: `guardrail-exempt: <why>`.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'apps/api/src';
const PRIMITIVE = 'apps/api/src/lib/deliveryCompletion.ts';
const RAW_CANCEL_ALLOW = new Set([
  'apps/api/src/lib/orderStatusService.ts',
  'apps/api/src/routes/customer/orders.ts',
  'apps/api/src/workers/order-timeout-sweep.ts',
]);
const violations = [];

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) walk(p);
    else if (p.endsWith('.ts')) scan(p);
  }
}

function scan(file) {
  const text = readFileSync(file, 'utf8');
  const lines = text.split('\n');

  // R2-1 parity: hold/trace writes only in the primitive.
  if (file.replace(/\\/g, '/') !== PRIMITIVE) {
    lines.forEach((line, i) => {
      if (/guardrail-exempt:/.test(line)) return;
      if (/INSERT\s+INTO\s+delivery_trace/i.test(line)) {
        violations.push(`${file}:${i + 1}: writes delivery_trace outside completeDelivery (R2-1 parity) → ${line.trim().slice(0, 100)}`);
      }
      if (/INSERT\s+INTO\s+courier_cash_ledger/i.test(line)) {
        violations.push(`${file}:${i + 1}: writes courier_cash_ledger outside completeDelivery (R2-1 parity) → ${line.trim().slice(0, 100)}`);
      }
    });
  }

  // R3-3 no-new-raw-cancel: scan a small window for `UPDATE orders SET status` reaching 'CANCELLED'.
  if (!RAW_CANCEL_ALLOW.has(file.replace(/\\/g, '/'))) {
    lines.forEach((line, i) => {
      if (!/UPDATE\s+orders\s+SET[\s\S]*status/i.test(line)) return;
      const window = lines.slice(i, i + 3).join(' ');
      if (/'CANCELLED'/.test(window) && !/guardrail-exempt:/.test(window)) {
        violations.push(`${file}:${i + 1}: raw UPDATE orders → 'CANCELLED' outside updateOrderStatus (R3-3) → ${line.trim().slice(0, 100)}`);
      }
    });
  }
}

walk(ROOT);

if (violations.length) {
  console.error(`✗ guardrail-deliver-v2: ${violations.length} violation(s) (ADR-deliver-v2-cash-as-proof):`);
  for (const v of violations) console.error('  ' + v);
  console.error('\nFix: route completions through lib/deliveryCompletion.ts::completeDelivery and order cancels through updateOrderStatus.');
  process.exit(1);
}
console.log('✓ guardrail-deliver-v2: completion-parity (R2-1) + no-new-raw-cancel (R3-3) hold.');
