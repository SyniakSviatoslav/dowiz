// CAMPAIGNS (C6, BLUEPRINT-CRM-CONSENT-LOYALTY section 3.6): write a message,
// pick who it is for from a closed list, PREVIEW the count and the cost, then
// send. Only people who ticked the offers box (or whose consent the owner
// filed with evidence) are ever counted, and the hub asks their consent again
// at the moment each message leaves. Nothing here holds a recipient list: the
// server computes it on every preview and every send.
//
// Routes: GET/POST /api/owner/campaigns, GET /api/owner/campaigns/:id,
// POST /api/owner/campaigns/:id/preview, POST /api/owner/campaigns/:id/send.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console. No apostrophes inside the words below, for the same reason.

import { $, esc, icon, t, api, post, toast, sheet, day, busy, confirm } from '/admin/core.js';
import { T, LANGS, intlLocale } from '/admin/i18n.js';
import * as Money from '/lib/money.js';

const WORDS = {
  sq: { cpHint: 'Mesazhi shkon vetëm te klientët që kanë dhënë pëlqimin për oferta në WhatsApp. Dërgohet si shabllon i miratuar nga Meta, që duhet të përmbajë udhëzimin STOP.', cpTpl: 'Shablloni WhatsApp (emri në Meta)', cpTplLang: 'Gjuha e shabllonit', cpTplParams: 'Vlerat {{1}}, {{2}}... (një për rresht)', cpTplHint: 'Regjistroni mesazhin në WhatsApp Manager dhe prisni miratimin. Pa shabllon, fushata nuk dërgohet.', cpNew: 'Fushatë e re', cpNone: 'Asnjë fushatë ende.', cpName: 'Emri (vetëm për ju)', cpText: 'Mesazhi', cpWho: 'Për kë', cpDays: 'Ditë pa porosi', cpTag: 'Etiketa', cpPromo: 'Kodi i zbritjes (opsional)', cpPromoHint: 'Përdorimet e kodit pas dërgimit janë matja e vetme.', cpPreview: 'Shiko sa njerëz', cpSend: 'Dërgo', cpCount: 'Marrës', cpAlready: 'tashmë të dërguar', cpCost: 'Kosto e përafërt (Meta)', cpSendQ: 'Dërgo te', cpPeople: 'njerëz?', cpQueued: 'Në radhë', cpLeft: 'presin shtypjen tjetër', cpSent: 'Dërguar', cpWaiting: 'Në pritje', cpWithdrawn: 'Tërhequr', cpAbandoned: 'Dështuar', cpDelivered: 'Dorëzuar', cpRedeemed: 'Porosi me kodin', cpLocked: 'E dërguar: nuk ndryshohet më.', cpZero: 'Askush në këtë grup nuk ka dhënë pëlqim.',
        seg_everyone_consented: 'Të gjithë me pëlqim', seg_not_seen_since: 'Pa ardhur prej disa ditësh', seg_tag: 'Me etiketë', seg_birthday_this_week: 'Ditëlindja këtë javë' },
  en: { cpHint: 'The message goes only to customers who agreed to offers on WhatsApp. It is sent as a template Meta approved, which must include the STOP instruction.', cpTpl: 'WhatsApp template (name in Meta)', cpTplLang: 'Template language', cpTplParams: 'Values for {{1}}, {{2}}... (one per line)', cpTplHint: 'Register the message in WhatsApp Manager and wait for approval. Without a template the campaign is not sent.', cpNew: 'New campaign', cpNone: 'No campaign yet.', cpName: 'Name (only for you)', cpText: 'Message', cpWho: 'Who for', cpDays: 'Days without an order', cpTag: 'Tag', cpPromo: 'Promo code (optional)', cpPromoHint: 'Uses of the code after the send are the only measure.', cpPreview: 'See how many people', cpSend: 'Send', cpCount: 'Recipients', cpAlready: 'already sent', cpCost: 'Estimated cost (Meta)', cpSendQ: 'Send to', cpPeople: 'people?', cpQueued: 'Queued', cpLeft: 'wait for the next press', cpSent: 'Sent', cpWaiting: 'Waiting', cpWithdrawn: 'Withdrawn', cpAbandoned: 'Failed', cpDelivered: 'Delivered', cpRedeemed: 'Orders with the code', cpLocked: 'Sent: it can no longer be changed.', cpZero: 'Nobody in this group has agreed.',
        seg_everyone_consented: 'Everyone who agreed', seg_not_seen_since: 'Not seen for some days', seg_tag: 'With a tag', seg_birthday_this_week: 'Birthday this week' },
  uk: { cpHint: 'Повідомлення отримають лише клієнти, які погодилися на пропозиції у WhatsApp. Надсилається як шаблон, схвалений Meta, і він має містити інструкцію STOP.', cpTpl: 'Шаблон WhatsApp (назва в Meta)', cpTplLang: 'Мова шаблону', cpTplParams: 'Значення {{1}}, {{2}}... (по одному в рядку)', cpTplHint: 'Зареєструйте повідомлення у WhatsApp Manager і дочекайтеся схвалення. Без шаблону розсилка не надсилається.', cpNew: 'Нова розсилка', cpNone: 'Розсилок ще немає.', cpName: 'Назва (лише для вас)', cpText: 'Повідомлення', cpWho: 'Для кого', cpDays: 'Днів без замовлення', cpTag: 'Мітка', cpPromo: 'Промокод (за бажанням)', cpPromoHint: 'Використання коду після розсилки - єдина міра.', cpPreview: 'Скільки людей', cpSend: 'Надіслати', cpCount: 'Отримувачі', cpAlready: 'вже надіслано', cpCost: 'Орієнтовна вартість (Meta)', cpSendQ: 'Надіслати', cpPeople: 'людям?', cpQueued: 'У черзі', cpLeft: 'чекають наступного натискання', cpSent: 'Надіслано', cpWaiting: 'Очікують', cpWithdrawn: 'Відкликано', cpAbandoned: 'Не вдалося', cpDelivered: 'Доставлено', cpRedeemed: 'Замовлення з кодом', cpLocked: 'Надіслано: змінити вже не можна.', cpZero: 'Ніхто в цій групі не погодився.',
        seg_everyone_consented: 'Усі, хто погодився', seg_not_seen_since: 'Давно не приходили', seg_tag: 'З міткою', seg_birthday_this_week: 'День народження цього тижня' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const SEGMENTS = ['everyone_consented', 'not_seen_since', 'tag', 'birthday_this_week'];
const DEFAULT_DAYS = 30;
const TPL_LANGS = ['sq', 'en', 'en_US', 'uk'];
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="marketing"></p><h2 data-t="campaigns"></h2><p class="muted small" data-t="cpHint"></p>`;
const cost = (n, cur) => esc(Money.formatter({ base: cur, display: cur, rates: null, locale: intlLocale() })(n));
const segText = s => t('seg_' + s.kind) + (s.kind === 'not_seen_since' ? ' · ' + s.days : s.kind === 'tag' ? ' · ' + s.tag : '');

/// The list: every campaign with how many it reached.
export async function open(){
  sheet(`${head}<div class="skel skel-row"></div>`, { name: 'campaigns', keepScroll: true });
  let d; try { d = await api('/owner/campaigns'); } catch (e) { return fail(e); }
  const list = d.campaigns || [];
  const rows = list.map(({ campaign: c, sent }) => `<button type="button" class="rowc" data-id="${esc(c.id)}">${icon('brand-whatsapp')}<span class="t"><b>${esc(c.name)}</b>
    <small>${esc(segText(c.segment))} · ${esc(day(c.atMs))}</small></span><span class="mono">${esc(sent)}</span></button>`).join('');
  sheet(`${head}<div class="btn-row"><button class="btn" id="cpNew">${icon('plus')}<span data-t="cpNew"></span></button></div>
    ${list.length ? `<div class="rows mt-3">${rows}</div>` : `<p class="muted small mt-3" data-t="cpNone"></p>`}`, { name: 'campaigns', keepScroll: true });
  $('#cpNew').onclick = () => edit(null, d.tags || []);
  for (const b of document.querySelectorAll('#sheetIn [data-id]')) b.onclick = () => detail(b.dataset.id, d.tags || []);
}

/// The form. `c` is an unsent campaign to edit, or null for a new one.
function edit(c, tags){
  const s = (c && c.segment) || { kind: 'everyone_consented' };
  const tp = (c && c.template) || {};
  sheet(`${head}
    <label for="cp-name" data-t="cpName"></label><input id="cp-name" maxlength="60" value="${esc(c ? c.name : '')}">
    <label for="cp-text" data-t="cpText"></label><textarea id="cp-text" rows="5" maxlength="1000">${esc(c ? c.text : '')}</textarea>
    <label for="cp-seg" data-t="cpWho"></label><select id="cp-seg">${SEGMENTS.map(k => `<option value="${k}" ${k === s.kind ? 'selected' : ''} data-t="seg_${k}"></option>`).join('')}</select>
    <div id="cp-days-box"><label for="cp-days" data-t="cpDays"></label><input id="cp-days" inputmode="numeric" value="${esc(s.days || DEFAULT_DAYS)}"></div>
    <div id="cp-tag-box"><label for="cp-tag" data-t="cpTag"></label><select id="cp-tag">${tags.map(x => `<option value="${esc(x)}" ${x === s.tag ? 'selected' : ''}>${esc(x)}</option>`).join('')}</select></div>
    <label for="cp-tpl" data-t="cpTpl"></label><input id="cp-tpl" maxlength="512" autocapitalize="off" spellcheck="false" value="${esc(tp.name || '')}">
    <label for="cp-tpl-lang" data-t="cpTplLang"></label><select id="cp-tpl-lang">${TPL_LANGS.map(l => `<option value="${l}" ${l === (tp.lang || 'sq') ? 'selected' : ''}>${l}</option>`).join('')}</select>
    <label for="cp-tpl-params" data-t="cpTplParams"></label><textarea id="cp-tpl-params" rows="3">${esc((tp.params || []).join('\n'))}</textarea><p class="hint" data-t="cpTplHint"></p>
    <label for="cp-promo" data-t="cpPromo"></label><input id="cp-promo" maxlength="16" value="${esc((c && c.promo) || '')}"><p class="hint" data-t="cpPromoHint"></p>
    <div class="btn-row"><button class="btn" id="cpSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'campaigns' });
  const sync = () => { const k = $('#cp-seg').value; $('#cp-days-box').hidden = k !== 'not_seen_since'; $('#cp-tag-box').hidden = k !== 'tag'; };
  $('#cp-seg').onchange = sync; sync();
  $('#cpSave').onclick = async () => {
    const kind = $('#cp-seg').value;
    const segment = kind === 'not_seen_since' ? { kind, days: Number($('#cp-days').value) | 0 } : kind === 'tag' ? { kind, tag: $('#cp-tag').value } : { kind };
    const body = { name: $('#cp-name').value, text: $('#cp-text').value, segment };
    const promo = $('#cp-promo').value.trim(); if (promo) body.promo = promo;
    const tname = $('#cp-tpl').value.trim();
    if (tname) body.template = { name: tname, lang: $('#cp-tpl-lang').value, params: $('#cp-tpl-params').value.split('\n').map(x => x.trim()).filter(Boolean) };
    if (c) body.id = c.id;
    try { const r = await busy($('#cpSave'), () => post('/owner/campaigns', body)); toast(t('saved')); detail(r.campaign.id, tags); } catch (e) { fail(e); }
  };
}

/// One campaign: its report, and the preview-then-send pair.
async function detail(id, tags){
  let d; try { d = await api('/owner/campaigns/' + encodeURIComponent(id)); } catch (e) { return fail(e); }
  const c = d.campaign, r = d.report || {};
  const line = (k, n) => `<div class="rowc"><span class="t"><b data-t="${k}"></b></span><span class="mono">${esc(n)}</span></div>`;
  const sent = (r.sent || 0) > 0;
  sheet(`<p class="eyebrow" data-t="campaigns"></p><h2>${esc(c.name)}</h2><p class="muted small">${esc(segText(c.segment))}${c.promo ? ' · ' + esc(c.promo) : ''}${c.template ? ' · ' + esc(c.template.name) + ' (' + esc(c.template.lang) + ')' : ''}</p>
    <div class="code">${esc(c.text)}</div>
    <div class="rows mt-3">${line('cpSent', r.sent || 0)}${line('cpWaiting', r.waiting || 0)}${line('cpDelivered', r.delivered || 0)}${line('cpWithdrawn', r.withdrawn || 0)}${line('cpAbandoned', r.abandoned || 0)}${c.promo ? line('cpRedeemed', r.redeemed || 0) : ''}</div>
    ${sent ? '<p class="muted small" data-t="cpLocked"></p>' : ''}
    <div id="cpOut"></div>
    <div class="btn-row">${sent ? '' : `<button class="btn ghost" id="cpEdit">${icon('note')}<span data-t="edit"></span></button>`}
      <button class="btn ghost" id="cpPrev">${icon('eye')}<span data-t="cpPreview"></span></button>
      <button class="btn" id="cpSend" disabled>${icon('send')}<span data-t="cpSend"></span></button></div>`, { name: 'campaigns', keepScroll: true });
  const ed = $('#cpEdit'); if (ed) ed.onclick = () => edit(c, tags);
  $('#cpPrev').onclick = () => preview(id, tags);
}

/// THE COUNT BEFORE ANYTHING IS QUEUED. The send button unlocks only here,
/// and only when there is somebody new to send to.
async function preview(id, tags){
  let d; try { d = await busy($('#cpPrev'), () => post('/owner/campaigns/' + encodeURIComponent(id) + '/preview', {})); } catch (e) { return fail(e); }
  const p = d.preview || {}, fresh = (p.count || 0) - (p.already || 0);
  $('#cpOut').innerHTML = `<div class="rows mt-3"><div class="rowc">${icon('user')}<span class="t"><b>${esc(t('cpCount'))}</b><small>${esc(p.already || 0)} ${esc(t('cpAlready'))}</small></span><span class="mono">${esc(p.count || 0)}</span></div>
    <div class="rowc">${icon('coin')}<span class="t"><b>${esc(t('cpCost'))}</b></span><span class="mono">${cost(p.cost_minor || 0, p.currency || 'USD')}</span></div></div>
    ${fresh > 0 ? '' : `<p class="muted small">${esc(t('cpZero'))}</p>`}`;
  const btn = $('#cpSend');
  btn.disabled = fresh <= 0;
  btn.onclick = async () => {
    if (!(await confirm(t('campaigns'), `${t('cpSendQ')} ${fresh} ${t('cpPeople')}`))) return detail(id, tags);
    try {
      const r = await post('/owner/campaigns/' + encodeURIComponent(id) + '/send', { confirm: true });
      toast(`${t('cpQueued')}: ${r.queued}${r.left ? ' · ' + r.left + ' ' + t('cpLeft') : ''}`);
    } catch (e) { fail(e); }
    detail(id, tags);
  };
}
