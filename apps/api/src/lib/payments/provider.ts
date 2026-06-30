// Crypto payments — provider-agnostic port (ADR-0017). The seam the council approved: the order/webhook code
// talks ONLY to this interface; provider vocabulary never leaks past parseEvent/verifyWebhook. `PAYMENTS_PROVIDER`
// selects the adapter. DARK behind PAYMENTS_PREPAID_ENABLED + PAYMENTS_CRYPTO_ENABLED.

export interface ChargeInit {
  /** Provider's payment/invoice id (Plisio txn_id) — stored as payments.provider_payment_id. */
  providerPaymentId: string;
  /** Where to send the customer to pay (Plisio hosted invoice URL). */
  redirectUrl: string;
  /** Initial status; crypto invoices start 'pending' (awaiting on-chain confirmation). */
  status: 'pending';
}

export interface NormalizedPaymentEvent {
  providerPaymentId: string;
  /** Normalized lifecycle type — provider statuses map onto these. */
  type: 'pending' | 'completed' | 'failed' | 'refunded' | 'mismatch';
  amountMinor: number | null;
  currencyCode: string | null;
}

export interface CreateChargeInput {
  /** Our payments.id — passed to the provider as order_number; comes back on the webhook. */
  paymentId: string;
  amountMinor: number;
  currencyCode: string; // location currency; the provider quotes the stablecoin equivalent
  minorUnit: number;    // locations.currency_minor_unit — integer↔decimal conversion has NO float (ADR-0005)
  idempotencyKey: string;
  returnUrl: string;
  orderName: string;
}

export interface PaymentProvider {
  readonly name: string;
  /** Create a hosted invoice; returns where to send the customer + the provider ref. */
  createCharge(input: CreateChargeInput): Promise<ChargeInit>;
  /** HMAC-verify a raw webhook body (signature, NOT secret-equality). */
  verifyWebhook(rawBody: Buffer, payload: Record<string, unknown>): boolean;
  /** Normalize a verified webhook body into our event shape. */
  parseEvent(payload: Record<string, unknown>): NormalizedPaymentEvent;
  /**
   * Refund. Crypto is IRREVERSIBLE (non-custodial) → UNSUPPORTED: refunds are owner-review manual
   * (payment_events('refund_due') → owner sends crypto back out-of-band → 'refund_sent'). C2 resolution.
   */
  refund(): Promise<never>;
}

/** Cash = no-op provider (COD): no charge, payment_status stays 'unpaid' (the cash-as-proof spine). */
export const cashProvider: PaymentProvider = {
  name: 'cash',
  async createCharge(): Promise<ChargeInit> {
    throw new Error('cash payment has no charge — COD uses the cash-as-proof spine');
  },
  verifyWebhook() { return false; },
  parseEvent() { throw new Error('cash has no webhook'); },
  async refund(): Promise<never> { throw new Error('UNSUPPORTED'); },
};
