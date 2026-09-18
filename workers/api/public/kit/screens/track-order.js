// Track Order — Figma node 1:7331 (dark) / 1:17751 (light).
//
// The order, its id, and a four-step timeline. The step a delivery is ON is the
// only thing here that changes by itself, so it is the one value the screen
// recomputes: the rail's fill, the lit dots and which ring is orange all follow
// from it rather than being set three times.
//
// dowiz's kernel is the authority on order state (kernel/src/order_machine.rs:
// decide -> Event, state = fold(events)). This screen never decides a
// transition; it renders the one the kernel reports.

import { icon, esc } from '/kit/app.js';
import { topBar, orderLine, factRow, ctaBar } from '/kit/parts.js';
import { formatMoney } from '/kit/data.js';
import * as Orders from '/kit/orders.js';

const ORDER = {
  id: '#FD785462',
  line: { name: 'ItaliaCrisp Pizza', kind: 'Pizza', variant: '8’ - Small', price: '$12.00',
          addons: ['Olives', 'Capsicum', 'Onion'] },
};

/// THE ORDER THIS CUSTOMER ACTUALLY PLACED.
///
/// This screen drew `#FD785462` and a pizza from the Figma frame for everybody,
/// forever: a customer who had just paid tapped "View Order" and was shown a
/// fictional order belonging to nobody, stopped on "On the Way". The order id
/// and its key are on this device (`orders.js`), and the STATUS comes from the
/// hub, which is the only thing entitled to say where a delivery is.
///
/// `?id=` picks one from the customer's own list; with none named it is the most
/// recent, which is what "View Order" straight after paying means. With no order
/// on this device at all the frame stands — that is the design being looked at,
/// and it is honest about being a design because there is nothing real to show.
let live = null;

/// The kernel's six states onto the four the frame draws. `READY` is not a step
/// of its own here: to a customer waiting at home, "ready" and "being prepared"
/// are the same wait, and inventing a fifth dot would not match the frame.
const STEP_OF = {
  PENDING: 'placed', CONFIRMED: 'placed',
  PREPARING: 'preparing', READY: 'preparing',
  IN_DELIVERY: 'on-way', DELIVERED: 'delivered',
};

const when = ms => new Date(ms).toLocaleString('uk-UA',
  { day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' });

// The four states the frame draws, in the kernel's order. `at` is null while a
// step has not happened: the frame shows an estimate there instead of a time.
const STEPS = [
  { id: 'placed',    title: 'Order Placed', icon: 'clipboard-tick', at: 'April 12, 2026 | 07:30 PM' },
  { id: 'preparing', title: 'Preparing',    icon: 'box-20',         at: 'April 12, 2026 | 07:35 PM' },
  { id: 'on-way',    title: 'On the Way',   icon: 'scooter-20',     at: 'April 12, 2026 | 07:42 PM' },
  { id: 'delivered', title: 'Delivered',    icon: 'box-tick',       at: null,
    estimate: 'Arriving by 07:55 PM' },
];

// The frame is drawn with the order on its third step.
const state = { at: 'on-way' };

const indexOf = id => Math.max(0, STEPS.findIndex(s => s.id === id));

export async function render(params){
  const wanted = params?.get('id');
  live = wanted ? await Orders.refresh(wanted)
       : (Orders.last() ? await Orders.refresh(Orders.last().id) : null);

  const now = live ? indexOf(STEP_OF[live.status] || 'placed') : indexOf(state.at);
  const lines = live?.lines?.length ? live.lines : null;
  const money = c => live?.currency ? formatMoney(c, live.currency) : ORDER.line.price;
  const over = live && Orders.isOver(live.status);

  return `
  ${topBar('Track Order')}

  <div class="wrap">
    ${lines
      ? lines.map(l => orderLine({ name: l.name, kind: l.kind, variant: l.variant,
                                   price: money(l.cents * l.qty), addons: l.addons })).join('')
      : orderLine(ORDER.line)}

    <div class="k-block">
      <h2 class="k-block-h">Order Details</h2>
      ${factRow({ icon: 'clipboard-text', title: 'Order ID',
                  sub: live ? '#' + String(live.id).slice(0, 8).toUpperCase() : ORDER.id })}
    </div>

    ${over ? `
      <div class="k-block">
        <h2 class="k-block-h">Order Status</h2>
        <div class="k-box">
          <p class="k-eta">${icon('info-circle')}${esc(
            live.status === 'REJECTED' ? 'Заклад відхилив це замовлення' : 'Замовлення скасовано')}</p>
        </div>
      </div>` : `
    <div class="k-block">
      <h2 class="k-block-h">Order Status</h2>
      <div class="k-box">
        <ol class="k-steps" id="steps" data-progress="${progressOf(now)}">
          ${STEPS.map((s, i) => `
            <li class="k-step${i < now ? ' is-done' : i === now ? ' is-now' : ''}">
              <span class="k-step-dot">${icon(
                i < now ? 'step-done' : i === now ? 'step-now' : 'step-next')}</span>
              <span class="k-step-body">
                <span class="k-step-t">${esc(s.title)}</span>
                <span class="k-step-s">${esc(stepWhen(s, i, now))}</span>
              </span>
              <span class="k-step-ring">${icon(s.icon)}</span>
            </li>`).join('')}
        </ol>
      </div>
    </div>`}
  </div>

  ${over ? '' : ctaBar('Track Live Location', { to: 'track-live' })}`;
}

/// WHAT TO PRINT UNDER A STEP.
///
/// The frame's own times (`April 12, 2026 | 07:30 PM`) belong to the frame. A
/// real order knows when it was PLACED and nothing else — the hub reports a
/// status, not a timestamp per step — so a step that has happened without a
/// known time says nothing rather than borrowing April's.
function stepWhen(s, i, now){
  if (!live) return i <= now ? (s.at || '') : (s.estimate || s.at || '');
  if (s.id === 'placed') return when(live.placedAtMs);
  if (i < now) return '';
  if (i === now) return 'Зараз';
  return '';
}

// The rail fills to the CENTRE of the current step's dot, not past it: a rail
// that runs beyond the lit dot reads as "already done".
function progressOf(now){
  if (STEPS.length < 2) return 0;
  return Math.round((now / (STEPS.length - 1)) * 100);
}

export function bind(root){
  // The fill is a computed length, so it is written through CSSOM -- the
  // `style=` attribute it would otherwise need is blocked by `style-src 'self'`.
  const steps = root.querySelector('#steps');
  if (steps) steps.style.setProperty('--k-progress', steps.dataset.progress + '%');
}
