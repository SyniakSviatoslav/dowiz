// §8c GRADUATE / Class-B proposal queue. Class B (auth/RLS/secrets/payments/PII/
// schema/architecture) is NEVER auto-applied — it's researched + PROPOSED and a
// human decides. This is the durable, deduplicated, human-gated queue those
// proposals land in (and where graduated learnings propose hard guards). It only
// PROPOSES — it never mutates product code. Append/upsert by id, frequency-weighted
// (count++), never auto-deleted; status transitions are an explicit human act.

import fs from 'node:fs';
import path from 'node:path';

export type ProposalStatus = 'queued' | 'approved' | 'rejected' | 'done';

export interface Proposal {
  id: string;
  source: string;        // e.g. 'autoupgrade:class-B' | 'graduate'
  kind: string;          // 'security' | 'schema' | 'architecture' | 'guard' | ...
  description: string;
  evidence: string;
  action: string;        // what a human would do (NEVER executed here)
  status: ProposalStatus;
  first_seen: string;
  last_seen: string;
  count: number;         // how many runs surfaced it (frequency-weighting, §8b)
}

export type ProposalInput = Pick<Proposal, 'id' | 'source' | 'kind' | 'description' | 'evidence' | 'action'>;

function queuePath(baseDir: string): string {
  return path.join(baseDir, 'proposals.json');
}

export function readProposals(baseDir: string): Proposal[] {
  const p = queuePath(baseDir);
  if (!fs.existsSync(p)) return [];
  try { const j = JSON.parse(fs.readFileSync(p, 'utf8')); return Array.isArray(j) ? j : []; }
  catch { return []; }
}

/**
 * Upsert a proposal by id. New → queued/count 1. Seen → count++ + refresh
 * last_seen, PRESERVING the human-set status (queued/approved/rejected/done).
 * Never deletes. Returns the resulting proposal.
 */
export function queueProposal(baseDir: string, input: ProposalInput, ts: string): Proposal {
  fs.mkdirSync(baseDir, { recursive: true });
  const all = readProposals(baseDir);
  const idx = all.findIndex((x) => x.id === input.id);
  let result: Proposal;
  if (idx >= 0) {
    const prev = all[idx]!;
    result = { ...prev, ...input, status: prev.status, first_seen: prev.first_seen, last_seen: ts, count: prev.count + 1 };
    all[idx] = result;
  } else {
    result = { ...input, status: 'queued', first_seen: ts, last_seen: ts, count: 1 };
    all.push(result);
  }
  fs.writeFileSync(queuePath(baseDir), JSON.stringify(all, null, 2) + '\n');
  return result;
}

/** Open (not yet decided) proposals — what a human still needs to review. */
export function openProposals(baseDir: string): Proposal[] {
  return readProposals(baseDir).filter((p) => p.status === 'queued');
}
