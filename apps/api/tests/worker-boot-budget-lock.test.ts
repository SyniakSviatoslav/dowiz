import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

// ── DEPLOY-SAFETY lock — WORKER_BOOT_BUDGET_MS ceiling (incident 2026-07-03) ──
//
// Fly's /livez health check (interval 15s, no grace_period) fails the WHOLE deploy if
// fastify.listen() hasn't happened by the time Fly gives up. server.ts races
// startBackgroundWorkers() against a WORKER_BOOT_BUDGET_MS timeout BEFORE listen() — so this
// constant IS the boot-to-listen latency ceiling. It was 25_000ms; once the operational DB
// role regained its function grants, workers actually ran to the full 25s budget every time,
// so the server never listened inside Fly's window -> every deploy health-failed -> a
// single-machine prod outage (fixed in db30d273, 25s -> 3s).
//
// This is a SOURCE-assertion lock (WORKER_BOOT_BUDGET_MS is an inline literal inside main() in
// server.ts, not an exported constant — extracting it would be a runtime refactor, out of scope
// for a lock). It cannot import server.ts directly: server.ts calls main() unconditionally at
// module load (side effects: DB pool, fastify.listen, process.exit on failure), so an import
// would attempt a real boot. Reading the source text is the only side-effect-free way to pin it.
//
// FLOOR here is a CEILING: Fly's health-check window is ~15s; keep meaningful margin so retries/
// jitter don't reopen this class. 5000ms was chosen as the guardrail ceiling (the fix landed at
// 3000ms) — this locks "stays small", not the exact value, so future tuning within budget is free.

const __dirname = dirname(fileURLToPath(import.meta.url));
const SERVER_SRC = resolve(__dirname, '../src/server.ts');
const CEILING_MS = 5_000;

test('WORKER_BOOT_BUDGET_MS stays well under Fly\'s health-check window (deploy-safety lock)', () => {
  const src = readFileSync(SERVER_SRC, 'utf8');

  const match = src.match(/const\s+WORKER_BOOT_BUDGET_MS\s*=\s*([\d_]+)/);
  assert.ok(match, 'server.ts: expected a WORKER_BOOT_BUDGET_MS literal — if it was renamed or ' +
    'moved, update this lock to track the new constant (do not delete the lock).');

  const value = Number(match![1].replace(/_/g, ''));
  assert.ok(
    value <= CEILING_MS,
    `server.ts: WORKER_BOOT_BUDGET_MS=${value}ms exceeds the ${CEILING_MS}ms deploy-safety ` +
      'ceiling — this budget runs BEFORE fastify.listen(), so a value this large risks missing ' +
      "Fly's /livez window and health-failing every deploy (incident 2026-07-03, commit db30d273). " +
      'If workers genuinely need longer to start, let them keep running in the background after ' +
      'listen() (as designed) rather than raising this budget.',
  );

  // The race must still resolve to fastify.listen() being reachable regardless of worker outcome
  // — i.e. the budget must be paired with a Promise.race (not a plain await), or a slow/hanging
  // startBackgroundWorkers() would block listen() no matter how small the budget is.
  assert.match(
    src,
    /Promise\.race\(\[\s*startBackgroundWorkers/,
    'server.ts: startBackgroundWorkers must be raced (Promise.race), not awaited directly — ' +
      'otherwise WORKER_BOOT_BUDGET_MS is decorative and a hang still blocks fastify.listen().',
  );
});
