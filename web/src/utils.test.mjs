import { sanitize, formatETA, estimateETA, validatePhone, validateAddress, generateId } from './lib/utils.mjs';

let passed = 0;
let failed = 0;
function assert(ok, msg) {
  if (ok) { passed++; }
  else { console.error(`FAIL: ${msg}`); failed++; }
}

assert(sanitize('<script>alert(1)</script>') === '&lt;script&gt;alert(1)&lt;/script&gt;', 'sanitize strips script tags');
assert(sanitize('hello') === 'hello', 'sanitize passes clean text');
assert(sanitize('') === '', 'sanitize handles empty string');
assert(sanitize(null) === '', 'sanitize handles null');

assert(formatETA(0) === 'зараз', 'formatETA 0');
assert(formatETA(5) === '~5 хв', 'formatETA 5min');
assert(formatETA(90) === '~1 год 30 хв', 'formatETA 90min');

assert(estimateETA({ status: 'pending' }) === '~5 хв', 'estimateETA pending');
assert(estimateETA({ status: 'delivered' }) === null, 'estimateETA delivered returns null');
assert(estimateETA(null) === null, 'estimateETA null returns null');

assert(validatePhone('+355 69 111 1111') === true, 'validatePhone valid');
assert(validatePhone('12') === false, 'validatePhone too short');

assert(validateAddress('Rruga e Dibrës 12') === true, 'validateAddress valid');
assert(validateAddress('AB') === false, 'validateAddress too short');

const id1 = generateId();
const id2 = generateId();
assert(typeof id1 === 'number', 'generateId returns number');
assert(id2 > id1 || id2 === id1, 'generateId monotonic');

console.log(`\nutils tests: ${passed} passed, ${failed} failed${failed > 0 ? ' ❌' : ' ✅'}`);
process.exit(failed > 0 ? 1 : 0);
