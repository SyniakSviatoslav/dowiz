import { z } from 'zod';

// Acquisition-channel attribution (QR/ATTRIBUTION build lane). The customer's storefront
// channel — how they landed on /s/:slug (QR sticker, NFC tag, Google Business Profile,
// a social link, ...) — travels as the `x-channel` request header on POST /orders (see
// routes/orders.ts) and is folded into the EXISTING orders.metadata jsonb column
// (order-persistence.ts), alongside {otp_verified, client_ip_hash}. NO new DB column, no
// migration. WRITE-ONLY: this value must NEVER be read by pricing, the order status
// state-machine, dispatch, or any authz/RLS decision — the owner dashboard's existing
// metadata passthrough is the one expected reader (analytics, out of scope to build here).
//
// Why a header and not a CreateOrderInput body field: `packages/shared-types` (where
// CreateOrderInput lives, `.strict()`-validated) is a protect-paths.sh hard-blocked zone
// for this build lane — see docs/design/order-channel-attribution/resolution.md §7. A
// header sidesteps that schema entirely (same pattern as the existing `x-otp-verified`
// header). This allowlist is mirrored in apps/web/src/lib/channel.ts (client) — the two
// copies should collapse into one `OrderChannel` export in shared-types once someone with
// access to that package makes the edit.
export const CHANNEL_ALLOWLIST = [
  'web-direct', 'qr', 'nfc', 'gbp', 'apple-maps', 'instagram', 'facebook',
  'whatsapp', 'telegram-tma', 'kiosk', 'widget', 'agent', 'other',
] as const;

export type Channel = (typeof CHANNEL_ALLOWLIST)[number];
export const DEFAULT_CHANNEL: Channel = 'web-direct';

const ChannelEnum = z.enum(CHANNEL_ALLOWLIST);
const MAX_HEADER_LEN = 32;

/**
 * Validate + normalize the raw `x-channel` header value. Never throws — a malformed
 * header must never block order creation:
 *   - missing / empty            -> 'web-direct' (direct/organic visit, no header sent)
 *   - not a string (e.g. an array from a duplicated header) -> 'other'
 *   - over max length, or not in the allowlist -> 'other'
 *   - otherwise (case-insensitive, trimmed)     -> the matched allowlist value
 */
export function normalizeChannel(raw: unknown): Channel {
  if (raw === undefined || raw === null || raw === '') return DEFAULT_CHANNEL;
  if (typeof raw !== 'string' || raw.length > MAX_HEADER_LEN) return 'other';
  const trimmed = raw.trim().toLowerCase();
  if (!trimmed) return DEFAULT_CHANNEL;
  const parsed = ChannelEnum.safeParse(trimmed);
  return parsed.success ? parsed.data : 'other';
}

/** Fastify header values are `string | string[] | undefined` (an array when the header
 *  is sent more than once). Take the first occurrence; normalizeChannel handles the rest. */
export function channelFromHeader(headerValue: string | string[] | undefined): Channel {
  return normalizeChannel(Array.isArray(headerValue) ? headerValue[0] : headerValue);
}
