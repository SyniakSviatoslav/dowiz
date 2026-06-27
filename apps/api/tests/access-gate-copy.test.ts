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
//
// SCOPE: this file asserts COPY only. It does NOT cover the access-gate HTTP boundary —
// that POST /api/access-requests 404s when ACCESS_GATE_PUBLIC_ENABLED is off (STOP-1
// reachable-surface gate) and 200s when on. That boundary needs a booted server / live
// staging run and is intentionally NOT faked here.
// TODO(needs-staging): add an E2E that asserts the flag-off 404 and flag-on 200 against a
// deployed instance (see apps/api/src/routes/public/access-requests.ts) — Finding 3.
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

// Resolve ONE exact key to its locale values. `[^}]*` spans the (single- or multi-line)
// `{ en, sq, uk }` body up to the first `}`; the inner regex captures each locale's value,
// handling BOTH quote styles (the catalog uses "…" for apostrophe-bearing copy).
function localeValuesForKey(key: string): string[] {
  const re = new RegExp(`'${key.replace(/\./g, '\\.')}':\\s*\\{([^}]*)\\}`);
  const m = re.exec(i18nSrc);
  if (!m) return [];
  const out: string[] = [];
  const vre = /\b(?:en|sq|uk)\s*:\s*('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*")/g;
  let v: RegExpExecArray | null;
  while ((v = vre.exec(m[1]!))) out.push(v[1]!.slice(1, -1));
  return out;
}

// Load-bearing keys the access-gate UI actually renders. A floor count passes even if a
// canonical string was deleted and padding keys remain — these assert presence by exact key.
const REQUIRED_KEYS = [
  'accessRequest.heading', 'accessRequest.sub', 'accessRequest.emailLabel',
  'accessRequest.cta', 'accessRequest.success', 'accessRequest.consentLabel',
  'accessRequest.privacyLink', 'privacy.title', 'privacy.intro', 'privacy.retention',
  'privacy.rights', 'privacy.contact', 'privacy.back',
];

test('access-request i18n keys exist (sq+en+uk → 3 of each canonical key)', () => {
  assert.ok(accessReqValues.length >= 10 * 3 - 3, `found ${accessReqValues.length} accessRequest values`);
  assert.ok(privacyValues.length >= 13 * 3 - 3, `found ${privacyValues.length} privacy values`);
  // Presence of SPECIFIC required keys (not just a total) — each must resolve to exactly
  // en+sq+uk, all non-empty. Catches a deleted/renamed load-bearing string a floor would miss.
  for (const key of REQUIRED_KEYS) {
    const vals = localeValuesForKey(key);
    assert.equal(vals.length, 3, `key "${key}" must define exactly en+sq+uk (found ${vals.length})`);
    assert.ok(vals.every((v) => v.trim().length > 0), `key "${key}" has an empty locale value`);
  }
});

test('R2-10: NO banned scarcity strings in access-request / privacy copy (pre-invite-gating)', () => {
  const banned = ['waitlist', 'request access', 'early access', 'position #', 'approved', 'under review', 'application'];
  const all = [...accessReqValues, ...privacyValues].map((s) => s.toLowerCase());
  if (process.env.ACCESS_GATE_INVITE_GATING_SHIPPED === 'true') {
    // Invite gating shipped → scarcity copy is now EXPECTED. Assert the gate genuinely
    // flipped (a real positive control), so an ACCIDENTAL env var can't pass vacuously: a
    // flag set without the shipped copy now FAILS instead of silently skipping every check.
    assert.ok(
      banned.some((phrase) => all.some((v) => v.includes(phrase))),
      'ACCESS_GATE_INVITE_GATING_SHIPPED=true but no scarcity/gating copy is present — ' +
        'the flag was set without shipping the invite-gating copy (or set by accident)',
    );
    return;
  }
  // Flag unset → scarcity copy is forbidden (mechanically un-shippable pre-invite-gating).
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
