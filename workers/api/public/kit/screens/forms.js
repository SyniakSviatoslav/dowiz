// Form screens — Your Profile 1:9481, Add Address 1:9552, Password Manager
// 1:9961, Add Money 1:9857, Add Card 1:3889, Complete Your Profile 1:2552.
// Light set: 1:19921, 1:19993, 1:20426, 1:20311, 1:14281, 1:12963.
//
// Six frames, one shape: 12/400 labels over filled fields, sometimes a prefix
// box, sometimes a row of choices, and one action at the foot. The copy is the
// file's own, read with design/spec.py <node> --text.
//
// These validate on submit. Not because the client is trusted -- dowiz's kernel
// checks everything again -- but because being told which field is wrong before
// a round trip is the difference between a form and a wall.

import { icon, esc, go, toast } from '/kit/app.js';
import { topBar, plate } from '/kit/parts.js';
import * as Me from '/kit/me.js';
import { menu, venue } from '/kit/data.js';

const FORMS = {
  'your-profile': {
    title: 'Your Profile', cta: 'Update', portrait: true,
    fields: [
      { id: 'name',  label: 'Name',          value: 'Jennifer Aaker' },
      { id: 'email', label: 'Email',         type: 'email', placeholder: 'example@gmail.com' },
      { id: 'phone', label: 'Phone Number',  type: 'tel', prefix: '+1', value: '(208) 555-0112' },
      { id: 'dob',   label: 'Date of Birth', value: '15/02/2002' },
      // "Change" was drawn beside this field as a button with nothing behind it
      // (`DEAD button.k-forgot "Change"`). The field itself is the control: a
      // native select is what a phone already knows how to present, and it needs
      // no second tap target to open.
      { id: 'gender', label: 'Gender', value: 'Female',
        options: ['Female', 'Male', 'Інше', 'Не вказувати'] },
    ],
  },

  'add-address': {
    title: 'Add Address', cta: 'Save Address',
    fields: [
      { id: 'address',  label: 'Complete address', placeholder: 'Enter address *', required: true },
      { id: 'floor',    label: 'Floor',    placeholder: 'Enter Floor' },
      { id: 'landmark', label: 'Landmark', placeholder: 'Enter Landmark' },
    ],
    choices: { label: 'Save address as *', id: 'as',
      options: ['Home', 'Parent’s House', 'Office', 'Friend’s House'], chosen: 'Home' },
  },

  'password-manager': {
    title: 'Password Manager', cta: 'Reset Password',
    fields: [
      { id: 'current', label: 'Current Password', type: 'password',
        placeholder: '••••••••••••', reveal: true, aside: 'Forgot Password?', asideTo: 'verify-code' },
      { id: 'next',    label: 'New Password', type: 'password',
        placeholder: '••••••••••••', reveal: true },
      { id: 'confirm', label: 'Confirm New Password', type: 'password',
        placeholder: '••••••••••••', reveal: true },
    ],
  },

  'add-card': {
    title: 'Add Card', cta: 'Add Card', card: true,
    fields: [
      { id: 'holder', label: 'Card Holder Name', value: 'Jennifer Aaker' },
      { id: 'number', label: 'Card Number', placeholder: '4716 9627 1635 8047',
        inputmode: 'numeric', required: true },
      { id: 'expiry', label: 'Expiry Date', placeholder: '02/30', required: true },
      { id: 'cvv',    label: 'CVV', placeholder: '000', inputmode: 'numeric',
        type: 'password', required: true },
    ],
  },

  'profile-complete': {
    title: 'Complete Your Profile', cta: 'Complete Profile', portrait: true,
    head: 'Complete Your Profile',
    body: 'Don’t worry, only you can see your personal data. No one else will be able to see it.',
    fields: [
      { id: 'name',   label: 'Name', placeholder: 'Ex. John Doe', required: true },
      { id: 'phone',  label: 'Phone Number', type: 'tel', prefix: '+1',
        placeholder: 'Enter Phone Number', required: true },
      { id: 'gender', label: 'Gender', placeholder: 'Select' },
    ],
  },
};

const state = { choice: new Map(), revealed: new Set() };

const field = f => `
  <div class="k-in">
    <label for="f-${esc(f.id)}">${esc(f.label)}</label>
    ${f.prefix ? '<span class="k-prefix">' : ''}
    ${f.prefix ? `<span>${esc(f.prefix)}</span>` : ''}
    ${f.options ? `
      <select id="f-${esc(f.id)}" name="${esc(f.id)}">
        ${f.options.map(o => `
          <option ${o === f.value ? 'selected' : ''}>${esc(o)}</option>`).join('')}
      </select>`
    : `
      <input id="f-${esc(f.id)}" name="${esc(f.id)}" type="${esc(f.type || 'text')}"
           ${f.value ? `value="${esc(f.value)}"` : ''}
           ${f.placeholder ? `placeholder="${esc(f.placeholder)}"` : ''}
           ${f.inputmode ? `inputmode="${esc(f.inputmode)}"` : ''}>`}
    ${f.prefix ? '</span>' : ''}
    ${f.reveal ? `
      <button class="k-in-eye" type="button" data-reveal="f-${esc(f.id)}"
              aria-label="Показати пароль" aria-pressed="false">${icon('eye-slash')}</button>` : ''}
    ${f.aside ? `<button class="k-forgot" type="button"
        ${f.asideTo ? `data-go="${esc(f.asideTo)}"` : ''}>${esc(f.aside)}</button>` : ''}
    <span class="k-in-err" hidden></span>
  </div>`;

// The card preview on Add Card: the numbers as typed, so the field and the card
// cannot show two different answers.
const cardPreview = () => `
  <div class="k-offer k-card-preview" aria-hidden="true">
    <span class="k-offer-tag" id="pvHolder">Jennifer Aaker</span>
    <span class="k-card-num" id="pvNumber">4716 9627 1635 8047</span>
    <span class="k-card-foot">
      <span><small>Card holder name</small><b id="pvHolder2">Jennifer Aaker</b></span>
      <span><small>Expiry date</small><b id="pvExpiry">02/30</b></span>
    </span>
  </div>`;

export async function render(params, routeName = 'your-profile'){
  const f = mine(await real(FORMS[routeName] || FORMS['your-profile'], routeName), routeName);
  if (f.choices && !state.choice.has(routeName)) state.choice.set(routeName, f.choices.chosen);
  return `
  ${topBar(f.title)}
  <div class="wrap k-page" data-form="${esc(routeName)}">
    ${f.head ? `<h1 class="k-page-h">${esc(f.head)}</h1>` : ''}
    ${f.body ? `<p class="k-page-p">${esc(f.body)}</p>` : ''}

    ${f.portrait ? `
      <div class="k-me-photo k-me-lg">
        ${plate('Jennifer Aaker')}
        <img class="k-me-img" id="meImg" alt="" hidden>
        <button class="k-me-edit" type="button" id="mePick" data-handoff="file"
                aria-label="Змінити фото">${icon('edit-2')}</button>
        <input id="meFile" type="file" accept="image/*" hidden aria-hidden="true" tabindex="-1">
      </div>` : ''}

    ${f.card ? cardPreview() : ''}

    <form class="k-form-page" novalidate>
      ${f.fields.map(field).join('')}

      ${f.choices ? `
        <div class="k-in">
          <label>${esc(f.choices.label)}</label>
          <div class="k-choice-row" role="group">
            ${f.choices.options.map(o => `
              <button class="k-choice-pill" type="button" data-choice="${esc(o)}"
                      aria-pressed="${o === state.choice.get(routeName)}">${esc(o)}</button>`).join('')}
          </div>
        </div>` : ''}

      <button class="k-submit" type="submit">${esc(f.cta)}</button>
    </form>
  </div>`;
}

export function bind(root){
  const host = root.querySelector('[data-form]');
  const routeName = host.dataset.form;
  const f = FORMS[routeName];

  // THE PORTRAIT IS THE PHONE'S CAMERA ROLL. The pencil used to be a button
  // with nothing behind it (`DEAD button.k-me-edit`, on both screens that draw
  // it). It opens the picker, and the picture it comes back with is shown and
  // kept ON THIS DEVICE: dowiz is local-first, and a customer's face is not a
  // thing this app needs to upload anywhere to draw.
  portrait(root);

  root.addEventListener('click', e => {
    const eye = e.target.closest('[data-reveal]');
    if (eye){
      const input = root.querySelector('#' + CSS.escape(eye.dataset.reveal));
      const shown = input.type === 'text';
      input.type = shown ? 'password' : 'text';
      eye.setAttribute('aria-pressed', String(!shown));
      eye.setAttribute('aria-label', shown ? 'Показати пароль' : 'Сховати пароль');
      return;
    }
    const pill = e.target.closest('[data-choice]');
    if (pill){
      state.choice.set(routeName, pill.dataset.choice);
      for (const b of root.querySelectorAll('[data-choice]'))
        b.setAttribute('aria-pressed', String(b.dataset.choice === state.choice.get(routeName)));
    }
  });

  // The card preview follows what is typed.
  if (f.card){
    root.addEventListener('input', e => {
      const map = { number: 'pvNumber', holder: 'pvHolder', expiry: 'pvExpiry' };
      const id = e.target.id?.replace(/^f-/, '');
      if (!map[id]) return;
      const out = root.querySelector('#' + map[id]);
      if (out) out.textContent = e.target.value || out.dataset.was || out.textContent;
      if (id === 'holder'){
        const second = root.querySelector('#pvHolder2');
        if (second) second.textContent = e.target.value;
      }
    });
  }

  root.querySelector('form')?.addEventListener('submit', e => {
    e.preventDefault();
    let ok = true;
    for (const x of f.fields){
      const input = root.querySelector('#f-' + CSS.escape(x.id));
      const err = input.parentElement.querySelector('.k-in-err')
        || input.closest('.k-in').querySelector('.k-in-err');
      const why = complain(x, input.value, root);
      err.textContent = why;
      err.hidden = !why;
      if (why) ok = false;
    }
    if (!ok) return;

    /// A FORM THAT VALIDATES AND FORGETS IS NOT A FORM.
    ///
    /// Every one of these used to end at `history.back()`, whatever had been
    /// typed into it. Two of them are the only place a customer can say where
    /// they live and what to call them, so checkout had one address list — the
    /// four New York examples in the Figma frame — and sent every order with an
    /// empty contact. What a form is FOR is now written down before it closes.
    keep(routeName, root);
    history.length > 1 ? history.back() : go('profile');
  });
}

/// THE VENUE'S COUNTRY DIALS THE PHONE, not the frame's.
///
/// `+1` is drawn in the prefix box because the Figma profile belongs to Jennifer
/// Aaker in New York. A customer of a venue in Durrës typed `69 234 5678` into
/// it and the order went out carrying `+1 69 234 5678`: a number that is not
/// theirs, in a country the venue does not deliver to, and it passes the hub's
/// check because that only counts digits. The venue publishes its own number, so
/// its dialling code is read from there.
///
/// The frame's NAME and NUMBER stop being values too. They are filled in, not
/// placeheld, so a customer who never edits them orders as `Jennifer Aaker` on
/// `(208) 555-0112` — the design's content posing as a person's answer. With a
/// real venue behind the screen they become hints, and an unanswered field is
/// visibly unanswered.
async function real(f, routeName){
  if (routeName !== 'your-profile' && routeName !== 'profile-complete') return f;
  const place = venue(await menu('uk').catch(() => null));
  if (!place) return f;
  const dial = /^\+\d{1,4}/.exec(place.phone || '')?.[0];
  return { ...f, fields: f.fields.map(x => {
    const y = x.id === 'phone' && dial ? { ...x, prefix: dial } : { ...x };
    if ((y.id === 'name' || y.id === 'phone') && y.value){
      y.placeholder = y.placeholder || y.value;
      delete y.value;
    }
    return y;
  }) };
}

/// THE CUSTOMER'S OWN ANSWERS, not the designer's.
///
/// The frame fills the profile with `Jennifer Aaker` and a New York number,
/// which is right for a design and wrong for a person who has already typed
/// their own. A form that shows somebody else's name back to you is one people
/// correct by hand every time, and eventually order with whatever was left.
function mine(f, routeName){
  if (routeName !== 'your-profile' && routeName !== 'profile-complete') return f;
  const me = Me.contact();
  if (!me.name && !me.phone) return f;
  const prefix = f.fields.find(x => x.id === 'phone')?.prefix || '';
  // The prefix is drawn in its own box, so the field holds the rest of the
  // number -- writing the whole thing back would print `+1` twice.
  const local = prefix && me.phone.startsWith(prefix)
    ? me.phone.slice(prefix.length).trim() : me.phone;
  return { ...f, fields: f.fields.map(x =>
    x.id === 'name'  && me.name  ? { ...x, value: me.name }
  : x.id === 'phone' && me.phone ? { ...x, value: local }
  : x) };
}

function value(root, id){
  return (root.querySelector('#f-' + CSS.escape(id))?.value || '').trim();
}

function keep(routeName, root){
  if (routeName === 'add-address'){
    // Floor and landmark are one thing to the person at the door, so they
    // travel as the order's note rather than as fields the hub has no place for.
    const note = [value(root, 'floor'), value(root, 'landmark')].filter(Boolean).join(', ');
    const saved = Me.addAddress({
      label: state.choice.get(routeName) || 'Home',
      line: value(root, 'address'),
      note,
    });
    toast(saved ? `${saved.label} — збережено` : 'Адресу не вдалося зберегти');
    return;
  }

  if (routeName === 'your-profile' || routeName === 'profile-complete'){
    // The prefix box is part of the number: the code beside `69 234 5678` is one
    // phone, and an order carrying only the second half cannot be dialled.
    //
    // It is read FROM THE SCREEN, not from `FORMS`: the table still holds the
    // frame's `+1`, and `real()` replaces it with the venue's own dialling code
    // before the field is drawn. Reading the table back here is how the fix got
    // written, ran, and changed nothing — the order still went out as `+1`.
    const prefix = root.querySelector('.k-prefix span')?.textContent?.trim() || '';
    const phone = value(root, 'phone');
    Me.setContact({ name: value(root, 'name'), phone: phone && prefix ? `${prefix} ${phone}` : phone });
    toast('Збережено');
  }
}

function complain(f, value, root){
  const v = (value || '').trim();
  if (f.required && !v) return 'Обов’язкове поле';
  if (!v) return '';
  if (f.type === 'email' && !/^[^@\s]+@[^@\s.]+\.[^@\s]+$/.test(v)) return 'Невірна пошта';
  if (f.type === 'tel' && v.replace(/\D/g, '').length < 7) return 'Невірний номер';
  if (f.id === 'number' && v.replace(/\D/g, '').length !== 16) return 'Потрібно 16 цифр';
  if (f.id === 'expiry' && !/^\d{2}\/\d{2}$/.test(v)) return 'Формат ММ/РР';
  if (f.id === 'cvv' && !/^\d{3,4}$/.test(v)) return '3 або 4 цифри';
  if (f.id === 'next' && v.length < 8) return 'Щонайменше 8 символів';
  if (f.id === 'confirm'){
    const first = root.querySelector('#f-next');
    if (first && first.value !== v) return 'Паролі не збігаються';
  }
  return '';
}

const AVATAR = 'dw_kit_avatar';

function portrait(root){
  const pick = root.querySelector('#mePick');
  const file = root.querySelector('#meFile');
  const img  = root.querySelector('#meImg');
  if (!pick || !file || !img) return;

  const show = src => { img.src = src; img.hidden = false; };
  try { const saved = localStorage.getItem(AVATAR); if (saved) show(saved); }
  catch { /* private window: the portrait is simply the plate */ }

  pick.addEventListener('click', () => file.click());

  file.addEventListener('change', () => {
    const chosen = file.files && file.files[0];
    if (!chosen) return;
    const reader = new FileReader();
    reader.onload = () => {
      const src = String(reader.result);
      show(src);
      try { localStorage.setItem(AVATAR, src); toast('Фото збережене на цьому пристрої'); }
      catch { toast('Фото показане, але не збережене: сховище недоступне'); }
    };
    reader.onerror = () => toast('Не вдалося прочитати файл');
    reader.readAsDataURL(chosen);
  });
}
