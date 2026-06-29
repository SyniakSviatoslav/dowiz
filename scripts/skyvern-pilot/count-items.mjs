#!/usr/bin/env node
// Helper for run-measurement.sh — read a Skyvern JSON response on stdin, print the extracted item
// count (or ERR). Kept as its own file so the runner has no fragile inline JS quoting.
let s = '';
process.stdin.on('data', (d) => (s += d)).on('end', () => {
  try {
    const j = JSON.parse(s);
    const n = Array.isArray(j.items) ? j.items.length
      : Array.isArray(j.products) ? j.products.length
      : Array.isArray(j?.output?.products) ? j.output.products.length : 0;
    process.stdout.write(String(n));
  } catch {
    process.stdout.write('ERR');
  }
});
