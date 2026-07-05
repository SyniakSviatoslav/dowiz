// Plisio non-custodial crypto adapter (ADR-0017). Funds settle direct to the merchant wallet; stablecoin-only
// (USDT-TRC20 + USDC). DARK behind PAYMENTS_CRYPTO_ENABLED. NEEDS-HUMAN before launch: a Plisio account + the
// secret API key (SOPS, never git) + a real test invoice to validate the verify_hash serialization (see below).
import crypto from 'node:crypto';
import { serialize as phpSerialize } from '../php-serialize.js';
import type { PaymentProvider, ChargeInit, NormalizedPaymentEvent, CreateChargeInput } from './provider.js';

const PLISIO_API = 'https://plisio.net/api/v1';

// Integer-only money ↔ decimal-string (NO float — integer-minor-unit invariant). minor=1250,unit=2 → "12.50".
function minorToAmountString(minor: number, unit: number): string {
  const neg = minor < 0 ? '-' : '';
  const a = Math.abs(minor).toString().padStart(unit + 1, '0');
  return unit <= 0 ? neg + a : neg + a.slice(0, a.length - unit) + '.' + a.slice(a.length - unit);
}
function amountStringToMinor(s: string, unit: number): number {
  const neg = s.trim().startsWith('-');
  const [maj, frac = ''] = s.replace('-', '').trim().split('.');
  const fracPadded = (frac + '0'.repeat(unit)).slice(0, unit);
  const minor = parseInt(maj || '0', 10) * (10 ** unit) + (unit > 0 ? parseInt(fracPadded || '0', 10) : 0);
  return neg ? -minor : minor;
}

// Map Plisio invoice status → our normalized lifecycle. Plisio: new/pending/expired/mismatch/error/completed.
function mapStatus(s: string): NormalizedPaymentEvent['type'] {
  switch (s) {
    case 'completed': return 'completed';
    case 'mismatch': return 'mismatch';          // under/over-payment → owner-review
    case 'expired':
    case 'error':     return 'failed';
    default:          return 'pending';           // new / pending / awaiting confirmation
  }
}

export function createPlisioAdapter(opts: { secretKey: string; callbackUrl: string }): PaymentProvider {
  const { secretKey, callbackUrl } = opts;
  return {
    name: 'plisio',

    async createCharge(input: CreateChargeInput): Promise<ChargeInit> {
      // Plisio quotes the stablecoin amount from the fiat `source_amount`+`source_currency`. We let the
      // customer pick USDT/USDC on the hosted invoice (`currencies` allowlist). order_number = our payments.id.
      const url = new URL(`${PLISIO_API}/invoices/new`);
      url.search = new URLSearchParams({
        api_key: secretKey,
        order_number: input.paymentId,
        order_name: input.orderName.slice(0, 100),
        source_amount: minorToAmountString(input.amountMinor, input.minorUnit),
        source_currency: input.currencyCode,
        currencies: 'USDT,USDC',              // stablecoin-only (no volatile coins) — Counsel/ADR-0017
        callback_url: callbackUrl,
        success_callback_url: input.returnUrl,
        fail_callback_url: input.returnUrl,
        email: '',
      }).toString();
      const res = await fetch(url, { method: 'GET' });
      if (!res.ok) throw new Error(`plisio createInvoice HTTP ${res.status}`);
      const body = await res.json() as any;
      if (body?.status !== 'success' || !body?.data?.txn_id) {
        throw new Error(`plisio createInvoice failed: ${body?.data?.message || body?.status}`);
      }
      return {
        providerPaymentId: String(body.data.txn_id),
        redirectUrl: String(body.data.invoice_url),
        status: 'pending',
      };
    },

    verifyWebhook(_rawBody: Buffer, payload: Record<string, unknown>): boolean {
      // Plisio: verify_hash = hash_hmac('sha1', php_serialize(post_without_verify_hash), secret_key).
      // Fail-closed: any mismatch / missing hash → false (the route returns 401, does NOT 200 — no silent
      // swallow of a forged body). ⚠️ NEEDS-VALIDATION against a real Plisio test invoice before the flag
      // flips (PHP-serialize key order must match Plisio's $_POST order); php-serialize helper is best-effort.
      const provided = payload['verify_hash'];
      if (typeof provided !== 'string' || !provided) return false;
      const { verify_hash: _omit, ...rest } = payload as Record<string, unknown>;
      const expected = crypto.createHmac('sha1', secretKey).update(phpSerialize(rest)).digest('hex');
      try {
        return crypto.timingSafeEqual(Buffer.from(expected), Buffer.from(provided));
      } catch {
        return false; // length mismatch
      }
    },

    parseEvent(payload: Record<string, unknown>): NormalizedPaymentEvent {
      return {
        providerPaymentId: String(payload['txn_id'] ?? ''),
        type: mapStatus(String(payload['status'] ?? '')),
        // unit=2 default for the webhook event record; the route validates against payments.amount_minor
        // (which carries the location's true minor_unit). No float.
        amountMinor: payload['source_amount'] != null
          ? amountStringToMinor(String(payload['source_amount']), 2) : null,
        currencyCode: payload['source_currency'] != null ? String(payload['source_currency']) : null,
      };
    },

    async refund(): Promise<never> {
      // Crypto is irreversible + non-custodial → no programmatic refund. C2: owner-review manual
      // (payment_events('refund_due') → owner sends crypto out-of-band → 'refund_sent').
      throw new Error('UNSUPPORTED');
    },
  };
}
