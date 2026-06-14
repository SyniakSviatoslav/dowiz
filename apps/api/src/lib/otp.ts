import argon2 from 'argon2';
import crypto from 'node:crypto';

export const OTP_CODE_LENGTH = 6;
export const OTP_DEFAULT_TTL_MS = 5 * 60 * 1000; // 5 min
export const OTP_SEND_RATE_LIMIT = 3;  // per 15 min per phone per location
export const OTP_VERIFY_RATE_LIMIT = 5; // per 15 min per phone per location
export const OTP_LOCKOUT_HOURS = 1;
export const VERIFIED_TOKEN_TTL_MS = 15 * 60 * 1000; // 15 min

export function generateOtpCode(): string {
  const code = crypto.randomInt(0, 999999).toString().padStart(OTP_CODE_LENGTH, '0');
  return code;
}

export async function hashOtpCode(code: string): Promise<string> {
  return argon2.hash(code, { type: argon2.argon2id, memoryCost: 19456, timeCost: 2, parallelism: 1 });
}

export async function verifyOtpCode(code: string, hash: string): Promise<boolean> {
  try {
    return await argon2.verify(hash, code);
  } catch (err: any) {
    console.warn('[otp] argon2 verify failed:', err?.message);
    return false;
  }
}

export function generateOpaqueToken(): { token: string; hash: string } {
  const token = crypto.randomBytes(32).toString('base64url');
  const hash = crypto.createHash('sha256').update(token).digest('hex');
  return { token, hash };
}

export function hashPhone(phone: string): string {
  const normalized = phone.replace(/\D/g, '');
  return crypto.createHash('sha256').update(normalized).digest('hex');
}

export function hashOrderIntent(items: Array<{ product_id: string; quantity: number }>): string {
  const canonical = items
    .map(i => `${i.product_id}:${i.quantity}`)
    .sort()
    .join(',');
  return crypto.createHash('sha256').update(canonical).digest('hex');
}

export function maskPhone(phone: string): string {
  const cleaned = phone.replace(/\D/g, '');
  if (cleaned.length < 4) return '+*** *** ****';
  return '+*** *** ' + cleaned.substring(cleaned.length - 4);
}

export async function checkOtpSendRateLimit(
  pool: any,
  locationId: string,
  phone: string,
  maxSends: number = OTP_SEND_RATE_LIMIT,
): Promise<boolean> {
  const res = await pool.query(
    `SELECT COUNT(*)::int AS cnt FROM phone_otp
     WHERE location_id = $1 AND phone = $2
       AND created_at > now() - interval '15 minutes'`,
    [locationId, phone],
  );
  return (res.rows[0]?.cnt ?? 0) < maxSends;
}

export async function checkOtpVerifyRateLimit(
  pool: any,
  locationId: string,
  phone: string,
  maxAttempts: number = OTP_VERIFY_RATE_LIMIT,
): Promise<boolean> {
  const res = await pool.query(
    `SELECT COUNT(*)::int AS attempts,
            MAX(CASE WHEN attempts >= $2 THEN created_at ELSE NULL END) AS lockout_start
     FROM phone_otp
     WHERE location_id = $1 AND phone = $3
       AND created_at > now() - interval '15 minutes'`,
    [locationId, OTP_VERIFY_RATE_LIMIT, phone],
  );

  const row = res.rows[0];
  if (!row) return true;

  const attempts = row.attempts ?? 0;

  if (attempts >= OTP_VERIFY_RATE_LIMIT) {
    // Check lockout expiry
    if (row.lockout_start) {
      const lockoutElapsed = Date.now() - new Date(row.lockout_start).getTime();
      if (lockoutElapsed < OTP_LOCKOUT_HOURS * 3600000) return false;
    }
    return false;
  }

  return true;
}
