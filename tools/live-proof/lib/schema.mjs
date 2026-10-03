// THE SCHEMA CHECK every live answer passes before a row may say LIVE-PROVEN.
//
// The subset of JSON Schema draft 2020-12 the contracts in ../contracts use:
// type (incl. a list, "integer" inside "number"), const, enum, required,
// properties, additionalProperties (false or a schema), items, minItems,
// minLength, pattern, minimum, maximum, $defs/$ref inside the schema.
// A keyword this file does not know is REPORTED, never ignored: a contract
// that tightens itself with an unknown word would otherwise pass everything
// (memory gates-that-measure-their-own-epitaph).
const KNOWN = new Set(['$schema', '$id', '$defs', '$ref', 'type', 'const', 'enum', 'required', 'properties',
  'additionalProperties', 'items', 'minItems', 'maxItems', 'minLength', 'maxLength', 'pattern', 'minimum', 'maximum',
  'description', 'title', 'examples']);

const typeOf = v => Array.isArray(v) ? 'array' : v === null ? 'null' : Number.isInteger(v) ? 'integer' : typeof v;

/// The list of violations of `v` against `s`, each with its JSON path. Empty = valid.
export function validate(v, s, root = s, at = '$') {
  if (s === true || s == null) return [];
  if (s === false) return [`${at}: no value is allowed here`];
  const out = [];
  for (const k of Object.keys(s)) if (!KNOWN.has(k)) out.push(`${at}: schema keyword "${k}" is not checked by tools/live-proof/lib/schema.mjs`);
  if (s.$ref) return out.concat(validate(v, root.$defs?.[s.$ref.replace(/^#\/\$defs\//, '')] ?? false, root, at));
  const t = typeOf(v);
  if (s.type) {
    const want = [].concat(s.type);
    if (!(want.includes(t) || (t === 'integer' && want.includes('number')))) return out.concat(`${at}: ${t}, wanted ${want.join('|')}`);
  }
  if ('const' in s && JSON.stringify(v) !== JSON.stringify(s.const)) out.push(`${at}: ${JSON.stringify(v).slice(0, 60)} is not ${JSON.stringify(s.const)}`);
  if (s.enum && !s.enum.some(e => JSON.stringify(e) === JSON.stringify(v))) out.push(`${at}: ${JSON.stringify(v).slice(0, 60)} not in ${JSON.stringify(s.enum)}`);
  if (typeof v === 'number') {
    if (s.minimum != null && v < s.minimum) out.push(`${at}: ${v} < ${s.minimum}`);
    if (s.maximum != null && v > s.maximum) out.push(`${at}: ${v} > ${s.maximum}`);
  }
  if (typeof v === 'string') {
    if (s.minLength != null && v.length < s.minLength) out.push(`${at}: length ${v.length} < ${s.minLength}`);
    if (s.maxLength != null && v.length > s.maxLength) out.push(`${at}: length ${v.length} > ${s.maxLength}`);
    if (s.pattern && !new RegExp(s.pattern, 'u').test(v)) out.push(`${at}: does not match /${s.pattern}/`);
  }
  if (Array.isArray(v)) {
    if (s.minItems != null && v.length < s.minItems) out.push(`${at}: ${v.length} items < ${s.minItems}`);
    if (s.maxItems != null && v.length > s.maxItems) out.push(`${at}: ${v.length} items > ${s.maxItems}`);
    if (s.items) v.forEach((x, i) => out.push(...validate(x, s.items, root, `${at}[${i}]`)));
  }
  if (t === 'object') {
    for (const k of s.required || []) if (!(k in v)) out.push(`${at}.${k}: missing`);
    const props = s.properties || {};
    for (const [k, x] of Object.entries(v)) {
      if (k in props) out.push(...validate(x, props[k], root, `${at}.${k}`));
      else if (s.additionalProperties !== undefined) out.push(...validate(x, s.additionalProperties, root, `${at}.${k}`));
    }
  }
  return out;
}
