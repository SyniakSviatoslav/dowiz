// Settings — Figma node 1:10015 (dark) / 1:20481 (light).
//
// Four rows. Three of them open a screen; "Theme" is the one that DOES
// something here, because the kit ships a complete light set and a complete
// dark set and the choice between them is this app's to make, not the phone's
// alone. The choice is written to `data-theme` on the root element, which is
// exactly the hook lib/figma.css already guards its dark block with.

import { icon, esc } from '/kit/app.js';
import { topBar, menuList } from '/kit/parts.js';
import { installed, canPrompt, promptInstall } from '/kit/install.js';

const THEMES = [
  { id: 'system', label: 'Як у системі' },
  { id: 'light',  label: 'Світла' },
  { id: 'dark',   label: 'Темна' },
];

// localStorage can throw in a private window and can come back empty, and the
// screen must render either way -- so every read and write is guarded and the
// default is the system's answer.
const KEY = 'dw_kit_theme';

function readTheme(){
  try { return THEMES.some(t => t.id === localStorage.getItem(KEY))
    ? localStorage.getItem(KEY) : 'system'; }
  catch { return 'system'; }
}

export function applyTheme(id){
  const root = document.documentElement;
  if (id === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', id);
  try { localStorage.setItem(KEY, id); } catch { /* private window */ }
}

const label = id => (THEMES.find(t => t.id === id) || THEMES[0]).label;

// The install row is shown only while there is something to install. Once the
// app IS installed the offer is noise, and a row that does nothing is worse
// than an absent one. It is here as well as in the banner because the banner
// can be dismissed for a month and this is where someone goes looking.
const rows = () => [
  ...(installed() ? [] : [{ icon: 'box', label: 'Встановити застосунок',
                            attrs: 'data-install-row' }]),
  { icon: 'notification-bing', label: 'Notification Settings', to: 'notification-access' },
  { icon: 'key',               label: 'Password Manager',      to: 'password-manager' },
  { icon: 'sun',               label: 'Theme', value: label(readTheme()),
    attrs: 'data-theme-row' },
  { icon: 'trash',             label: 'Delete Account',        to: 'logout', tone: 'danger' },
];

export function render(){
  return `
  ${topBar('Settings')}
  <div class="wrap k-settings">${menuList(rows())}</div>

  <div class="k-scrim" id="scrim" hidden></div>
  <div class="k-sheet" id="sheet" role="dialog" aria-modal="true"
       aria-label="Тема" hidden>
    <div class="k-sheet-grab" aria-hidden="true"></div>
    <h2 class="k-pick-h">Тема</h2>
    <div class="k-list">
      ${THEMES.map(t => `
        <button class="k-row" type="button" data-theme-set="${esc(t.id)}"
                aria-pressed="${t.id === readTheme()}">
          <span class="k-row-label">${esc(t.label)}</span>
          <span class="k-row-tick">${icon('check')}</span>
        </button>`).join('')}
    </div>
  </div>`;
}

export function bind(root){
  const sheet = root.querySelector('#sheet');
  const scrim = root.querySelector('#scrim');
  const open = on => { sheet.hidden = !on; scrim.hidden = !on; };

  root.addEventListener('click', async e => {
    const inst = e.target.closest('[data-install-row]');
    if (inst){
      // Chrome can show the real dialog. Everything else installs from its own
      // share menu, so the row says how instead of pretending to do it.
      if (canPrompt()){
        if (await promptInstall() === 'accepted') inst.remove();
      } else {
        const label = inst.querySelector('.k-row-label');
        if (label) label.textContent = 'Поділитися → На початковий екран';
        inst.setAttribute('aria-disabled', 'true');
      }
      return;
    }
    if (e.target.closest('[data-theme-row]')) return open(true);
    if (e.target.closest('#scrim')) return open(false);

    const pick = e.target.closest('[data-theme-set]');
    if (!pick) return;
    applyTheme(pick.dataset.themeSet);
    for (const b of root.querySelectorAll('[data-theme-set]'))
      b.setAttribute('aria-pressed', String(b === pick));
    const value = root.querySelector('[data-theme-row] .k-row-value');
    if (value) value.textContent = label(pick.dataset.themeSet);
    open(false);
  });

  // Escape closes the sheet: it is a dialog, and a dialog that only closes by
  // tapping the scrim is a trap on a keyboard.
  root.addEventListener('keydown', e => { if (e.key === 'Escape') open(false); });
}

// The choice has to survive a reload of any screen, not just this one, so it is
// applied the moment the module is first imported and again from the shell.
applyTheme(readTheme());
