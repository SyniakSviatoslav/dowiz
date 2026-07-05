/**
 * Channel attribution (QR/ATTRIBUTION build lane). Captures the acquisition channel from
 * `?ch=<value>` on `/s/:slug`, persists it per-slug for the session (sessionStorage — a
 * fresh tab/session re-attributes; scoped per slug so browsing restaurant A via a QR code,
 * then restaurant B in the same tab, never lets A's channel bleed into B's order), and
 * carries it write-only into order creation via the `x-channel` request header — NOT the
 * CreateOrderInput body (see apps/api/src/lib/channel.ts for why: `packages/shared-types`,
 * the natural home for a shared `OrderChannel` export, is a protect-paths.sh hard-blocked
 * zone for this build lane; docs/design/order-channel-attribution/resolution.md §7).
 *
 * NEVER read for pricing/status/dispatch — purely descriptive metadata.
 *
 * This allowlist is a second copy of apps/api/src/lib/channel.ts's CHANNEL_ALLOWLIST.
 * Collapse the two into one `OrderChannel` export in shared-types once someone with
 * access to that package makes the edit.
 */
export const CHANNEL_ALLOWLIST = [
  'web-direct',
  'qr',
  'nfc',
  'gbp',
  'apple-maps',
  'instagram',
  'facebook',
  'whatsapp',
  'telegram-tma',
  'kiosk',
  'widget',
  'agent',
  'other',
] as const;

export type Channel = (typeof CHANNEL_ALLOWLIST)[number];
export const DEFAULT_CHANNEL: Channel = 'web-direct';

const storageKey = (slug: string) => `dos_channel:${slug}`;

/**
 * Pure: validate a raw string against the allowlist (case-insensitive, trimmed).
 * Empty/missing -> 'web-direct' (direct/organic visit, no channel tag at all);
 * anything non-empty but unrecognized -> 'other'.
 */
export function normalizeChannel(raw: string | null | undefined): Channel {
  if (!raw) return DEFAULT_CHANNEL;
  const lower = raw.trim().toLowerCase();
  if (!lower) return DEFAULT_CHANNEL;
  return (CHANNEL_ALLOWLIST as readonly string[]).includes(lower) ? (lower as Channel) : 'other';
}

/**
 * Pure: what to persist from a `location.search` string. `null` when there's nothing to
 * capture (no `?ch=` on this navigation) — the caller then leaves any prior capture alone
 * instead of overwriting it with a default.
 */
export function resolveCapturedChannel(search: string): Channel | null {
  const raw = new URLSearchParams(search).get('ch');
  return raw ? normalizeChannel(raw) : null;
}

/**
 * Side-effecting: call once on mount of the `/s/:slug` shell (ClientLayout) with the
 * current slug + `location.search`. Best-effort — storage unavailable (private mode /
 * sandboxed iframe) silently no-ops, matching the rest of the app's guarded-storage
 * convention (see apiClient.ts's `try { sessionStorage... } catch { /* private mode *\/ }`).
 */
export function captureChannel(slug: string, search: string): void {
  const resolved = resolveCapturedChannel(search);
  if (!resolved) return;
  try {
    sessionStorage.setItem(storageKey(slug), resolved);
  } catch {
    /* storage unavailable — best-effort only, never blocks the page */
  }
}

/**
 * Side-effecting: the channel to send with order creation for this slug. Defaults to
 * 'web-direct' when nothing was captured this session (direct/organic visit) or storage
 * is unavailable.
 */
export function getOrderChannel(slug: string): Channel {
  try {
    return normalizeChannel(sessionStorage.getItem(storageKey(slug)));
  } catch {
    return DEFAULT_CHANNEL;
  }
}
