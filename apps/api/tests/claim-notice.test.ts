import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildArt14Notice } from '../src/modules/acquisition/claim.js';

// P6 CLAIM PHASE (counsel CC1/CC2) — the first-contact notice must be honest Art-14, written for the
// hostile recipient, with an equally-prominent one-click decline+erase that needs NO account.
test('Art-14 notice: honest, names the source, offers claim AND decline, not a growth-hack CTA', () => {
  const n = buildArt14Notice({
    previewUrl: 'https://x/claim?preview=1',
    claimUrl: 'https://x/claim?token=t',
    declineUrl: 'https://x/claim/decline?token=t',
  });
  assert.match(n.body, /did not ask/i, 'acknowledges it was unsolicited');
  assert.match(n.body, /public website|Places/i, 'names the data SOURCE (Art-14)');
  assert.match(n.body, /NOT a live store|cannot take orders/i, 'honest: not a live store');
  assert.match(n.body, /erase|delete/i, 'erasure right present');
  assert.match(n.body, /data protection|authority/i, 'right to complain to a supervisory authority');
  // decline is present, one-click, and explicitly no-account
  assert.match(n.body, /claim\/decline\?token=t/, 'decline link present');
  assert.match(n.body, /no account needed/i, 'decline requires no registration (CC2)');
  // NOT a growth-hack subject
  assert.doesNotMatch(n.subject, /claim your free store|grow your business/i, 'subject is not a growth-hack CTA');
});
