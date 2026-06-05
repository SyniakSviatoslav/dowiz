import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';
import { Upload } from '@aws-sdk/lib-storage';
import { Readable } from 'node:stream';

export interface R2Config {
  accountId: string; // The R2 endpoint usually contains the account ID, or pass the full endpoint directly
  endpoint: string;
  accessKeyId: string;
  secretAccessKey: string;
  bucket: string;
}

let s3Client: S3Client | null = null;

export function getS3Client(config: R2Config): S3Client {
  if (!s3Client) {
    s3Client = new S3Client({
      region: 'auto',
      endpoint: config.endpoint,
      credentials: {
        accessKeyId: config.accessKeyId,
        secretAccessKey: config.secretAccessKey,
      },
    });
  }
  return s3Client;
}

export async function uploadStream(
  config: R2Config,
  key: string,
  stream: Readable | NodeJS.ReadableStream,
  metadata?: Record<string, string>
) {
  const client = getS3Client(config);

  const upload = new Upload({
    client,
    params: {
      Bucket: config.bucket,
      Key: key,
      Body: stream as any,
      Metadata: metadata,
    },
    // Optional tags
    queueSize: 4, // optional concurrency configuration
    partSize: 1024 * 1024 * 5, // optional size of each part, in bytes, at least 5MB
    leavePartsOnError: false, // optional manually handle dropped parts
  });

  return upload.done();
}

export async function uploadJson(
  config: R2Config,
  key: string,
  data: any,
  metadata?: Record<string, string>
) {
  const client = getS3Client(config);
  const command = new PutObjectCommand({
    Bucket: config.bucket,
    Key: key,
    Body: JSON.stringify(data),
    ContentType: 'application/json',
    Metadata: metadata,
  });

  return client.send(command);
}
