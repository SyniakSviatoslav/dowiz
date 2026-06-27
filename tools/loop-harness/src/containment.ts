// §4 containment — what makes the autonomy survivable.
//
// 1. Credential isolation: the autonomous loop must run WITHOUT production secrets
//    in context, so a compromised step has nothing to exfiltrate. assertCredential-
//    Isolation refuses to run if secret-shaped env vars are present.
// 2. Trusted-source allowlist (§0): only candidates from TRUSTED, MECHANICAL
//    detectors may be auto-applied. Anything from an untrusted source (web research,
//    an LLM-written patch) is forced to propose-only — the loop never executes code
//    it found/derived from untrusted content.

// Secret-shaped env var names. Non-empty values for these = prod credentials present.
const SECRET_NAME_RE = /(SECRET|PRIVATE_KEY|_TOKEN$|^TOKEN|PASSWORD|DATABASE_URL|API_KEY|VAPID_PRIVATE|FLY_API)/i;

export interface IsolationResult { ok: boolean; present: string[] }

/** Names of secret-shaped env vars with non-empty values (excluding `allow`). */
export function findCredentials(env: Record<string, string | undefined>, allow: string[] = []): string[] {
  const skip = new Set(allow);
  return Object.keys(env)
    .filter((k) => SECRET_NAME_RE.test(k) && !skip.has(k) && (env[k] ?? '').trim() !== '')
    .sort();
}

export function checkCredentialIsolation(env: Record<string, string | undefined>, allow: string[] = []): IsolationResult {
  const present = findCredentials(env, allow);
  return { ok: present.length === 0, present };
}

/**
 * Refuse to run the autonomous loop when prod credentials are in context (§4).
 * Throws with the offending names. Pass `allow` for known-safe test stubs.
 */
export function assertCredentialIsolation(env: Record<string, string | undefined>, allow: string[] = []): void {
  const { ok, present } = checkCredentialIsolation(env, allow);
  if (!ok) {
    throw new Error(
      `CONTAINMENT (§4): autonomous loop refused — ${present.length} credential-shaped env var(s) in context: ${present.join(', ')}. ` +
      `Run credential-isolated (clean container / broker via agent-vault) so a compromised step has nothing to exfiltrate.`,
    );
  }
}

// §0 — only these (mechanical, deterministic, in-repo) detector sources may
// produce auto-applicable candidates. A candidate from any other source is
// untrusted → propose-only, never auto-applied.
export const TRUSTED_DETECTOR_SOURCES: readonly string[] = [
  'config-tune detector (operator-declared tunable)',
];

export function isTrustedSource(source: string): boolean {
  return TRUSTED_DETECTOR_SOURCES.includes(source);
}
