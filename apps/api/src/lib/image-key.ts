// Pure image-key validator extracted from spa-proxy.ts. Storefront/brand image
// fields must hold an object-storage KEY, never an inline data:/blob: URL — a
// data URL would bloat the row and bypass the upload pipeline (size/type checks,
// R2 placement). Rejecting it forces clients through the image upload endpoint.
// null/undefined pass through (cleared field); anything else is coerced to string.

export function validateImageKey(val: unknown): string | null | undefined {
  if (val === undefined || val === null) return val;
  const s = String(val);
  if (s.startsWith('data:') || s.startsWith('blob:')) {
    throw new Error('Image must be uploaded via the image upload endpoint, not sent as a data URL');
  }
  return s;
}
