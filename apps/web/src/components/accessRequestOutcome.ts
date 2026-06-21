// Pure outcome decision for the access-request form (ADR-soft-access-gate, R3-3a).
// Extracted from AccessRequestForm so it is unit-testable without React.
export type FormState = 'idle' | 'submitting' | 'success' | 'error';
export type ErrKind = 'rate' | 'generic';

/**
 * R3-3a: success requires BOTH (a) the body we actually sent carried consent === true
 * (a post-serialize self-check — "I sent real consent") AND (b) a 2xx. A no-consent send
 * (consent dropped / string-coerced / autofilled honeypot) yields `error`, NEVER a false
 * success — even on a server 200 (which stays uniform for anti-enumeration).
 */
export function decideOutcome(
  sentConsent: boolean,
  res: { ok: boolean; status: number },
): { state: FormState; err?: ErrKind } {
  if (res.ok && sentConsent) return { state: 'success' };
  if (res.status === 429) return { state: 'error', err: 'rate' };
  return { state: 'error', err: 'generic' };
}
