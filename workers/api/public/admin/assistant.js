// THE AGENT IN THE HUB (lane W-KITCHEN, operator Q9, 2026-09-26): a chat
// panel on every console screen, for the owner AND the kitchen.
//
// A line typed here goes where the mic's words go (`POST /api/voice`): the hub
// reads it against the person's capabilities and answers now (a count, a
// screen to open), hands a question to the assistant (the kitchen's own,
// with no customer data, for a member of staff), or PROPOSES a change with a
// read-back. A change happens only on the one tap of "Yes, do it": the signed
// confirmation's own instruction goes through the SAME route the console's
// button calls (`voice-plan.js` planOf), which authorises again.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, t, lang, api, post, store, toast, sheet } from '/admin/core.js';
import * as ui from '/lib/ui/index.js';
import { planOf, lineOf, assistPath } from '/admin/voice-plan.js';
import { principalOf } from '/admin/kitchen-logic.js';
import * as A from '/admin/assistant-logic.js';
import '/admin/assistant-i18n.js';

let log = [];
let ctx = { show: async () => {}, refresh: async () => {} };

/// The header's button: one tap opens the panel from any screen.
export function mountAssistant(header, before, c = {}){
  ctx = { ...ctx, ...c };
  if (!header || header.querySelector('#asstBtn')) return;
  const tmp = document.createElement('div');
  tmp.innerHTML = ui.iconButton({ id: 'asstBtn', icon: 'sparkles', ariaLabel: { t: 'asTitle' }, attrs: { data: { tour: 'hud.assistant' } } });
  const b = tmp.firstElementChild;
  header.insertBefore(b, before || null);
  b.onclick = open;
}

/// The panel: the conversation, the starter lines, the input.
export function open(){
  const me = principalOf(store.t);
  const starters = A.starters(me).map(k => ui.chip({ as: 'button', label: { t: k }, attrs: { data: { sug: k, tour: 'assistant.starter' } } })).join('');
  sheet(`<p class="eyebrow" data-t="asTitle"></p><h2 data-t="asTitle"></h2><p class="muted small" data-t="asHint"></p>
    <div id="asLog" class="as-log" role="log" aria-live="polite" data-tour="assistant.log"></div>
    <div class="chips" role="group">${starters}</div>
    ${ui.inputRow({ id: 'asQ', label: { t: 'asTitle' }, placeholder: { t: 'asPlaceholder' }, enterkeyhint: 'send', attrs: { data: { tour: 'assistant.input' } },
      action: ui.button({ id: 'asGo', variant: 'primary', icon: 'send', label: { t: 'asSend' }, attrs: { data: { tour: 'assistant.send' } } }) })}
    ${me.staff ? ui.button({ id: 'asKey', variant: 'ghost', icon: 'key', label: { t: 'asAgentKey' }, cls: 'mt-3', attrs: { data: { tour: 'assistant.agentKey' } } }) : ''}`, { name: 'assistant' });
  linkStyle();
  draw();
  const q = $('#asQ');
  $('#asGo').onclick = () => { const v = q.value; q.value = ''; return send(v); };
  q.onkeydown = e => { if (e.key === 'Enter') { e.preventDefault(); $('#asGo').onclick(); } };
  for (const c of $$('[data-sug]', $('#sheetIn'))) c.onclick = () => send(t(c.dataset.sug));
  // A MEMBER OF STAFF'S OWN AGENT (MCP): the key is minted for this person by
  // the room's sheet (`/api/staff/mcp/keys`), the tools are their role's.
  const key = $('#asKey');
  if (key) key.onclick = () => import('/room/mcp.js').then(m => m.openMcp({ api, lang: () => lang, toast, closeLabel: t('close'),
    role: me.caps.has('take_orders') ? 'waiter' : 'kitchen' })).catch(e => toast(String(e.message || e)));
}

function linkStyle(){
  if (document.querySelector('link[data-asst]')) return;
  const l = document.createElement('link');
  l.setAttribute('rel', 'stylesheet'); l.setAttribute('href', '/admin/assistant.css'); l.setAttribute('data-asst', '1');
  document.head?.appendChild(l);
}

/// One message, as markup.
function bubble(m, i){
  const who = m.from === 'you' ? 'as-you' : 'as-hub';
  if (m.kind !== 'propose') return `<p class="as-msg ${who}">${esc(m.text)}</p>`;
  const reason = m.reason && m.state === 'open'
    ? ui.field({ id: `asWhy${i}`, label: { t: 'kReason' }, attrs: { data: { tour: 'assistant.reason' } } }) : '';
  const done = m.state === 'open' ? `<div class="btn-row">
      ${ui.button({ variant: 'ghost', label: { t: 'asCancel' }, attrs: { data: { no: String(i), tour: 'assistant.cancel' } } })}
      ${ui.button({ variant: m.reason ? 'danger' : 'primary', size: 'lg', icon: 'check', label: { t: 'asConfirm' }, attrs: { data: { yes: String(i), tour: 'assistant.confirm' } } })}</div>`
    : `<p class="as-state">${esc(t(m.state === 'done' ? 'asDone' : m.state === 'cancelled' ? 'asCancelled' : 'asFailed'))}</p>`;
  return `<div class="as-msg as-hub as-propose" data-tour="assistant.proposal"><p>${esc(m.text)}?</p>${reason}${done}</div>`;
}

function draw(){
  const host = $('#asLog'); if (!host) return;
  host.innerHTML = log.map(bubble).join('');
  for (const b of $$('[data-yes]', host)) b.onclick = () => confirmAt(Number(b.dataset.yes));
  for (const b of $$('[data-no]', host)) b.onclick = () => { log = A.settle(log, Number(b.dataset.no), 'cancelled'); draw(); };
}

const say = text => { log = A.push(log, { from: 'hub', kind: 'say', text }); draw(); };

/// A line typed or a starter tapped.
export async function send(text){
  const line = A.clean(text); if (!line) return;
  log = A.push(log, { from: 'you', kind: 'say', text: line }); draw();
  let r;
  try { r = await post('/voice', A.lineBody(line, lang)); } catch (e) { return say(String(e.message || e)); }
  const m = A.reading(r);
  if (m.kind === 'propose') { log = A.push(log, m); return draw(); }
  if (m.kind === 'status') return say(lineOf({ action: 'status', open: m.open, waiting: m.waiting }, t));
  if (m.kind === 'show') { say(t('asOpened')); return ctx.show(m.screen); }
  if (m.kind === 'ask') {
    say(t('asThinking'));
    try {
      const d = await api(assistPath(principalOf(store.t).staff, store.loc), { method: 'POST', body: { question: m.question } });
      return say(d && d.answer ? d.answer : t('asNotUnderstood'));
    } catch (e) { return say(String(e.message || e)); }
  }
  return say(m.text ? `${m.text}${m.heard ? ' · «' + m.heard + '»' : ''}` : t('asNotUnderstood'));
}

/// "Yes, do it": the token back to the hub, and its instruction to the route
/// the console's own button calls.
export async function confirmAt(i){
  const m = log[i]; if (!m || m.kind !== 'propose' || m.state !== 'open') return;
  const why = m.reason ? String($(`#asWhy${i}`)?.value || '').trim() : '';
  if (m.reason && !why) return toast(t('kReasonNeeded'));
  try {
    const done = await post('/voice', { confirm: m.token, lang });
    const p = done && done.understood ? planOf(done, store.loc, why) : null;
    if (!p) { log = A.settle(log, i, 'failed'); draw(); return toast((done && done.say) || t('asFailed')); }
    await post(p.path, p.body);
    log = A.settle(log, i, 'done'); draw();
    toast(t('saved')); navigator.vibrate?.(12);
    await ctx.refresh();
  } catch (e) { log = A.settle(log, i, 'failed'); draw(); toast(String(e.message || e)); }
}

/// For tests: the conversation as it stands, and a fresh one.
export const conversation = () => log;
export function reset(){ log = []; }
