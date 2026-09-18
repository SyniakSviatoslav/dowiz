// Chat 1:8477, Chat Detail 1:8635, Voice Call 1:8767
// (light 1:18907, 1:19066, 1:19198).
//
// MESSAGING IS A DOWIZ DOMAIN NOW. `dowiz_kernel::thread` is an append-only log
// ordered by a SENDER-ASSIGNED SEQUENCE and not by a clock, because two phones
// that were offline must agree on the order of what they each wrote and a wall
// clock cannot give them that. A resend under the same id is a no-op; a
// different message under a used id is refused.
//
// Reading is a poll, not a socket: a thread only ever needs "everything after
// N", which is one GET that survives a tunnel and a locked phone. There is no
// presence and no typing indicator, because neither is a fact a thread knows.
//
// VOICE CALLS ARE STILL NOT A DOMAIN. dowiz has no signalling and the call
// screen says so — drawing a ringing phone that cannot ring would be the
// dishonest option.

import { icon, esc } from '/kit/app.js';
import { topBar, plate, paintPlates, navbar } from '/kit/parts.js';
import { threads as api } from '/kit/data.js';

const THREADS = [
  { id: 't1', who: 'Charlotte Taylor', last: 'I’ve arrived at the gate.', at: '09:34 PM',
    unread: 3 },
  { id: 't2', who: 'Bessi Cooper', last: 'Lorem Ipsum is simply dummy text.', at: '08:04 PM',
    unread: 2 },
  { id: 't3', who: 'Ava Mitchell', last: 'Thank you!', at: 'Yesterday', unread: 0 },
  { id: 't4', who: 'Harper Lane', last: 'See you soon.', at: 'Yesterday', unread: 0 },
];

const MESSAGES = [
  { mine: false, who: 'Bessi Cooper', at: '08:04 pm',
    body: 'Lorem Ipsum is simply dummy text of the printing and typesetting industry.' },
  { mine: true,  who: 'Jennifer Aaker', at: '08:04 pm',
    body: 'Lorem Ipsum is simply dummy text of the printing and typesetting industry.' },
  { mine: false, who: 'Bessi Cooper', at: '08:04 pm', body: 'On my way.' },
];

// THE STORY ROW IS THE THREAD LIST, one avatar per conversation. The frame
// draws five invented names above four conversations, and as markup that is
// five buttons leading nowhere: tapping a face for a person you cannot write to
// is the defect the interaction gate reported five times on this screen. The
// faces are the people this customer HAS a thread with, and tapping one opens
// it — the same destination as the row below, reached the way the design meant.
const STORIES = () => THREADS;

const state = { filter: 'All', sent: [], threadId: null };

const threadRow = t => `
  <button class="k-thread" type="button" data-go="chat-detail?thread=${esc(t.id)}">
    <span class="k-thread-img">${plate(t.who)}</span>
    <span class="k-thread-body">
      <span class="k-thread-n">${esc(t.who)}</span>
      <span class="k-thread-m${t.unread ? ' is-unread' : ''}">${esc(t.last)}</span>
    </span>
    <span class="k-thread-side">
      <span class="k-thread-t">${esc(t.at)}</span>
      ${t.unread ? `<span class="k-badge">${t.unread}</span>` : ''}
    </span>
  </button>`;

const listPane = () => {
  const rows = state.filter === 'Unread' ? THREADS.filter(t => t.unread) : THREADS;
  return rows.length ? rows.map(threadRow).join('')
    : `<div class="k-empty">${icon('linear-messages-conversation-chat-round-dots')}
         <p class="t-title">Немає непрочитаних</p></div>`;
};

export async function render(params, routeName = 'chat'){
  state.threadId = params?.get('thread') || null;
  if (routeName === 'voice-call'){
    return `
    <div class="k-call">
      <span class="k-call-img">${plate('Bessi Cooper')}</span>
      <h1 class="k-call-n">Bessi Cooper</h1>
      <p class="k-call-r">Manager</p>
      <p class="k-call-t" id="timer">02:10</p>
      <p class="k-page-p">Дзвінки ще не підключені: у dowiz немає сигналізації,
        тільки номер телефону закладу.</p>
      <!-- There is no call, so there is nothing to mute and nothing to put on
           the speaker. The two controls keep the frame's shape and are
           DISABLED, because a live-looking button over a call that cannot ring
           is the dishonest half of the same screen that says so in words. -->
      <div class="k-call-acts">
        <button class="k-call-btn" type="button" disabled
                aria-label="Мікрофон — дзвінки не підключені">${icon('notification-bing')}</button>
        <button class="k-call-btn is-end" type="button" data-back
                aria-label="Завершити">${icon('arrow-left')}</button>
        <button class="k-call-btn" type="button" disabled
                aria-label="Динамік — дзвінки не підключені">${icon('reserve-20')}</button>
      </div>
    </div>`;
  }

  if (routeName === 'chat-detail'){
    // The thread is named by the route; without one the frame's own sample
    // conversation stands, which is what a design review wants to see. The
    // header names WHOSE thread this is: every row and every face on the list
    // screen leads here, and a header that always said one name made four
    // different conversations look like the same one.
    const t = THREADS.find(x => x.id === state.threadId);
    return `
    ${topBar(t ? t.who : 'Bessi Cooper', `<button class="k-top-btn" type="button" data-go="voice-call"
        aria-label="Подзвонити">${icon('notification-bing')}</button>`)}
    <div class="wrap">
      <p class="k-day-mark">TODAY</p>
      <div class="k-msgs" id="msgs">${bubbles()}</div>
    </div>
    <div class="k-composer">
      <label class="k-field">
        <input id="msg" type="text" placeholder="Повідомлення" aria-label="Повідомлення">
      </label>
      <button class="k-send" type="button" id="send" aria-label="Надіслати" disabled>
        ${icon('next-arrow')}</button>
    </div>`;
  }

  return `
  ${topBar('Chat')}
  <div class="wrap">
    <div class="k-story-row">
      ${STORIES().map(t => `
        <button class="k-story" type="button" data-go="chat-detail?thread=${esc(t.id)}">
          <span class="k-story-img">${plate(t.who)}</span>
          <span class="k-story-n">${esc(t.who.split(' ')[0])}</span>
        </button>`).join('')}
    </div>
    <div class="k-chips" role="group" aria-label="Фільтр">
      ${['All', 'Unread'].map(f => `
        <button class="k-chip${f === state.filter ? ' on' : ''}" type="button"
                data-filter="${esc(f)}" aria-pressed="${f === state.filter}">${esc(f)}</button>`)
        .join('')}
    </div>
    <div id="threads" class="k-block">${listPane()}</div>
  </div>
  ${navbar('')}`;
}

function bubbles(){
  const all = [...MESSAGES, ...state.sent];
  return all.map(m => `
    <div class="k-msg${m.mine ? ' is-mine' : ''}">
      ${esc(m.body)}
      <span class="k-msg-t">${esc(m.at)}${
        m.unsent ? ` · не надіслано${m.why ? `: ${esc(m.why)}` : ''}` : ''}</span>
    </div>`).join('');
}

export function bind(root){
  if (state.threadId && root.querySelector('#msgs')) loadThread(root);

  // SEND IS DISABLED UNTIL THERE IS SOMETHING TO SEND. It used to be tappable
  // with an empty composer and answered that tap by returning — a control that
  // looks ready and does nothing, which the interaction gate reported as
  // `DEAD button#send`. Disabled says the same thing honestly, and to a screen
  // reader as well as to the eye.
  const composer = root.querySelector('#msg');
  const sendBtn = root.querySelector('#send');
  if (composer && sendBtn)
    composer.addEventListener('input', () => { sendBtn.disabled = !composer.value.trim(); });

  root.addEventListener('click', e => {
    const f = e.target.closest('[data-filter]');
    if (f){
      state.filter = f.dataset.filter;
      for (const b of root.querySelectorAll('[data-filter]')){
        const on = b === f;
        b.setAttribute('aria-pressed', String(on));
        b.classList.toggle('on', on);
      }
      const host = root.querySelector('#threads');
      host.innerHTML = listPane();
      paintPlates(host);
      return;
    }

    if (!e.target.closest('#send')) return;
    send(root);
  });

  root.addEventListener('keydown', e => {
    if (e.target.id === 'msg' && e.key === 'Enter') send(root);
  });
}

async function send(root){
  const input = root.querySelector('#msg');
  if (!input) return;
  const body = input.value.trim();
  if (!body) return;

  const now = new Date();
  const at = `${String(now.getHours()).padStart(2, '0')}:${
    String(now.getMinutes()).padStart(2, '0')}`;

  // Shown immediately and marked unsent, then corrected by the answer. A
  // message that vanishes while the network thinks about it is a message the
  // sender believes they lost.
  const pending = { mine: true, body, unsent: true, at };
  state.sent.push(pending);
  input.value = '';
  const btn = root.querySelector('#send');
  if (btn) btn.disabled = true;
  repaint(root);

  const id = state.threadId;
  if (!id) return;                      // a design view: nothing to send to

  const answer = await api.send(id, {
    from: 'CUSTOMER',
    kind: 'TEXT',
    body,
    clientId: `${now.getTime()}-${Math.random().toString(36).slice(2, 8)}`,
  });

  pending.unsent = !answer || !!answer.error;
  pending.why = answer && answer.error ? answer.error : '';
  repaint(root);
}

function repaint(root){
  const msgs = root.querySelector('#msgs');
  if (!msgs) return;
  msgs.innerHTML = bubbles();
  msgs.lastElementChild?.scrollIntoView({ block: 'nearest' });
}

async function loadThread(root){
  const answer = await api.messages(state.threadId, 0);
  if (!answer || answer.error) return;   // the sample conversation stands
  MESSAGES.length = 0;
  for (const m of answer.messages || []){
    if (m.kind !== 'TEXT') continue;
    const d = new Date(m.sentAtMs);
    MESSAGES.push({
      mine: m.from === 'CUSTOMER',
      who: m.from,
      at: `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`,
      body: m.body,
    });
  }
  repaint(root);
}
