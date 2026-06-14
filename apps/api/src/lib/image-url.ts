export function getImageUrl(
  imageKey: string | null | undefined,
  baseUrl?: string,
): string | null {
  if (!imageKey) return null;
  if (imageKey.startsWith('http://') || imageKey.startsWith('https://') || imageKey.startsWith('data:')) {
    return imageKey;
  }
  const cleanKey = imageKey.startsWith('/') ? imageKey.slice(1) : imageKey;
  const r2PublicUrl = process.env.R2_PUBLIC_URL;
  if (r2PublicUrl) {
    const joined = r2PublicUrl.endsWith('/') ? r2PublicUrl : r2PublicUrl + '/';
    return joined + cleanKey;
  }
  const r2Endpoint = process.env.R2_ENDPOINT;
  const r2Bucket = process.env.R2_BUCKET;
  if (r2Endpoint && r2Bucket) {
    const joined = r2Endpoint.endsWith('/') ? r2Endpoint : r2Endpoint + '/';
    return joined + r2Bucket + '/' + cleanKey;
  }
  const appBase = baseUrl || process.env.APP_BASE_URL || 'https://dowiz.fly.dev';
  const base = appBase.endsWith('/') ? appBase.slice(0, -1) : appBase;
  return base + '/images/' + cleanKey;
}