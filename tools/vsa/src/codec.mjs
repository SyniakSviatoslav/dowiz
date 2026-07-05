// VSA Transmitter codec — deterministic, LOSSLESS JSON→frame→JSON projection.
//
// The token economy comes from structure, not magic:
//   1. COLUMNAR: an array of ≥3 objects sharing one ordered key tuple collapses to
//      {"§t":[cols],"§r":[[row],…]} — keys paid ONCE instead of ×N (a 49-product menu
//      pays "price"/"available"/… once, not 49 times).
//   2. DICT: any string (key or value) whose repetition saves more than its dictionary
//      overhead moves to "§d" and is referenced as "§<idx>" — UUIDs repeated across a
//      payload shrink from ~15 tokens to ~3 per occurrence.
//   3. MINIFY: no whitespace.
//
// LOSSLESS is a hard invariant: decode(encode(x)) deep-equals x for every JSON value —
// null-vs-absent, key order, unicode, and strings that look like our own sigils (escaped).
// A codec that loses shape is corruption, not compression (see the imageUrl null-vs-absent
// parity incident, 2026-07-05). Raw hypervectors are NEVER emitted here — an LLM cannot
// decode them and they would COST tokens; hypervectors live in hv.mjs for local math only.
//
// Frame: {"§v":1,"§d":[…]?,"§b":<body>} serialized minified. Escape rule: any literal
// string starting with "§" is stored as "§~"+literal; "§<digits>" is a dict ref; an object
// with exactly the keys "§t","§r" is a column table (source objects can never produce one —
// their "§…" keys get escaped).

const REF_RE = /^§\d+$/;

function escStr(s) {
  return s.startsWith('§') ? '§~' + s : s;
}
function unescStr(s) {
  return s.startsWith('§~') ? s.slice(2) : s;
}

function isPlainObject(v) {
  return v !== null && typeof v === 'object' && !Array.isArray(v);
}

/** Pass 1 — columnar transform (bottom-up). */
function columnarize(v) {
  if (Array.isArray(v)) {
    const mapped = v.map(columnarize);
    if (mapped.length >= 3 && mapped.every(isPlainObject)) {
      const cols = Object.keys(mapped[0]);
      const tuple = JSON.stringify(cols);
      const uniform =
        cols.length > 0 && mapped.every((o) => JSON.stringify(Object.keys(o)) === tuple);
      if (uniform) {
        return { '§t': cols, '§r': mapped.map((o) => cols.map((c) => o[c])) };
      }
    }
    return mapped;
  }
  if (isPlainObject(v)) {
    const out = {};
    for (const [k, val] of Object.entries(v)) out[k] = columnarize(val);
    return out;
  }
  return v;
}

function decolumnarize(v) {
  if (Array.isArray(v)) return v.map(decolumnarize);
  if (isPlainObject(v)) {
    const keys = Object.keys(v);
    if (keys.length === 2 && keys[0] === '§t' && keys[1] === '§r') {
      const cols = v['§t'];
      return v['§r'].map((row) => {
        const o = {};
        cols.forEach((c, i) => (o[c] = decolumnarize(row[i])));
        return o;
      });
    }
    const out = {};
    for (const [k, val] of Object.entries(v)) out[k] = decolumnarize(val);
    return out;
  }
  return v;
}

/** Pass 2 — count every string occurrence (keys + values) post-columnar. */
function countStrings(v, counts) {
  if (typeof v === 'string') {
    counts.set(v, (counts.get(v) || 0) + 1);
  } else if (Array.isArray(v)) {
    for (const x of v) countStrings(x, counts);
  } else if (isPlainObject(v)) {
    for (const [k, val] of Object.entries(v)) {
      if (k !== '§t' && k !== '§r') counts.set(k, (counts.get(k) || 0) + 1);
      countStrings(val, counts);
    }
  }
}

function rewrite(v, dictIndex) {
  if (typeof v === 'string') {
    const idx = dictIndex.get(v);
    return idx !== undefined ? `§${idx}` : escStr(v);
  }
  if (Array.isArray(v)) return v.map((x) => rewrite(x, dictIndex));
  if (isPlainObject(v)) {
    const out = {};
    for (const [k, val] of Object.entries(v)) {
      const nk =
        k === '§t' || k === '§r'
          ? k
          : dictIndex.has(k)
            ? `§${dictIndex.get(k)}`
            : escStr(k);
      out[nk] = rewrite(val, dictIndex);
    }
    return out;
  }
  return v;
}

function restore(v, dict) {
  if (typeof v === 'string') {
    if (REF_RE.test(v)) return dict[Number(v.slice(1))];
    return unescStr(v);
  }
  if (Array.isArray(v)) return v.map((x) => restore(x, dict));
  if (isPlainObject(v)) {
    const out = {};
    for (const [k, val] of Object.entries(v)) {
      if (k === '§t' || k === '§r') {
        out[k] = restore(val, dict);
        continue;
      }
      const nk = REF_RE.test(k) ? dict[Number(k.slice(1))] : unescStr(k);
      out[nk] = restore(val, dict);
    }
    return out;
  }
  return v;
}

/**
 * Encode any JSON value into a frame string.
 * Deterministic: same input → byte-identical frame, everywhere, always.
 */
export function encode(value) {
  const body = columnarize(value);
  const counts = new Map();
  countStrings(body, counts);

  // Greedy dictionary by net character savings. Ref "§<i>" costs ~4-6 chars quoted;
  // a dict entry costs len+3. Only strings that actually pay their way get a slot.
  const candidates = [...counts.entries()]
    .map(([s, n]) => ({ s, n, save: (n - 1) * (s.length - 4) - 7 }))
    .filter((c) => c.save > 0)
    .sort((a, b) => b.save - a.save);
  const dict = candidates.map((c) => c.s);
  const dictIndex = new Map(dict.map((s, i) => [s, i]));

  const frame = { '§v': 1 };
  if (dict.length) frame['§d'] = dict;
  frame['§b'] = rewrite(body, dictIndex);
  return JSON.stringify(frame);
}

/** Decode a frame string back to the exact original JSON value. */
export function decode(frameStr) {
  const frame = JSON.parse(frameStr);
  if (frame['§v'] !== 1) throw new Error(`unknown frame version: ${frame['§v']}`);
  const dict = frame['§d'] || [];
  return decolumnarize(restore(frame['§b'], dict));
}

/**
 * The fixed decode spec for an agent seeing a frame for the first time. Costs ~90 tokens
 * ONCE per conversation — the project convention (AGENTS.md) makes even that optional.
 */
export const FRAME_SPEC =
  'VSA1 frame: JSON {"§v":1,"§d":[dictionary strings],"§b":body}. In body: string "§<n>" = §d[n] ' +
  '(applies to object keys too); leading "§~" escapes a literal "§"; {"§t":[cols],"§r":[[row],…]} = ' +
  'array of objects (zip cols with each row). Everything else is plain JSON.';

/**
 * Crossover-aware framing — the VSA-VIZ finding applied to DATA. A compression is only a win
 * ABOVE a break-even: a small/irregular payload frames LARGER than raw JSON (columnar+dict
 * overhead, plus the once-per-conversation FRAME_SPEC), so blindly framing everything can COST
 * tokens. Measure both with the same ruler and return the cheaper — never a net loss. Pass
 * `specTokens` = the amortized FRAME_SPEC cost for THIS payload (0 if the spec is already paid /
 * the AGENTS.md convention makes it implicit; ~90/N if this is 1 of N fresh attachments).
 */
export async function frameIfCheaper(value, { specTokens = 0 } = {}) {
  const { countTokens } = await import('./tokens.mjs');
  const raw = JSON.stringify(value);
  const framed = encode(value);
  const [rawTok, frameTok] = await Promise.all([countTokens(raw), countTokens(framed)]);
  const frameCost = frameTok + specTokens;
  const useFrame = frameCost < rawTok;
  return {
    repr: useFrame ? 'frame' : 'raw',
    text: useFrame ? framed : raw,
    rawTok,
    frameTok,
    specTokens,
    saved: useFrame ? rawTok - frameCost : 0,
  };
}
