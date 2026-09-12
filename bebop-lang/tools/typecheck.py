#!/usr/bin/env python3
"""typecheck.py (T48 census, 2026-09-05): infer value types over bpref's AST and report
the places where the DECLARED types disagree with USE: an i64 indexed like an array, an
array used in arithmetic, a call with the wrong arity or an array/i64 mismatch, a return
value of the wrong kind. Read-only; exit 0. Usage: typecheck.py file.bp [...]
Types: 'i64' | '[i64]' | 'str' | '?' (unknown)."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpref
# The only sanctioned i64 <-> [i64] casts: the store's mapping base (selfhost/prelude/store.bp, T111).
CASTS = {'st_cells', 'st_addr', 'addr_of'}
# T48b (2026-09-06): `ref T` is a distinct type. It is produced only by these store fns
# (they return 'ref *', compatible with every `ref T`) or by params/returns declared `ref T`;
# arithmetic on a ref, an i64 where a `ref T` is declared, or a ref where a scalar is
# declared are findings. The generic store helpers take a ref where they declare i64.
REF_PRODUCERS = {'st_alloc', 'st_ref', 'st_root', 'st_mig', 'st_forward', 'st_copy_obj', 'st_open_at', 'st_croot', 'st_head'}  # C3: st_open_at is st_root at a named generation, st_croot the root ref inside a commit object, st_head the commit ref a branch name points at -- all three return a ref through st_ref
REF_TOLERANT = {'st_get', 'st_put', 'st_seal', 'st_check', 'st_len', 'st_link', 'st_supersede', 'st_ref', 'st_commit', 'st_commit_m', 'st_commit_sync', 'st_mig', 'st_cobj', 'st_publish', 'st_commit_c', 'st_croot', 'st_commit_at', 'st_heads_set', 'st_sb_write_ch', 'st_sb_write_m', 'st_parttab_write', 'q1_scan', 'q6_scan'}  # C3's siblings of st_commit take the same root/commit ref against the same i64 declaration. st_sb_write_m and st_parttab_write were added to store.bp by B5 step 1 (4d86e96) and never listed here, which is why the census read 98 findings on 2026-09-12: 74 of them were these two, the direct siblings of st_sb_write_ch and st_commit_m, taking the same root ref against the same i64 declaration. Enumeration omission, not a compiler concession. q1_scan/q6_scan (2026-09-12) are the TPCH query kernels: they take column refs from st_ref and dereference them as base offsets, which is the thesis's own model -- 'queries are ordinary compiled functions' over object-relative offsets. Declaring their params `ref T` instead would only move the 10 findings onto the ref-as-index rule that bench/typecheck_neg/ref_misuse.bp exists to catch, so tolerance is the honest answer, not a declaration change.
STORE_INTERNAL = {'st_compact', 'st_link', 'st_ref', 'st_forward', 'st_copy_obj'}  # the fns that DEFINE object-relative offsets
def is_ref(t): return isinstance(t, str) and t.startswith('ref ')
def ref_ok(pt, at): return pt == at or at == 'ref *' or pt == 'ref *'
BUILTIN = {'zeros': (['i64'], '[i64]'), 'str_len': (['str'], 'i64'), 'char': (['str', 'i64'], 'i64'), 'clock_ms': ([], 'i64'),
           'sys_open': (['[i64]', 'i64', 'i64'], 'i64'), 'sys_slurp': (['i64', 'i64'], 'str'), 'sys_close': (['i64'], 'i64'),
           'sys_read': (['i64', '[i64]', 'i64'], 'i64'), 'sys_write': (['i64', '?', 'i64'], 'i64'), 'sys_exit': (['i64'], 'i64'),
           'sys_export': (['i64', '[i64]', 'i64'], 'i64'), 'sys_arena_base': ([], 'i64'), 'sys_clone': (['i64', 'i64'], 'i64'),
           'sys_cond_set': (['i64', '[i64]', 'i64', 'i64'], 'i64'), 'sys_futex_wait_guard': (['i64', '[i64]', 'i64', 'i64'], 'i64'),
           'sys_futex_wake': (['[i64]', 'i64', 'i64'], 'i64'), 'sys_atomic_add': (['[i64]', 'i64', 'i64'], 'i64'),
           'sys_exit_thread_guard': (['i64', 'i64'], 'i64'), 'sys_readbuf': (['i64', 'i64'], 'str'), 'sys_mmap': (['i64'] * 6, 'i64'),
           'sys_munmap': (['i64', 'i64'], 'i64'), 'sys_ftruncate': (['i64', 'i64'], 'i64'), 'sys_rename': (['[i64]', 'i64', '[i64]', 'i64'], 'i64'),
           'sys_arena_end': ([], 'i64'), 'sys_setaffinity': (['[i64]', 'i64'], 'i64'), 'clz': (['i64'], 'i64'), 'crc32': (['[i64]', 'i64'], 'i64'), 'crc32x': (['[i64]', 'i64', 'i64'], 'i64'), 'sys_msync': (['i64', 'i64', 'i64'], 'i64'), 'sys_mprotect': (['i64', 'i64', 'i64'], 'i64'), 'sys_fsync': (['i64'], 'i64'), 'hvham': (['[i64]', '[i64]', 'i64'], 'i64'), 'hvham2': (['[i64]', 'i64', '[i64]', 'i64', 'i64'], 'i64'),
           'sys_run': (['i64', 'i64', 'i64', '[i64]'], 'i64'), 'sys_wait4': (['i64', '[i64]', 'i64', 'i64'], 'i64'),
            # ROADMAP A7 step 1 (2026-09-09): crc32b(s) -- zlib crc32 of a string's bytes.
            'crc32b': (['str'], 'i64'),
           # A9 step 3 scalar form, added to this table 2026-09-09 -- it had been
           # dispatched by the compiler (emit_sys_scan, 46 words) and implemented by
           # bpref since it landed, but was absent HERE, so every `scan(...)` call
           # typed `?` and no rung ever looked at it. Signature from bebop.bp's own
           # emitter comment: scan(s, pos, class) -> new pos; x0 = s (raw byte base,
           # the same address form char() takes), x1 = the pos ARRAY (pos[0] is
           # advanced and written back, pos[1] is the length bound), x2 = class, a
           # runtime value. tools/builtin_surface.py is the gate that makes an
           # omission like this RED instead of silent.
           'scan': (['str', '[i64]', 'i64'], 'i64')}
class TC:
    def __init__(self, p, fname):
        self.p, self.fname, self.findings = p, fname, []
    def note(self, fn, what):
        self.findings.append('%s:%s: %s' % (self.fname, fn, what))
    def ty(self, e, env, fn):
        t = e[0]
        if t == 'num': return 'i64'
        if t == 'str': return 'str'
        if t == 'var': return env.get(e[1], 'i64' if e[1] in self.p.ctors else '?')
        if t in ('neg', 'not'):
            if self.ty(e[1], env, fn) == '[i64]': self.note(fn, 'unary op on an array')
            return 'i64'
        if t == 'bin':
            a, b = self.ty(e[2], env, fn), self.ty(e[3], env, fn)
            # ROADMAP A5 step 1b (2026-09-08): a `[i64]` VALUE is an x17-relative CELL
            # INDEX, so `arr + k` / `arr - k` with an i64 step is CELL STEPPING and yields
            # another `[i64]` -- the one arithmetic the blueprint's pointer census (§3,
            # class (a)) declares meaningful in index units. Every OTHER operator on an
            # array, and `+`/`-` between two arrays, stays a finding: those are the byte
            # arithmetic (`a + 8`, `a * 8`) the model outlaws.
            step = e[1] in ('+', '-') and (a == '[i64]') != (b == '[i64]') and 'str' not in (a, b) \
                and not is_ref(a) and not is_ref(b)
            if e[1] in ('+', '-', '*', '/', '%', '&', '|', '^', '<<', '>>', '>>>'):
                if ('[i64]' in (a, b) and not step) or 'str' in (a, b): self.note(fn, 'arithmetic %s on %s / %s' % (e[1], a, b))
                # ROADMAP F3 commit 2 (2026-09-09): CELL STEPPING IS NOT IN THE LANGUAGE.
                # It was legal under A5 as "class (a) stepping", and this is the one line
                # that retires it. The reason is not tidiness: a bounds check needs the
                # LENGTH to travel with the array, and the only place to put it is a header
                # cell at data-1 -- which `b = a + 1` makes unsound, because b-1 is then a
                # DATA cell and the length read there is garbage. So the header is not an
                # alternative to banning stepping, it is only sound afterwards.
                # A PARTIAL BAN IS WORTHLESS: catching only the `let b = a + k` form still
                # leaves `f(a + 1)`, and one surviving stepped value is enough to make every
                # header unsound. It is enforced HERE, over bpref's AST, because this is the
                # only checker that infers array-ness for every expression position.
                # Measured before landing: 3 sites in 207 files, both of them A5's own
                # constructs, which are rewritten in this commit and keep their exact values.
                if step: self.note(fn, 'cell stepping (%s %s i64): an array value names a whole allocation, not a slice of one -- index it instead' % (a if a == '[i64]' else b, e[1]))
                if (is_ref(a) or is_ref(b)) and fn not in STORE_INTERNAL: self.note(fn, 'arithmetic %s on a ref (%s / %s): a ref is an object-relative offset, deref it with st_ref' % (e[1], a, b))
            return 'i64'
        if t == 'if':
            self.ty(e[1], env, fn); a, b = self.ty(e[2], env, fn), self.ty(e[3], env, fn)
            return a if a == b else ('?' if '?' in (a, b) else a)
        if t == 'letin':
            env2 = dict(env); env2[e[1]] = self.ty(e[2], env, fn); return self.ty(e[3], env2, fn)
        if t == 'arr': [self.ty(x, env, fn) for x in e[1]]; return '[i64]'
        if t in ('get', 'set'):
            base = env.get(e[1], '?')
            if base == 'i64' and e[1] in self.declared: self.note(fn, 'index into `%s` declared i64' % e[1])
            if base == 'str': self.note(fn, 'index into str `%s`' % e[1])
            self.ty(e[2], env, fn)
            if is_ref(self.ty(e[2], env, fn)): self.note(fn, 'index into `%s` with a ref: a ref is not an array index' % e[1])
            if t == 'set': self.ty(e[3], env, fn)
            return 'str' if base == '[str]' else 'i64'
        if t == 'field': self.ty(e[1], env, fn); return 'i64'
        if t == 'call':
            name, args = e[1], e[2]
            ats = [self.ty(a, env, fn) for a in args]
            if name in self.p.sigs:
                pts, rt = self.p.sigs[name]
                if len(pts) != len(ats): self.note(fn, 'call %s: %d args, %d params' % (name, len(ats), len(pts)))
                for i, (pt, at) in enumerate(zip(pts, ats)):
                    if pt in ('i64', '[i64]', 'str', '[str]') and at in ('i64', '[i64]', 'str', '[str]') and pt != at:
                        self.note(fn, 'call %s: arg %d is %s, param declared %s' % (name, i, at, pt))
                    if is_ref(pt) and at in ('i64', '[i64]', 'str', '[str]') and args[i] != ('num', 0):  # 0 is the null ref (§4a)
                        self.note(fn, 'call %s: arg %d is %s, param declared %s (only a store ref may flow into a ref)' % (name, i, at, pt))
                    if is_ref(pt) and is_ref(at) and not ref_ok(pt, at):
                        self.note(fn, 'call %s: arg %d is %s, param declared %s' % (name, i, at, pt))
                    if is_ref(at) and pt in ('[i64]', 'str', '[str]'):
                        self.note(fn, 'call %s: arg %d is %s, param declared %s' % (name, i, at, pt))
                    if is_ref(at) and pt == 'i64' and name not in REF_TOLERANT:
                        self.note(fn, 'call %s: arg %d is %s, param declared i64 (a ref never leaves the store helpers as a number)' % (name, i, at))
                if name in REF_PRODUCERS: return 'ref *'
                return rt
            if name in BUILTIN:
                pts, rt = BUILTIN[name]
                for i, (pt, at) in enumerate(zip(pts, ats)):
                    if pt in ('i64', '[i64]', 'str') and at in ('i64', '[i64]', 'str') and pt != at:
                        self.note(fn, 'builtin %s: arg %d is %s, expects %s' % (name, i, at, pt))
                return rt
            if name in self.p.ctors: return 'i64'
            return '?'
        if t == 'match':
            return '?'
        return '?'
    def body(self, items, env, fn):
        val = 'i64'
        for it in items:
            if it[0] == 'let':
                r = it[2]
                if r[0] == 'assign':
                    tr = self.ty(r[2], env, fn)
                    if r[1] in env and env[r[1]] != '?' and tr != '?' and env[r[1]] != tr and not (is_ref(env[r[1]]) and is_ref(tr) and ref_ok(env[r[1]], tr)):
                        self.note(fn, 'assign changes `%s` from %s to %s' % (r[1], env[r[1]], tr))
                else:
                    tr = self.ty(r, env, fn)
                    if it[1] != '_':
                        if it[1] in env and env[it[1]] not in ('?', tr) and tr != '?' and not ('i64' in (env[it[1]], tr) and (is_ref(env[it[1]]) or is_ref(tr))) and not (is_ref(env[it[1]]) and is_ref(tr) and ref_ok(env[it[1]], tr)):  # `let prev = 0` then a ref: the null idiom; and `ref *` is compatible with every `ref T` (ref_ok)
                            self.note(fn, 'rebind changes `%s` from %s to %s' % (it[1], env[it[1]], tr))
                        env[it[1]] = tr
                val = 'i64'
            elif it[0] == 'while':
                self.ty(it[1], env, fn); self.body(it[2], env, fn); val = 'i64'
            elif it[0] == 'return':
                val = self.ty(it[1], env, fn)
            elif it[0] == 'break':
                pass
            else:
                val = self.ty(it[1], env, fn)
        return val
    def run(self):
        for fn, (params, body) in self.p.fns.items():
            pts, rt = self.p.sigs.get(fn, ([], 'i64'))
            env = {n: t for n, t in zip(params, pts)}
            self.declared = set(params)
            v = self.body(body, env, fn)
            if rt in ('i64', '[i64]', 'str') and v in ('i64', '[i64]', 'str') and v != rt and fn not in CASTS:
                self.note(fn, 'returns %s, declared -> %s' % (v, rt))
            if is_ref(rt) and v in ('i64', '[i64]', 'str') and fn not in REF_PRODUCERS:
                self.note(fn, 'returns %s, declared -> %s (only a store ref may be returned as a ref)' % (v, rt))
            if is_ref(v) and rt in ('[i64]', 'str'):
                self.note(fn, 'returns %s, declared -> %s' % (v, rt))
        return self.findings
total = 0
for f in sys.argv[1:]:
    src = bpref.expand_use(open(f, encoding='utf-8', errors='replace').read())
    try:
        p = bpref.Parser(src); p.program()
    except Exception as ex:
        print('%s: PARSE %s' % (f, ex)); continue
    fs = TC(p, f).run(); total += len(fs)
    for x in fs: print(x)
    print('%s: %d findings' % (f, len(fs)))
print('typecheck census: %d findings' % total)
