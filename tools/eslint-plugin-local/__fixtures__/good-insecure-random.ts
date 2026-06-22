// Test fixture for no-insecure-random rule (valid code — must NOT be flagged).
import crypto from 'node:crypto';

// CORRECT: security values use crypto.*
const sessionToken = crypto.randomBytes(32).toString('hex');
const otpCode = crypto.randomInt(0, 1_000_000);
const resetToken = crypto.randomUUID();

// CORRECT: Math.random() for NON-security purposes (jitter / animation / toast id)
// — these are the legitimate uses across the real codebase and stay green.
const jitter = Math.random() * 1000;
const particleHue = 38 + Math.random() * 14;
const toastId = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

export { sessionToken, otpCode, resetToken, jitter, particleHue, toastId };
