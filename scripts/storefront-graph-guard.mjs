#!/usr/bin/env node
// Sense 4 — storefront eager-graph guard (PR gate, exits 1 on regression).
//
// WHAT IT PROVES: admin-only code is NOT in the storefront's EAGER (statically
// imported) module graph. Vite emits lazy routes as dynamic `import("./X.js")`
// and eager deps as bare `from"./X.js"` / `import"./X.js"`. We walk the
// transitive STATIC-import closure from the HTML entry chunk and fail if any
// admin chunk is reachable eagerly. A lazy `import(...)` of AdminRoutes is fine
// (that is the whole point of code-splitting); a static `from"./AdminRoutes…"`
// in the closure means admin JS now ships in the first storefront paint.
//
// This is load-bearing: it goes RED the moment someone replaces a React.lazy()
// admin route with a top-level import, or a shared module starts statically
// pulling an admin chunk.

import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ASSETS_DIR = join(ROOT, 'apps/web/dist/assets');
const INDEX_HTML = join(ROOT, 'apps/web/dist/index.html');

// Chunk-name patterns considered admin-only. Extend this allowlist as new
// admin surfaces are split out. (Courier is a separate surface; add /courier/i
// here if/when the storefront must also exclude it.)
const ADMIN_PATTERNS = [/admin/i, /analytics/i];

function fail(msg) {
  console.error(`\n✗ storefront-graph-guard: ${msg}\n`);
  process.exit(1);
}

if (!existsSync(ASSETS_DIR) || !existsSync(INDEX_HTML)) {
  fail(
    `dist not found (looked for ${INDEX_HTML} and ${ASSETS_DIR}). ` +
      `Run \`pnpm build\` first — this guard runs against the built output.`,
  );
}

// 1. Resolve the eager entry chunk from the HTML <script type="module" src>.
const html = readFileSync(INDEX_HTML, 'utf8');
const entryMatch = html.match(
  /<script[^>]*type=["']module["'][^>]*src=["']\/assets\/(index-[A-Za-z0-9_-]+\.js)["']/i,
);
if (!entryMatch) {
  fail(
    'could not locate the module entry script (assets/index-*.js) in index.html. ' +
      'Vite output shape may have changed — update the regex.',
  );
}
const entry = entryMatch[1];

// Extract ONLY static-import specifiers from a chunk's source. Dynamic
// `import("./X.js")` is intentionally NOT matched (lazy = allowed).
function staticImportsOf(file) {
  const src = readFileSync(join(ASSETS_DIR, file), 'utf8');
  const out = new Set();
  // `from"./X.js"` (re-export / import) and bare `import"./X.js"` side-effect
  // import. Dynamic `import("./X.js")` is NOT matched (it is preceded by `(`).
  for (const m of src.matchAll(/\bfrom\s*["']\.\/([A-Za-z0-9_.-]+\.js)["']/g)) out.add(m[1]);
  for (const m of src.matchAll(/(^|[;{}\s])import\s*["']\.\/([A-Za-z0-9_.-]+\.js)["']/g))
    out.add(m[2]);
  return [...out];
}

// 2. BFS the transitive static-import closure from the entry.
const eager = new Set([entry]);
const queue = [entry];
while (queue.length) {
  const cur = queue.shift();
  for (const dep of staticImportsOf(cur)) {
    if (!eager.has(dep) && existsSync(join(ASSETS_DIR, dep))) {
      eager.add(dep);
      queue.push(dep);
    }
  }
}

// 3. Fail if any eagerly-reachable chunk matches an admin pattern.
const offenders = [...eager].filter((f) => ADMIN_PATTERNS.some((p) => p.test(f)));
if (offenders.length) {
  fail(
    `admin-only chunk(s) reachable from the storefront EAGER graph:\n` +
      offenders.map((f) => `    - ${f}`).join('\n') +
      `\n  These must be lazy (React.lazy / dynamic import), not statically imported by the entry.`,
  );
}

console.log(
  `✓ storefront-graph-guard: entry=${entry}; ${eager.size} eager chunks; ` +
    `no admin chunk (${ADMIN_PATTERNS.map(String).join(', ')}) in the eager graph.`,
);
