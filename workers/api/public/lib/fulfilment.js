/// HOW AN ORDER REACHES THE PERSON WHO ORDERED IT — the browsers' half.
///
/// THERE WERE THREE WAYS AND EVERY SURFACE KNEW ABOUT TWO. Both the Rust and
/// the JavaScript asked the same question — "is this a pickup?" — and treated
/// everything else as a delivery. `workers/api/src/services/ordering/
/// fulfilment.rs` closed four copies of it on the server. This is the fifth:
/// `admin/orders.js` had `const isPickup = o => o.fulfilment?.kind ===
/// 'pickup'`, so an order placed at a table showed a bicycle, said "delivery",
/// and exported an empty address column in the owner's own CSV.
///
/// IT IS NOT GENERATED, AND THAT IS A GAP WITH A NAME ON IT. `lib/vocab.js` is
/// emitted from the kernel by `tools/gen-vocab` precisely so a status list
/// cannot be retyped; the fulfilment kinds live in the WORKER, not the kernel,
/// so there is nothing for that generator to read yet. Until `KINDS` moves into
/// `dowiz-core`, this file is a hand copy of three strings — one hand copy
/// instead of the five that were here, with the price of it written down.
///
/// ASKED AS "DOES IT LEAVE THE BUILDING", never as "is it a pickup": an
/// allow-list means the FOURTH kind is refused a courier and a travel time by
/// default rather than inheriting them.

export const KINDS = ['delivery', 'pickup', 'dine_in'];

/// The kind an order carries. Absent means `delivery`, and so does a word this
/// build does not know — the same compatibility rule the server applies, for
/// the same reason: an order restored from an old archive predates the field,
/// and answering with an unknown word would miss every branch.
export const kindOf = o => {
  const k = o?.fulfilment?.kind;
  return KINDS.includes(k) ? k : 'delivery';
};

export const leavesTheBuilding = kind => kind === 'delivery';

/// The table, trimmed, or null. A table sent as "  7 " is table 7 on screen
/// and a different string to every comparison.
export const tableOf = o => {
  const t = o?.fulfilment?.table;
  return typeof t === 'string' && t.trim() ? t.trim() : null;
};

/// The icon and the i18n key for one order, so a surface cannot pick one and
/// forget the other.
export const badgeOf = o => {
  const kind = kindOf(o);
  if (kind === 'dine_in') return { icon: 'utensils', key: 'dineIn', table: tableOf(o) };
  if (kind === 'pickup') return { icon: 'walk', key: 'pickup', table: null };
  return { icon: 'bike', key: 'delivery', table: null };
};

/// Where it is going, for a list column and for the CSV export.
///
/// A TABLE ORDER HAS NO ADDRESS AND THAT IS NOT AN EMPTY ADDRESS. The export
/// wrote `''` for anything that was not a delivery, so a table order and a
/// counter pickup were indistinguishable in a spreadsheet — and the owner's
/// own record of where their trade came from lost a whole channel.
export const destinationOf = o => {
  const kind = kindOf(o);
  if (kind === 'dine_in') {
    const t = tableOf(o);
    return t ? `table ${t}` : 'in the venue';
  }
  if (kind === 'pickup') return '';
  return o?.fulfilment?.address?.line || '';
};
