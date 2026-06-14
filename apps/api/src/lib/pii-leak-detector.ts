export interface PiiLeakResult {
  source: 'json_key' | 'json_value' | 'html_attribute' | 'string_value';
  pattern: string;
  value: string;
  context: string;
}

/**
 * Albanian phone number pattern: +355 XX XXX XXXX
 * Email pattern
 * Address patterns (Rruga, Blvd, etc.)
 */
const PII_VALUE_PATTERNS = [
  /\+355\s*\d[\d\s]{6,}/g,          // Albanian phone numbers
  /[\w.+-]+@[\w-]+\.[\w.-]+/g,       // Emails
  /\b(Rruga|Rr\.|Blvd|Bulevardi|Lgj|Lagja)\s+\w+/gi, // Address indicators
];

/**
 * Dangerous JSON keys that should never appear in public/API responses
 */
const DANGEROUS_JSON_KEYS = [
  /^customer_id$/i,
  /^courier_id$/i,
  /^owner_id$/i,
  /^user_id$/i,
  /^phone$/i,
  /^email$/i,
  /^email_encrypted$/i,
  /^phone_encrypted$/i,
  /^full_name_encrypted$/i,
  /^password_hash$/i,
  /^token_hash$/i,
  /^ip_hash$/i,
  /^user_agent_hash$/i,
  /^deactivated_by_owner_id$/i,
  /^added_by_owner_id$/i,
  /^created_by_owner_id$/i,
  /^invited_email_hash$/i,
];

export function detectPiiLeak(input: string, contextLabel: string = 'unknown'): PiiLeakResult[] {
  const leaks: PiiLeakResult[] = [];

  let parsed: any;
  let isJson = false;
  try {
    parsed = JSON.parse(input);
    isJson = true;
  } catch (err: any) {
    console.debug('[pii-leak-detector] input is not valid JSON:', err?.message);
  }

  if (isJson && typeof parsed === 'object') {
    scanJsonValue(parsed, '', leaks, contextLabel);
  }

  // String-level scans (works for both HTML and JSON-stringified)
  for (const pattern of PII_VALUE_PATTERNS) {
    let match;
    while ((match = pattern.exec(input)) !== null) {
      leaks.push({
        source: 'string_value',
        pattern: pattern.source.substring(0, 30),
        value: match[0],
        context: contextLabel,
      });
    }
  }

  // HTML attribute scan (original behavior)
  const htmlPatterns = [
    /owner_[a-zA-Z0-9_-]+/g,
    /customer_[a-zA-Z0-9_-]+/g,
    /courier_[a-zA-Z0-9_-]+/g,
    /user_(?!select|agent|scalable)[a-zA-Z0-9_-]+/g,
  ];
  for (const pattern of htmlPatterns) {
    let match;
    while ((match = pattern.exec(input)) !== null) {
      leaks.push({
        source: 'html_attribute',
        pattern: pattern.source.substring(0, 30),
        value: match[0],
        context: contextLabel,
      });
    }
  }

  return leaks;
}

function scanJsonValue(value: any, path: string, leaks: PiiLeakResult[], contextLabel: string) {
  if (value === null || value === undefined) return;

  if (typeof value === 'object' && !Array.isArray(value)) {
    for (const [key, val] of Object.entries(value)) {
      const fullPath = path ? `${path}.${key}` : key;

      // Check if key matches a dangerous pattern
      for (const pattern of DANGEROUS_JSON_KEYS) {
        if (pattern.test(key)) {
          // Only flag if the value itself looks like raw data (UUID, long string, etc.)
          if (typeof val === 'string' && val.length > 4) {
            leaks.push({
              source: 'json_key',
              pattern: key,
              value: `${key}: ${maskValue(val)}`,
              context: contextLabel,
            });
          }
        }
      }

      // Recurse into nested objects
      if (typeof val === 'object') {
        scanJsonValue(val, fullPath, leaks, contextLabel);
      }

      // Check string values for PII
      if (typeof val === 'string') {
        for (const piiPattern of PII_VALUE_PATTERNS) {
          let match;
          while ((match = piiPattern.exec(val)) !== null) {
            leaks.push({
              source: 'json_value',
              pattern: piiPattern.source.substring(0, 30),
              value: `${fullPath}: ${match[0]}`,
              context: contextLabel,
            });
          }
        }
      }
    }
  }

  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) {
      scanJsonValue(value[i], `${path}[${i}]`, leaks, contextLabel);
    }
  }
}

function maskValue(val: string): string {
  if (val.length <= 8) return '***';
  return val.substring(0, 4) + '...' + val.substring(val.length - 4);
}
