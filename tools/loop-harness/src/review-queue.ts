// §2 Human-Review Queue — the SINK for every "→ human" branch (autoupgrade Class B,
// builder Class B, graduation proposals, security-carve-out edits). The bridge
// between the autonomous machine and the one human. Built on the proposals queue;
// adds the DECISION layer + the feedback loop that makes rejection teach.
//
// - accept → status 'approved' (ready to apply through the standard oracle + graduate).
// - reject → status 'rejected' = a NEGATIVE LEARNING: isRejected() lets the source
//   loops STOP re-proposing it (reject must teach, or you get the same proposal forever).
// - defer → stays queued.
// Decided items append to a PERMANENT decisions log (never cleaned); pending stays small.

import fs from 'node:fs';
import path from 'node:path';
import { readProposals, writeProposals, type Proposal } from './proposals.js';

export type Decision = 'accept' | 'reject' | 'defer';

export interface DecisionRecord { ts: string; id: string; decision: Decision; rationale: string; prior_status: string }

function decisionsPath(baseDir: string): string {
  return path.join(baseDir, 'decisions.jsonl');
}

const STATUS_OF: Record<Decision, Proposal['status']> = { accept: 'approved', reject: 'rejected', defer: 'queued' };

/** Decide on a queued proposal. Updates its status + appends to the permanent log. */
export function decide(baseDir: string, id: string, decision: Decision, rationale: string, ts: string): Proposal | null {
  const all = readProposals(baseDir);
  const p = all.find((x) => x.id === id);
  if (!p) return null;
  const prior = p.status;
  p.status = STATUS_OF[decision];
  writeProposals(baseDir, all);
  fs.mkdirSync(baseDir, { recursive: true });
  fs.appendFileSync(decisionsPath(baseDir), JSON.stringify({ ts, id, decision, rationale, prior_status: prior } satisfies DecisionRecord) + '\n');
  return p;
}

/** Pending (undecided) items — what the human still needs to review. */
export function listReview(baseDir: string): Proposal[] {
  return readProposals(baseDir).filter((p) => p.status === 'queued');
}

/** The permanent decisions log (accept/reject + rationale) — never cleaned. */
export function decisionsLog(baseDir: string): DecisionRecord[] {
  const p = decisionsPath(baseDir);
  if (!fs.existsSync(p)) return [];
  return fs.readFileSync(p, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l) as DecisionRecord);
}

/** Negative learning — a proposal a human rejected. Source loops call this to STOP
 *  re-proposing the same thing (the feedback that prevents groundhog-day proposals). */
export function isRejected(baseDir: string, id: string): boolean {
  return readProposals(baseDir).some((p) => p.id === id && p.status === 'rejected');
}
