// The SMS screen's pure parts (W-SMS), tested by sms-view.test.mjs.
//
// ASCII QUOTES ONLY in this file.

/// The status pill: off, failing (with the reason), or working. PURE.
export function statusOf(cfg, h){
  if (!cfg.on) return ['', 'off'];
  if (cfg.missing) return ['warn', cfg.missing];
  const err = h.last_err, ok = h.last_ok_ms || 0;
  if (err && err.at_ms > ok) return ['bad', 'sms_failing'];
  return ['ok', 'sms_live'];
}

/// The owner's edit as the hub takes it (config::CfgIn). PURE over the values.
export function editBody(v){
  const daily = Number(String(v.daily || '').trim());
  const b = { on: !!v.on, provider: v.provider || 'smsgate', url: (v.url || '').trim(), user: (v.user || '').trim(), from: (v.from || '').trim(), daily: Number.isInteger(daily) && daily > 0 ? daily : 60 };
  // An absent secret KEEPS the stored one; it is sent only when typed.
  if (v.secret) b.secret = v.secret;
  return b;
}

