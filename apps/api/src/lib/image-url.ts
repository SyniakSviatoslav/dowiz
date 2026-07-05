export function getImageUrl(
  imageKey: string | null | undefined,
  baseUrl?: string,
): string | null {
  if (!imageKey) return null;
  if (imageKey.startsWith('http://') || imageKey.startsWith('https://') || imageKey.startsWith('data:')) {
    return imageKey;
  }
  const cleanKey = imageKey.startsWith('/') ? imageKey.slice(1) : imageKey;
  // A public R2 bucket / CDN custom domain (r2.dev or your own) serves images
  // directly. The S3 API endpoint (R2_ENDPOINT) is NOT this — it's private and
  // requires SigV4 signing, so we never build a browser URL from it. With a
  // private bucket (no R2_PUBLIC_URL), images are served through the app's
  // /images/* proxy, which reads from R2 with the server's credentials.
  const r2PublicUrl = process.env.R2_PUBLIC_URL;
  if (r2PublicUrl) {
    const joined = r2PublicUrl.endsWith('/') ? r2PublicUrl : r2PublicUrl + '/';
    return joined + cleanKey;
  }
  const appBase = baseUrl || process.env.APP_BASE_URL || 'https://dowiz.fly.dev';
  const base = appBase.endsWith('/') ? appBase.slice(0, -1) : appBase;
  return base + '/images/' + cleanKey;
}