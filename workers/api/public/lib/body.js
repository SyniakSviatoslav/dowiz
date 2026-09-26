// A response body as an object, whatever the hub answered (lane W-QA, 2026-09-26).
//
// The hub refuses in PLAIN TEXT (`Response::error`: "invalid credentials",
// "no active owner membership") and answers in JSON. A sign-in screen that
// called `r.json()` on a refusal showed `Unexpected token 'i', "invalid cr"...
// is not valid JSON` instead of the hub's words -- the owner console, and the
// courier app, on every mistyped password; and the console never reached the
// staff door at all, so no member of staff could sign in (QA walk Q2).
//
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11). Tested by body.test.mjs.

/// JSON as parsed; text (or a bare JSON string) as `{ error: text }`.
export async function bodyOf(r){
  const text = await r.text().catch(() => '');
  try {
    const v = JSON.parse(text);
    return v && typeof v === 'object' ? v : { error: String(v) };
  } catch {
    return { error: text.trim() };
  }
}
