// The customer CARD: what the venue would otherwise write on a paper card by
// the till (§3.1 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22). A note, tags
// from a CLOSED list, EU-14 allergens, the usual table, a language and a
// birthday with no year. Everything else about a person -- orders, spend,
// last visit, name, phone -- is a fold over the orders and is never typed in.
//
// THE CLOSED LISTS ARE THE HUB'S (`services/customers/record.rs`), repeated
// here only to draw the chips; the hub refuses anything else with a reason.

import { $, $$, esc, icon, t, lang, api, post, toast, sheet, busy, hydrate, confirm, closeSheet } from '/admin/core.js';
import { T, LANGS, retranslate } from '/admin/i18n.js';
import * as ui from '/lib/ui/index.js';
import { btn, field, select, rowBtn, rowDiv } from '/admin/parts.js';

ui.useTranslator(t);

/// `record.rs::TAGS`. A tag is not a tier: nothing here ranks a person.
const TAGS = ['regular', 'office_lunch', 'family', 'group', 'takeaway', 'delivery', 'tourist', 'event'];
/// `dowiz_hub::allergens::EU14`, in the regulation's order.
const EU14 = ['gluten', 'crustaceans', 'eggs', 'fish', 'peanuts', 'soy', 'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs'];
/// `record.rs::NOTE_MAX_CHARS` and `TABLE_MAX_CHARS`.
const NOTE_MAX = 280, TABLE_MAX = 16;

const WORDS = {
  sq: { privTitle: 'Privatësia', privHint: 'Tërheqja ndalon mesazhet e marketingut në çdo numër të lidhur. Harrimi fshin emrin, telefonin dhe adresën nga porositë, rezervimet dhe mesazhet në pritje; porositë dhe paratë mbeten.', withdrawConsent: 'Tërhiq pëlqimin', withdrawQ: 'Të ndalen mesazhet e marketingut për këtë klient, në çdo numër të tij?', withdrawn: 'Pëlqimi u tërhoq', forget: 'Harroje këtë klient', forgetQ: 'Të harrohet përgjithmonë ky klient? Kjo nuk kthehet mbrapsht.', forgetReason: 'Arsyeja (p.sh. kërkesa e klientit)', forgotten: 'Klienti u harrua', linkTitle: 'Lidhje', linkHint: 'Dy numra të të njëjtit person shfaqen si një rresht. Asgjë nuk bashkohet: shkëputja i kthen dy rreshtat.', linkTo: 'Lidh me...', linkPick: 'Zgjidh klientin që është i njëjti person', linkReason: 'Arsyeja e lidhjes', linkedKeys: 'Numra të lidhur në këtë rresht', unlink: 'Shkëput', linkDone: 'U lidh; hape listën përsëri', unlinkDone: 'U shkëput; hape listën përsëri',
        card: 'Kartela', cardHint: 'Vetëm çfarë do shkruanit në një kartelë letre. Shënimi mund t’i tregohet klientit nëse e kërkon.', cardNote: 'Shënim', cardTags: 'Etiketa', cardAllergens: 'Alergji (porosia refuzohet për pjatat që i përmbajnë)', cardTable: 'Tavolina e zakonshme', cardLang: 'Gjuha', cardBirthday: 'Ditëlindja (MM-DD, pa vit)', cardSave: 'Ruaj kartelën', cardSaved: 'Kartela u ruajt',
        tag_regular: 'i rregullt', tag_office_lunch: 'drekë zyre', tag_family: 'familje', tag_group: 'grup', tag_takeaway: 'me vete', tag_delivery: 'dërgesë', tag_tourist: 'turist', tag_event: 'eveniment',
        al_gluten: 'Gluten', al_crustaceans: 'Guaskorë', al_eggs: 'Vezë', al_fish: 'Peshk', al_peanuts: 'Kikirikë', al_soy: 'Soja', al_milk: 'Qumësht', al_nuts: 'Arra', al_celery: 'Selino', al_mustard: 'Mustardë', al_sesame: 'Susam', al_sulphites: 'Sulfite', al_lupin: 'Lupin', al_molluscs: 'Molusqe' },
  en: { privTitle: 'Privacy', privHint: 'Withdrawing stops marketing messages on every linked number. Forgetting removes the name, phone and address from orders, bookings and waiting messages; the orders and the money stay.', withdrawConsent: 'Withdraw consent', withdrawQ: 'Stop marketing messages to this customer, on every number of theirs?', withdrawn: 'Consent withdrawn', forget: 'Forget this customer', forgetQ: 'Forget this customer for good? This cannot be undone.', forgetReason: 'Reason (e.g. the customer asked)', forgotten: 'Customer forgotten', linkTitle: 'Links', linkHint: 'Two numbers of one person are shown as one row. Nothing is merged: unlinking brings the two rows back.', linkTo: 'Link to...', linkPick: 'Pick the customer who is the same person', linkReason: 'Reason for linking', linkedKeys: 'Numbers linked into this row', unlink: 'Unlink', linkDone: 'Linked; reopen the list', unlinkDone: 'Unlinked; reopen the list',
        card: 'Card', cardHint: 'Only what you would write on a paper card. The note can be shown to the customer if they ask.', cardNote: 'Note', cardTags: 'Tags', cardAllergens: 'Allergies (orders with dishes that contain them are refused)', cardTable: 'Usual table', cardLang: 'Language', cardBirthday: 'Birthday (MM-DD, no year)', cardSave: 'Save card', cardSaved: 'Card saved',
        tag_regular: 'regular', tag_office_lunch: 'office lunch', tag_family: 'family', tag_group: 'group', tag_takeaway: 'takeaway', tag_delivery: 'delivery', tag_tourist: 'tourist', tag_event: 'event',
        al_gluten: 'Gluten', al_crustaceans: 'Crustaceans', al_eggs: 'Eggs', al_fish: 'Fish', al_peanuts: 'Peanuts', al_soy: 'Soy', al_milk: 'Milk', al_nuts: 'Nuts', al_celery: 'Celery', al_mustard: 'Mustard', al_sesame: 'Sesame', al_sulphites: 'Sulphites', al_lupin: 'Lupin', al_molluscs: 'Molluscs' },
  uk: { privTitle: 'Приватність', privHint: 'Відкликання зупиняє маркетингові повідомлення на кожному зв\'язаному номері. Забування видаляє ім\'я, телефон і адресу із замовлень, бронювань і повідомлень у черзі; замовлення й гроші лишаються.', withdrawConsent: 'Відкликати згоду', withdrawQ: 'Зупинити маркетингові повідомлення цьому клієнтові, на всіх його номерах?', withdrawn: 'Згоду відкликано', forget: 'Забути клієнта', forgetQ: 'Забути цього клієнта назавжди? Це не можна скасувати.', forgetReason: 'Причина (напр. прохання клієнта)', forgotten: 'Клієнта забуто', linkTitle: 'Зв\'язки', linkHint: 'Два номери однієї людини показано одним рядком. Нічого не зливається: від\'єднання повертає два рядки.', linkTo: 'Зв\'язати з...', linkPick: 'Оберіть клієнта, який є тією самою людиною', linkReason: 'Причина зв\'язку', linkedKeys: 'Номери, зв\'язані з цим рядком', unlink: 'Від\'єднати', linkDone: 'Зв\'язано; відкрийте список знову', unlinkDone: 'Від\'єднано; відкрийте список знову',
        card: 'Картка', cardHint: 'Лише те, що ви записали б на паперовій картці. Нотатку можна показати клієнтові на його прохання.', cardNote: 'Нотатка', cardTags: 'Мітки', cardAllergens: 'Алергії (замовлення зі стравами, що їх містять, відхиляється)', cardTable: 'Звичний стіл', cardLang: 'Мова', cardBirthday: 'День народження (ММ-ДД, без року)', cardSave: 'Зберегти картку', cardSaved: 'Картку збережено',
        tag_regular: 'постійний', tag_office_lunch: 'офісний обід', tag_family: 'сім’я', tag_group: 'група', tag_takeaway: 'з собою', tag_delivery: 'доставка', tag_tourist: 'турист', tag_event: 'подія',
        al_gluten: 'Глютен', al_crustaceans: 'Ракоподібні', al_eggs: 'Яйця', al_fish: 'Риба', al_peanuts: 'Арахіс', al_soy: 'Соя', al_milk: 'Молоко', al_nuts: 'Горіхи', al_celery: 'Селера', al_mustard: 'Гірчиця', al_sesame: 'Кунжут', al_sulphites: 'Сульфіти', al_lupin: 'Люпин', al_molluscs: 'Молюски' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// The card's fields, as one line under the row. Empty when there is no card.
export function cardLine(c){
  const bits = [];
  if (c.allergens?.length) bits.push('⚠ ' + c.allergens.map(a => t('al_' + a)).join(', '));
  if (c.tags?.length) bits.push(c.tags.map(x => t('tag_' + x)).join(', '));
  if (c.note) bits.push(c.note);
  return bits.join(' · ');
}

/// A closed list as toggle chips: pressed = on the card. `data-<name>` is what
/// the save reads back.
const toggles = (name, all, on, label, tour) => `<div class="chips" role="group" aria-label="${esc(t(name))}" data-tour="${tour}">${all.map(v =>
  ui.chip({ as: 'button', selected: on.includes(v), label: t(label + v), attrs: { data: { [name]: v } } })).join('')}</div>`;

/// Open one customer's card. `c` is the row from `/owner/customers`; `done`
/// is called with the saved record so the list can repaint without a refetch.
/// `forgotten` is called when the owner leaves the "forgotten" sheet: the row
/// no longer exists as it was, so the list has to be fetched again.
export function openCard(c, done, forgotten){
  sheet(`<p class="eyebrow" data-t="customers"></p><h2 data-t="card"></h2><p class="muted small" data-t="cardHint"></p>
    ${field({ id: 'cd-note', key: 'cardNote', rows: 3, maxlength: NOTE_MAX, value: c.note || '', tour: 'customers.note' })}
    <p class="eyebrow mt-3" data-t="cardAllergens"></p>${toggles('al', EU14, c.allergens || [], 'al_', 'customers.allergens')}
    <p class="eyebrow mt-3" data-t="cardTags"></p>${toggles('tag', TAGS, c.tags || [], 'tag_', 'customers.tags')}
    ${field({ id: 'cd-table', key: 'cardTable', maxlength: TABLE_MAX, value: c.usualTable || '', autocomplete: 'off', tour: 'customers.table' })}
    ${select({ id: 'cd-lang', key: 'cardLang', value: c.lang || '', options: [{ value: '', label: '' }, ...LANGS.map(l => ({ value: l, label: l }))], tour: 'customers.lang' })}
    ${field({ id: 'cd-bd', key: 'cardBirthday', inputmode: 'numeric', placeholder: 'MM-DD', maxlength: 5, value: c.birthdayMd || '', autocomplete: 'off', tour: 'customers.birthday' })}
    <div class="btn-row">${btn({ id: 'cdGo', variant: 'primary', icon: 'check', key: 'cardSave', tour: 'customers.save' })}</div>
    ${linksHtml(c)}${privacyHtml()}`, { name: 'card' });
  retranslate($('#sheetIn')); hydrate($('#sheetIn'));
  wireLinks(c);
  wirePrivacy(c, forgotten);
  for (const b of $$('[data-al],[data-tag]', $('#sheetIn'))) b.onclick = () => b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true'));
  $('#cdGo').onclick = async () => {
    const picked = k => $$(`[data-${k}]`, $('#sheetIn')).filter(i => i.getAttribute('aria-pressed') === 'true').map(i => i.dataset[k]);
    const body = { note: $('#cd-note').value, tags: picked('tag'), allergens: picked('al'),
                   usualTable: $('#cd-table').value, lang: $('#cd-lang').value, birthdayMd: $('#cd-bd').value.trim() };
    try {
      const r = await busy($('#cdGo'), () => api(`/owner/customers/${encodeURIComponent(c.key)}/record`, { method: 'PUT', body }));
      toast(t('cardSaved'));
      done?.(r.record || {});
    } catch (e) { toast(String(e.message || e)); }
  };
}

/// §3.4 LINK, NEVER MERGE. The keys shown under this row (`c.linked`), each
/// with an unlink, and a "link to..." that picks another row. Every link and
/// unlink is audited on both people, so each takes a written reason.
const LINK_REASON_MIN = 3;
function linksHtml(c){
  const linked = c.linked || [];
  return `<p class="eyebrow mt-3" data-t="linkTitle"></p><p class="muted small" data-t="linkHint"></p>
    ${field({ id: 'cd-lreason', key: 'linkReason', autocomplete: 'off', tour: 'customers.linkReason' })}
    ${linked.length ? `<p class="eyebrow mt-3" data-t="linkedKeys"></p><div class="rows">${linked.map(k => rowDiv({ leading: icon('user'), title: '', sub: `<span class="mono">${esc(k)}</span>`,
      trailing: btn({ variant: 'ghost', icon: 'x', key: 'unlink', data: { unlink: k }, tour: 'customers.unlink' }) })).join('')}</div>` : ''}
    <div class="btn-row">${btn({ id: 'cdLink', variant: 'ghost', icon: 'user-plus', key: 'linkTo', tour: 'customers.link' })}</div><div id="cdPick"></div>`;
}
function linkReason(){
  const r = $('#cd-lreason').value.trim();
  if (r.length < LINK_REASON_MIN) { toast(t('required')); $('#cd-lreason').focus(); return null; }
  return r;
}
function wireLinks(c){
  for (const b of $$('[data-unlink]', $('#sheetIn'))) b.onclick = async () => {
    const reason = linkReason(); if (!reason) return;
    try {
      await busy(b, () => post(`/owner/customers/${encodeURIComponent(b.dataset.unlink)}/unlink`, { reason }));
      b.closest('.ui-row').remove(); toast(t('unlinkDone'));
    } catch (e) { toast(String(e.message || e)); }
  };
  $('#cdLink').onclick = async () => {
    let d; try { d = await busy($('#cdLink'), () => api('/owner/customers?sort=recent')); } catch (e) { return toast(String(e.message || e)); }
    const mine = new Set([c.key, ...(c.linked || [])]);
    const others = (d.customers || []).filter(o => !mine.has(o.key));
    $('#cdPick').innerHTML = `<p class="eyebrow mt-3" data-t="linkPick"></p><div class="rows">${others.map(o => rowBtn({ data: { to: o.key }, tour: 'customers.linkPick', leading: icon('user'), title: o.name || o.phone || o.key,
      sub: `<span class="mono">${esc(o.phone || '')} · ${Number(o.orders) | 0}</span>`, trailing: icon('chevron-right', 'chev') })).join('')}</div>`;
    retranslate($('#cdPick'));
    for (const o of $$('[data-to]', $('#cdPick'))) o.onclick = async () => {
      const reason = linkReason(); if (!reason) return;
      try {
        await busy(o, () => post(`/owner/customers/${encodeURIComponent(c.key)}/link`, { to: o.dataset.to, reason }));
        $('#cdPick').innerHTML = ''; toast(t('linkDone'));
      } catch (e) { toast(String(e.message || e)); }
    };
  };
}

/// G8: THE TWO PRIVACY ACTS ON THE CARD. Both reach the person's whole alias
/// circle on the server (`consent_routes::owner_act`, `forget::forget_customer`);
/// the console sends the row's key and nothing else. Forget asks twice -- a
/// written reason, then a danger confirmation -- and shows the server's own
/// sentence about backups, in the console's language.
function privacyHtml(){
  return `<p class="eyebrow mt-3" data-t="privTitle"></p><p class="muted small" data-t="privHint"></p>
    <div class="btn-row">${ui.button({ id: 'cdWithdraw', variant: 'ghost', icon: 'x', label: { t: 'withdrawConsent' }, attrs: { data: { tour: 'customers.withdraw' } } })}
    ${ui.button({ id: 'cdForget', variant: 'danger', icon: 'trash', label: { t: 'forget' }, attrs: { data: { tour: 'customers.forget' } } })}</div>`;
}
function wirePrivacy(c, forgotten){
  const key = encodeURIComponent(c.key);
  $('#cdWithdraw').onclick = async () => {
    if (!(await confirm(t('privTitle'), t('withdrawQ')))) return;
    try { await post(`/owner/customers/${key}/consent`, { state: 'withdrawn' }); toast(t('withdrawn')); }
    catch (e) { toast(String(e.message || e)); }
  };
  $('#cdForget').onclick = async () => {
    const ok = await confirm(t('privTitle'), t('forgetQ'), { danger: true, reasonLabel: t('forgetReason') });
    if (!ok) return;
    if (ok.reason.length < LINK_REASON_MIN) return toast(t('required'));
    try {
      const r = await post(`/owner/customers/${key}/forget`, { reason: ok.reason, lang });
      toast(t('forgotten'));
      sheet(`<p class="eyebrow" data-t="privTitle"></p><h2 data-t="forgotten"></h2><p class="muted small">${esc(r.notice || '')}</p>
        <div class="btn-row">${ui.button({ id: 'cdDone', variant: 'primary', label: { t: 'done' } })}</div>`, { name: 'forgotten' });
      retranslate($('#sheetIn'));
      // Back to a FRESH list: the forgotten row's name and phone are gone on
      // the server, and the list on screen still shows them until refetched.
      $('#cdDone').onclick = () => (forgotten ? forgotten() : closeSheet());
    } catch (e) { toast(String(e.message || e)); }
  };
}
