#!/usr/bin/env python3
"""bpref — reference interpreter for the IMPLEMENTED .bp surface (T39).

Semantic oracle for the COMPILER (bebop.bin): `python3 tools/bpref.py prog.bp`
prints main()'s i64 exactly like `seed prog.bin | tail -1`.

Grammar mirrors bebop.bp's emitter tiers (ground truth), NOT a textbook one:
  cmp    := expr ((== != < > <= >=) expr)*        -- lowest, left-assoc, gives 0/1
  expr   := term ((+ -) term)*
  term   := bitlvl ((* / %) bitlvl)*
  bitlvl := factor ((& | ^ << >>) factor)*        -- binds TIGHTER than * /
  factor := ( cmp ) | if cmp then cmp else cmp | let NAME = rhs (in|;) cmp
          | match CTOR[(cmp)] { CTOR[(v)] => cmp, ... } | [cmp, ...] | "str" | NUM
          | NAME | NAME(args) | NAME[cmp] | NAME[cmp] = cmp   (set, value 0)
          | - factor | ! factor | 0xHEX                          (T99 forms)
  rhs    := NAME = cmp (chain-assign, value discarded) | cmp
  body   := (let NAME = rhs ; | while cmp { body } ; | NAME op= cmp ; | cmp ;)* -- value = last cmp
            a FN body must end in a tail cmp with no `;` (SyntaxError otherwise, T42);
            a while body may be empty
Semantics (aarch64 runtime): i64 wraparound; `/` truncates, x/0 = 0,
MIN/-1 = MIN; `%` = a - (a/b)*b (so a%0 = a); shifts take amount mod 64,
`>>` is LOGICAL; `let` is fn-scoped assignment (rebinding mutates, incl.
inside while/let-in); no unary minus, no `!`; `++` is a SyntaxError (T42(d)).
Exit codes: 0 value printed; 2 `bpref error:` (parse/runtime error in the
oracle); 3 `bpref depth:` call depth exceeded BPREF_DEPTH (default 5000) --
the program recurses without bound, a generator/program defect, not a
compiler verdict (fuzz category BPREF-DEPTH).
"""
import os
import re
import sys

DEPTH_CAP = int(os.environ.get('BPREF_DEPTH', '5000'))


class DepthError(Exception):
    pass

MASK = (1 << 64) - 1


def wrap(x):
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
    '^': lambda a, b: wrap(a ^ b),
    '<<': lambda a, b: wrap(a << (b & 63)),
    '>>': lambda a, b: wrap((a & MASK) >> (b & 63)),
    '>>>': lambda a, b: wrap(a >> (b & 63)),  # T42(b)/D9: arithmetic shift is its own form
    '==': lambda a, b: int(a == b), '!=': lambda a, b: int(a != b),
    '<': lambda a, b: int(a < b), '>': lambda a, b: int(a > b),
    '<=': lambda a, b: int(a <= b), '>=': lambda a, b: int(a >= b),
}
# T42(a) 2026-09-04 (D5 measured: zero fold delta): C precedence
#   cmp < | < ^ < & < shifts < +- < */%   (bebop.bp emit_cmp/bor/bxor/band/shift/expr/term)
TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('|',), ('^',), ('&',),
         ('<<', '>>', '>>>'), ('+', '-'), ('*', '/', '%')]
# BPREF_OLDPREC=1 -> the pre-2026-09-04 grammar (bit ops tighter than * /), archaeology only
# BPREF_ASR=1     -> `>>` is ARITHMETIC (sign-propagating) instead of logical (T42(b), operator)
if os.environ.get('BPREF_OLDPREC') == '1':
    TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('+', '-'), ('*', '/', '%'),
             ('&', '|', '^', '<<', '>>', '>>>')]
if os.environ.get('BPREF_ASR') == '1':
    BIN['>>'] = lambda a, b: wrap(a >> (b & 63))

TOK = re.compile(r'\s+|//[^\n]*|(0x[0-9a-fA-F]+|\d+)|([A-Za-z_][A-Za-z0-9_]*)|("(?:[^"\\]|\\.)*")'
                 r'|(\+\+|==|!=|<=|>=|<<|>>>|>>|=>|->|\+=|-=|\*=|/=|%=|[-+*/%&|^<>=(){}\[\],;:.!])')


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

    # ---- top level ----
    def program(self):
        while not self.at('<eof>'):
            v = self.peek()[1]
            if v == 'fn':
                self.next()
                name = self.ident()
                self.expect('(')
                params = []
                while not self.at(')'):
                    params.append(self.ident())
                    self.expect(':')
                    if self.at('['):
                        self.next(); self.next(); self.expect(']')
                    else:
                        self.next()
                    if self.at(','):
                        self.next()
                self.expect(')')
                if self.at('->'):
                    self.next(); self.next()
                    if self.at('['):
                        self.next(); self.expect(']')
                self.expect('{')
                body = self.body()
                # T42: a fn body ends in a tail EXPRESSION (bebop.bin exits 97
                # otherwise): not empty, not a statement, not `e;`
                if not body or body[-1][0] != 'expr' or self.t[self.p - 1][1] == ';':
                    raise SyntaxError('fn %s: body has no tail expression (bebop.bin exits 97)' % name)
                self.expect('}')
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
            elif v in ('module', 'struct'):
                self.next(); self.ident(); self.skip_block()
            else:
                raise SyntaxError('unexpected top-level %r' % (v,))

    def _skip_parens(self):
        self.expect('(')
        d = 1
        while d:
            v = self.next()[1]
            d += (v == '(') - (v == ')')

    # ---- statements ----
    def body(self):
        items = []
        while not self.at('}') and not self.at('<eof>'):
            k, v = self.peek()
            if v == 'let':
                self.next()
                name = self.ident()
                self.expect('=')
                r = self.rhs()
                if self.at('in'):  # statement-level let-in is an expression
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

    # ---- expressions ----
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
        if v == '-':                      # T99 unary minus (neg)
            return ('neg', self.factor())
        if v == '!':                      # T99 unary not (cmp #0; cset eq)
            return ('not', self.factor())
        if k == 's':
            return ('str', v)
        if v == '(':
            e = self.cmp()
            self.expect(')')
            return e
        if v == 'if':
            c = self.cmp(); self.expect('then')
            a = self.cmp(); self.expect('else')
            return ('if', c, a, self.cmp())
        if v == 'let':
            name = self.ident()
            self.expect('=')
            r = self.rhs()
            if self.at(';'): self.next()   # `;` is a synonym for `in` (T42)
            else: self.expect('in')
            return ('letin', name, r, self.cmp())
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
            raise SyntaxError('bad factor %r at tok %d' % (v, self.p - 1))
        if self.at('('):
            self.next()
            args = []
            while not self.at(')'):
                args.append(self.cmp())
                if self.at(','):
                    self.next()
            self.expect(')')
            return ('call', v, args)
        if self.at('['):
            self.next()
            idx = self.cmp()
            self.expect(']')
            if self.at('=') and not self.at('==', 1):
                self.next()
                return ('set', v, idx, self.cmp())
            return ('get', v, idx)
        return ('var', v)

    def match(self):
        cname = self.ident()
        payload = None
        if self.at('('):
            self.next(); payload = self.cmp(); self.expect(')')
        self.expect('{')
        chosen = None
        while not self.at('}'):
            aname = self.ident()
            var = None
            if self.at('('):
                self.next(); var = self.ident(); self.expect(')')
            self.expect('=>')
            b = self.cmp()
            if aname == cname and chosen is None:
                chosen = (var, b)
            if self.at(','):
                self.next()
        self.expect('}')
        if chosen is None:
            raise SyntaxError('match: no arm for %s' % cname)
        var, b = chosen
        if var is not None and payload is not None:
            return ('letin', var, payload, b)
        return b


class Interp:
    def __init__(self, fns, ctors):
        self.fns = fns
        self.ctors = ctors
        self.depth = 0

    def call(self, name, args):
        params, body = self.fns[name]
        env = dict(zip(params, args))
        self.depth += 1
        if self.depth > DEPTH_CAP:
            raise DepthError('call depth > %d in %s' % (DEPTH_CAP, name))
        try:
            return self.run_body(body, env)
        finally:
            self.depth -= 1

    def run_body(self, items, env):
        val = 0
        for it in items:
            if it[0] == 'let':
                r = it[2]
                if r[0] == 'assign':
                    env[r[1]] = self.ev(r[2], env)
                else:
                    env[it[1]] = self.ev(r, env)
                val = 0
            elif it[0] == 'while':
                while self.ev(it[1], env) != 0:
                    self.run_body(it[2], env)
                val = 0
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
        if t == 'not':
            return int(self.ev(e[1], env) == 0)
        if t == 'if':
            return self.ev(e[2] if self.ev(e[1], env) != 0 else e[3], env)
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
            return env[e[1]][self.ev(e[2], env)]
        if t == 'set':
            arr = env[e[1]]
            i = self.ev(e[2], env)
            arr[i] = self.ev(e[3], env)
            return 0
        if t == 'arr':
            return [self.ev(x, env) for x in e[1]]
        if t == 'str':
            return e[1]
        raise RuntimeError('bad node %r' % (t,))

    def builtin_or_call(self, name, args):
        if name in self.fns:
            return self.call(name, args)
        if name in self.ctors:
            return ('ctor', self.ctors[name], args)
        if name == 'zeros':
            return [0] * args[0]
        if name == 'str_len':
            return len(args[0])
        if name == 'char':
            return args[0][args[1]]
        if name == 'sys_exit':
            raise SystemExit(args[0] & 255)
        if name == 'sys_write':
            fd, buf, n = args[0], args[1], args[2]
            data = bytes(x & 255 for x in buf[:n]) if isinstance(buf, list) else buf[:n]
            (sys.stdout.buffer if fd == 1 else sys.stderr.buffer).write(data)
            return n
        if name == 'clock_ms' or name.startswith('sys_'):
            return 0  # ponytail: stubs; real fs/mmap syscalls are out of the fuzzed surface
        raise NameError('unknown fn %s' % name)


def run(src):
    p = Parser(src)
    p.program()
    return Interp(p.fns, p.ctors).call('main', [])


def main():
    sys.setrecursionlimit(1 << 20)
    import threading
    threading.stack_size(512 << 20)
    src = open(sys.argv[1], encoding='utf-8', errors='replace').read()
    res = []

    def go():
        try:
            res.append(run(src))
        except SystemExit as ex:
            res.append(('exit', ex.code))
        except DepthError as ex:
            res.append(('depth', str(ex)))
        except BaseException as ex:  # noqa
            res.append(('err', '%s: %s' % (type(ex).__name__, ex)))
    th = threading.Thread(target=go)
    th.start(); th.join()
    r = res[0]
    if isinstance(r, tuple) and r[0] == 'err':
        print('bpref error: ' + r[1], file=sys.stderr)
        sys.exit(2)
    if isinstance(r, tuple) and r[0] == 'depth':
        print('bpref depth: ' + r[1], file=sys.stderr)
        sys.exit(3)
    if isinstance(r, tuple) and r[0] == 'exit':
        sys.exit(r[1])
    print(r)


if __name__ == '__main__':
    main()
