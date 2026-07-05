import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

// ── B3 P0-5 · W2 — argon2 params LOCK (guardrail, not a behavior change) ──
//
// The password-hash posture is already OWASP-solid: argon2id, memoryCost 65536 (64 MiB),
// timeCost 3, parallelism 4. The RISK is a SILENT weakening — someone edits one of the
// inline `hashOptions` blocks down to a cheaper param during an unrelated change, and the
// whole credential store gets easier to crack with no test firing. This locks the floor.
//
// SCOPE: only the PASSWORD-hashing sites. OTP (`lib/otp.ts`) deliberately uses a lighter
// 19456/2/1 profile (short-lived numeric code, not a password) and is intentionally EXCLUDED
// — locking it here would be mis-scoped and flag legitimate code.
//
// This is a SOURCE-assertion lock (the params are inline literals, not an exported constant;
// extracting them to a shared const would be a runtime refactor, out of scope for a lock).

const __dirname = dirname(fileURLToPath(import.meta.url));
const SRC = resolve(__dirname, '../src');

// Every file that hashes a real PASSWORD (or a password-equivalent long-lived credential).
const PASSWORD_HASH_SITES = [
  'routes/courier/auth.ts',
  'routes/courier/me.ts',
  'routes/owner/courier-invites.ts',
  'routes/dev/mock-auth.ts',
];

// OWASP Argon2id floor (the pinned posture). Assert >= so a future HARDENING never trips it,
// but any WEAKENING (a smaller number) goes red.
const FLOOR = { memoryCost: 65536, timeCost: 3, parallelism: 4 };

function readSite(rel: string): string {
  return readFileSync(resolve(SRC, rel), 'utf8');
}

test('argon2 password-hash params are pinned at the OWASP floor (W2 lock)', async (t) => {
  for (const rel of PASSWORD_HASH_SITES) {
    await t.test(rel, () => {
      const src = readSite(rel);

      // 1. The algorithm must be argon2id (never argon2i / argon2d).
      assert.match(
        src,
        /type:\s*argon2\.argon2id/,
        `${rel}: password hashing must use argon2id`,
      );
      assert.doesNotMatch(
        src,
        /type:\s*argon2\.argon2(?:i|d)\b/,
        `${rel}: must not use the weaker argon2i / argon2d variant`,
      );

      // 2. Every numeric param in this file must clear the floor. These files hash only
      //    passwords/credentials (no OTP), so EVERY memoryCost/timeCost/parallelism here
      //    is a password-strength param — assert the whole set.
      for (const [param, floor] of Object.entries(FLOOR)) {
        const re = new RegExp(`${param}:\\s*(\\d+)`, 'g');
        const found = [...src.matchAll(re)].map((m) => Number(m[1]));
        assert.ok(found.length > 0, `${rel}: expected at least one ${param} literal`);
        for (const v of found) {
          assert.ok(
            v >= floor,
            `${rel}: ${param}=${v} is below the OWASP floor ${floor} — password hashing weakened`,
          );
        }
      }
    });
  }
});
