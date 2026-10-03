// THE OWNER'S AI, the PURE half (lane W-AI, 2026-10-03): the OpenRouter preset,
// the budget meter's numbers, the provider words, what a test state means and
// where each number of an answer leads. No DOM, no network (`ai-logic.test.mjs`);
// `admin/ai.js` is the DOM half.
//
// THE KEY IS NEVER HERE. The hub answers `keySet: true|false` and nothing else;
// the field the owner pastes into is sent once and emptied.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// OpenRouter's free models, the documented default for an owner's own key
/// (operator 2026-10-03: each hub owner sets it up with THEIR key; the
/// platform holds none). The model id was listed live by
/// https://openrouter.ai/api/v1/models on 2026-10-03 and may be retired by
/// OpenRouter; the owner can type another.
export const OPENROUTER = {
  endpoint: 'https://openrouter.ai/api/v1',
  model: 'google/gemma-4-26b-a4b-it:free',
  keysUrl: 'https://openrouter.ai/keys',
};

/// The provider words the hub accepts for `ai.provider`, in the order shown.
export const MODES = ['auto', 'own', 'workers'];

/// The settings writes "Save" makes, in order. The key is written only when
/// typed: an empty box keeps the stored key (clearing is its own button).
export function saveWrites(form){
  const w = [
    ['ai.enabled', form.enabled ? '1' : '0'],
    ['ai.provider', MODES.includes(form.mode) ? form.mode : 'auto'],
    ['ai.endpoint', String(form.endpoint || '').trim()],
    ['ai.model', String(form.model || '').trim()],
  ];
  const key = String(form.key || '').trim();
  if (key) w.push(['ai.token', key]);
  return w.map(([key, value]) => ({ key, value }));
}

/// The OpenRouter preset applied to a form: endpoint and model, and the
/// provider set to the owner's own.
export const withOpenRouter = form => ({ ...form, endpoint: OPENROUTER.endpoint, model: OPENROUTER.model, mode: form.mode === 'workers' ? 'own' : form.mode });

/// Today's Workers AI share as the meter draws it: used, cap, percent (0..100).
export function meter(b){
  const used = Math.max(0, Number(b && b.used) | 0), cap = Math.max(0, Number(b && b.cap) | 0);
  return { used, cap, left: Math.max(0, cap - used), pct: cap ? Math.min(100, Math.round(100 * used / cap)) : 0 };
}

/// The i18n key for a test's state.
export function stateKey(state){
  return ({ ok: 'ai_state_ok', 'needs-key': 'ai_state_needs_key', 'workers-ai-unavailable': 'ai_state_no_binding',
    'budget-spent': 'ai_state_budget', 'not-https': 'aiNotHttps', disabled: 'ai_state_disabled', failed: 'ai_state_failed' })[state] || 'ai_state_failed';
}

/// The i18n key of a reason a route was skipped.
export function whyKey(why){
  return ({ 'needs-key': 'ai_why_needs_key', 'workers-ai-unavailable': 'ai_why_no_binding', 'budget-spent': 'ai_why_budget',
    'not-https': 'aiNotHttps', disabled: 'ai_why_disabled', 'not-chosen': 'ai_why_not_chosen' })[why] || 'ai_why_disabled';
}

/// A number of an answer and where it leads: the console path of the route
/// it was read from (without `/api`), and one path per day of records.
export function trail(num){
  const strip = p => String(p || '').replace(/^\/api/, '');
  return { source: strip(num && num.source), days: ((num && num.trace) || []).map(strip) };
}

/// A question worth sending: trimmed, 1..400 characters.
export const MAX_QUESTION = 400;
export function clean(q){
  const s = String(q ?? '').replace(/\s+/g, ' ').trim();
  return s && s.length <= MAX_QUESTION ? s : null;
}

/// The starter questions, as i18n keys whose TEXT is sent (so each language
/// asks in its own words, and the hub's lexicon reads them).
export const STARTERS = ['ai_q_revenue', 'ai_q_best', 'ai_q_worst_monday', 'ai_q_hour', 'ai_q_food_cost', 'ai_q_low'];
