// The living style guide: every /lib/ui component, in every state, in the
// three languages and both themes. No real data -- every name, address and
// amount below is invented -- and nothing here talks to the network.
//
// It is built with the same calls a surface makes, so what it shows is what a
// surface gets. A component that looks wrong here looks wrong everywhere.
import * as ui from '/lib/ui/index.js';
import { formatter } from '/lib/money.js';

const W = {
  en: { title: 'dowiz design system', lead: 'Every component, every state. Pure functions from options to escaped HTML; behaviour binds to an element.',
    language: 'Language', theme: 'Theme', system: 'System', light: 'Light', dark: 'Dark',
    buttons: 'Buttons', badges: 'Badges and status', chips: 'Chips', fields: 'Fields', choice: 'Segmented and tabs',
    rows: 'Rows and lists', states: 'Empty, failure, loading', feedback: 'Toast and sheet', money: 'Money, stats, time', cards: 'Cards',
    save: 'Save', deliver: 'Delivered', cancel: 'Cancel', back: 'Back', refund: 'Refund', saving: 'Saving…', busy: 'Try busy',
    paid: 'Paid online', unsent: '3 unsent', offered: 'Offered to you', late: 'Late', ready: 'Ready', onTheWay: 'On the way',
    onShift: 'On shift', gps: 'GPS weak', filter: 'Vegetarian',
    email: 'Email or phone', password: 'Password', pwHint: 'At least 8 characters.', cash: 'Cash received', note: 'Note',
    noteHint: 'e.g. nobody opened the door', badEmail: 'That address has no @', ask: 'Ask about my deliveries…', send: 'Send',
    day: 'Day', week: 'Week', month: 'Month', orders: 'Orders', menu: 'Menu', stock: 'Stock',
    street: 'Rruga Tregtare 5', street2: 'Bulevardi Epidamn 12', noneFree: 'No free orders', noneHint: 'As soon as something is ready, it appears here',
    noLink: 'No connection to the venue', retry: 'Try again', loading: 'Loading',
    showToast: 'Show a toast', toastText: 'Saved. The kitchen has it.', openSheet: 'Open a sheet', confirm: 'Ask to confirm',
    sheetTitle: 'Refund this order?', sheetBody: 'The customer gets the full amount back. This cannot be undone.',
    cashInHand: 'Cash in hand', tips: 'Tips', today: 'Today', offer: 'New order', timeLeft: 'Time left' },
  sq: { title: 'Sistemi i dizajnit dowiz', lead: 'Çdo komponent, çdo gjendje. Funksione të pastra nga opsionet në HTML të shpëtuar; sjellja lidhet me një element.',
    language: 'Gjuha', theme: 'Tema', system: 'Sistemi', light: 'E çelët', dark: 'E errët',
    buttons: 'Butonat', badges: 'Etiketat dhe statusi', chips: 'Çipat', fields: 'Fushat', choice: 'Zgjedhje dhe skeda',
    rows: 'Rreshtat dhe listat', states: 'Bosh, dështim, ngarkim', feedback: 'Njoftim dhe fletë', money: 'Para, shifra, kohë', cards: 'Kartat',
    save: 'Ruaj', deliver: 'U dorëzua', cancel: 'Anulo', back: 'Prapa', refund: 'Rimburso', saving: 'Po ruhet…', busy: 'Provo në pritje',
    paid: 'Paguar online', unsent: '3 të padërguara', offered: 'Ju ofrohet', late: 'Vonë', ready: 'Gati', onTheWay: 'Në rrugë',
    onShift: 'Në turn', gps: 'GPS i dobët', filter: 'Vegjetariane',
    email: 'Email ose telefon', password: 'Fjalëkalimi', pwHint: 'Të paktën 8 shenja.', cash: 'Para të marra', note: 'Shënim',
    noteHint: 'p.sh. askush nuk hapi derën', badEmail: 'Adresa nuk ka @', ask: 'Pyet për dërgesat e mia…', send: 'Dërgo',
    day: 'Dita', week: 'Java', month: 'Muaji', orders: 'Porositë', menu: 'Menuja', stock: 'Stoku',
    street: 'Rruga Tregtare 5', street2: 'Bulevardi Epidamn 12', noneFree: 'S’ka porosi të lira', noneHint: 'Sapo diçka të jetë gati, shfaqet këtu',
    noLink: 'S’ka lidhje me lokalin', retry: 'Provo përsëri', loading: 'Po ngarkohet',
    showToast: 'Shfaq një njoftim', toastText: 'U ruajt. Kuzhina e ka.', openSheet: 'Hap një fletë', confirm: 'Kërko konfirmim',
    sheetTitle: 'Ta rimbursoj porosinë?', sheetBody: 'Klienti merr gjithë shumën. Kjo nuk kthehet mbrapsht.',
    cashInHand: 'Para në dorë', tips: 'Bakshishe', today: 'Sot', offer: 'Porosi e re', timeLeft: 'Koha e mbetur' },
  uk: { title: 'Дизайн-система dowiz', lead: 'Кожен компонент, кожен стан. Чисті функції з опцій у екранований HTML; поведінка прив’язується до елемента.',
    language: 'Мова', theme: 'Тема', system: 'Системна', light: 'Світла', dark: 'Темна',
    buttons: 'Кнопки', badges: 'Бейджі та статус', chips: 'Чипи', fields: 'Поля', choice: 'Перемикачі та вкладки',
    rows: 'Рядки та списки', states: 'Порожньо, збій, завантаження', feedback: 'Сповіщення та аркуш', money: 'Гроші, показники, час', cards: 'Картки',
    save: 'Зберегти', deliver: 'Доставлено', cancel: 'Скасувати', back: 'Назад', refund: 'Повернути', saving: 'Записуємо…', busy: 'Спробувати очікування',
    paid: 'Оплачено онлайн', unsent: '3 не надіслано', offered: 'Пропонують вам', late: 'Запізнюється', ready: 'Готове', onTheWay: 'В дорозі',
    onShift: 'На зміні', gps: 'Слабкий GPS', filter: 'Вегетаріанське',
    email: 'Email або телефон', password: 'Пароль', pwHint: 'Щонайменше 8 символів.', cash: 'Отримано готівки', note: 'Примітка',
    noteHint: 'напр. ніхто не відчинив', badEmail: 'В адресі немає @', ask: 'Спитати про мої доставки…', send: 'Надіслати',
    day: 'День', week: 'Тиждень', month: 'Місяць', orders: 'Замовлення', menu: 'Меню', stock: 'Склад',
    street: 'Rruga Tregtare 5', street2: 'Bulevardi Epidamn 12', noneFree: 'Вільних замовлень немає', noneHint: 'Щойно щось буде готове — з’явиться тут',
    noLink: 'Немає зв’язку із закладом', retry: 'Спробувати ще раз', loading: 'Завантажуємо',
    showToast: 'Показати сповіщення', toastText: 'Збережено. Кухня вже бачить.', openSheet: 'Відкрити аркуш', confirm: 'Попросити підтвердження',
    sheetTitle: 'Повернути кошти за замовлення?', sheetBody: 'Клієнт отримає всю суму. Це не можна скасувати.',
    cashInHand: 'Готівка на руках', tips: 'Чайові', today: 'Сьогодні', offer: 'Нове замовлення', timeLeft: 'Залишилось' },
};
const LOCALE = { en: 'en-GB', sq: 'sq-AL', uk: 'uk-UA' };
const THEMES = ['', 'light', 'dark'];

const store = (() => { try { return globalThis.localStorage; } catch { return null; } })();
const get = k => { try { return store?.getItem(k) || ''; } catch { return ''; } };
const put = (k, v) => { try { store?.setItem(k, v); } catch { /* private window */ } };

let lang = W[get('ui-gallery-lang')] ? get('ui-gallery-lang') : 'en';
let theme = THEMES.includes(get('ui-gallery-theme')) ? get('ui-gallery-theme') : '';
const t = k => W[lang][k] ?? W.en[k] ?? k;
ui.useTranslator(t);
const k = key => ({ t: key });
const money = n => formatter({ base: 'ALL', display: 'ALL', rates: null, locale: lang })(n);
// A fixed instant, so the page renders the same every time: 24 Sep 2026 14:05 UTC.
const AT = Date.UTC(2026, 8, 24, 14, 5);
const XSS = '<img src=x onerror=alert(1)>';

const demo = (id, title, body) => `<section class="g-sec" id="${id}" aria-labelledby="${id}-h">
  <h2 class="g-h" id="${id}-h" data-t="${title}">${ui.esc(t(title))}</h2><div class="g-body">${body}</div></section>`;
const specimen = (name, html) => `<figure class="g-spec"><div class="g-stage">${html}</div><figcaption class="g-cap">${ui.esc(name)}</figcaption></figure>`;

function sections(){
  const buttons = ui.VARIANTS.map(v => specimen(`button · ${v}`, ui.button({ variant: v, label: k(v === 'danger' ? 'refund' : v === 'ghost' ? 'back' : 'save'), icon: v === 'ghost' ? 'arrow-left' : 'check' }))).join('')
    + specimen('size lg · block', ui.button({ variant: 'success', size: 'lg', block: true, icon: 'circle-check', label: k('deliver') }))
    + specimen('disabled', ui.button({ variant: 'primary', label: k('save'), disabled: true }))
    + specimen('busy', ui.button({ variant: 'primary', busy: true, busyLabel: k('saving'), label: k('save') }))
    + specimen('pressed', ui.button({ variant: 'secondary', label: k('filter'), pressed: true }))
    + specimen('href', ui.button({ href: '#buttons', variant: 'secondary', icon: 'external-link', label: k('back') }))
    + specimen('iconButton', ui.iconButton({ icon: 'send', ariaLabel: k('send') }) + ui.iconButton({ icon: 'x', ariaLabel: k('cancel'), variant: 'plain' }))
    + specimen('setBusy()', ui.button({ id: 'gBusy', variant: 'primary', icon: 'player-play', label: k('busy') }));
  const badges = ui.TONES.map(tn => specimen(`badge · ${tn}`, ui.badge({ tone: tn, label: k(tn === 'danger' ? 'late' : tn === 'success' ? 'paid' : tn === 'warning' ? 'unsent' : 'offered'), dot: tn === 'accent' }))).join('')
    + specimen('status · READY', ui.status({ status: 'READY', label: k('ready') }))
    + specimen('status · IN_DELIVERY pulse', ui.status({ status: 'IN_DELIVERY', label: k('onTheWay'), pulse: true }))
    + specimen('escaped', ui.badge({ label: XSS }));
  const chips = specimen('readout · dot · success', ui.chip({ label: k('onShift'), dot: true, tone: 'success' }))
    + specimen('readout · warning', ui.chip({ label: k('gps'), icon: 'gps', tone: 'warning' }))
    + specimen('floating', `<div class="g-map">${ui.chip({ label: k('onShift'), dot: true, floating: true })}</div>`)
    + specimen('toggle', ui.chip({ as: 'button', label: k('filter'), selected: false }) + ui.chip({ as: 'button', label: k('filter'), selected: true }));
  const fields = specimen('field', ui.field({ id: 'gEm', label: k('email'), autocomplete: 'off' }))
    + specimen('hint', ui.field({ id: 'gPw', label: k('password'), type: 'password', hint: k('pwHint') }))
    + specimen('error', ui.field({ id: 'gBad', label: k('email'), value: 'ana.example.com', error: k('badEmail') }))
    + specimen('money', ui.field({ id: 'gCash', label: k('cash'), money: true, value: 1500 }))
    + specimen('textarea', ui.field({ id: 'gNote', label: k('note'), rows: 3, placeholder: k('noteHint') }))
    + specimen('inputRow', ui.inputRow({ id: 'gAsk', label: k('ask'), placeholder: k('ask'), action: ui.iconButton({ icon: 'send', ariaLabel: k('send') }) }))
    + specimen('alert · danger / info', ui.alert({ label: k('noLink') }) + ui.alert({ label: k('pwHint'), tone: 'info' }));
  const choice = specimen('segmented', ui.segmented({ id: 'gSeg', label: k('day'), value: 'week',
      options: ['day', 'week', 'month'].map(v => ({ value: v, label: k(v) })) }))
    + specimen('tabs', ui.tabs({ id: 'gTabs', label: k('orders'), active: 'gt1', items: [
        { id: 'gt1', label: k('orders'), badge: 3 }, { id: 'gt2', label: k('menu') }, { id: 'gt3', label: k('stock') }] })
      + ['gt1', 'gt2', 'gt3'].map((id, i) => `<p class="ui-hint g-panel" id="${id}-panel" role="tabpanel"${i ? ' hidden' : ''}>${ui.esc(t(['orders', 'menu', 'stock'][i]))}</p>`).join(''));
  const rows = specimen('plain · inset', ui.list([
      ui.row({ title: k('today'), trailing: ui.amount(money(4500), { strong: true }) }),
      ui.row({ title: k('street'), sub: '24 Sep · DELIVERED', trailing: ui.amount(money(1200)) })], { inset: true }))
    + specimen('selectable', ui.list([
      ui.row({ select: true, pressed: true, title: '#a1b2c3d4', sub: t('street'), trailing: ui.amount(money(1500), { strong: true }) }),
      ui.row({ select: true, pressed: false, title: '#e5f6a7b8', sub: t('street2'), trailing: ui.amount(money(900), { strong: true }) })], { label: t('ready') }))
    + specimen('escaped', ui.list([ui.row({ title: XSS, sub: XSS })]));
  const states = specimen('empty', ui.emptyState({ icon: 'radar-2', title: k('noneFree'), body: k('noneHint') }))
    + specimen('failure (role=alert)', ui.emptyState({ icon: 'plug-connected-x', title: k('noLink'), reason: 'HTTP 503', alert: true,
        action: ui.button({ variant: 'success', icon: 'refresh', label: k('retry') }) }))
    + specimen('skeleton', ui.skeleton({ shapes: ['title', 'block', 'button'], label: t('loading') }))
    + specimen('skeleton rows', ui.skeleton({ shape: 'row', count: 2, label: t('loading') }));
  const feedback = specimen('toast', ui.button({ id: 'gToast', variant: 'secondary', icon: 'message-2', label: k('showToast') }))
    + specimen('sheet', ui.button({ id: 'gSheet', variant: 'secondary', icon: 'chevron-up', label: k('openSheet') }))
    + specimen('confirmSheet', ui.button({ id: 'gConfirm', variant: 'danger', icon: 'receipt', label: k('confirm') })
      + '<p class="ui-hint" id="gConfirmOut" aria-live="polite"></p>');
  const figures = specimen('amount sizes', ['sm', 'md', 'lg', 'xl'].map(s => ui.amount(money(1500), { size: s, strong: s === 'xl' })).join(' '))
    + specimen('amount tones', ui.amount(money(300), { sign: '+', tone: 'success' }) + ' ' + ui.amount(money(200), { sign: '-', tone: 'danger' }))
    + specimen('stat · hero', ui.stat({ label: k('cashInHand'), value: ui.amount(money(4500), { size: 'xl', strong: true }), emphasis: true }))
    + specimen('stat', ui.stat({ label: k('tips'), value: ui.amount(money(600), { sign: '+', tone: 'success' }), hint: k('today') }))
    + specimen('when · time / date / datetime', ['time', 'date', 'datetime'].map(s => ui.when(AT, { style: s, locale: LOCALE[lang], timeZone: 'Europe/Tirane' })).join(' · '))
    + specimen('mmss', `<b class="ui-time">${ui.mmss(245)}</b>`);
  const cards = specimen('card · accent', ui.card({ tone: 'accent', eyebrow: k('offer'), title: '#a1b2c3d4',
      body: `${ui.amount(money(1500), { size: 'lg', strong: true })}${ui.para(k('street'), { hint: true })}${ui.para(`${t('timeLeft')} ${ui.mmss(58)}`, { hint: true })}` }))
    + specimen('section', ui.section({ title: k('cashInHand'), sub: k('today'), back: { id: 'gBack', label: k('back') } }))
    + specimen('card · danger', ui.card({ tone: 'danger', title: k('late'), body: ui.para(k('noLink'), { hint: true }) }));
  return [['buttons', buttons], ['badges', badges], ['chips', chips], ['fields', fields], ['choice', choice],
    ['rows', rows], ['states', states], ['feedback', feedback], ['money', figures], ['cards', cards]]
    .map(([id, body]) => demo(id, id, body)).join('');
}

function head(){
  return `<header class="g-top">
    <div class="g-titles"><h1 class="ui-title" data-t="title">${ui.esc(t('title'))}</h1>${ui.para(k('lead'))}</div>
    <div class="g-controls">
      ${ui.segmented({ id: 'gLang', label: k('language'), value: lang, options: Object.keys(W).map(l => ({ value: l, label: l.toUpperCase() })) })}
      ${ui.segmented({ id: 'gTheme', label: k('theme'), value: theme, options: THEMES.map(v => ({ value: v, label: k(v || 'system'), icon: v === 'dark' ? 'moon' : v === 'light' ? 'sun' : 'sun-moon' })) })}
    </div>
  </header>
  <nav class="g-nav" aria-label="Components">${['buttons', 'badges', 'chips', 'fields', 'choice', 'rows', 'states', 'feedback', 'money', 'cards']
    .map(id => `<a class="g-link" href="#${id}" data-t="${id}">${ui.esc(t(id))}</a>`).join('')}</nav>`;
}

let toaster = null;
function render(){
  const root = document.documentElement;
  root.lang = lang;
  if (theme) root.dataset.theme = theme; else delete root.dataset.theme;
  document.getElementById('app').innerHTML = head() + `<main class="g-main">${sections()}</main>`;
  ui.bindSegmented(document.getElementById('gLang'), v => { lang = v; put('ui-gallery-lang', v); render(); });
  ui.bindSegmented(document.getElementById('gTheme'), v => { theme = v; put('ui-gallery-theme', v); render(); });
  ui.bindSegmented(document.getElementById('gSeg'), () => {});
  ui.bindTabs(document.getElementById('gTabs'), () => {});
  toaster ??= ui.createToaster(document.getElementById('toast'));
  document.getElementById('gToast').onclick = () => toaster.show(t('toastText'), { icon: 'circle-check', tone: 'success' });
  document.getElementById('gBusy').onclick = e => { const undo = ui.setBusy(e.currentTarget, t('saving')); setTimeout(undo, 1500); };
  document.getElementById('gSheet').onclick = () => ui.openSheet({ title: k('sheetTitle'), closeLabel: k('cancel'),
    body: ui.para(k('sheetBody')) + ui.field({ id: 'gSheetNote', label: k('note'), rows: 2 }),
    actions: ui.button({ variant: 'ghost', label: k('cancel'), attrs: { 'data-ui-close': 'no' } })
      + ui.button({ variant: 'primary', label: k('save'), attrs: { 'data-ui-close': 'yes' } }) });
  document.getElementById('gConfirm').onclick = async () => {
    const yes = await ui.confirmSheet({ title: k('sheetTitle'), body: k('sheetBody'), danger: true,
      confirmLabel: t('refund'), cancelLabel: t('cancel') });
    document.getElementById('gConfirmOut').textContent = yes ? '✓ ' + t('refund') : '✗ ' + t('cancel');
  };
}
render();
