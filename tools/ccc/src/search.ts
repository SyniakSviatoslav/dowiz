/**
 * ccc search (ADR-0012 C1) — rank symbols by name relevance over a built index.
 *
 * Deliberately simple (no embeddings): exact > prefix > camelCase-token > substring, with a small
 * boost for exported symbols. Enough to answer "where is function X" / "what handles Y" without a
 * grep sweep, at a fraction of the tokens.
 */
import type { Index, Symbol } from './indexer.js';

function score(sym: Symbol, q: string): number {
  const name = sym.name.toLowerCase();
  const query = q.toLowerCase();
  if (name === query) return 100;
  if (name.startsWith(query)) return 70;
  // camelCase / snake_case token boundary match (e.g. "send" hits sendError, send_error)
  const tokens = sym.name.split(/(?=[A-Z])|[_\-.]/).map((t) => t.toLowerCase());
  if (tokens.includes(query)) return 55;
  if (tokens.some((t) => t.startsWith(query))) return 40;
  if (name.includes(query)) return 25;
  return 0;
}

export interface SearchOpts {
  kind?: string;
  limit?: number;
}

export function search(index: Index, query: string, opts: SearchOpts = {}): Symbol[] {
  const limit = opts.limit ?? 25;
  const ranked = index.symbols
    .filter((s) => (opts.kind ? s.kind === opts.kind : true))
    .map((s) => ({ s, base: score(s, query) }))
    .filter((r) => r.base > 0) // a name match is REQUIRED — the export boost only re-ranks
    .map((r) => ({ s: r.s, score: r.base + (r.s.exported ? 3 : 0) }))
    .sort((a, b) => b.score - a.score || a.s.file.localeCompare(b.s.file) || a.s.line - b.s.line)
    .slice(0, limit)
    .map((r) => r.s);
  return ranked;
}

export function formatResults(results: Symbol[]): string {
  if (!results.length) return 'No symbols matched.';
  const pad = Math.min(Math.max(...results.map((r) => r.kind.length)), 10);
  return results
    .map((r) => `${r.file}:${r.line}  ${r.kind.padEnd(pad)}  ${r.signature}`)
    .join('\n');
}
