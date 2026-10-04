// PURE. The guest's allergen filter: what it may HIDE and what it may only WARN about.
//
// TWO RULES, both decided by law and by the operator, not by taste (W-MR0 2026-10-04;
// memory dowiz-dynamic-menu-decisions-2026-10-03):
//   1. ONLY THE GUEST'S EXPLICIT CHOICE HIDES. The chips the guest tapped (`dw_avoid`) hide a
//      dish that declares one of them, and hide every UNDECLARED dish too: "nobody said" is not
//      "safe", and a guest who asked to avoid shellfish must not be shown a dish nobody checked.
//   2. AN INFERRED ALLERGY NEVER HIDES. Whatever the page guesses on the device (a taste memory,
//      a past order) may re-order or warn, never remove: EU 1169/2011 is information BEFORE the
//      purchase, and a dish taken off the screen by a guess is information withheld.
// The three states are the hub's (`crates/dowiz-hub/src/allergens.rs`): an absent list is
// UNDECLARED, an empty one is "none of the fourteen", a list is what the dish contains.

/// The dish's declaration: { kind: 'undeclared' | 'none' | 'contains', codes }.
export function declaration(p){
  if (!p || !Array.isArray(p.allergens)) return { kind: 'undeclared', codes: [] };
  const codes = p.allergens.filter(c => typeof c === 'string' && c);
  return codes.length ? { kind: 'contains', codes } : { kind: 'none', codes: [] };
}

/// Why the guest's OWN choice hides this dish, or null. `chosen` is what the guest tapped;
/// nothing else may be passed here (rule 1).
export function hiddenBecause(p, chosen){
  if (!Array.isArray(chosen) || !chosen.length) return null;
  const d = declaration(p);
  if (d.kind === 'undeclared') return 'undeclared';
  return d.codes.some(c => chosen.includes(c)) ? 'contains' : null;
}

/// What an INFERRED allergy may do: a warning, never a hide (rule 2). Same answers as
/// `hiddenBecause`, but the caller shows them beside the dish and leaves it on screen.
export function warnBecause(p, inferred){
  if (!Array.isArray(inferred) || !inferred.length) return null;
  const d = declaration(p);
  if (d.kind === 'undeclared') return 'undeclared';
  return d.codes.some(c => inferred.includes(c)) ? 'contains' : null;
}

/// What the filter hid, by reason, for the line under the chips.
export function hiddenCounts(products, chosen){
  const n = { contains: 0, undeclared: 0 };
  for (const p of products || []) { const why = hiddenBecause(p, chosen); if (why) n[why] += 1; }
  return n;
}
