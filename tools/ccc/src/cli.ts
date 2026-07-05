#!/usr/bin/env node
/**
 * ccc CLI (ADR-0012 C1, dev-only) — AST-semantic code search.
 *
 *   tsx tools/ccc/src/cli.ts index [--root .] [--out .ccc/index.json] [--label <sha>]
 *   tsx tools/ccc/src/cli.ts search <query> [--kind function|class|…] [--limit N] [--index .ccc/index.json]
 *
 * Dev-only: the index lands in `.ccc/` (gitignored, NOT in `dist/`). The walker never reads a
 * secret (B10) — see tools/ccc/src/ignore.ts + the secret-scan merge gate (verify:ccc-secrets).
 */
import { writeFileSync, mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import { buildIndex, type Index } from './indexer.js';
import { search, formatResults } from './search.js';

function arg(flag: string, fallback?: string): string | undefined {
  const i = process.argv.indexOf(flag);
  return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
}

function cmdIndex(): void {
  const root = resolve(arg('--root', '.')!);
  const out = resolve(arg('--out', '.ccc/index.json')!);
  const label = arg('--label', 'local')!;
  // ADR-0012: zero index artifacts in dist/. Refuse to write there.
  if (out.split(sep).includes('dist')) {
    console.error('ccc: refusing to write the index into dist/ (ADR-0012: zero artifacts in dist).');
    process.exit(2);
  }
  const { index, readPaths } = buildIndex(root, label);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, JSON.stringify(index, null, 0));
  console.log(`ccc: indexed ${index.symbols.length} symbols from ${readPaths.length} files → ${out}`);
}

function cmdSearch(): void {
  const query = process.argv[3];
  if (!query || query.startsWith('--')) {
    console.error('Usage: ccc search <query> [--kind K] [--limit N] [--index path]');
    process.exit(2);
  }
  const indexPath = resolve(arg('--index', '.ccc/index.json')!);
  if (!existsSync(indexPath)) {
    console.error(`ccc: no index at ${indexPath} — run "ccc index" first.`);
    process.exit(2);
  }
  const index = JSON.parse(readFileSync(indexPath, 'utf8')) as Index;
  const kind = arg('--kind');
  const limit = arg('--limit') ? parseInt(arg('--limit')!, 10) : undefined;
  console.log(formatResults(search(index, query, { kind, limit })));
}

const cmd = process.argv[2];
if (cmd === 'index') cmdIndex();
else if (cmd === 'search') cmdSearch();
else {
  console.error('ccc (ADR-0012 C1, dev-only). Commands:\n  index   build the symbol index\n  search  query the index');
  process.exit(2);
}
