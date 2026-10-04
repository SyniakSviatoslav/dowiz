// GATE-2 (W-MR0, 2026-10-04): NOTHING THE GUEST DOES ON THE MENU LEAVES THE DEVICE.
//
// The operator allowed guest profiling ON THE GUEST'S DEVICE ONLY ("усе збирати можна, але не
// передавати нікому, усе на пристрої клієнта", memory dowiz-dynamic-menu-decisions-2026-10-03;
// docs/research/2026-10-03-dynamic-menu-and-resilience.md §6 row 2). This gate is the half of
// that promise a reader can check without trusting the next lane: it reads every .js under
// workers/api/public/store/ and REFUSES
//   1. `sendBeacon` anywhere: a beacon exists to report behaviour on the way out of a page;
//   2. a scroll / wheel / touchmove listener, an `onscroll =`, or an IntersectionObserver whose
//      handler (or a function of the same file that the handler calls by name, one level deep)
//      makes a network call: that is dwell and scroll-depth telemetry, whatever it is named;
//   3. an on-device profile key (`dowiz.taste*`, `dw_profile*`) inside the arguments of a
//      network call or of a `JSON.stringify(` (the shape of every request body here);
//   4. the ONE field allowed to carry the guest's taste to the venue, `taste_sync` (operator
//      2026-10-04, ruling 2: guest taste is scored on the device AND on the server, ON by default
//      under legitimate interest, stopped by one tap -- an objection; DECISIONS.md D0), on any line
//      that does not also make the objection check `notObjected()`. It is the aggregated vector,
//      never raw events, and it never leaves once the guest said stop;
//   5. FINGERPRINTING (operator 2026-10-04: "never fingerprinting"): recognising a guest by their
//      device instead of by what they gave. Refused anywhere in the storefront: canvas read-back
//      (`toDataURL`, `getImageData`), audio fingerprinting (`OfflineAudioContext`), and the device
//      surfaces that only a fingerprint reads (`hardwareConcurrency`, `deviceMemory`,
//      `navigator.plugins`, `navigator.platform`, `navigator.keyboard`, `screen.colorDepth`,
//      `getBattery`). `navigator.userAgent` is allowed ONLY as `/regex/.test(navigator.userAgent)`
//      (a yes/no such as "is this iOS", which cannot tell two phones apart); any other read is refused.
// What stays refused whatever the guest chose: beacons and scroll/visibility telemetry (rules 1-2),
// the raw profile under its storage key (rule 3) and fingerprinting (rule 5).
// It is a static gate, so it is a floor, not a proof of innocence: a value read from the profile
// into a variable and sent under another name passes it. Rule 3 stops the obvious path; the
// review stops the rest. The scan root is argv[2] (the proof points it at a scratch copy).
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = process.argv[2] || join(new URL('.', import.meta.url).pathname, '../..');
const dir = join(root, 'workers/api/public/store');
const NET = /\bfetch\w*\s*\(|\bsendBeacon\b|\bXMLHttpRequest\b|\bnew\s+WebSocket\b|\.send\s*\(|\bEventSource\b/;
const PROFILE = /dowiz\.taste|dw_profile/;
const NOT_OBJECTED = /\bnotObjected\s*\(\s*\)/;
const FINGERPRINT = /\btoDataURL\s*\(|\bgetImageData\s*\(|\bOfflineAudioContext\b|\bhardwareConcurrency\b|\bdeviceMemory\b|\bnavigator\.(plugins|platform|keyboard)\b|\bscreen\.colorDepth\b|\bgetBattery\s*\(/g;
const UA_READ = /\bnavigator\.userAgent\b/g;
const UA_TEST = /\/[^/\n]+\/[a-z]*\.test\(\s*navigator\.userAgent\s*\)/g;
const WATCH = /\bnew\s+IntersectionObserver\s*\(|\baddEventListener\s*\(\s*['"`](scroll|scrollend|wheel|touchmove)['"`]\s*,|\bonscroll\s*=/g;

function files(d) {
  const out = [];
  for (const n of readdirSync(d)) {
    const p = join(d, n);
    if (statSync(p).isDirectory()) out.push(...files(p));
    else if (/\.m?js$/.test(n) && !/\.test\.m?js$/.test(n)) out.push(p);
  }
  return out.sort();
}
/// Comments out, strings kept (a profile key is a string). Line numbers survive: newlines stay.
function strip(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, m => m.replace(/[^\n]/g, ' '))
            .replace(/(^|[^:'"`\\])\/\/[^\n]*/g, (m, a) => a + ' '.repeat(m.length - a.length));
}
/// The balanced (...) or { ... } text starting at the first opener at or after `i`.
function balanced(s, i) {
  let k = i; while (k < s.length && s[k] !== '(' && s[k] !== '{') k++;
  if (k >= s.length) return s.slice(i, i + 400);
  const open = s[k], close = open === '(' ? ')' : '}';
  let depth = 0;
  for (let j = k; j < s.length; j++) {
    if (s[j] === open) depth++;
    else if (s[j] === close && --depth === 0) return s.slice(k, j + 1);
  }
  return s.slice(k);
}
/// End of a handler that starts at `i`: the first balanced group, and when it was only an arrow's
/// or a `function`'s parameter list, the body after it too (`(e) => f(e)`, `function (e) { .. }`).
function handlerEnd(s, i) {
  const span = balanced(s, i);
  let end = s.indexOf(span, i) + span.length;
  const rest = s.slice(end);
  const arrow = rest.match(/^\s*=>\s*/), block = rest.match(/^\s*(?=\{)/);
  if (span.startsWith('(') && (arrow || block)) {
    const at = end + (arrow || block)[0].length;
    const next = balanced(s, at);
    end = s.indexOf(next, at) + next.length;
  }
  return end;
}
const lineOf = (s, i) => s.slice(0, i).split('\n').length;
/// Bodies of `function name(` and `const name = (...) =>` in the file, by name.
function localFns(s) {
  const m = new Map();
  for (const x of s.matchAll(/\bfunction\s+([A-Za-z_$][\w$]*)\s*\(/g)) m.set(x[1], balanced(s, s.indexOf('{', x.index)));
  for (const x of s.matchAll(/\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/g)) m.set(x[1], balanced(s, x.index + x[0].length));
  return m;
}

const refusals = [];
let scanned = 0;
for (const f of files(dir)) {
  scanned++;
  const s = strip(readFileSync(f, 'utf8'));
  const rel = relative(root, f);
  for (const x of s.matchAll(/\bsendBeacon\b/g)) refusals.push(`${rel}:${lineOf(s, x.index)}: sendBeacon (rule 1)`);
  const fns = localFns(s);
  for (const x of s.matchAll(WATCH)) {
    // From the end of the match through the first balanced (...) or {...} after it: covers
    // `new IntersectionObserver(cb)`, `addEventListener('scroll', e => fetch(..))` and `onscroll = () => {..}`.
    const from = x.index + x[0].length;
    const body = s.slice(from, handlerEnd(s, from));
    let hit = NET.test(body) ? 'itself' : null;
    if (!hit) for (const c of body.matchAll(/\b([A-Za-z_$][\w$]*)\s*\(/g)) {
      const fb = fns.get(c[1]);
      if (fb && NET.test(fb)) { hit = `via ${c[1]}()`; break; }
    }
    if (hit) refusals.push(`${rel}:${lineOf(s, x.index)}: a scroll/visibility handler makes a network call ${hit} (rule 2)`);
  }
  s.split('\n').forEach((line, i) => {
    if (/\btaste_sync\b/.test(line) && !NOT_OBJECTED.test(line)) refusals.push(`${rel}:${i + 1}: taste_sync without notObjected() on its line (rule 4)`);
    for (const x of line.matchAll(FINGERPRINT)) refusals.push(`${rel}:${i + 1}: ${x[0].replace(/\s*\($/, '')} reads a device fingerprint surface (rule 5)`);
    const uaAll = [...line.matchAll(UA_READ)].length, uaOk = [...line.matchAll(UA_TEST)].length;
    if (uaAll > uaOk) refusals.push(`${rel}:${i + 1}: navigator.userAgent read other than /re/.test(navigator.userAgent) (rule 5)`);
  });
  for (const x of s.matchAll(/\bfetch\w*\s*\(|\bsendBeacon\s*\(|\.send\s*\(|\bJSON\.stringify\s*\(/g)) {
    const args = balanced(s, x.index + x[0].length - 1);
    if (PROFILE.test(args)) refusals.push(`${rel}:${lineOf(s, x.index)}: an on-device profile key inside a request body (rule 3)`);
  }
}
if (scanned === 0) { console.log(`no-tracking: REFUSED -- no .js under ${relative(root, dir) || dir}; a gate that read nothing proves nothing`); process.exit(2); }
for (const r of refusals) console.log('  ' + r);
if (refusals.length) {
  console.log(`no-tracking: REFUSED -- ${refusals.length} site(s) in ${scanned} file(s) send guest behaviour off the device`);
  process.exit(1);
}
console.log(`no-tracking: 0 sites in ${scanned} storefront file(s) send guest behaviour off the device`);
