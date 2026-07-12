import crypto from 'node:crypto';
import { z } from 'zod';

/**
 * Synthetic visual-net courier — shared identity constants (Item 3, council-hardened).
 *
 * The visual-regression net renders the live courier active-delivery view
 * (/courier/delivery/:assignmentId) against a REAL encrypted courier + shift + assignment
 * seeded by /dev/seed-visual-state, impersonatable ONLY as this one synthetic identity via
 * /dev/mock-auth (role:'courier', synthetic:true). Both seams ride the ADR-0003 dev gate
 * (server.ts onRequest → isDevRequestAuthorized: ALLOW_DEV_LOGIN==='true' AND a matching
 * x-dev-auth-secret; fails closed 404 on prod) — this module adds NO route, only constants.
 *
 * SENTINEL HASH (ethical-decisions.md constraint #3 / resolution M4 / round-3 note):
 * The synthetic courier's email_hash is a NAMESPACED NON-EMAIL sentinel — the sha256 of a
 * string that is NOT a parseable email address. No `z.string().email()` input can ever
 * produce it, so the seed's `ON CONFLICT (email_hash) DO UPDATE` provably reaches ONLY the
 * synthetic row — it can never resurrect/shadow a real courier (couriers.email_hash is the
 * UNIQUE key). The constant is the single source of truth shared by the seed (insert),
 * mock-auth (re-derive the id by SELECTing on it — NEVER echo a caller-supplied id), and the
 * owner couriers list/count (exclude the synthetic row so "N active" stays honest).
 */
export const SYNTHETIC_COURIER_EMAIL_HASH = crypto
  .createHash('sha256')
  .update('synthetic:visual-net-courier:v1')
  .digest('hex');

/** Unmistakable display name (Counsel A4) so a staging owner-walkthrough can't mistake it for a real courier. */
export const SYNTHETIC_COURIER_DISPLAY_NAME = 'Visual Net Courier';

/**
 * RFC-2606 / RFC-6761 reserved TLDs that pass `z.string().email()` vacuously (zod does not
 * validate TLD existence). Rejecting them at every real registration/auth email-parse site is
 * registration-namespace hygiene (constraint #4 / NEW-H3) — an INDEPENDENT ship-blocker from
 * the sentinel hash (they close different threats: the hash closes seed-resurrection; this
 * closes namespace collision so no real email can ever sit in the synthetic-adjacent space).
 */
const RESERVED_TLDS = ['test', 'example', 'invalid', 'localhost'] as const;

function hasReservedTld(email: string): boolean {
  const at = email.lastIndexOf('@');
  if (at < 0) return false;
  const domain = email.slice(at + 1).toLowerCase().trim();
  // bare `localhost` (no dot) or any `*.localhost`/`*.test`/… form.
  if ((RESERVED_TLDS as readonly string[]).includes(domain)) return true;
  const lastDot = domain.lastIndexOf('.');
  if (lastDot < 0) return false;
  const tld = domain.slice(lastDot + 1);
  return (RESERVED_TLDS as readonly string[]).includes(tld);
}

/**
 * Zod refinement that 400s a reserved-TLD email (`*.test`, `*.example`, `*.invalid`,
 * `*.localhost`). Apply to EVERY `z.string().email()` registration/auth/access-request parse
 * site — the proof obligation (round-3) is "no `z.string().email(` without rejectReservedTld".
 *
 * Usage: `z.string().email().refine(...rejectReservedTld)` — spread the tuple so the message is
 * attached. (zod's .refine takes (predicate, message); we export both.)
 */
export const rejectReservedTld: [(email: string) => boolean, { message: string }] = [
  (email: string) => !hasReservedTld(email),
  { message: 'Reserved TLD email addresses (.test/.example/.invalid/.localhost) are not allowed' },
];

/** Convenience: a ready-made email schema with the reserved-TLD reject already applied. */
export function emailWithReservedTldReject(): z.ZodEffects<z.ZodString, string, string> {
  return z.string().email().refine(rejectReservedTld[0], rejectReservedTld[1]);
}
