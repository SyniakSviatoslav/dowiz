#!/bin/sh
# S7c — A FORMAT CHANGE SHIPS ITS MIGRATION LAW, OR IT DOES NOT MERGE (W-TASTE2, 2026-10-06).
#
# R-S7 §E6 (docs/research/2026-10-06-s7-sheaf-engine-arxiv-and-experiments.md): bebop never
# rewrites a log when a record gains a field; the DECODER supplies the default (Spivak's Sigma_F
# done at read time). That is only safe while two laws hold for every change -- Delta o Sigma = id
# on old records, and readers agree, decode(old) == decode(encode(decode(old))) -- and CLAUDE.md
# records what skipping them cost (B5 step 1: five gates, two oracles and a harness model). The
# operator (2026-10-06) agreed this gate MAY block a lane's merge.
#
# WHAT IS A FORMAT, as this gate reads the source (comments stripped):
#   * every stock record kind: the field names its `encode` writes, in order
#     (crates/dowiz-hub/src/stock/codec.rs, `{"k":"<kind>",...}`), as `stock.<kind>`;
#   * every DG7 block schema string (crates/dowiz-hub/src/block/schema.rs), as `block.<name>`;
#   * THE ORDER LOG (W-P0-LOG, R-BINREC P0, 2026-10-29) -- before this an order-log payload could
#     change shape and pass unnoticed. Its formats, each read from the code that writes it:
#       log.kinds  the kind byte of every `EventKind` (event.rs `from_byte`) and forget's REDACTED_BIT;
#       log.frame  what `Hub::append` pushes, in order (log.rs: `[kind][id_len][id][json]`, then `Entropy::stamp` frames it under the log's scheme);
#       log.order  the kernel's order snapshot, the keys `order_to_value`/`item_to_value` write
#                  (crates/dowiz-core/src/json_api.rs) -- the base envelope of Placed/Advanced;
#       log.delta  the delta marks every order kind folds by (room/delta.rs DELTA_MARK, DELTA_DROP);
#       log.<kind> per EventKind, lowercase: the JSON keys LITERALLY written (`"k":` in a json!
#                  or a raw string, `x["k"] =`) in every non-test fn that writes that kind -- an
#                  `append(..EventKind::K`, `events.push((EventKind::K`, `Ok(Some((..EventKind::K`,
#                  `kind: EventKind::K`, `payload.push(EventKind::K`, `body.push(EventKind::K` or a `const ..: EventKind =`
#                  alias of it -- plus the same-file fns it calls, one level. `-` when none.
#                  NOT seen: keys stamped by helpers in OTHER files (storefront's envelope, tax,
#                  loyalty); a writer that adds a key there changes no fingerprint. A writer site
#                  this gate cannot place in a fn is refused, so a new writer cannot hide.
# THE REGISTER (tools/gates/migration-law.registry), one line per VERSION of a format:
#     <format> <version> <fingerprint> <law test path>
# REFUSED, one hit each:
#   1. a format the source has and the register does not;
#   2. a format whose NEWEST registered version does not match the source (changed, no law shipped);
#   3. a law test the source does not hold as `#[test] fn <last segment>` (crates/dowiz-hub/src,
#      workers/api/src, crates/dowiz-core/src) -- a register naming a test nobody wrote proves nothing;
#   4. two versions of one format naming the same law test (each change brings its own);
#   5. a registered format the source no longer has (a stale register is a lying one).
# It does NOT run the tests: cargo does, and run-all runs cargo. It proves they EXIST and are named.
#
# `sh tools/gates/migration-law.sh [ROOT]` -- ROOT defaults to the repo, so `.prove.sh` can run
# this same code on a scratch copy holding a deliberate defect. Exit 1 on any hit.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
REGISTRY="${MIGRATION_LAW_REGISTRY:-$ROOT/tools/gates/migration-law.registry}"

ROOT="$ROOT" REGISTRY="$REGISTRY" python3 - <<'PY'
import os, re, glob, sys
ROOT, REG = os.environ['ROOT'], os.environ['REGISTRY']
strip = lambda s: re.sub(r'//[^\n]*', '', s)
def read(p):
    with open(os.path.join(ROOT, p), encoding='utf-8') as f:
        return strip(f.read())

found = {}
codec = read('crates/dowiz-hub/src/stock/codec.rs')
a, b = codec.find('pub fn encode'), codec.find('pub fn decode')
enc = codec[a:b] if a >= 0 and b > a else ''
for kind, body in re.findall(r'\{\{"k":"(\w+)"(.*?)\}\}"#', enc, re.S):
    found['stock.' + kind] = ','.join(re.findall(r'"(\w+)":', body))
for name, s in re.findall(r'name:\s*"(\w+)",\s*string:\s*"([^"]+)"', read('crates/dowiz-hub/src/block/schema.rs')):
    found['block.' + name] = s

# ── THE ORDER LOG ── a small Rust lexer: strings and comments blanked, string contents kept.
def lex(s):
    out, strs, i, n = [], [], 0, len(s)
    while i < n:
        c = s[i]
        if s.startswith('//', i):
            j = s.find('\n', i); j = n if j < 0 else j; out.append(' ' * (j - i)); i = j; continue
        if s.startswith('/*', i):
            j = s.find('*/', i + 2); j = n if j < 0 else j + 2; out.append(re.sub(r'[^\n]', ' ', s[i:j])); i = j; continue
        word = i > 0 and (s[i - 1].isalnum() or s[i - 1] == '_')
        m = None if word else re.match(r'b?r(#*)"', s[i:i + 12])
        if m:
            end = '"' + m.group(1); k = i + len(m.group(0)); j = s.find(end, k); j = n if j < 0 else j + len(end)
            strs.append((i, j, s[k:j - len(end)])); out.append(' ' * (j - i - 1) + '"'); i = j; continue
        if c == '"' or (c == 'b' and not word and s[i + 1:i + 2] == '"'):
            k = i + (1 if c == '"' else 2); j = k
            while j < n and s[j] != '"':
                j += 2 if s[j] == '\\' else 1
            j += 1; strs.append((i, j, s[k:j - 1])); out.append(' ' * (j - i - 1) + '"'); i = j; continue
        if c == "'":
            m = re.match(r"'(\\[^']{1,8}|[^\\'])'", s[i:i + 12])
            if m:
                out.append(' ' * len(m.group(0))); i += len(m.group(0)); continue
        out.append(c); i += 1
    return ''.join(out), strs

def block_end(code, b):
    d = 0
    for j in range(b, len(code)):
        d += {'{': 1, '}': -1}.get(code[j], 0)
        if d == 0:
            return j
    return len(code)

def fns(code):
    res = []
    for m in re.finditer(r'\bfn\s+(\w+)', code):
        d, j = 0, m.end()  # the body is the first `{` outside the signature's () and [] (`[T; 2]`)
        while j < len(code) and not (d == 0 and code[j] in '{;'):
            d += {'(': 1, '[': 1, ')': -1, ']': -1}.get(code[j], 0); j += 1
        if j < len(code) and code[j] == '{':
            res.append((m.group(1), m.start(), block_end(code, j)))
    return res

def keys_in(code, strs, a, b):
    ks = set()
    for i, j, c in strs:
        if a <= i < b:
            if re.match(r'\s*:(?!:)', code[j:j + 4]) or (code[i - 1:i] == '[' and re.match(r'\]\s*=(?!=)', code[j:j + 6])):
                ks.add(c)
            ks.update(re.findall(r'"(\w+)":', c) + re.findall(r'\b(\w+)=\{', c))  # json, `k={}` text
    return ks

ev = read('crates/dowiz-hub/src/event.rs')
table = re.findall(r'(\d+)\s*=>\s*Some\(EventKind::(\w+)\)', ev)
bit = re.search(r'REDACTED_BIT\s*:\s*u8\s*=\s*(\w+)', read('crates/dowiz-hub/src/forget.rs'))
found['log.kinds'] = ','.join(f'{b}={k}' for b, k in table) + ';redacted=' + (bit.group(1) if bit else '?')
lg = read('crates/dowiz-hub/src/log.rs')
a = lg.find('pub fn append('); b = lg.find('.stamp(', a)
found['log.frame'] = '|'.join((['stamp'] if b > 0 else []) + [re.sub(r'\s+', '', x) for x in re.findall(r'(?:payload|body)\.(?:push|extend_from_slice)\((.*?)\);', lg[a:b])])
ja = read('crates/dowiz-core/src/json_api.rs')
part = lambda f: ','.join(re.findall(r'String::from\("(\w+)"\)', ja[ja.find('fn ' + f):ja.find('\n}', ja.find('fn ' + f))]))
found['log.order'] = part('order_to_value').replace('items', 'items[' + part('item_to_value') + ']')
dl = read('crates/dowiz-hub/src/room/delta.rs')
found['log.delta'] = ','.join(re.findall(r'pub const DELTA_(?:MARK|DROP)\s*:\s*&str\s*=\s*"(\w+)"', dl))

kinds = [k for _, k in table]
logkeys, unplaced = {k: set() for k in kinds}, []
WRITE = re.compile(r'(append\w*\s*\(|events\.push\(\(|Ok\(Some\(\(|kind:\s*|(?:payload|body)\.push\(|EventKind\s*=\s*)[^;{}]*$')
for d in ('crates/dowiz-hub/src', 'workers/api/src'):
    for f in sorted(glob.glob(os.path.join(ROOT, d, '**', '*.rs'), recursive=True)):
        rel = os.path.relpath(f, ROOT)
        if re.search(r'(^|/)tests?(\.rs|/)|_tests?\.rs$', rel) or rel.endswith('/event.rs'):
            continue
        code, strs = lex(open(f, encoding='utf-8').read())
        tmods = [(m.end() - 1, block_end(code, m.end() - 1)) for m in re.finditer(r'#\[cfg\(test\)\]\s*mod\s+\w+\s*\{', code)]
        F = fns(code)
        alias = {m.group(1): m.group(2) for m in re.finditer(r'const\s+(\w+)\s*:\s*[\w:]*EventKind\s*=\s*[\w:]*EventKind::(\w+)', code)}
        pat = r'EventKind::(\w+)\b' + ''.join(f'|\\b({a})\\b' for a in alias)
        for m in re.finditer(pat, code):
            k = m.group(1) or alias.get(m.group(0))
            if k not in logkeys or any(x <= m.start() <= y for x, y in tmods):
                continue
            pre = code[max(0, m.start() - 120):m.start()]
            if not WRITE.search(pre) or re.search(r'(broadcast\s*\(|const\s+\w+\s*:)[^;]*$', pre):
                continue
            enc = [x for x in F if x[1] <= m.start() <= x[2]]
            if not enc:
                unplaced.append(f'{rel}:{code.count(chr(10), 0, m.start()) + 1}'); continue
            name, a, b = enc[-1]
            logkeys[k] |= keys_in(code, strs, a, b)
            called = set(re.findall(r'\b(\w+)\s*\(', code[a:b])) - {name}
            for n2, a2, b2 in F:
                if n2 in called:
                    logkeys[k] |= keys_in(code, strs, a2, b2)
for k in kinds:
    found['log.' + k.lower()] = ','.join(sorted(logkeys[k])) or '-'
nlog = sum(1 for k in found if k.startswith('log.'))

if len([k for k in found if k.startswith('stock.')]) < 10 or len([k for k in found if k.startswith('block.')]) < 4 \
        or len(kinds) < 8 or nlog < 12 or not bit or found['log.frame'].count('|') < 3 \
        or found['log.order'].count(',') < 8 or found['log.delta'].count(',') != 1:
    print(f'migration-law: REFUSED -- read only {len(found)} formats; the source moved, update this gate')
    sys.exit(1)

tests = set()
for d in ('crates/dowiz-hub/src', 'workers/api/src', 'crates/dowiz-core/src'):
    for f in glob.glob(os.path.join(ROOT, d, '**', '*.rs'), recursive=True):
        src = strip(open(f, encoding='utf-8').read())
        tests.update(re.findall(r'#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)', src))

reg = {}
hits = []
for n, line in enumerate(open(REG, encoding='utf-8'), 1):
    line = line.split('#', 1)[0].strip()
    if not line:
        continue
    parts = line.split()
    if len(parts) != 4 or not parts[1].isdigit():
        hits.append(f'register line {n}: want "<format> <version> <fingerprint> <law test>", got {line!r}')
        continue
    fmt, ver, fp, law = parts[0], int(parts[1]), parts[2], parts[3]
    reg.setdefault(fmt, []).append((ver, fp, law))
    if law.split('::')[-1] not in tests:
        hits.append(f'{fmt} v{ver}: law test {law} is not a #[test] fn in the source')
for fmt, cur in sorted(found.items()):
    vs = sorted(reg.get(fmt, []))
    if not vs:
        hits.append(f'{fmt}: a format with no migration law registered ({cur})')
        continue
    if vs[-1][1] != cur:
        hits.append(f'{fmt}: changed to {cur!r}; newest registered v{vs[-1][0]} is {vs[-1][1]!r} -- ship the migration law test and register v{vs[-1][0] + 1}')
    laws = [l for _, _, l in vs]
    if len(set(laws)) != len(laws):
        hits.append(f'{fmt}: two versions name the same law test; each change brings its own')
for fmt in sorted(set(reg) - set(found)):
    hits.append(f'{fmt}: registered, but the source has no such format')

for u in unplaced:
    hits.append(f'{u}: an order-log writer outside any fn; this gate cannot fingerprint it')
for h in hits:
    print('  REFUSED ' + h)
print(f'migration-law: {len(found)} formats ({sum(1 for k in found if k.startswith("stock."))} stock kinds, '
      f'{sum(1 for k in found if k.startswith("block."))} block schemas, {nlog} order-log formats), {sum(len(v) for v in reg.values())} registered versions, {len(hits)} refusal(s)')
sys.exit(1 if hits else 0)
PY
