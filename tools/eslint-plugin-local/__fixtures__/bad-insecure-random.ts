// Test fixture for no-insecure-random rule.
// ANTI-PATTERN: predictable Math.random() for security-sensitive values.
// Recurrent class: dev-login backdoor / auth-token predictability (ADR-0003).

// ANTI-PATTERN: token via Math.random — should be flagged
const sessionToken = Math.random().toString(36).slice(2);

// ANTI-PATTERN: otp code via Math.random — should be flagged
const otpCode = Math.floor(Math.random() * 1_000_000);

// ANTI-PATTERN: assignment form — should be flagged
let resetToken = '';
resetToken = `r_${Math.random().toString(36)}`;

// ANTI-PATTERN: csrf nonce in template — should be flagged
const csrfNonce = `n-${Date.now()}-${Math.random()}`;

export { sessionToken, otpCode, resetToken, csrfNonce };
