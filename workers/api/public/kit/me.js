// The customer's own record, on the customer's own device.
//
// WHY THIS EXISTS. Two forms in this kit validated what was typed and then threw
// it away: `Add Address` checked that an address was non-empty and called
// `history.back()`, and `Complete Your Profile` did the same with a name and a
// phone number. So the only address a customer could order to was one of the
// four New York addresses drawn in the Figma frame, and every order the kit
// placed carried `contact: {name:"", phone:""}` — a delivery nobody can ring the
// door for.
//
// WHAT IT IS NOT. It is not an account, and none of it is sent anywhere except
// as part of an order the customer places. dowiz is local-first: a guest's name,
// phone and address list live in their browser, and the hub learns them only
// when an order needs them. That is also why the hub keys its customer registry
// by a HASH of the phone (`storefront.rs`) and not by the number.
//
// Storage can throw in a private window and can come back empty or corrupted by
// another version of this app, so every read is guarded and validated, and an
// unreadable store behaves as an empty one rather than breaking checkout.

const ADDRESSES = 'dowiz.kit.addresses';
const ADDRESS   = 'dowiz.kit.address';    // the ONE that checkout sends
const CONTACT   = 'dowiz.kit.contact';

const read = key => {
  try { return JSON.parse(localStorage.getItem(key) || 'null'); }
  catch { return null; }
};
const write = (key, value) => {
  try { localStorage.setItem(key, JSON.stringify(value)); return true; }
  catch { return false; }                 // private window: it simply does not persist
};

const str = (x, max = 200) => (typeof x === 'string' ? x.trim().slice(0, max) : '');

/// One address as the customer wrote it. `note` is the floor and the landmark —
/// what a courier reads at the door, which is why it travels with the order.
const clean = a => {
  const line = str(a?.line);
  if (!line) return null;
  return { id: str(a?.id) || 'a' + Date.now().toString(36),
           label: str(a?.label, 40) || 'Address', line, note: str(a?.note, 200) || null };
};

/** Every address this device has saved, newest first. */
export function addresses(){
  const raw = read(ADDRESSES);
  return Array.isArray(raw) ? raw.map(clean).filter(Boolean) : [];
}

/**
 * Keep an address and make it the one checkout will use.
 *
 * Saving the same line twice replaces it rather than growing a list of
 * duplicates that the customer then has to tell apart.
 */
export function addAddress(a){
  const next = clean(a);
  if (!next) return null;
  const all = addresses().filter(x => x.line !== next.line);
  all.unshift(next);
  write(ADDRESSES, all.slice(0, 20));     // a phone is not an address book
  setChosen(next);
  return next;
}

/** The address checkout sends, or null when none has been chosen. */
export const chosen = () => clean(read(ADDRESS));

/** Choose one — from the saved list or from the frame's own examples. */
export function setChosen(a){
  const next = clean(a);
  if (next) write(ADDRESS, { label: next.label, line: next.line, note: next.note });
  return next;
}

/**
 * Who to hand the order to.
 *
 * The storefront on this same origin has always kept these under `dw_name` and
 * `dw_phone`; a customer who ordered there once should not be asked again just
 * because they opened a different surface of the same shop.
 */
export function contact(){
  const c = read(CONTACT);
  const name  = str(c?.name, 80) || legacy('dw_name');
  const phone = str(c?.phone, 32) || legacy('dw_phone');
  return { name, phone };
}

export function setContact({ name, phone }){
  const next = { name: str(name, 80), phone: str(phone, 32) };
  write(CONTACT, next);
  return next;
}

function legacy(key){
  try { return str(localStorage.getItem(key), 80); } catch { return ''; }
}
