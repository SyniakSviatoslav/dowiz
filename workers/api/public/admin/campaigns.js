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
import { btn, field, select, loading, rowBtn, rowDiv } from '/admin/parts.js';

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
  sheet(`${head}${loading()}`, { name: 'campaigns', keepScroll: true });
  let d; try { d = await api('/owner/campaigns'); } catch (e) { return fail(e); }
  const list = d.campaigns || [];
  const rows = list.map(({ campaign: c, sent }) => rowBtn({ leading: icon('brand-whatsapp'), title: c.name,
    sub: `${esc(segText(c.segment))} · ${esc(day(c.atMs))}`, trailing: `<span class="mono">${esc(sent)}</span>`, data: { id: c.id }, tour: 'campaigns.row' })).join('');
  sheet(`${head}<div class="btn-row">${btn({ id: 'cpNew', variant: 'primary', icon: 'plus', key: 'cpNew', tour: 'campaigns.new' })}</div>
    ${list.length ? `<div class="rows mt-3">${rows}</div>` : `<p class="muted small mt-3" data-t="cpNone"></p>`}`, { name: 'campaigns', keepScroll: true });
  $('#cpNew').onclick = () => edit(null, d.tags || []);
  for (const b of document.querySelectorAll('#sheetIn [data-id]')) b.onclick = () => detail(b.dataset.id, d.tags || []);
}

/// The form. `c` is an unsent campaign to edit, or null for a new one.
function edit(c, tags){
  const s = (c && c.segment) || { kind: 'everyone_consented' };
  const tp = (c && c.template) || {};
  sheet(`${head}
    ${field({ id: 'cp-name', key: 'cpName', maxlength: 60, value: c ? c.name : '', tour: 'campaigns.name' })}
    ${field({ id: 'cp-text', key: 'cpText', rows: 5, maxlength: 1000, value: c ? c.text : '', tour: 'campaigns.text' })}
    ${select({ id: 'cp-seg', key: 'cpWho', value: s.kind, options: SEGMENTS.map(k => ({ value: k, key: 'seg_' + k })), tour: 'campaigns.segment' })}
    <div id="cp-days-box">${field({ id: 'cp-days', key: 'cpDays', inputmode: 'numeric', value: s.days || DEFAULT_DAYS, tour: 'campaigns.days' })}</div>
    <div id="cp-tag-box">${select({ id: 'cp-tag', key: 'cpTag', value: s.tag, options: tags.map(x => ({ value: x, label: x })), tour: 'campaigns.tag' })}</div>
    ${field({ id: 'cp-tpl', key: 'cpTpl', maxlength: 512, autocapitalize: 'off', spellcheck: false, value: tp.name || '', tour: 'campaigns.template' })}
    ${select({ id: 'cp-tpl-lang', key: 'cpTplLang', value: tp.lang || 'sq', options: TPL_LANGS.map(l => ({ value: l, label: l })), tour: 'campaigns.templateLang' })}
    ${field({ id: 'cp-tpl-params', key: 'cpTplParams', rows: 3, value: (tp.params || []).join('\n'), hintKey: 'cpTplHint', tour: 'campaigns.templateParams' })}
    ${field({ id: 'cp-promo', key: 'cpPromo', maxlength: 16, value: (c && c.promo) || '', hintKey: 'cpPromoHint', tour: 'campaigns.promo' })}
    <div class="btn-row">${btn({ id: 'cpSave', variant: 'primary', icon: 'check', key: 'save', tour: 'campaigns.save' })}</div>`, { name: 'campaigns' });
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
  const line = (k, n) => rowDiv({ title: { t: k }, trailing: `<span class="mono">${esc(n)}</span>` });
  const sent = (r.sent || 0) > 0;
  sheet(`<p class="eyebrow" data-t="campaigns"></p><h2>${esc(c.name)}</h2><p class="muted small">${esc(segText(c.segment))}${c.promo ? ' · ' + esc(c.promo) : ''}${c.template ? ' · ' + esc(c.template.name) + ' (' + esc(c.template.lang) + ')' : ''}</p>
    <div class="code">${esc(c.text)}</div>
    <div class="rows mt-3">${line('cpSent', r.sent || 0)}${line('cpWaiting', r.waiting || 0)}${line('cpDelivered', r.delivered || 0)}${line('cpWithdrawn', r.withdrawn || 0)}${line('cpAbandoned', r.abandoned || 0)}${c.promo ? line('cpRedeemed', r.redeemed || 0) : ''}</div>
    ${sent ? '<p class="muted small" data-t="cpLocked"></p>' : ''}
    <div id="cpOut"></div>
    <div class="btn-row">${sent ? '' : btn({ id: 'cpEdit', variant: 'ghost', icon: 'note', key: 'edit', tour: 'campaigns.edit' })}
      ${btn({ id: 'cpPrev', icon: 'eye', key: 'cpPreview', tour: 'campaigns.preview' })}
      ${btn({ id: 'cpSend', variant: 'primary', icon: 'send', key: 'cpSend', disabled: true, tour: 'campaigns.send' })}</div>`, { name: 'campaigns', keepScroll: true });
  const ed = $('#cpEdit'); if (ed) ed.onclick = () => edit(c, tags);
  $('#cpPrev').onclick = () => preview(id, tags);
}

/// THE COUNT BEFORE ANYTHING IS QUEUED. The send button unlocks only here,
/// and only when there is somebody new to send to.
async function preview(id, tags){
  let d; try { d = await busy($('#cpPrev'), () => post('/owner/campaigns/' + encodeURIComponent(id) + '/preview', {})); } catch (e) { return fail(e); }
  const p = d.preview || {}, fresh = (p.count || 0) - (p.already || 0);
  $('#cpOut').innerHTML = `<div class="rows mt-3" data-tour="campaigns.count">${rowDiv({ leading: icon('user'), title: t('cpCount'), sub: `${esc(p.already || 0)} ${esc(t('cpAlready'))}`, trailing: `<span class="mono">${esc(p.count || 0)}</span>` })}
    ${rowDiv({ leading: icon('coin'), title: t('cpCost'), trailing: `<span class="mono">${cost(p.cost_minor || 0, p.currency || 'USD')}</span>` })}</div>
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
