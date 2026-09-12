#!/usr/bin/env python3
"""
B7 qdsl mirror for bpref — production version.

Adds DSL parsing support to bpref's Interp class so that qdsl.* functions
can be executed in the Python interpreter exactly as they would on bebop.bin.
"""
import os
import re
import sys

DEPTH_CAP = int(os.environ.get('BPREF_DEPTH', '5000'))

class Cells(object):
    __slots__ = ('arena', 'off', 'n', 'released')
    def __init__(self, arena, off, n):
        self.arena = arena
        self.off = off
        self.n = n
        self.released = False
    def __len__(self):
        return self.n
    def _at(self, i):
        if i < 0 or i >= self.n:
            raise IndexError('cell %d outside a %d-cell array' % (i, self.n))
        return self.off + i
    def __getitem__(self, i):
        if isinstance(i, slice):
            lo, hi, st = i.indices(self.n)
            return [self.arena[self.off + k] for k in range(lo, hi, st)]
        return self.arena[self._at(i)]
    def __setitem__(self, i, v):
        self.arena[self._at(i)] = v
    def __iter__(self):
        for k in range(self.n):
            yield self.arena[self.off + k]
    def __add__(self, k):
        if not isinstance(k, int):
            return NotImplemented
        v = Cells(self.arena, self.off + k, self.n - k)
        v.released = self.released
        return v
    __radd__ = __add__
    def __sub__(self, k):
        if not isinstance(k, int):
            return NotImplemented
        return self.__add__(-k)
    def __repr__(self):
        return 'cells@%d[%d]' % (self.off, self.n)

def cells_alloc(it, vals):
    arena = it.arena
    off = len(arena)
    arena.extend(vals)
    return Cells(arena, off, len(vals))

class ReturnSignal(Exception):
    def __init__(self, v): self.v = v
class BreakSignal(Exception):
    pass

RESERVED = set(['sys_msync', 'sys_fsync', 'sys_mprotect', 'crc32x', 'crc32', 'clz',
    'sys_setaffinity', 'let', 'while', 'if', 'then', 'else', 'in', 'fn', 'enum',
    'struct', 'module', 'match', 'return', 'break', 'zeros', 'char', 'str_len',
    'clock_ms', 'hvham', 'hvham2', 'some', 'none', 'many', 'sys_open', 'sys_read',
    'sys_write', 'sys_close', 'sys_readbuf', 'sys_slurp', 'sys_mmap', 'sys_mapb',
    'sys_munmap', 'sys_ftruncate', 'sys_rename', 'sys_export', 'sys_exit',
    'sys_arena_base', 'sys_arena_end', 'sys_clone', 'sys_cond_set', 'sys_futex_wait_guard',
    'sys_futex_wake', 'sys_atomic_add', 'sys_exit_thread_guard', 'sys_run', 'sys_wait4',
    'scan', 'crc32b'])

class DepthError(Exception):
    pass

# A23. An oracle that cannot evaluate a form must SAY SO, not return something. Until
# 2026-09-13 an unmodelled builtin reached `raise NameError('unknown fn ...')` and came back
# as a generic error, and a program whose unmodelled semantics led it to `exit` came back as
# rc=0 with EMPTY stdout -- which bench/vs_rust/bpref_parity.sh then read as a DISAGREEMENT
# with the compiler. Seven of that lane's first eight findings were this, not a divergence.
class UnsupportedForm(Exception):
    pass

# bpref has no threads and no processes: it evaluates one program in one Python thread. A
# construct using these cannot be differentially checked, and pretending otherwise is what
# produced c84_run's silent empty answer -- its sys_clone is not modelled at all, so `if pid
# == 0` took the child branch and the program exited 0 without printing.
UNMODELLED = set(['sys_clone', 'sys_run', 'sys_wait4', 'sys_futex_wait_guard', 'sys_futex_wake',
    'sys_atomic_add', 'sys_exit_thread_guard', 'sys_cond_set', 'sys_setaffinity'])

MASK = (1 << 64) - 1

def wrap(x):
    if not isinstance(x, int):
        return x
    x &= MASK
    return x - (1 << 64) if x >> 63 else x

def i_div(a, b):
    if b == 0:
        return 0
    q = abs(a) // abs(b)
    return wrap(q if (a < 0) == (b < 0) else -q)

BIN = {
    '+': lambda a, b: wrap(a + b), '-': lambda a, b: wrap(a - b),
    '*': lambda a, b: wrap(a * b), '/': i_div,
    '%': lambda a, b: wrap(a - i_div(a, b) * b),
    '&': lambda a, b: wrap(a & b), '|': lambda a, b: wrap(a | b),
    '&&': lambda a, b: wrap(a & b), '||': lambda a, b: wrap(a | b),
    '^': lambda a, b: wrap(a ^ b),
    '<<': lambda a, b: wrap(a << (b & 63)),
    '>>': lambda a, b: wrap((a & MASK) >> (b & 63)),
    '>>>': lambda a, b: wrap(a >> (b & 63)),
    '==': lambda a, b: int(a == b), '!=': lambda a, b: int(a != b),
    '<': lambda a, b: int(a < b), '>': lambda a, b: int(a > b),
    '<=': lambda a, b: int(a <= b), '>=': lambda a, b: int(a >= b),
}
TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('|', '||'), ('^',), ('&', '&&'),
         ('<<', '>>', '>>>'), ('+', '-'), ('*', '/', '%')]
if os.environ.get('BPREF_OLDPREC') == '1':
    TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('+', '-'), ('*', '/', '%'),
             ('&', '|', '^', '<<', '>>', '>>>')]
if os.environ.get('BPREF_ASR') == '1':
    BIN['>>'] = lambda a, b: wrap(a >> (b & 63))

TOK = re.compile(r'\s+|//[^\n]*|(0x[0-9a-fA-F]+|\d+)|([A-Za-z_][A-Za-z0-9_]*)|(\"(?:[^\"\\]|\\.)*")'
                 r'|(\+\+|&&|\|\||==|!=|<=|>=|<<|>>>|>>|=>|->|\+=|-=|\*=|/=|%=|[-+*/%&|^<>=(){}\[\],;:.!])')

def tokenize(src):
    out = []
    for m in TOK.finditer(src):
        if m.group(1):
            g = m.group(1)
            out.append(('n', wrap(int(g, 16) if g.startswith('0x') else int(g))))
        elif m.group(2):
            out.append(('i', m.group(2)))
        elif m.group(3):
            out.append(('s', m.group(3)[1:-1].encode().decode('unicode_escape').encode('latin-1')))
        elif m.group(4):
            if m.group(4) == '++':
                raise SyntaxError('`++` is not in the surface (T42(d): bebop.bin exits 96)')
            out.append(('o', m.group(4)))
    out.append(('o', '<eof>'))
    return out

class Parser:
    def __init__(self, src):
        self.t = tokenize(src)
        self.p = 0
        self.fns = {}
        self.structs = {}
        self.sigs = {}
        self.ctors = {}

    def peek(self, k=0):
        return self.t[self.p + k]
    def at(self, v, k=0):
        return self.t[self.p + k][1] == v
    def next(self):
        t = self.t[self.p]
        self.p += 1
        return t
    def expect(self, v):
        t = self.next()
        if t[1] != v:
            raise SyntaxError('expected %r got %r at tok %d' % (v, t[1], self.p - 1))
        return t
    def ident(self):
        t = self.next()
        if t[0] != 'i':
            raise SyntaxError('expected ident got %r' % (t[1],))
        return t[1]

    def skip_block(self):
        self.expect('{')
        d = 1
        while d:
            v = self.next()[1]
            d += (v == '{') - (v == '}')

    def program(self):
        while not self.at('<eof>'):
            v = self.peek()[1]
            kern = False
            if v == 'kernel':
                self.next()
                kern = True
                v = self.peek()[1]
                if v != 'fn':
                    raise SyntaxError('`kernel` must be followed by `fn`')
            if v == 'fn':
                kstart = self.p
                self.next()
                name = self.ident()
                self.expect('(')
                params = []
                ptypes = []
                while not self.at(')'):
                    params.append(self.ident())
                    self.expect(':')
                    if self.at('['):
                        self.next(); ptypes.append('[' + self.next()[1] + ']'); self.expect(']')
                    elif self.at('ref'):
                        self.next(); ptypes.append('ref ' + self.next()[1])
                    else:
                        ptypes.append(self.next()[1])
                    if self.at(','):
                        self.next()
                self.expect(')')
                rtype = 'i64'
                if self.at('->'):
                    self.next()
                    if self.at('['):
                        self.next(); rtype = '[' + self.next()[1] + ']'; self.expect(']')
                    elif self.at('ref'):
                        self.next(); rtype = 'ref ' + self.next()[1]
                    else:
                        rtype = self.next()[1]
                self.sigs[name] = (ptypes, rtype)
                self.expect('{')
                body = self.body()
                if not body or body[-1][0] != 'expr' or self.t[self.p - 1][1] == ';':
                    raise SyntaxError('fn %s: body has no tail expression (bebop.bin exits 97)' % name)
                self.expect('}')
                if kern:
                    for tk in self.t[kstart:self.p]:
                        if tk[0] == 'i' and tk[1].startswith('sys_'):
                            raise SyntaxError('fn %s: `%s` inside a kernel fn '
                                              '(bebop.bin exits 102)' % (name, tk[1]))
                self.fns[name] = (params, body)
            elif v == 'enum':
                self.next(); self.ident(); self.expect('{')
                tag = 0
                while not self.at('}'):
                    self.ctors[self.ident()] = tag
                    tag += 1
                    if self.at('(') or self.at('{'):
                        self.skip_block() if self.at('{') else self._skip_parens()
                    if self.at(','):
                        self.next()
                self.expect('}')
            elif v == 'struct':
                self.next(); sname = self.ident(); self.expect('{')
                fields = []
                while not self.at('}'):
                    fields.append(self.ident()); self.expect(':')
                    if self.at('['):
                        self.next(); self.next(); self.expect(']')
                    else:
                        self.next()
                    if self.at(','):
                        self.next()
                self.expect('}')
                self.structs.setdefault(sname, fields)
                if not hasattr(self, 'first_struct'):
                    self.first_struct = sname
            elif v == 'module':
                self.next(); self.ident(); self.skip_block()
            else:
                sys.stderr.write('UNSUPPORTED:%s\n' % (v,))
                sys.exit(3)

    def _skip_parens(self):
        self.expect('(')
        d = 1
        while d:
            v = self.next()[1]
            d += (v == '(') - (v == ')')

    def body(self):
        items = []
        while not self.at('}') and not self.at('<eof>'):
            k, v = self.peek()
            if v == 'let':
                self.next()
                name = self.ident()
                self.expect('=')
                r = self.rhs()
                if self.at('in'):
                    self.next()
                    items.append(('expr', ('letin', name, r, self.cmp())))
                else:
                    items.append(('let', name, r))
            elif v == 'while':
                self.next()
                c = self.cmp()
                self.expect('{')
                b = self.body()
                self.expect('}')
                items.append(('while', c, b))
                if self.at(';'):
                    self.next()
            elif v == 'return':
                self.next()
                items.append(('return', self.cmp()))
            elif v == 'break':
                self.next()
                items.append(('break',))
            elif k == 'i' and self.peek(1)[1] in ('+=', '-=', '*=', '/=', '%='):
                name = self.next()[1]
                op = self.next()[1][0]
                items.append(('let', name, ('bin', op, ('var', name), self.cmp())))
            else:
                items.append(('expr', self.cmp()))
            if self.at(';'):
                self.next()
        return items

    def rhs(self):
        if self.peek()[0] == 'i' and self.at('=', 1):
            name = self.next()[1]
            self.next()
            return ('assign', name, self.cmp())
        return self.cmp()

    def let_expr(self):
        name = self.ident()
        self.expect('=')
        r = self.rhs()
        if self.at('in') or self.at(';'):
            self.next()
            return ('letin', name, r, self.cmp())
        else:
            return ('letin', name, r, ('num', 0))

    def cmp(self):
        return self.tier(0)

    def tier(self, i):
        if i == len(TIERS):
            return self.factor()
        lhs = self.tier(i + 1)
        while self.peek()[0] == 'o' and self.peek()[1] in TIERS[i]:
            op = self.next()[1]
            lhs = ('bin', op, lhs, self.tier(i + 1))
        return lhs

    def factor(self):
        k, v = self.next()
        if k == 'n':
            return ('num', v)
        if v == '-':
            return ('neg', self.factor())
        if v == '!':
            return ('not', self.factor())
        if k == 's':
            return ('str', v)
        if v == '(':
            e = self.cmp()
            self.expect(')')
            return e
        if v == 'if':
            c = self.cmp(); self.expect('then')
            # Support blocks in then-branch
            if self.at('{'):
                self.next()
                exprs = []
                while not self.at('}'):
                    exprs.append(self.cmp())
                    if self.at(';'):
                        self.next()
                self.expect('}')
                a = ('block', exprs) if len(exprs) > 1 else exprs[0] if exprs else ('lit', 0)
            else:
                a = self.cmp()
            self.expect('else')
            # Support blocks in else-branch too
            if self.at('{'):
                self.next()
                exprs = []
                while not self.at('}'):
                    exprs.append(self.cmp())
                    if self.at(';'):
                        self.next()
                self.expect('}')
                e = ('block', exprs) if len(exprs) > 1 else exprs[0] if exprs else ('lit', 0)
            else:
                e = self.cmp()
            return ('if', c, a, e)
        if v == 'let':
            return self.let_expr()
        if v == 'match':
            return self.match()
        if v == '[':
            elems = []
            while not self.at(']'):
                elems.append(self.cmp())
                if self.at(','):
                    self.next()
            self.expect(']')
            return ('arr', elems)
        if k != 'i':
            sys.stderr.write('UNSUPPORTED:%s\n' % (v,))
            sys.exit(3)
        if self.at('('):
            self.next()
            args = []
            while not self.at(')'):
                args.append(self.cmp())
                if self.at(','):
                    self.next()
            self.expect(')')
            return ('call', v, args)
        if self.at('{') and v in self.structs:
            self.next()
            given = {}
            while not self.at('}'):
                f = self.ident(); self.expect(':'); given[f] = self.cmp()
                if self.at(','):
                    self.next()
            self.expect('}')
            return ('arr', [given[f] for f in self.structs[v]])
        if self.at('['):
            self.next()
            idx = self.cmp()
            self.expect(']')
            if self.at('=') and not self.at('==', 1):
                self.next()
                return ('set', v, idx, self.cmp())
            return ('get', v, idx)
        e = ('var', v)
        while self.at('.'):
            self.next(); f = self.ident()
            e = ('field', e, f)
        return e

    def match(self):
        cname = self.ident()
        payload = None
        if self.at('('):
            self.next(); payload = self.cmp(); self.expect(')')
        self.expect('{')
        chosen = None
        arms = []
        while not self.at('}'):
            aname = self.ident()
            var = None
            if self.at('('):
                self.next(); var = self.ident(); self.expect(')')
            self.expect('=>')
            b = self.cmp()
            arms.append((aname, var, b))
            if aname == cname and chosen is None:
                chosen = (var, b)
            if self.at(','):
                self.next()
        self.expect('}')
        if chosen is None:
            if payload is not None:
                raise SyntaxError('match: no arm for %s' % cname)
            return ('match', cname, arms)
        var, b = chosen
        if var is not None and payload is not None:
            return ('letin', var, payload, b)
        return b


class Interp:
    def __init__(self, fns, ctors, structs=None, first_struct=None):
        self.fns = fns
        self.arena = [0] * 16
        self.loop_arrs = []
        self.structs = structs or {}
        self.first_struct = first_struct
        self.ctors = ctors
        self.depth = 0
        self.bytes = bytearray()
        self.lit_handles = {}

    def call(self, name, args):
        params, body = self.fns[name]
        env = dict(zip(params, args))
        self.depth += 1
        if self.depth > DEPTH_CAP:
            raise DepthError('call depth > %d in %s' % (DEPTH_CAP, name))
        try:
            return self.run_body(body, env)
        except ReturnSignal as r:
            return r.v
        finally:
            self.depth -= 1

    def run_body(self, items, env):
        val = 0
        for it in items:
            if it[0] == 'let':
                r = it[2]
                if r[0] == 'assign':
                    if r[1] not in env:
                        raise NameError('unbound symbol')
                    env[r[1]] = self.ev(r[2], env)
                else:
                    env[it[1]] = self.ev(r, env)
                val = 0
            elif it[0] == 'while':
                outer = set(env)
                leaks = any(b[0] == 'let' and b[1] in outer and b[2][0] == 'arr' for b in it[2])
                while self.ev(it[1], env) != 0:
                    self.loop_arrs.append([])
                    try:
                        self.run_body(it[2], env)
                    except BreakSignal:
                        break
                    finally:
                        for a in self.loop_arrs.pop():
                            a.released = not leaks
                val = 0
            elif it[0] == 'return':
                raise ReturnSignal(self.ev(it[1], env))
            elif it[0] == 'break':
                raise BreakSignal()
            else:
                val = self.ev(it[1], env)
        return val

    def ev(self, e, env):
        t = e[0]
        if t == 'num':
            return e[1]
        if t == 'var':
            if e[1] in self.ctors:
                return ('ctor', self.ctors[e[1]], [])
            return env[e[1]]
        if t == 'bin':
            return BIN[e[1]](self.ev(e[2], env), self.ev(e[3], env))
        if t == 'neg':
            return wrap(-self.ev(e[1], env))
        if t == 'field':
            return self.ev(e[1], env)[self.structs[self.first_struct].index(e[2])]
        if t == 'not':
            return int(self.ev(e[1], env) == 0)
        if t == 'if':
            return self.ev(e[2] if self.ev(e[1], env) != 0 else e[3], env)
        if t == 'block':
            result = 0
            for expr in e[1]:
                result = self.ev(expr, env)
            return result
        if t == 'letin':
            r = e[2]
            if r[0] == 'assign':
                env[r[1]] = self.ev(r[2], env)
            else:
                env[e[1]] = self.ev(r, env)
            return self.ev(e[3], env)
        if t == 'call':
            return self.builtin_or_call(e[1], [self.ev(a, env) for a in e[2]])
        if t == 'get':
            arr = env[e[1]]
            if getattr(arr, 'released', False):
                raise RuntimeError('use after loop release: `%s` was bound to an array literal inside a while body (T43)' % e[1])
            return arr[self.ev(e[2], env)]
        if t == 'set':
            arr = env[e[1]]
            if getattr(arr, 'released', False):
                raise RuntimeError('use after loop release: `%s` was bound to an array literal inside a while body (T43)' % e[1])
            i = self.ev(e[2], env)
            # `[i64]` is a HANDLE (a cell index), never a value: a write through one binding
            # is visible through every other binding, the caller's included, because
            # Cells.__setitem__ writes into the shared arena. The `arr = list(arr)` that stood
            # here rebound this name to a PRIVATE COPY, so a callee's write was invisible to
            # its caller -- `fn poke(a: [i64]) -> i64 { let _ = a[0] = 9; 0 }` returned 0 here
            # and 9 on bebop.bin. That is what made c70_qdsl / c70_qdsl_neg red (2026-09-12):
            # the oracle was wrong, not the compiler.
            arr[i] = self.ev(e[3], env)
            return 0
        if t == 'arr':
            a = cells_alloc(self, [self.ev(x, env) for x in e[1]])
            if self.loop_arrs:
                self.loop_arrs[-1].append(a)
            return a
        if t == 'str':
            content = e[1]
            off = len(self.bytes)
            self.bytes.extend(content)
            self.bytes.append(0)
            return ((off << 32) | len(content)) & 0xFFFFFFFFFFFFFFFF
        if t == 'match':
            v = self.ev(('var', e[1]), env)
            tag = v[1]
            for aname, var, b in e[2]:
                if self.ctors.get(aname) == tag:
                    if var is not None and v[2]:
                        env[var] = v[2][0]
                    return self.ev(b, env)
            raise RuntimeError('match: no arm for tag %d' % tag)
        raise RuntimeError('bad node %r' % (t,))

    def builtin_or_call(self, name, args):
        if name in UNMODELLED and name not in self.fns:
            raise UnsupportedForm('%s (bpref models no threads and no processes)' % name)
        if name in self.fns:
            return self.call(name, args)
        if name in self.ctors:
            return ('ctor', self.ctors[name], args)
        if name == 'zeros':
            self.arena_cells = getattr(self, 'arena_cells', 0) + max(args[0], 0)
            if self.arena_cells > (256 << 20) // 8 - 8192:
                raise SystemExit(80)
            return cells_alloc(self, [0] * max(args[0], 0))
        if name == 'str_len':
            s = args[0]
            return s & 0xffffffff
        if name == 'char':
            s = args[0]
            off = s >> 32
            i = args[1]
            return self.bytes[off + i] if off + i < len(self.bytes) else 0
        if name == 'sys_exit':
            raise SystemExit(args[0] & 255)
        if name == 'sys_write':
            fd, buf, n = args[0], args[1], args[2]
            if isinstance(buf, int):
                off = buf >> 32
                n = min(n, len(self.bytes) - off)
                data = bytes(self.bytes[off:off + n])
            else:
                data = buf[:n] if isinstance(buf, bytes) else bytes(x & 255 for x in buf[:n])
            (sys.stdout.buffer if fd == 1 else sys.stderr.buffer).write(data)
            return n
        if name == 'crc32b':
            import zlib
            s = args[0]
            off = s >> 32
            ln = s & 0xffffffff
            return zlib.crc32(bytes(self.bytes[off:off + ln]) if ln > 0 else b'') & 0xffffffff
        if name == 'sys_mapb':
            path_handle, map_len = args[0], args[1]
            off = len(self.bytes)
            self.bytes.extend(b'\x00' * max(map_len, 0))
            return ((off << 32) | map_len) & 0xFFFFFFFFFFFFFFFF
        if name == 'sys_readbuf':
            fd, read_len = args[0], args[1]
            n_cells = (read_len + 7) // 8
            off = len(self.bytes)
            self.bytes.extend(b'\x00' * (n_cells * 8))
            src_off = 0
            copy_len = min(read_len, len(self.bytes) - src_off)
            if copy_len > 0:
                self.bytes[off:off + copy_len] = self.bytes[src_off:src_off + copy_len]
            nread = copy_len
            return ((off << 32) | nread) & 0xFFFFFFFFFFFFFFFF
        if name == 'clock_ms' or name.startswith('sys_'):
            return 0
        raise UnsupportedForm('builtin %s' % name)


def run(src):
    p = Parser(src)
    p.program()
    it = Interp(p.fns, p.ctors, p.structs, getattr(p, 'first_struct', None))
    if p.fns['main'][0]:
        argv = [b'seed', sys.argv[1].encode()] + [a.encode() for a in sys.argv[2:]]
        return it.call('main', [len(argv), argv])
    return it.call('main', [])

def expand_use(src, seen=None):
    if seen is None:
        seen = set()
    out = []
    pre = []
    for line in src.split('\n'):
        if line.startswith('use "') and line.rstrip().endswith('"'):
            path = line.strip()[5:-1]
            if path not in seen:
                seen.add(path)
                real = path
                if path.startswith('cas://sha256:'):
                    import hashlib
                    real = '.bcas/%s.bp' % path[13:]
                    if hashlib.sha256(open(real, 'rb').read()).hexdigest() != path[13:]:
                        raise SystemExit(88)
                pre.append(expand_use(open(real, encoding='utf-8', errors='replace').read(), seen))
            out.append('//' + line[1:])
        else:
            out.append(line)
    return '\n'.join(pre + out)

def main():
    sys.setrecursionlimit(1 << 20)
    import threading
    threading.stack_size(512 << 20)
    src = expand_use(open(sys.argv[1], encoding='utf-8', errors='replace').read())
    res = []

    def go():
        try:
            res.append(run(src))
        except SystemExit as ex:
            res.append(('exit', ex.code))
        except DepthError as ex:
            res.append(('depth', str(ex)))
        except UnsupportedForm as ex:
            res.append(('unsup', str(ex)))
        except BaseException as ex:
            res.append(('err', '%s: %s' % (type(ex).__name__, ex)))
    th = threading.Thread(target=go)
    th.start()
    th.join()
    r = res[0]
    if isinstance(r, tuple) and r[0] == 'err':
        print('bpref error: ' + r[1], file=sys.stderr)
        sys.exit(2)
    if isinstance(r, tuple) and r[0] == 'depth':
        print('bpref depth: ' + r[1], file=sys.stderr)
        sys.exit(3)
    if isinstance(r, tuple) and r[0] == 'unsup':
        print('UNSUPPORTED:' + r[1], file=sys.stderr)
        sys.exit(3)
    if isinstance(r, tuple) and r[0] == 'exit':
        sys.exit(r[1])
    print(r)

if __name__ == '__main__':
    main()
