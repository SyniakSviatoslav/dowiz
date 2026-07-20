import type { NotifyResult } from '../provider.js';

/**
 * EmailAdapter — a thin, self-contained SYSTEM-channel email sender (B6 FIX,
 * ADR-soft-access-gate). It deliberately does NOT implement the tenant
 * `NotificationProvider.notify(target, event, data)` signature: a platform-level
 * ops alert has no `locationId`, and the tenant dispatcher's value (per-tenant
 * dedup / prefs / quiet-hours / `notification_outbox_audit` writes, which require
 * `location_id NOT NULL`) does not apply. So `access-request.notify` calls
 * `sendOps(...)` directly. Observability for this channel lives in the
 * `access_requests` row (`notified_at`) + the §8 counters, not the tenant audit table.
 *
 * Resend REST API (https://resend.com/docs/api-reference/emails/send-email),
 * AbortSignal.timeout(5000) mirroring TelegramAdapter. Gated on RESEND_API_KEY:
 * absent → `{ delivered:false, reason:'email-disabled' }` (submits still persist;
 * the reconcile sweep + bulk `status='new'` view are the fallback).
 */
export interface SendOpsInput {
  to: string;
  subject: string;
  text: string;
  html?: string;
  from?: string;
}

export class EmailAdapter {
  readonly id = 'email-ops';
  private apiKey: string | undefined;
  private apiBase: string;
  private defaultFrom: string;

  constructor(apiKey: string | undefined, opts?: { apiBase?: string; from?: string }) {
    this.apiKey = apiKey;
    this.apiBase = opts?.apiBase ?? 'https://api.resend.com';
    // Resend requires a verified sender. onboarding@resend.dev works for any account
    // until a domain is verified; override via `from` once a real sender exists.
    this.defaultFrom = opts?.from ?? 'dowiz <onboarding@resend.dev>';
  }

  async sendOps(input: SendOpsInput): Promise<NotifyResult> {
    if (!this.apiKey) {
      return { delivered: false, reason: 'email-disabled' };
    }

    try {
      const response = await fetch(`${this.apiBase}/emails`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.apiKey}`,
        },
        body: JSON.stringify({
          from: input.from ?? this.defaultFrom,
          to: input.to,
          subject: input.subject,
          text: input.text,
          ...(input.html ? { html: input.html } : {}),
        }),
        signal: AbortSignal.timeout(5000),
      });

      if (response.ok) {
        const body = await response.json().catch(() => ({}));
        return { delivered: true, providerMessageId: body?.id };
      }

      // A present-but-invalid key (401/403) is an operator config error — surface the
      // distinct reason so the boot-time one-shot alert can fire (B7).
      if (response.status === 401 || response.status === 403) {
        return { delivered: false, reason: `AUTH:${response.status}` };
      }

      if (response.status === 429) {
        const retryAfter = response.headers.get('retry-after');
        return {
          delivered: false,
          reason: 'RATE_LIMIT',
          retryAfter: retryAfter ? parseInt(retryAfter, 10) * 1000 : 5000,
        };
      }

      return { delivered: false, reason: `HTTP_${response.status}` };
    } catch (err: any) {
      // AbortSignal.timeout → AbortError; network failure → TypeError. Either way:
      // not delivered, let the caller decide to retry (it does — pg-boss backoff).
      return { delivered: false, reason: err?.name === 'AbortError' ? 'TIMEOUT' : (err?.message || 'NETWORK_ERROR') };
    }
  }
}
