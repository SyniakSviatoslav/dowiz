// The More screen's search, PURE (design doc HUB-UX-2026-09-26 §3.3): a map of
// thirty destinations is found by name, in the reader's language, from the
// title and the one-line subtitle. No DOM; `more-find.test.mjs` drives it.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// Lower-case, accents stripped, so "Cilesimet" finds "Cilësimet".
export const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

/// Does the row `key` match every term of `q`? `t` is the surface's t():
/// the title is t(key), the subtitle t(key + 'Sub') when the surface has it.
export function matches(key, q, t){
  const terms = norm(q).split(/\s+/).filter(Boolean);
  if (!terms.length) return true;
  const sub = t(key + 'Sub');
  const hay = norm(`${t(key)} ${sub === key + 'Sub' ? '' : sub} ${key}`);
  return terms.every(term => hay.includes(term));
}

/// The groups ([groupKey, [[rowKey, icon, ...], ...]]) with only the rows that
/// match `q`; a group left with no row is dropped. An empty query returns the
/// groups untouched (the same array), so the plain render costs nothing.
export function filter(groups, q, t){
  if (!norm(q).trim()) return groups;
  return groups.map(([g, rows]) => [g, rows.filter(r => matches(r[0], q, t))]).filter(([, rows]) => rows.length);
}

/// How many rows `groups` hold, for the "nothing found" state.
export const count = groups => groups.reduce((n, [, rows]) => n + rows.length, 0);
