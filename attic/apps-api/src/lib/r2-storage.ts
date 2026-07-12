import type { StorageProvider } from '../ports.js';

/**
 * Cloudflare R2 (S3-compatible) durable object storage for product images.
 *
 * Env (same convention as the backup health check in routes/health.ts):
 *   R2_ENDPOINT, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET
 *
 * Unlike LocalFsStorageProvider (ephemeral fly disk), objects here survive
 * redeploys and are shared across machines. The S3 client is imported lazily
 * so dev/local (LocalFs) never loads the AWS SDK.
 */
export class R2StorageProvider implements StorageProvider {
  private bucket: string;
  private clientPromise: Promise<any> | null = null;

  constructor() {
    const bucket = process.env.R2_BUCKET;
    if (!bucket) throw new Error('R2StorageProvider requires R2_BUCKET');
    if (!process.env.R2_ENDPOINT) throw new Error('R2StorageProvider requires R2_ENDPOINT');
    this.bucket = bucket;
  }

  private async client() {
    if (!this.clientPromise) {
      this.clientPromise = (async () => {
        const { S3Client } = await import('@aws-sdk/client-s3');
        return new S3Client({
          endpoint: process.env.R2_ENDPOINT,
          region: 'auto',
          credentials: {
            accessKeyId: process.env.R2_ACCESS_KEY_ID || '',
            secretAccessKey: process.env.R2_SECRET_ACCESS_KEY || '',
          },
        });
      })();
    }
    return this.clientPromise;
  }

  private static contentType(key: string): string {
    if (key.endsWith('.webp')) return 'image/webp';
    if (key.endsWith('.png')) return 'image/png';
    if (key.endsWith('.jpg') || key.endsWith('.jpeg')) return 'image/jpeg';
    return 'application/octet-stream';
  }

  async put(key: string, data: Buffer, _ttlSeconds?: number): Promise<void> {
    const { PutObjectCommand } = await import('@aws-sdk/client-s3');
    const client = await this.client();
    await client.send(
      new PutObjectCommand({
        Bucket: this.bucket,
        Key: key,
        Body: data,
        ContentType: R2StorageProvider.contentType(key),
      }),
    );
  }

  async get(key: string): Promise<Buffer | null> {
    const { GetObjectCommand } = await import('@aws-sdk/client-s3');
    const client = await this.client();
    try {
      const res = await client.send(new GetObjectCommand({ Bucket: this.bucket, Key: key }));
      if (!res.Body) return null;
      const bytes = await res.Body.transformToByteArray();
      return Buffer.from(bytes);
    } catch (err: any) {
      if (err?.name === 'NoSuchKey' || err?.$metadata?.httpStatusCode === 404) return null;
      throw err;
    }
  }

  async delete(key: string): Promise<void> {
    const { DeleteObjectCommand } = await import('@aws-sdk/client-s3');
    const client = await this.client();
    await client.send(new DeleteObjectCommand({ Bucket: this.bucket, Key: key }));
  }
}
