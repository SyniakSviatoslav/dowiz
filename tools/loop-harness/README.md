# loop-harness (Living Loop System — foundation)

Dev-plane only. Design: [`docs/operating-model/living-loop-system-v3.md`](../../docs/operating-model/living-loop-system-v3.md).

The harness is the only way an agent loop runs (§1). It drives `iterate()`, guards
the loop with a no-progress **breaker** (§3), captures per-iteration **telemetry**
(§2), builds the canonical **run-record** (§7), **always prints the full report**
(§5), and persists **permanently + losslessly** (gzip + append-only, never cleaned).

## Built (§10 steps 1–3, deterministic core)
- `src/types.ts` — the `Loop` contract + telemetry/record shapes.
- `src/breaker.ts` — trips on stall (K non-improving iters) · max_iter · budget · time_cap.
- `src/report.ts` — `renderReport(record)` (pure view) + `computeHistory(metrics)` (§6 VS-HISTORY).
- `src/storage.ts` — append-only `metrics.jsonl`, gzipped run-records (lossless round-trip), iteration traces. Nothing here deletes or overwrites.
- `src/harness.ts` — `runLoop(loop, initialState, opts)` ties it together.

## Deferred (steps 4–7 — integration-heavy, same seam)
- §6 eco block (EcoLogits/CodeCarbon wiring).
- §8 per-iteration RECALL + DISTILL + human-gated GRADUATE (needs session-file parsing).
- §4 fresh-context reviewer gate (a separate clean-context Claude Code invocation).

## Run
```bash
# tests (17, all green)
node --test --import tsx tools/loop-harness/tests/*.test.ts
# typecheck (no tsconfig — infra path is gated; one-off)
npx tsc --noEmit --strict --module nodenext --moduleResolution nodenext --target es2022 --skipLibCheck tools/loop-harness/src/*.ts
```

## Minimal usage
```ts
import { runLoop } from './tools/loop-harness/src/index.js';
const record = await runLoop(myLoop, initialState, { baseDir: 'runs', ctx });
// report printed to terminal; record + metrics line persisted under runs/<loop>/
```
A loop implements 5 hooks (`goal`, `iterate`, `progressMetric`, `reflect?`, `isTerminal`);
the harness provides breaker, telemetry, record, report, and storage by construction.
