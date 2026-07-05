# Cart Architecture

- **Storage**: `localStorage`, key: `dowiz:cart:<locationId>`.
- **Schema version**: `v: 1`
- **Zero Cookies**: The cart relies exclusively on `localStorage`.

## Drift Protection
Cart items are synced against `menuVersion` fetched from `GET /public/locations/:slug/menu`.
If versions mismatch, checkout submit is blocked until the drift modal is resolved.
