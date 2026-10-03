// THE OWNER'S AI, the DOM half (lane W-AI, 2026-10-03): one sheet with the
// question box (every number in an answer opens where it came from), and the
// AI connection card -- provider, the OpenRouter preset with the owner's OWN
// key, the test button and today's Workers AI share. `explainCard` is the
// "what the numbers say" card the analytics and kitchen screens mount.
//
// The hub decides every number (`/api/owner/ai/*`); this file only draws.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, t, lang, api, post, toast, sheet, busy, switchEl, retranslate, hydrate } from '/admin/core.js';
import { btn, iconBtn, field, select, chips, rowDiv, loading, k } from '/admin/parts.js';
import * as L from '/admin/ai-logic.js';
import '/admin/ai-i18n.js';

const SECRET_SET_MARK = '•••• set';
const fail = e => toast(String(e.message || e));
const paint = root => { retranslate(root); hydrate(root); };

/// The sheet: questions first (what an owner opens it for), the connection under them.
export async function openAi(){
  let st = {}; try { st = await api('/owner/ai'); } catch (e) { fail(e); }
  let v = {}; try { v = (await api('/owner/settings')).values || {}; } catch {}
  const form = { enabled: !!st.enabled, mode: st.mode || 'auto', endpoint: st.endpoint || '', model: v['ai.model'] || st.model || '', key: '' };
  const m = L.meter(st.budget);
  sheet(`<p class="eyebrow" data-t="ai_title"></p><h2 data-t="ai_ask_h"></h2><p class="sheet-hint" data-t="ai_hint"></p>
    ${chips({ values: L.STARTERS.map(s => ({ value: s, key: s })), attr: 'aisq', tour: 'ai.starter' })}
    ${field({ id: 'ai-q', key: 'ai_ask_h', rows: 2, phKey: 'ai_ask_ph', tour: 'assistant.question' })}
    <div class="btn-row">${btn({ id: 'aiAsk', variant: 'primary', icon: 'sparkles', key: 'ai_send', tour: 'assistant.ask' })}</div>
    <div id="aiOut" aria-live="polite"></div>
    <div id="aiExplain"></div>
    <p class="eyebrow mt-3" data-t="ai_settings_h"></p>
    ${switchEl('ai-on', form.enabled, 'aiEnabled', 'aiEnabledHint', 'assistant.enabled')}
    ${select({ id: 'ai-mode', key: 'ai_provider', value: form.mode, tour: 'ai.provider', options: L.MODES.map(x => ({ value: x, key: 'ai_mode_' + x })) })}
    <div class="btn-row">${btn({ id: 'aiOr', icon: 'plug-connected-x', key: 'ai_openrouter', tour: 'ai.openrouter' })}
      ${btn({ variant: 'ghost', icon: 'key', key: 'ai_get_key', href: L.OPENROUTER.keysUrl, target: '_blank' })}</div>
    <p class="hint" data-t="ai_free_note"></p>
    ${field({ id: 'ai-endpoint', key: 'aiEndpoint', inputmode: 'url', value: form.endpoint, placeholder: L.OPENROUTER.endpoint, tour: 'assistant.endpoint' })}
    <div class="grid2">${field({ id: 'ai-model', key: 'aiModel', value: form.model, placeholder: L.OPENROUTER.model, tour: 'assistant.model' })}
      ${field({ id: 'ai-token', key: 'aiToken', autocomplete: 'off', spellcheck: false, placeholder: st.keySet ? SECRET_SET_MARK : 'sk-or-v1-...', tour: 'assistant.token' })}</div>
    <div class="btn-row">${btn({ id: 'aiSave', variant: 'primary', icon: 'check', key: 'save', tour: 'assistant.save' })}${btn({ id: 'aiTest', icon: 'radar-2', key: 'ai_test', tour: 'ai.test' })}
      ${st.keySet ? btn({ id: 'aiClear', variant: 'ghost', icon: 'trash', key: 'ai_key_clear' }) : ''}</div>
    <p id="aiTestOut" class="small" aria-live="polite"></p>
    <div class="rows mt-3">${rowDiv({ title: k('ai_budget'), tour: 'ai.budget',
      sub: `<span class="mono">${m.used} / ${m.cap} ${esc(t('ai_neurons'))}${st.workersAi && !st.workersAi.available ? ' · ' + esc(t('ai_why_no_binding')) : ''}</span><span class="gauge"><i class="${m.pct >= 100 ? 'bad' : m.pct >= 80 ? 'warn' : ''}" data-w="${m.pct}"></i></span>` })}
      ${(st.skipped || []).filter(s => s.why !== 'not-chosen').map(s => rowDiv({ title: k(s.route === 'own' ? 'ai_route_own' : 'ai_route_workers'), sub: esc(t(L.whyKey(s.why))) })).join('')}</div>`, { name: 'assistant' });
  const root = $('#sheetIn');
  for (const c of $$('[data-aisq]', root)) c.onclick = () => { $('#ai-q').value = t(c.dataset.aisq); ask(); };
  $('#aiAsk').onclick = ask;
  $('#aiOr').onclick = () => { const f = L.withOpenRouter({ ...readForm() }); $('#ai-endpoint').value = f.endpoint; $('#ai-model').value = f.model; $('#ai-mode').value = f.mode; $('#ai-token').focus(); };
  $('#aiSave').onclick = () => busy($('#aiSave'), save).catch(fail);
  $('#aiTest').onclick = () => busy($('#aiTest'), test).catch(fail);
  const clear = $('#aiClear'); if (clear) clear.onclick = () => post('/owner/settings', { key: 'ai.token', value: '' }).then(openAi, fail);
  // The week's numbers in words, under the questions.
  explainCard($('#aiExplain'), 'analytics', 7);
}

function readForm(){
  return { enabled: $('#ai-on').checked, mode: $('#ai-mode').value, endpoint: $('#ai-endpoint').value, model: $('#ai-model').value, key: $('#ai-token').value };
}

async function save(){
  for (const w of L.saveWrites(readForm())) await post('/owner/settings', w);
  $('#ai-token').value = '';
  toast(t('saved'));
  await openAi();
}

async function test(){
  const r = await post('/owner/ai/test', { provider: $('#ai-mode').value });
  const who = r.provider ? ` · ${t('ai_answered_by')}: ${t(r.provider === 'own' ? 'ai_route_own' : 'ai_route_workers')} (${r.model || ''})` : '';
  const why = (r.failed || []).map(f => `${f.status} ${f.why}`).join('; ');
  $('#aiTestOut').textContent = `${t(L.stateKey(r.state))}${who}${why ? ' · ' + why : ''}`;
}

/// One answer: the sentence, then each number as a button that opens its source.
function drawAnswer(r){
  if (!r.understood) return `<div class="answer mt-3"><p>${esc(r.answer || t('ai_not_understood'))}</p>${r.why ? `<p class="hint">${esc(r.why)}</p>` : ''}</div>`;
  const nums = (r.numbers || []).map((n, i) => btn({ variant: 'ghost', label: String(n.value), attrs: { data: { ainum: String(i), tour: 'ai.number' } } })).join('');
  const how = [r.pickedBy === 'model' ? t('ai_by_model') : '', r.reworded ? t('ai_reworded') : ''].filter(Boolean).join(' · ');
  return `<div class="answer mt-3"><p>${esc(r.answer)}</p>${r.reworded ? `<p class="hint">${esc(r.template)}</p>` : ''}
    <div class="btn-row">${nums}</div>${how ? `<p class="hint">${esc(how)}</p>` : ''}<div id="aiTrail"></div></div>`;
}

async function ask(){
  const q = L.clean($('#ai-q').value); if (!q) return;
  const out = $('#aiOut');
  out.innerHTML = loading(1);
  let r; try { r = await busy($('#aiAsk'), () => post('/owner/ai/ask', { question: q, lang })); } catch (e) { out.innerHTML = ''; return fail(e); }
  out.innerHTML = drawAnswer(r); paint(out);
  for (const b of $$('[data-ainum]', out)) b.onclick = () => openTrail((r.numbers || [])[Number(b.dataset.ainum)], $('#aiTrail'));
}

/// Where a number came from: the source route's cells it sums, and each day's records.
export async function openTrail(num, host){
  if (!num || !host) return;
  const tr = L.trail(num);
  host.innerHTML = loading(1);
  let src = null; try { src = await api(tr.source); } catch (e) { host.innerHTML = ''; return fail(e); }
  const cells = (num.pointers || []).map(p => `<li class="mono">${esc(p)} = ${esc(JSON.stringify(pointer(src, p)))}</li>`).join('');
  const days = tr.days.map(d => btn({ variant: 'ghost', label: d.split('=').pop(), attrs: { data: { aiday: d } } })).join('');
  host.innerHTML = `<p class="hint">${esc(t('ai_source'))}: <span class="mono">${esc(tr.source)}</span></p><ul class="small">${cells}</ul>
    ${days ? `<p class="hint">${esc(t('ai_records'))}</p><div class="btn-row">${days}</div>` : ''}<pre id="aiRec" class="small mono"></pre>`;
  for (const b of $$('[data-aiday]', host)) b.onclick = async () => { try { $('#aiRec').textContent = JSON.stringify(await api(b.dataset.aiday), null, 1).slice(0, 4000); } catch (e) { fail(e); } };
}

/// RFC 6901, read only.
function pointer(v, p){
  return String(p).split('/').slice(1).reduce((o, s) => (o == null ? o : o[s.replace(/~1/g, '/').replace(/~0/g, '~')]), v);
}

/// "What the numbers say": the screen's explain cards, mounted in `host`.
/// A member of staff is refused by the hub (the cards hold margins), and the
/// card then draws nothing.
export async function explainCard(host, screen, days, reword = false){
  if (!host) return;
  host.innerHTML = loading(1);
  let r; try { r = await api(`/owner/ai/explain?screen=${encodeURIComponent(screen)}&days=${Number(days) | 0}&lang=${encodeURIComponent(lang)}${reword ? '&reword=1' : ''}`); }
  catch { host.innerHTML = ''; return; }
  const cards = r.cards || [];
  if (!cards.length) { host.innerHTML = ''; return; }
  host.innerHTML = `<div class="answer mt-3" data-tour="ai.explain"><p class="eyebrow" data-t="ai_explain_h"></p>
    ${cards.map((c, i) => `<p>${esc(c.text)}${(c.numbers || []).length ? ' ' + iconBtn({ icon: 'search', ariaKey: 'ai_source', attrs: { data: { aicard: String(i) } } }) : ''}</p>`).join('')}
    ${cards.some(c => c.reworded) ? `<p class="hint" data-t="ai_reworded"></p>` : ''}
    <div class="btn-row">${btn({ id: 'aiReword' + screen, variant: 'ghost', icon: 'sparkles', key: 'ai_explain_reword', tour: 'ai.reword' })}</div><div class="ai-trail"></div></div>`;
  paint(host);
  // A card's numbers share one source: its trail lists every cell they read.
  for (const b of $$('[data-aicard]', host)) b.onclick = () => {
    const ns = cards[Number(b.dataset.aicard)].numbers;
    openTrail({ ...ns[0], pointers: ns.flatMap(n => n.pointers || []) }, host.querySelector('.ai-trail'));
  };
  const rw = host.querySelector('#aiReword' + screen); if (rw) rw.onclick = () => explainCard(host, screen, days, true);
}

