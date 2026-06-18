# Image upload path — investigation result + durable-storage fix

## Diagnosis (2026-06-18)
The admin image-upload pipeline is **correctly coded** end-to-end:
1. Admin `MenuManagerPage` → `POST /api/owner/menu/products/:id/image` (multipart).
2. `spa-proxy.ts:134` → sharp resize to 800px webp (q82) → `storage.put(key)` → `UPDATE products SET image_key, image_url`.
3. `GET /images/*` (`spa-proxy.ts:118`) → `storage.get(key)` → serves `image/webp`, immutable cache.
4. Storefront `getImageUrl(image_key)` renders it.

## The real bug — ephemeral storage
- `storage` = `LocalFsStorageProvider` (`server.ts:299`), baseDir defaults to **`tmp/imports`** (relative to cwd) → `fs.writeFile` to the **fly machine's local disk**.
- **`fly.toml` has NO `[mounts]`** → the disk is ephemeral.
- ⇒ Every uploaded product image is **LOST on the next redeploy / machine restart / move**. `GET /images/*` then 404s and the card shows the fallback.
- Compounding: the seeded sushi-durres/demo products use external `image_key` URLs (`sushi-durres-menu.netlify.app/...`) that are now **dead** (return `text/html`).

## Fix landed (code, safe/non-breaking)
`server.ts:299` now reads `STORAGE_DIR`:
```ts
const storage = new LocalFsStorageProvider(process.env.STORAGE_DIR || 'tmp/imports');
```
Default unchanged; set `STORAGE_DIR` to a durable path to persist uploads.

## Durable-storage options (pick one — infra, needs approval)

### Option A — Fly volume (quick, single-machine)
```bash
fly volumes create dowiz_images --region fra --size 1   # 1 GB
fly secrets set STORAGE_DIR=/data/images                 # or [env] in fly.toml
```
Add to `fly.toml` (governance-protected — apply manually):
```toml
[mounts]
  source = "dowiz_images"
  destination = "/data"
```
Caveat: a fly volume is **per-machine + per-region** — if the app scales past 1 machine, uploads on one aren't visible on others. Fine for a single primary VM.

### Option B — Cloudflare R2 / S3 (proper, scalable — the code's TODO)
- Implement `R2StorageProvider implements StorageProvider` with `@aws-sdk/client-s3` (R2 is S3-compatible; the dep is already used in `health.ts`).
- Create an R2 bucket + token; set `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`.
- Swap the provider in `server.ts`. Durable, shared across machines, CDN-friendly.

## Then: reseed
Once durable storage is live, reseed demo/sushi-durres products with images (upload via the now-durable pipeline, or set valid `image_key`/`image_url`). The dead netlify URLs must be replaced.
