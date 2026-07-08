// Bebop knowledge seam — reuse the repo's existing intelligence, do not reinvent it.
//
// Ground truth: tools/vsa (VSA token economy — route.mjs / match.mjs) and
// spikes/living-knowledge (deterministic recall@5 retriever, the §0·GP engine). We shell out to
// them the same way scripts/agents-mesh.sh and the mesh already do — ground-truth over proxy.
// PLUS: the in-process livingMemory() singleton (src/memory.ts) — ONE always-on memory shared by
// every agentic CLI, this Hermes session included. recall() consults BOTH.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { livingMemory } from './memory.ts';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, '..', '..', '..');

export interface Recall {
  found: boolean;
  hits: { id: string; text: string; score?: number }[];
  note: string;
}

// Query the in-process living memory (ONE memory, this session included) — content-addressed recall.
export function recallLocal(query: string): { id: string; text: string }[] {
  const mem = livingMemory();
  const ids = mem.recall(query, 3);
  if (ids.length === 0) return [];
  // we don't expose payload via id cheaply; re-derive by nearest for display
  return ids.map((id) => ({ id, text: id.slice(0, 12) }));
}

// Call the living-knowledge §0·GP retriever. Returns ranked {id,text} hits.
export function recall(query: string): Recall {
  const hits = recallLocal(query).map((h) => ({ id: h.id, text: h.text, score: 1 }));
  const script = path.join(REPO_ROOT, 'spikes', 'living-knowledge', 'search.mjs');
  try {
    const out = execFileSync('node', [script, query], { encoding: 'utf8', timeout: 20000 });
    const remote = parseRecall(out);
    return {
      found: true,
      hits: [...hits, ...remote],
      note: `in-process livingMemory + living-knowledge §0·GP recall`,
    };
  } catch (e: any) {
    // local memory still works even if the repo retriever is absent — degrade honestly
    return {
      found: hits.length > 0,
      hits,
      note: `in-process livingMemory only (living-knowledge unavailable: ${String(e.message ?? e).split('\n')[0]})`,
    };
  }
}

export function rememberLocal(concept: string, payload: string, linkTo?: string[]): string {
  return livingMemory().remember(concept, payload, linkTo);
}

// VSA token estimate via tools/vsa/cli.mjs tokens. Returns null if vsa absent.
export function estimateTokens(text: string): number | null {
  const cli = path.join(REPO_ROOT, 'tools', 'vsa', 'cli.mjs');
  try {
    const tmp = path.join(REPO_ROOT, 'tools', 'bebop', '.tmp-recall.json');
    fs.writeFileSync(tmp, text);
    const out = execFileSync('node', [cli, 'tokens', tmp], { encoding: 'utf8', timeout: 10000 });
    const m = out.match(/(\d+)/);
    return m ? Number(m[1]) : null;
  } catch {
    return null;
  }
}

function parseRecall(out: string): { id: string; text: string; score?: number }[] {
  // tolerate JSON array or line-delimited; best-effort parse
  try {
    const j = JSON.parse(out);
    if (Array.isArray(j)) return j.map((h: any) => ({ id: String(h.id ?? ''), text: String(h.text ?? ''), score: h.score }));
  } catch { /* fall through */ }
  return out
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .slice(0, 5)
    .map((l, i) => ({ id: `hit-${i}`, text: l }));
}
