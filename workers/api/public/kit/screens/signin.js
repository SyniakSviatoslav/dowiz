// Sign in — Figma node 1:2305 (dark) / 1:12718 (light).
//
// Also renders Create Account (1:2379) and New Password (1:2507): the file
// draws one card with a different field list and heading, and one module that
// takes the field list is what keeps the three from drifting apart. The route
// names which: `#/signin`, `#/create-account`, `#/new-password`.

import { icon, esc, go, toast } from '/kit/app.js';

const FORMS = {
  signin: {
    head: 'Let’s get you Login!',
    sub: 'Hi! Welcome back, you’ve been missed',
    social: true,
    submit: 'Sign In',
    forgot: true,
    swap: { text: 'Don’t have an account? ', link: 'Sign Up', to: 'create-account' },
    fields: [
      { id: 'email', label: 'Email', type: 'email', placeholder: 'example@gmail.com',
        autocomplete: 'username' },
      { id: 'password', label: 'Password', type: 'password', placeholder: '••••••••••••',
        autocomplete: 'current-password', reveal: true },
    ],
  },
  'create-account': {
    head: 'Create your Account',
    sub: 'Just a few details and you’re in',
    social: true,
    submit: 'Sign Up',
    swap: { text: 'Already have an account? ', link: 'Sign In', to: 'signin' },
    fields: [
      { id: 'name', label: 'Full Name', type: 'text', placeholder: 'Jane Doe',
        autocomplete: 'name' },
      { id: 'email', label: 'Email', type: 'email', placeholder: 'example@gmail.com',
        autocomplete: 'username' },
      { id: 'phone', label: 'Phone', type: 'tel', placeholder: '+355 …',
        autocomplete: 'tel' },
      { id: 'password', label: 'Password', type: 'password', placeholder: '••••••••••••',
        autocomplete: 'new-password', reveal: true },
    ],
  },
  'new-password': {
    head: 'Set a New Password',
    sub: 'It must be different from the old one',
    social: false,
    submit: 'Save Password',
    swap: { text: 'Remembered it? ', link: 'Sign In', to: 'signin' },
    fields: [
      { id: 'password', label: 'New Password', type: 'password', placeholder: '••••••••••••',
        autocomplete: 'new-password', reveal: true },
      { id: 'confirm', label: 'Confirm Password', type: 'password', placeholder: '••••••••••••',
        autocomplete: 'new-password', reveal: true },
    ],
  },
};

const which = params => FORMS[params?.get('as')] ? params.get('as') : null;

export function render(params, routeName = 'signin'){
  const key = which(params) || (FORMS[routeName] ? routeName : 'signin');
  const f = FORMS[key];
  return `
  <div class="k-auth-hero" aria-hidden="true"></div>
  <div class="k-auth" data-form="${esc(key)}">
    <span class="k-auth-logo">${icon('logo-union')}</span>
    <h1 class="k-auth-h">${esc(f.head)}</h1>
    <p class="k-auth-sub">${esc(f.sub)}</p>

    <div class="k-card">
      ${f.social ? `
<!-- THESE THREE CANNOT WORK, AND THEY SAY SO.
           There is no OAuth of any kind on the server — no Apple, no Google, no
           Facebook, checked against workers/api/src — so a button here that
           opened a spinner would be a lie with better manners. They keep the
           frame's layout, they are marked as unavailable for a screen reader,
           and tapping one says which piece is missing rather than doing
           nothing at all, which is what they did before. -->
      <div class="k-social">
        ${[['Apple', 'social-ring-dark'], ['Google', 'brand-google'],
           ['Facebook', 'brand-facebook']].map(([who, ico]) => `
          <button type="button" class="is-off" aria-disabled="true"
                  data-social="${esc(who)}"
                  aria-label="Увійти через ${esc(who)} — ще не підключено">
            ${icon(ico)}
          </button>`).join('')}
      </div>
      <p class="k-or">Or sign in with</p>` : ''}

      <form class="k-form" novalidate>
        ${f.fields.map(x => `
          <div class="k-in">
            <label for="f-${esc(x.id)}">${esc(x.label)}</label>
            <input id="f-${esc(x.id)}" name="${esc(x.id)}" type="${esc(x.type)}"
                   placeholder="${esc(x.placeholder)}" autocomplete="${esc(x.autocomplete)}"
                   ${x.type === 'email' ? 'inputmode="email"' : ''}
                   ${x.type === 'tel' ? 'inputmode="tel"' : ''}>
            ${x.reveal ? `
              <button class="k-in-eye" type="button" data-reveal="f-${esc(x.id)}"
                      aria-label="Показати пароль" aria-pressed="false">
                ${icon('eye-slash')}
              </button>` : ''}
            <span class="k-in-err" hidden></span>
          </div>`).join('')}
        ${f.forgot ? `<button class="k-forgot" type="button" data-go="verify-code">
          Forgot Password?</button>` : ''}
        <button class="k-submit" type="submit">${esc(f.submit)}</button>
      </form>

      <p class="k-swap">${esc(f.swap.text)}<button type="button"
         data-go="${esc(f.swap.to)}">${esc(f.swap.link)}</button></p>
    </div>
  </div>`;
}

export function bind(root){
  root.addEventListener('click', e => {
    const social = e.target.closest('[data-social]');
    if (social){
      toast(`Вхід через ${social.dataset.social} ще не підключений`);
      return;
    }

    const eye = e.target.closest('[data-reveal]');
    if (!eye) return;
    const input = root.querySelector('#' + CSS.escape(eye.dataset.reveal));
    const shown = input.type === 'text';
    input.type = shown ? 'password' : 'text';
    eye.setAttribute('aria-pressed', String(!shown));
    eye.setAttribute('aria-label', shown ? 'Показати пароль' : 'Сховати пароль');
  });

  // The form validates here because the screen must WORK, not because the
  // client is trusted: the API checks everything again. What this buys is a
  // customer who is told which field is wrong before a round trip.
  root.querySelector('form')?.addEventListener('submit', e => {
    e.preventDefault();
    const key = root.querySelector('[data-form]').dataset.form;
    let ok = true;
    for (const x of FORMS[key].fields){
      const input = root.querySelector('#f-' + CSS.escape(x.id));
      const err = input.parentElement.querySelector('.k-in-err');
      const why = complain(x, input.value, root);
      err.textContent = why || '';
      err.hidden = !why;
      if (why) ok = false;
    }
    if (ok) go('home');
  });
}

function complain(field, value, root){
  const v = value.trim();
  if (!v) return 'Обов’язкове поле';
  if (field.type === 'email' && !/^[^@\s]+@[^@\s.]+\.[^@\s]+$/.test(v)) return 'Невірна пошта';
  if (field.type === 'tel' && v.replace(/\D/g, '').length < 8) return 'Невірний номер';
  if (field.type === 'password' && v.length < 8) return 'Щонайменше 8 символів';
  if (field.id === 'confirm'){
    const first = root.querySelector('#f-password');
    if (first && first.value !== v) return 'Паролі не збігаються';
  }
  return '';
}
