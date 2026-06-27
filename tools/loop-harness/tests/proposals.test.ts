import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { queueProposal, readProposals, openProposals } from '../src/proposals.js';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'prop-'));
const input = (id = 'p1') => ({ id, source: 'autoupgrade:class-B', kind: 'security', description: 'd', evidence: 'e', action: 'a' });

test('queueProposal — new proposal is queued, count 1', () => {
  const dir = tmp();
  const p = queueProposal(dir, input(), 'T0');
  assert.equal(p.status, 'queued');
  assert.equal(p.count, 1);
  assert.equal(p.first_seen, 'T0');
  assert.equal(readProposals(dir).length, 1);
});

test('queueProposal — re-seen proposal bumps count + last_seen, keeps first_seen', () => {
  const dir = tmp();
  queueProposal(dir, input(), 'T0');
  const p2 = queueProposal(dir, input(), 'T1');
  assert.equal(p2.count, 2);
  assert.equal(p2.first_seen, 'T0');
  assert.equal(p2.last_seen, 'T1');
  assert.equal(readProposals(dir).length, 1, 'deduped by id');
});

test('queueProposal — preserves a human-set status (never silently re-opens)', () => {
  const dir = tmp();
  queueProposal(dir, input(), 'T0');
  // human approves it (edits the queue)
  const all = readProposals(dir);
  all[0]!.status = 'approved';
  fs.writeFileSync(path.join(dir, 'proposals.json'), JSON.stringify(all, null, 2));
  // loop surfaces it again
  const p = queueProposal(dir, input(), 'T2');
  assert.equal(p.status, 'approved', 'human decision is not overwritten');
  assert.equal(p.count, 2);
});

test('openProposals — only queued (undecided) ones', () => {
  const dir = tmp();
  queueProposal(dir, input('a'), 'T0');
  queueProposal(dir, input('b'), 'T0');
  const all = readProposals(dir);
  all[0]!.status = 'done';
  fs.writeFileSync(path.join(dir, 'proposals.json'), JSON.stringify(all, null, 2));
  assert.deepEqual(openProposals(dir).map((p) => p.id), ['b']);
});

test('readProposals — absent file → []', () => {
  assert.deepEqual(readProposals(tmp()), []);
});
