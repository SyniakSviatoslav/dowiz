// ingest.mjs — the "living" feed: read the project's MAIN LAYERS into the store as a graph+vectors.
//
// Layers (the harness's own structure — this is the "brain inside the brain" corpus):
//   core-rules      — CLAUDE.md, AGENTS.md, the operating-model rule docs
//   infra           — hooks, guardrail armaments, run-armaments, circuits registry, settings
//   self-evolution  — loop specs + registry, the living-loop/fable-audit/living-knowledge docs
//   living-memory   — loop memories + reflections (+ the ~/.claude auto-memory if present)
//
// Nodes = files (id = repo-relative path, label = layer). Edges = EXPLICIT cross-file references
// (file A's text mentions file B's path/basename) — a real, checkable structure, not synthesized.
// Edge type: 'governs' when the source is a core-rule (feeds the `why` band), else 'references'.
// Deterministic given the checkout (recency from mtime is checkout-relative; the determinism test
// runs same-process so it holds). Bounded: a curated file set, text capped for embedding.
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join, basename, relative } from 'node:path';

export const ROOT = (() => { try { return execSync('git rev-parse --show-toplevel', { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim(); } catch { return process.cwd(); } })();

const lsFiles = (dir, re) => { try { return readdirSync(join(ROOT, dir)).filter((f) => re.test(f)).map((f) => `${dir}/${f}`); } catch { return []; } };

function collect() {
  const layers = {
    'core-rules': [
      '.claude/CLAUDE.md', 'AGENTS.md',
      'docs/operating-model/verified-by-math.md',
      'docs/operating-model/model-agnostic-playbook.md',
      'docs/operating-model/task-exit-rule.md',
      'docs/operating-model/claude-md-reference.md',
    ],
    infra: [
      ...lsFiles('.claude/hooks', /\.sh$/),
      ...lsFiles('scripts', /^guardrail-.*\.mjs$/),
      'scripts/run-armaments.sh', 'scripts/run-circuits.mjs', 'scripts/audit-token-router.mjs',
      'scripts/module-integrity.mjs', 'docs/operating-model/circuits/registry.json', '.claude/settings.json',
    ],
    'self-evolution': [
      ...lsFiles('loops', /\.yaml$/), 'loops/registry.md',
      'docs/operating-model/living-loop-system-v3.md',
      'docs/operating-model/fable-audit-findings-2026-07-07.md',
      'docs/operating-model/living-knowledge-helixdb-arc.md',
    ],
    'living-memory': [
      ...lsFiles('loops/memory', /\.md$/),
      ...lsFiles('docs/reflections', /\.md$/),
      ...lsFiles('docs/reflections/INBOX', /\.md$/),
    ],
  };
  const files = [];
  for (const [layer, list] of Object.entries(layers)) {
    for (const rel of list) {
      const abs = join(ROOT, rel);
      if (!existsSync(abs)) continue;
      let text = ''; let mtime = 0;
      try { text = readFileSync(abs, 'utf8'); mtime = statSync(abs).mtimeMs; } catch { continue; }
      files.push({ rel, layer, title: basename(rel), text, mtime });
    }
  }
  return files;
}

export function buildStore(store) {
  const files = collect();
  if (files.length === 0) throw new Error('ingest: no files collected (wrong ROOT?)');
  const minT = Math.min(...files.map((f) => f.mtime)), maxT = Math.max(...files.map((f) => f.mtime));
  const span = maxT - minT || 1;

  for (const f of files) {
    store.addNode({ id: f.rel, label: f.layer, title: f.title, text: f.text.slice(0, 8000), meta: { path: f.rel, recency: (f.mtime - minT) / span } });
  }
  // reference edges: file A mentions file B's repo-path OR distinctive basename.
  const byBase = new Map();
  for (const f of files) { const b = f.title; if (!byBase.has(b)) byBase.set(b, []); byBase.get(b).push(f.rel); }
  let edgeCount = 0;
  for (const a of files) {
    const seen = new Set();
    for (const b of files) {
      if (a.rel === b.rel) continue;
      // match full repo-path (strong) or a distinctive basename (>=8 chars, unique) (weaker).
      const hitPath = a.text.includes(b.rel);
      const uniqueBase = (byBase.get(b.title) || []).length === 1 && b.title.length >= 8 && a.text.includes(b.title);
      if (hitPath || uniqueBase) {
        if (seen.has(b.rel)) continue; seen.add(b.rel);
        const type = a.layer === 'core-rules' ? 'governs' : 'references';
        store.addEdge({ from: a.rel, to: b.rel, type, weight: hitPath ? 1 : 0.6 });
        edgeCount++;
      }
    }
  }
  store.finalize();
  return { files: files.length, edges: edgeCount };
}

// CLI: print the ingest summary.
if (import.meta.url === `file://${process.argv[1]}`) {
  const { MemoryStore } = await import('./lib/store.mjs');
  const s = new MemoryStore();
  const info = buildStore(s);
  console.log(`ingested ${info.files} files, ${info.edges} reference edges across layers:`);
  const byLayer = {};
  for (const n of s.nodes()) byLayer[n.label] = (byLayer[n.label] || 0) + 1;
  for (const [l, c] of Object.entries(byLayer).sort()) console.log(`  ${l}: ${c} nodes`);
}
