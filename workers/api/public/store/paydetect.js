// WHICH WALLET THIS BROWSER OFFERS, to put its button first (W-SENSE row 8, 2026-10-04).
//
// DEVICE CONTEXT, NOT A FINGERPRINT: two yes/no feature checks the browser answers for every
// site -- Apple Pay (`ApplePaySession.canMakePayments()`) and a Payment Request capable Android
// browser (`PaymentRequest` + `/Android/.test(navigator.userAgent)`, the one form of a user-agent
// read tools/gates/no-tracking.mjs allows). Nothing is stored, nothing is sent, the server never
// learns the device: only the ORDER of the buttons changes; no rail is added or hidden.
//
// PURE `orderRails`; `support()` reads the browser once.

/// The rails with the wallet this device offers first; the rest keep their order.
export function orderRails(list, s = {}){
  const first = kind => (kind === 'apple_pay' && s.applePay) || (kind === 'google_pay' && s.googlePay);
  return [...(list || []).filter(r => first(r[0])), ...(list || []).filter(r => !first(r[0]))];
}

let cached = null;
/// `{ applePay, googlePay }`, both false wherever the checks cannot run.
export function support(g = globalThis){
  if (cached && g === globalThis) return cached;
  let applePay = false, googlePay = false;
  try { applePay = !!(g.ApplePaySession && g.ApplePaySession.canMakePayments && g.ApplePaySession.canMakePayments()); } catch { applePay = false; }
  try { const { navigator } = g; googlePay = typeof g.PaymentRequest === 'function' && !!navigator && /Android/.test(navigator.userAgent); } catch { googlePay = false; }
  const out = { applePay, googlePay };
  if (g === globalThis) cached = out;
  return out;
}
