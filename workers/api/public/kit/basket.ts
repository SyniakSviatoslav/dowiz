// The basket — one place that knows what the customer has chosen.
//
// WHY THIS EXISTS. The cart screen held two hard-coded lines and the item
// screen's "Add item" button carried a `data-cta` attribute that nothing
// listened for. So the most important button in a food-delivery app did
// nothing, and the cart could not have shown the result if it had.
//
// WHAT IT IS NOT. It is not an order. An order is the kernel's: it has a state
// machine, integer money and a double-entry ledger behind it, and it is created
// on the server when the customer checks out. This is the list of what they
// intend to buy, on their own device, before any of that starts.
//
// MONEY IS INTEGER CENTS, as it is everywhere else in dowiz. Nothing here does
// arithmetic on a float.
//
// It survives a reload because a basket that forgets when a phone locks is not a
// basket. localStorage can throw in a private window and can come back empty, so
// every read and write is guarded and an unreadable store behaves as an empty
// one rather than breaking the screen.

/// One chosen dish, as the basket holds it.
///
/// `cents` is the field's historical name and it means MINOR UNITS of the
/// venue's currency, which for a venue trading in lek is whole lek -- lek has no
/// subdivision. Nothing here converts or formats: `lib/money.js` is the one
/// place that knows how many decimals a currency has, and the last time that
/// rule was retyped somewhere else a price came out a hundred times too small.
export interface Line {
  /// THE LINE's identity, which is NOT the dish's.
  ///
  /// The same dish in two sizes is two lines, so this is `productId:sizeId` --
  /// and that is why the order must not be built from it. It was: checkout sent
  /// `item-05:s` as `product_id`, the hub looks that up in the venue's own
  /// catalogue (`storefront.rs`: `unknown product`), and the customer would have
  /// been refused with a 400 after pressing Confirm.
  id: string;
  /// The dish in the venue's catalogue. What an ORDER names.
  productId?: string;
  name: string;
  kind: string;
  variant: string;
  cents: number;
  qty: number;
  addons: string[];
}

/// The venue's product behind a basket line.
///
/// Lines written before `productId` existed carry only `id`, and its shape has
/// always been `productId:sizeId` -- so the dish is the part before the LAST
/// colon. This is the format's own rule, not a guess about the id.
export function productOf(l: Line): string {
  if (l.productId) return l.productId;
  const cut = l.id.lastIndexOf(':');
  return cut > 0 ? l.id.slice(0, cut) : l.id;
}

const KEY = 'dowiz.basket.v1';
const SEEDED = 'dowiz.basket.seeded';

// The two lines the Figma cart frame draws. They are seeded ONCE, so the cart
// looks like its design on a first visit and is genuinely the customer's from
// the first change they make. Emptying the basket does not bring them back.
const SEED: Line[] = [
  { id: 'l1', name: 'ItaliaCrisp Pizza', kind: 'Pizza', variant: '8’ - Small',
    cents: 1200, qty: 1, addons: ['Olives', 'Capsicum', 'Onion'] },
  { id: 'l2', name: 'Mexican Tacos', kind: 'Tacos', variant: '2 Tacos',
    cents: 2200, qty: 1, addons: ['Extra Cheese', 'Extra Sauce'] },
];

const read = (): Line[] | null => {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(KEY) || 'null');
    // What comes back out of storage is UNTRUSTED: another version of this app
    // wrote it, or a person did. Each line is checked field by field rather
    // than cast, because a line with no `qty` becomes `NaN` in the total and a
    // basket that says NaN is worse than one that lost a line.
    if (!Array.isArray(raw)) return null;
    return raw.filter((l): l is Line =>
      !!l && typeof l === 'object'
      && typeof (l as Line).id === 'string' && (l as Line).id !== ''
      && ((l as Line).productId === undefined || typeof (l as Line).productId === 'string')
      && Number.isInteger((l as Line).cents)
      && Number.isInteger((l as Line).qty)
      && Array.isArray((l as Line).addons));
  } catch { return null; }
};

const write = (lines: Line[]): void => {
  try { localStorage.setItem(KEY, JSON.stringify(lines)); } catch { /* private window */ }
  dispatchEvent(new CustomEvent('basket', { detail: { lines } }));
};

function load(): Line[] {
  return read() ?? [];
}

/// Put the frame's two example lines in an empty basket, ONCE.
///
/// This used to happen inside `load()`, so every basket everywhere started with
/// `ItaliaCrisp Pizza` and `Mexican Tacos` -- two dishes that exist in the Figma
/// frame and in no venue's catalogue. On a real hub that is not decoration: the
/// customer's cart contains rows the kitchen has never heard of, and the order
/// built from them is refused by the hub with "unknown product" after they have
/// pressed Confirm.
///
/// So seeding is now something a SCREEN asks for, and the cart asks only when
/// there is no venue behind it -- which is exactly when this app is being looked
/// at as a design rather than used to buy dinner.
export function seedFrame(): Line[] {
  if (read()?.length) return load();
  try {
    if (localStorage.getItem(SEEDED)) return load();
    localStorage.setItem(SEEDED, '1');
  } catch { return SEED.slice(); }   // no storage at all: show the design
  write(SEED);
  return SEED.slice();
}

/** Every line, newest last. */
export const lines = (): Line[] => load();

/** How many items, counting quantity — the number on the tab bar. */
export const count = (): number => load().reduce((n, l) => n + l.qty, 0);

/** The sum in integer cents. */
export const subtotalCents = (): number => load().reduce((n, l) => n + l.cents * l.qty, 0);

/**
 * Add a line. Two adds of the SAME dish with the SAME options are one line with
 * a bigger quantity — a cart that lists "Tuna Bowl" four times is a cart that
 * makes its customer do the adding up.
 */
export function add(
  { id, productId, name, kind = '', variant = '', cents, qty = 1, addons = [] }:
  { id: string; productId?: string; name: string; kind?: string; variant?: string;
    cents: number; qty?: number; addons?: string[] },
): Line[] {
  const all = load();
  const sameness = JSON.stringify([id, variant, [...addons].sort()]);
  const at = all.findIndex(l => JSON.stringify([l.id, l.variant, [...l.addons].sort()]) === sameness);
  const found = at >= 0 ? all[at] : undefined;
  if (found) all[at] = { ...found, qty: Math.min(99, found.qty + qty) };
  else all.push({ id, productId, name, kind, variant, cents, qty: Math.min(99, qty), addons });
  write(all);
  return all;
}

/** Set a line's quantity. Zero removes it, because that is what zero means. */
export function setQty(id: string, qty: number): Line[] {
  const all = load()
    .map(l => (l.id === id ? { ...l, qty: Math.max(0, Math.min(99, qty)) } : l))
    .filter(l => l.qty > 0);
  write(all);
  return all;
}

export function remove(id: string): void {
  write(load().filter(l => l.id !== id));
}

export function clear(): void {
  write([]);
}

/** Run `fn` now and whenever the basket changes, in any tab. */
export function onChange(fn: (lines: Line[]) => void): void {
  addEventListener('basket', () => fn(lines()));
  // Another tab of the same app writing the basket fires `storage`, not our
  // own event, so a second tab is kept honest too.
  addEventListener('storage', e => { if (e.key === KEY) fn(lines()); });
  fn(lines());
}
