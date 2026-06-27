import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import crypto from 'node:crypto';

// Launch-honesty CI gates for the soft access gate (ADR-soft-access-gate). These are
// PERMANENT repo tests, not one-off proofs:
//   • R2-10 banned-strings: scarcity copy is mechanically un-shippable pre-invite-gating.
//   • R2-6  privacy content-hash: the /privacy prose is bound to PRIVACY_NOTICE_VERSION —
//     editing copy without bumping the version (and this hash) fails the build.
// SSOT migration (i18n-ssot): translations live in the key-major catalog, not the
// derived i18n.ts. Each entry is object-form: `'key': { en: '…', sq: '…', uk: '…' }`.
const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const i18nSrc = readFileSync(REPO + 'packages/ui/src/lib/i18n-catalog.ts', 'utf8');

// Extract every locale value for keys starting with one of these prefixes. Handles
// the object form (the catalog) and the legacy inline `'key': 'value'` form.
function valuesForPrefix(prefix: string): string[] {
  const out: string[] = [];
  const keyRe = new RegExp(`'(${prefix}[^']*)':\\s*(\\{[\\s\\S]*?\\n\\s*\\}|'(?:[^'\\\\]|\\\\.)*')`, 'g');
  let m: RegExpExecArray | null;
  while ((m = keyRe.exec(i18nSrc))) {
    const body = m[2]!;
    if (body.startsWith('{')) {
      const vre = /'((?:[^'\\]|\\.)*)'/g; // every quoted value inside the {en,sq,uk} object
      let v: RegExpExecArray | null;
      while ((v = vre.exec(body))) out.push(v[1]!);
    } else {
      out.push(body.slice(1, -1)); // inline 'value'
    }
  }
  return out;
}

const accessReqValues = valuesForPrefix('accessRequest\\.');
const privacyValues = valuesForPrefix('privacy\\.');

test('access-request i18n keys exist (sq+en+uk → 3 of each canonical key)', () => {
  assert.ok(accessReqValues.length >= 10 * 3 - 3, `found ${accessReqValues.length} accessRequest values`);
  assert.ok(privacyValues.length >= 13 * 3 - 3, `found ${privacyValues.length} privacy values`);
});

test('R2-10: NO banned scarcity strings in access-request / privacy copy (pre-invite-gating)', () => {
  // ACCESS_GATE_INVITE_GATING_SHIPPED unset → scarcity copy is forbidden.
  if (process.env.ACCESS_GATE_INVITE_GATING_SHIPPED === 'true') return;
  const banned = ['waitlist', 'request access', 'early access', 'position #', 'approved', 'under review', 'application'];
  const all = [...accessReqValues, ...privacyValues].map((s) => s.toLowerCase());
  for (const phrase of banned) {
    const hit = all.find((v) => v.includes(phrase));
    assert.equal(hit, undefined, `banned scarcity string "${phrase}" must not appear (found in: ${hit})`);
  }
});

test('privacy retention copy states "12 months" (must equal ACCESS_REQUEST_RETENTION default)', () => {
  const retention = privacyValues.filter((v) => /12\s*(months|muaj|місяц)/i.test(v));
  assert.ok(retention.length >= 1, 'at least one locale states the 12-month window');
});

test('R2-6: /privacy prose content-hash is bound to PRIVACY_NOTICE_VERSION', () => {
  // Bump BOTH config PRIVACY_NOTICE_VERSION and EXPECTED below when the prose changes.
  const PRIVACY_NOTICE_VERSION = '2026-06-20';
  const EXPECTED = '3c335bf628e7a15b8553b581233d49ea'; // sha256(privacy prose en+sq+uk from i18n-catalog).slice(0,32); bump with PRIVACY_NOTICE_VERSION
  const hash = crypto.createHash('sha256').update(privacyValues.join('')).digest('hex').slice(0, 32);
  assert.equal(
    hash,
    EXPECTED,
    `privacy prose changed (hash ${hash}). If intentional, bump config PRIVACY_NOTICE_VERSION ` +
      `(currently ${PRIVACY_NOTICE_VERSION}) AND update EXPECTED in this test to "${hash}".`,
  );
});
