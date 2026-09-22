// GENERATED FILE — DO NOT EDIT.
//
// Emitted by `tools/gen-vocab`, which LINKS the kernel and calls it:
//
//     cd tools/gen-vocab && cargo run > ../../workers/api/public/lib/vocab.js
//
// `tools/gates/vocab.sh` regenerates into a temporary file and refuses any tree
// where the two differ, so this file cannot drift from the kernel the way a
// hand copy does.
//
// SOURCES, by name, so a reviewer can go and read them:
//   crates/dowiz-core/src/order_machine.rs — OrderStatus, is_terminal,
//     took_money, is_active, assert_transition, FSM_GOLDEN_SIGNATURE
//   crates/dowiz-core/src/money.rs — Currency, MONEY_SCALE_MICRO
//
// THE DEFECT THIS CLOSES. `OrderStatus` has twelve members. The owner console
// held `DEAD = new Set(['REJECTED', 'CANCELLED'])`, the storefront's sea held
// the same set again under the name `FAILED`, the storefront's tracker held a
// third copy, and the kit spelled the test out inline — four copies of one
// kernel predicate, and every one of them short by `COMPENSATED_REFUND`, the
// terminal state the FSM has been able to reach since the P07 compensation
// edges landed. Nothing emits it yet, so nothing looked broken; the day a
// refund route lands, a refunded order counts as money the venue kept, on
// every screen at once. `tools/gates/vocabulary.sh` catches the missing WORD
// and the missing COLOUR. Only generating the list catches the missing MEMBER.
//
// WHAT IS DELIBERATELY NOT HERE. Translations (`admin/i18n.js`, `store/i18n.js`)
// and the colour per status (three stylesheets) are human choices, not kernel
// facts, and `vocabulary.sh` already refuses a status that is missing either.
// The happy-path progression the consoles draw as a stepper is a presentation
// decision too: the FSM's longest path from PENDING ties between DELIVERED and
// COMPENSATED_REFUND, so there is nothing here to derive it from.

/// Every member of `OrderStatus`, in the order the enum declares them.
export const STATUSES = [
  'PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED',
  'REJECTED', 'CANCELLED', 'SCHEDULED', 'PICKED_UP', 'REFUNDING',
  'COMPENSATED_REFUND',
];

/// `OrderStatus::is_terminal` — the order has stopped moving.
export const TERMINAL = new Set([
  'DELIVERED', 'REJECTED', 'CANCELLED', 'PICKED_UP', 'COMPENSATED_REFUND',
]);

/// The complement of `OrderStatus::took_money` — the order ended with the
/// venue holding none of the customer's money. This is the set three
/// surfaces spelled out by hand as `['REJECTED', 'CANCELLED']`, each of
/// them short by `COMPENSATED_REFUND`.
export const REFUSED = new Set([
  'REJECTED', 'CANCELLED', 'COMPENSATED_REFUND',
]);

/// `OrderStatus::is_active` — accepted, not finished. `PENDING` is not
/// active: it is a decision the venue still owes the customer.
export const ACTIVE = new Set([
  'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'REFUNDING',
]);

/// States the kernel refuses to enter or leave at all. A surface must not
/// offer one as a step, and `vocabulary.sh` still wants a word for it
/// because the FSM can be asked about it.
export const SCAFFOLD = new Set([
  'SCHEDULED',
]);

/// The transition table, recovered by asking `assert_transition` about every
/// ordered pair of statuses (144 of them, 14 accepted). `NEXT[s]` is every
/// status the kernel will let `s` become; an empty list is a dead end.
export const NEXT = {
  PENDING: ['CONFIRMED', 'REJECTED', 'CANCELLED'],
  CONFIRMED: ['PREPARING', 'IN_DELIVERY', 'REFUNDING'],
  PREPARING: ['READY', 'REFUNDING'],
  READY: ['IN_DELIVERY', 'PICKED_UP', 'REFUNDING'],
  IN_DELIVERY: ['DELIVERED', 'REFUNDING'],
  DELIVERED: [],
  REJECTED: [],
  CANCELLED: [],
  SCHEDULED: [],
  PICKED_UP: [],
  REFUNDING: ['COMPENSATED_REFUND'],
  COMPENSATED_REFUND: [],
};

/// Every code `money::Currency::from_code` accepts, found by exhaustion
/// over [A-Z]{1,4} rather than retyped. MINOR UNITS ARE NOT HERE: the
/// kernel has no minor-unit authority to read (`Currency` carries only a
/// code), so `lib/money.js` still owns `DECIMALS` and `tools/gates/vocab.sh`
/// checks that it covers exactly this list.
export const CURRENCIES = [
  'ALL', 'EUR', 'USD',
];

/// `money::MONEY_SCALE_MICRO` — the one scale the rate endpoint, the Worker
/// and `lib/money.js` all divide by. Parts per million of the target
/// currency per one unit of the base.
export const MONEY_SCALE_MICRO = 1000000;

/// The predicates, so a surface asks the question instead of holding the set.
export const isTerminal = s => TERMINAL.has(s);
export const isRefused = s => REFUSED.has(s);
export const isActive = s => ACTIVE.has(s);
/// Every status `s` may legally become. An UNKNOWN status answers `[]` rather
/// than `undefined`: a client that has not been redeployed since a member was
/// added must offer no step, not throw while drawing a row.
export const nextOf = s => NEXT[s] || [];
export const canAdvance = (from, to) => nextOf(from).includes(to);
