// Living Loop System foundation — public surface.
// Design: docs/operating-model/living-loop-system-v3.md
// Built: §10 steps 1–3 (contract, breaker, telemetry/record/report/storage).
// Deferred: §6 eco, §8 recall/distill/graduate, §4 fresh-context reviewer.
export * from './types.js';
export { DEFAULT_BREAKER, initBreaker, stepBreaker, breakerReasonText } from './breaker.js';
export { renderReport, computeHistory } from './report.js';
export {
  nextRunIndex, appendIter, writeRunRecord, readRunRecord,
  appendMetricsLine, readMetrics, iterTracePath, runRecordPath,
} from './storage.js';
export { runLoop } from './harness.js';
export type { HarnessOptions } from './harness.js';
