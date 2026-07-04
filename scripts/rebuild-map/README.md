# scripts/rebuild-map/ — Phase-0 map-coverage tooling

Implements REBUILD-MAP.md §3 Phase-0 items 1-2 (`docs/design/rebuild-plan/REBUILD-MAP.md`):
per-namespace extractors against the CURRENT (Node/React) tree, a reconcile check against
the inventory docs' recorded counts, a `traceability.csv` seeder, and a coverage gate v0.

Plain Node ESM, zero new dependencies. Every script can be run standalone (`node
scripts/rebuild-map/<script>.mjs`) or imported (each exports pure, unit-testable functions
plus an `extract()`/`main()` entry point).

## Files

| File | Role |
|---|---|
| `lib/common.mjs` | repo-root resolution, deterministic file walking, `{ns,id,file,line}` sort/print helpers |
| `extract-routes.mjs` | `routes` — fastify HTTP route registrations (`apps/api/src`) |
| `extract-fe-routes.mjs` | `fe-routes` — react-router `<Route>` elements (`apps/web/src/{main.tsx,routes/*.tsx}`) |
| `extract-components.mjs` | `components` — `packages/ui/src` + `apps/web/src/components` `.tsx` files |
| `extract-vite-flags.mjs` | `vite-flags` — `VITE_*` tokens referenced in `apps/web/src` + `packages/ui/src` |
| `extract-server-flags-envs.mjs` | `server-flags-envs` — `EnvSchema` fields (programmatic brace-depth parse, not line-regex) + raw `process.env.*` reads |
| `extract-i18n-keys.mjs` | `i18n-keys` — top-level keys of the `catalog` object in `packages/ui/src/lib/i18n-catalog.ts` |
| `extract-ws-types.mjs` | `ws-types` — inbound `msg.type ===` + outbound `type: '...'` literals under `apps/api/src` |
| `extract-queues.mjs` | `queues` — `QUEUE_NAMES` registry (`packages/shared-types/src/queue-names.ts`) + local ad-hoc dotted-name consts |
| `extract-error-codes.mjs` | `error-codes` — unique `.sendError(status, 'CODE')` codes under `apps/api/src` |
| `extract-tables.mjs` | `tables` — `CREATE TABLE`/`DROP TABLE` replay over `packages/db/migrations/` (`up()` sections only, last-write-wins) |
| `extract-scripts-gates.mjs` | `scripts-gates` — root `package.json` scripts + `verify-all.ts` gates + `eslint-plugin-local` rules + `guardrail-*.mjs` files |
| `extract-all.mjs` | driver — runs every extractor, writes `out/inventory-current.jsonl`, prints per-namespace counts |
| `verify-counts.mjs` | reconcile check — live counts vs REBUILD-MAP.md §1 table (hardcoded, sourced); MATCH/DELTA report, always exits 0 |
| `seed-traceability.mjs` | generates `docs/design/rebuild-plan/traceability.csv` from `out/inventory-current.jsonl` |
| `map-coverage.mjs` | coverage gate v0 — UNMAPPED / ORPHAN / UNBUILT(stubbed); `--strict` exits 1 on UNMAPPED>0 |
| `__tests__/` | `node --test` unit tests (fixture strings) for the two trickiest extractors: routes (multi-line call parsing) and envs (brace-depth Zod parse) |

## Usage

```sh
# 1. Extract everything from the current tree -> out/inventory-current.jsonl
node scripts/rebuild-map/extract-all.mjs

# 2. Reconcile live counts against REBUILD-MAP.md §1 (report only, always exit 0)
node scripts/rebuild-map/verify-counts.mjs

# 3. Seed/refresh the traceability matrix from the extraction
node scripts/rebuild-map/seed-traceability.mjs

# 4. Run the coverage gate (report mode, or --strict to fail CI on UNMAPPED>0)
node scripts/rebuild-map/map-coverage.mjs
node scripts/rebuild-map/map-coverage.mjs --strict

# Unit tests (fixture-string, no filesystem/tree dependency)
node --test scripts/rebuild-map/__tests__/
```

## Root `package.json` script — PROPOSAL (root package.json is protected; not added here)

Add for the operator/lead:

```json
{
  "scripts": {
    "rebuild:map": "node scripts/rebuild-map/extract-all.mjs && node scripts/rebuild-map/map-coverage.mjs"
  }
}
```

## Design notes / known limitations (by design, not bugs)

- **Deterministic, not exhaustive.** Every extractor is deliberately "dumb" (grep/regex/one
  file parse) per REBUILD-MAP §8b — sophistication lives in the census regexes the inventory
  docs already proved, not in this tooling. IDs are content-derived (never a bare positional
  index) so re-running with no source changes diffs clean; see each extractor's header for
  its exact source-of-truth command.
- **Deltas vs the inventory docs are expected at this stage.** The docs are a point-in-time
  census of a tree that keeps moving (see `verify-counts.mjs` output). Some deltas are also
  unit-of-measure mismatches (e.g. `fe-routes` counts raw `<Route>` elements, not addressable
  paths) — each is annotated inline in `verify-counts.mjs`.
- **`server-flags-envs` found a real bug in the doc's OWN grep command**: the naive
  `grep -rhoE 'process\.env\.[A-Z_0-9]+' ...` misses `apps/api/src/lib/ai-ocr-parser.ts`
  entirely because grep's binary-file heuristic misfires on it (`binary file matches`, no
  `-a`). This extractor reads files as UTF-8 text directly and finds the 19 hits the shell
  pipeline silently drops — see the extractor's header comment for detail.
- **`tables` only sees `packages/db/migrations/`.** The 2 "out-of-band" tables the doc notes
  (applied by hand outside the migrations tree) are structurally invisible to a
  migrations-only extractor; this is disclosed, not silently swallowed.
- **UNBUILT is stubbed.** There is no Rust/Astro tree yet at Phase 0, so `map-coverage.mjs`
  documents the extract-new.<ns> interface it expects (§8b) rather than fabricating a
  same-side comparison.
- **Redline heuristic (`seed-traceability.mjs`) is intentionally imperfect** — a regex over
  `money|auth|rls|gdpr|payment|courier-dispatch|dispatch|ws|websocket` plus an automatic
  redline for the whole `ws-types` namespace. False positives/negatives are expected; the
  lead refines redline rows while drafting the 8 council packets (REBUILD-MAP §3 Phase-0
  item 5).
