# Product media (owner photo upload) — operator enablement

The owner photo-upload UI (`apps/web/src/components/admin/MediaManager.tsx`) and its server endpoints
(`apps/api/src/routes/owner/product-media.ts`: presign/confirm/set-primary/reorder/available) are
**fully built**. Going live needs three operator/infra steps — none are code work.

## 1. Object storage (R2) — REQUIRED (presign returns 503 without it)
The presign route mints S3 presigned PUT URLs; staging currently has **no R2 secrets**. Set on staging
(values from the R2/S3 bucket dedicated to product media):
```
flyctl secrets set -a dowiz-staging \
  R2_BUCKET=… R2_ENDPOINT=… R2_ACCESS_KEY_ID=… R2_SECRET_ACCESS_KEY=… R2_PUBLIC_URL=…
```
`apps/api/src/server.ts:298` instantiates the R2 provider when `R2_BUCKET`+`R2_ENDPOINT` are present.

## 2. Client visibility flag — REQUIRED (the UI is hidden without it)
`MenuManagerPage` gates the MediaManager on the **build-time** `VITE_MEDIA_RICH_ENABLED` (`MenuManagerPage.tsx:29`).
It is NOT a Dockerfile build-arg yet (`Dockerfile` is protect-paths). Add, mirroring the existing
`VITE_ACCESS_GATE_PUBLIC_ENABLED` pattern (Dockerfile:20-21):
```dockerfile
ARG VITE_MEDIA_RICH_ENABLED=false
ENV VITE_MEDIA_RICH_ENABLED=$VITE_MEDIA_RICH_ENABLED
```
Then build/deploy staging with `--build-arg VITE_MEDIA_RICH_ENABLED=true`.

## 3. Server flag + tenant tier — already done on staging
- `MEDIA_RICH_ENABLED=true` — set on staging 2026-06-29 (the storefront card-image resolution + the lazy
  /media endpoint both gate on it).
- The serving gate also requires `locations.plan='business'` (`mediaServingAllowed`). The demo is already
  `business`; set it per tenant who buys the tier.

## After enablement — verify end-to-end (the proof that was infra-blocked)
1. As owner (test@dowiz.com), open a product in the menu manager → the **Media gallery** section shows.
2. Upload a webp/jpeg → presign → PUT → confirm → it appears; **Set primary**.
3. The public storefront card for that product shows the photo (the card-image resolution shipped in
   `5893f355`). E2E: extend `e2e/tests/flow-ui-images.spec.ts`.

## Bundle: lazy-load the 1MB map (token/perf — scoped follow-up)
`maplibre` already builds into its own `map-*.js` chunk (~1MB), but `OrderStatusPage` (`CourierLiveMap`)
and `CheckoutPage` (`MapWithPin`) import it STATICALLY from the `@deliveryos/ui` barrel, so the route
chunk has a static edge → the 1MB loads on every delivery order-status view (even PENDING, no courier yet).
Fix is apps/web-only (no protected change needed since `packages/ui` has no `exports` field):
- Add `packages/ui/src/maps.ts` re-exporting `{ MapLibreBase, MapWithPin, CourierLiveMap }`, build it.
- In OrderStatusPage/CheckoutPage: `const CourierLiveMap = lazy(() => import('@deliveryos/ui/dist/maps.js').then(m => ({ default: m.CourierLiveMap })))` + wrap in `<Suspense fallback={<SkeletonBase className="h-64 w-full"/>}>`.
- **Verify with a build+measure cycle**: `pnpm --filter @deliveryos/web build` then confirm the route chunk
  no longer statically references `map-*.js` (the 1MB moves to an async chunk fetched on map mount).
  Gate the OrderStatus map render on an active-delivery state so PENDING views skip the fetch entirely.
Deferred from this session because it needs the build-measure loop to prove the edge actually moved
(perf change, not correctness — don't ship blind).

## Recommended polish (code, when enablement lands)
- Add client-side resize/transcode-to-webp in `MediaManager.handleUpload` (native `createImageBitmap` +
  `OffscreenCanvas` → webp blob) before presign: caps upload size, enforces webp, protects the 150 MB
  per-location budget, and cuts storage/bandwidth (token-efficiency goal). Deferred until R2 is up so it
  can be verified end-to-end rather than shipped blind.
