/**
 * Pure validation helpers for the cinematic product-media upload flow
 * (ADR-0002 product-media seam). Kept dependency-free + side-effect-free so the
 * security-critical logic — magic-byte sniffing, per-location storage budget,
 * spin frame-count bounds, mime allow-list — is unit-testable without R2, a DB,
 * or a running server (R2 is absent on staging — Phase-2 contract §Upload).
 */

export type ProductMediaKind = 'image' | 'video' | 'spin' | 'model';

// Mime allow-list. Poster rasters share the image set; SVG is never allowed
// (active content / XSS vector) — enforced by omission here.
export const IMAGE_MIMES = ['image/webp', 'image/jpeg'] as const;
export const VIDEO_MIMES = ['video/mp4'] as const;
export const ALLOWED_MIMES = [...IMAGE_MIMES, ...VIDEO_MIMES] as const;

export type AllowedMime = (typeof ALLOWED_MIMES)[number];

// Per-file size ceilings (bytes). Images are small; a clip is the heavy case.
export const MAX_IMAGE_BYTES = 8 * 1024 * 1024; // 8 MB
export const MAX_VIDEO_BYTES = 25 * 1024 * 1024; // 25 MB

// Per-location storage budget. SUM(existing) + incoming must stay under this.
export const LOCATION_BUDGET_BYTES = 150 * 1024 * 1024; // 150 MB

// Spin frame-count bounds (inclusive). Below 12 isn't a believable spin; above
// 72 is wasteful decode/bandwidth for a 360°.
export const SPIN_MIN_FRAMES = 12;
export const SPIN_MAX_FRAMES = 72;

/** True when the mime is in the upload allow-list (never SVG). */
export function isAllowedMime(mime: string): mime is AllowedMime {
  return (ALLOWED_MIMES as readonly string[]).includes(mime);
}

/** Poster images are raster-only — webp/jpeg, NEVER svg. */
export function isAllowedPosterMime(mime: string): boolean {
  return (IMAGE_MIMES as readonly string[]).includes(mime);
}

/** File extension for a content-addressed key, per allowed mime. */
export function extForMime(mime: string): string | null {
  switch (mime) {
    case 'image/webp':
      return 'webp';
    case 'image/jpeg':
      return 'jpg';
    case 'video/mp4':
      return 'mp4';
    default:
      return null;
  }
}

export function maxBytesForMime(mime: string): number {
  return (VIDEO_MIMES as readonly string[]).includes(mime) ? MAX_VIDEO_BYTES : MAX_IMAGE_BYTES;
}

/**
 * Sniff the leading bytes of a buffer and return the detected container mime,
 * or null if unrecognised. This is the server-side defence: a client can claim
 * any Content-Type, so confirm() re-checks the actual bytes before persisting.
 *
 * Recognises: WebP (RIFF....WEBP), JPEG (FF D8 FF), MP4/ISO-BMFF (....ftyp).
 * Deliberately does NOT recognise SVG (it's text/XML — no magic number — and is
 * an active-content vector) or executables, so they sniff to null → rejected.
 */
export function sniffMime(buf: Uint8Array): AllowedMime | null {
  if (!buf || buf.length < 12) return null;

  // JPEG: FF D8 FF
  if (buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) {
    return 'image/jpeg';
  }

  // WebP: "RIFF" .... "WEBP"
  if (
    buf[0] === 0x52 && buf[1] === 0x49 && buf[2] === 0x46 && buf[3] === 0x46 && // RIFF
    buf[8] === 0x57 && buf[9] === 0x45 && buf[10] === 0x42 && buf[11] === 0x50 // WEBP
  ) {
    return 'image/webp';
  }

  // MP4 / ISO-BMFF: bytes 4..7 == "ftyp"
  if (buf[4] === 0x66 && buf[5] === 0x74 && buf[6] === 0x79 && buf[7] === 0x70) {
    return 'video/mp4';
  }

  return null;
}

/** Bytes (sha256 hex etc.) → true when the sniffed type matches the claimed mime. */
export function magicBytesMatch(buf: Uint8Array, claimedMime: string): boolean {
  const sniffed = sniffMime(buf);
  return sniffed !== null && sniffed === claimedMime;
}

export interface FrameCountResult {
  ok: boolean;
  reason?: string;
}

/** Spin frame-count must be within [SPIN_MIN_FRAMES, SPIN_MAX_FRAMES]. */
export function checkFrameCount(count: number): FrameCountResult {
  if (!Number.isInteger(count)) return { ok: false, reason: 'frame count must be an integer' };
  if (count < SPIN_MIN_FRAMES) return { ok: false, reason: `spin needs at least ${SPIN_MIN_FRAMES} frames` };
  if (count > SPIN_MAX_FRAMES) return { ok: false, reason: `spin allows at most ${SPIN_MAX_FRAMES} frames` };
  return { ok: true };
}

export interface BudgetResult {
  ok: boolean;
  used: number;
  incoming: number;
  total: number;
  limit: number;
}

/**
 * Per-location storage budget check: existing usage + incoming must be ≤ limit.
 * Returns a structured result so the caller can answer 413 with the figures.
 */
export function checkBudget(
  existingBytes: number,
  incomingBytes: number,
  limit: number = LOCATION_BUDGET_BYTES,
): BudgetResult {
  const used = Math.max(0, existingBytes || 0);
  const incoming = Math.max(0, incomingBytes || 0);
  const total = used + incoming;
  return { ok: total <= limit, used, incoming, total, limit };
}

/** Sum of the `bytes` field across a list of incoming items. */
export function sumIncomingBytes(items: ReadonlyArray<{ bytes: number }>): number {
  return items.reduce((acc, it) => acc + (Number(it.bytes) || 0), 0);
}

/**
 * Server-side feature gate for the lazy media endpoint: rich media is served
 * only when the global flag is on AND the location is on the 'business' plan.
 * Pure so the gate is unit-testable independently of the DB/route. When this
 * returns false the endpoint must answer `{ media: [] }` (defence-in-depth).
 */
export function mediaServingAllowed(flagEnabled: boolean, plan: string | null | undefined): boolean {
  return flagEnabled === true && plan === 'business';
}
