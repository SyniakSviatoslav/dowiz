// Minimal PHP serialize() — for replicating Plisio's verify_hash (HMAC over php_serialize($_POST)). Plisio
// posts a FLAT assoc array of string values, so only the string/scalar branches are exercised. PHP uses BYTE
// length for strings and preserves insertion order. ⚠️ key order must match Plisio's $_POST order — validate
// against a real Plisio test callback before PAYMENTS_CRYPTO_ENABLED is flipped (verifyWebhook is fail-closed).
function serializeVal(v: unknown): string {
  if (typeof v === 'string') return `s:${Buffer.byteLength(v, 'utf8')}:"${v}";`;
  if (typeof v === 'number') return Number.isInteger(v) ? `i:${v};` : `d:${v};`;
  if (typeof v === 'boolean') return `b:${v ? 1 : 0};`;
  if (v === null || v === undefined) return 'N;';
  if (Array.isArray(v)) {
    let out = `a:${v.length}:{`;
    v.forEach((item, i) => { out += `i:${i};` + serializeVal(item); });
    return out + '}';
  }
  if (typeof v === 'object') {
    const entries = Object.entries(v as Record<string, unknown>);
    let out = `a:${entries.length}:{`;
    for (const [k, val] of entries) out += serializeVal(k) + serializeVal(val);
    return out + '}';
  }
  const s = String(v);
  return `s:${Buffer.byteLength(s, 'utf8')}:"${s}";`;
}

export function serialize(data: Record<string, unknown>): string {
  return serializeVal(data);
}
