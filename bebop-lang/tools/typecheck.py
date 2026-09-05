#!/usr/bin/env python3
"""typecheck.py (T48 census, 2026-09-05): infer value types over bpref's AST and report
the places where the DECLARED types disagree with USE: an i64 indexed like an array, an
array used in arithmetic, a call with the wrong arity or an array/i64 mismatch, a return
value of the wrong kind. Read-only; exit 0. Usage: typecheck.py file.bp [...]
Types: 'i64' | '[i64]' | 'str' | '?' (unknown)."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpref
BUILTIN = {'zeros': (['i64'], '[i64]'), 'str_len': (['str'], 'i64'), 'char': (['str', 'i64'], 'i64'), 'clock_ms': ([], 'i64'),
           'sys_open': (['[i64]', 'i64', 'i64'], 'i64'), 'sys_slurp': (['i64', 'i64'], 'str'), 'sys_close': (['i64'], 'i64'),
           'sys_read': (['i64', '[i64]', 'i64'], 'i64'), 'sys_write': (['i64', '?', 'i64'], 'i64'), 'sys_exit': (['i64'], 'i64'),
           'sys_export': (['i64', '[i64]', 'i64'], 'i64'), 'sys_arena_base': ([], 'i64'), 'sys_clone': (['i64', 'i64'], 'i64'),
           'sys_cond_set': (['i64', '[i64]', 'i64', 'i64'], 'i64'), 'sys_futex_wait_guard': (['i64', '[i64]', 'i64', 'i64'], 'i64'),
           'sys_futex_wake': (['[i64]', 'i64', 'i64'], 'i64'), 'sys_atomic_add': (['[i64]', 'i64', 'i64'], 'i64'),
           'sys_exit_thread_guard': (['i64', 'i64'], 'i64'), 'sys_readbuf': (['i64', 'i64'], 'str'), 'sys_mmap': (['i64'] * 6, 'i64'),
           'sys_munmap': (['i64', 'i64'], 'i64'), 'sys_ftruncate': (['i64', 'i64'], 'i64'), 'sys_rename': (['[i64]', 'i64', '[i64]', 'i64'], 'i64'),
           'sys_arena_end': ([], 'i64'), 'sys_setaffinity': (['[i64]', 'i64'], 'i64'), 'clz': (['i64'], 'i64'), 'hvham': (['[i64]', '[i64]', 'i64'], 'i64'), 'hvham2': (['[i64]', 'i64', '[i64]', 'i64', 'i64'], 'i64')}
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
            if e[1] in ('+', '-', '*', '/', '%', '&', '|', '^', '<<', '>>', '>>>'):
                if '[i64]' in (a, b) or 'str' in (a, b): self.note(fn, 'arithmetic %s on %s / %s' % (e[1], a, b))
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
                    if r[1] in env and env[r[1]] != '?' and tr != '?' and env[r[1]] != tr:
                        self.note(fn, 'assign changes `%s` from %s to %s' % (r[1], env[r[1]], tr))
                else:
                    tr = self.ty(r, env, fn)
                    if it[1] != '_':
                        if it[1] in env and env[it[1]] not in ('?', tr) and tr != '?':
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
            if rt in ('i64', '[i64]', 'str') and v in ('i64', '[i64]', 'str') and v != rt:
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
