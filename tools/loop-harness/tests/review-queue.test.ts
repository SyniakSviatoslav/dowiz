import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { queueProposal } from '../src/proposals.js';
import { decide, listReview, decisionsLog, isRejected } from '../src/review-queue.js';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'rq-'));
const seed = (dir: string, id: string) => queueProposal(dir, { id, source: 's', kind: 'security', description: 'd', evidence: 'e', action: 'a' }, 'T0');

test('decide accept → approved + permanent decisions log entry', () => {
  const d = tmp();
  seed(d, 'p1');
  const p = decide(d, 'p1', 'accept', 'looks good', 'T1');
  assert.equal(p!.status, 'approved');
  const log = decisionsLog(d);
  assert.equal(log.length, 1);
  assert.equal(log[0]!.decision, 'accept');
  assert.equal(log[0]!.prior_status, 'queued');
});

test('decide reject → rejected = negative learning (isRejected true)', () => {
  const d = tmp();
  seed(d, 'p2');
  decide(d, 'p2', 'reject', 'wrong approach', 'T1');
  assert.equal(isRejected(d, 'p2'), true);
  // re-queueing keeps it rejected (status preserved) → it stays out of the pending list
  queueProposal(d, { id: 'p2', source: 's', kind: 'security', description: 'd', evidence: 'e', action: 'a' }, 'T2');
  assert.equal(isRejected(d, 'p2'), true);
});

test('listReview — only undecided (queued) items', () => {
  const d = tmp();
  seed(d, 'a'); seed(d, 'b'); seed(d, 'c');
  decide(d, 'a', 'accept', '', 'T1');
  decide(d, 'b', 'reject', '', 'T1');
  assert.deepEqual(listReview(d).map((p) => p.id), ['c']);
});

test('decide — defer keeps it queued', () => {
  const d = tmp();
  seed(d, 'p');
  decide(d, 'p', 'defer', 'later', 'T1');
  assert.deepEqual(listReview(d).map((p) => p.id), ['p']);
});

test('decide — unknown id → null', () => {
  assert.equal(decide(tmp(), 'nope', 'accept', '', 'T1'), null);
});
