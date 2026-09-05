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


class Arr(list):
    """An array literal evaluated inside a `while` body (T43 frame-heap reset, 2026-09-06):
    bebop releases it at the back-edge / loop exit, so a later read is a use-after-release
    (DIVERGE-20056 was exactly this). bpref marks it released and refuses the access."""
    released = False


class ReturnSignal(Exception):
    def __init__(self, v): self.v = v
class BreakSignal(Exception):
    pass
RESERVED = set(['sys_msync', 'crc32x', 'crc32', 'clz', 'sys_setaffinity', 'let', 'while', 'if', 'then', 'else', 'in', 'fn', 'enum', 'struct', 'module', 'match', 'return', 'break', 'zeros', 'char', 'str_len', 'clock_ms', 'hvham', 'hvham2', 'some', 'none', 'many', 'sys_open', 'sys_read', 'sys_write', 'sys_close', 'sys_readbuf', 'sys_slurp', 'sys_mmap', 'sys_munmap', 'sys_ftruncate', 'sys_rename', 'sys_export', 'sys_exit', 'sys_arena_base', 'sys_arena_end', 'sys_clone', 'sys_cond_set', 'sys_futex_wait_guard', 'sys_futex_wake', 'sys_atomic_add', 'sys_exit_thread_guard'])
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
    # T125 (2026-09-06): `&&` / `||` are the same tiers as `&` / `|` on 0/1 comparison values,
    # NOT short-circuit -- bebop.bin's `&` tier consumes one `&` and the second is parsed by
    # the right operand (morph.bp is the only user); construct c46_andor pins the semantics.
    '&&': lambda a, b: wrap(a & b), '||': lambda a, b: wrap(a | b),
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
TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('|', '||'), ('^',), ('&', '&&'),
         ('<<', '>>', '>>>'), ('+', '-'), ('*', '/', '%')]
# BPREF_OLDPREC=1 -> the pre-2026-09-04 grammar (bit ops tighter than * /), archaeology only
# BPREF_ASR=1     -> `>>` is ARITHMETIC (sign-propagating) instead of logical (T42(b), operator)
if os.environ.get('BPREF_OLDPREC') == '1':
    TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('+', '-'), ('*', '/', '%'),
             ('&', '|', '^', '<<', '>>', '>>>')]
if os.environ.get('BPREF_ASR') == '1':
    BIN['>>'] = lambda a, b: wrap(a >> (b & 63))

TOK = re.compile(r'\s+|//[^\n]*|(0x[0-9a-fA-F]+|\d+)|([A-Za-z_][A-Za-z0-9_]*)|("(?:[^"\\]|\\.)*")'
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
        self.structs = {}  # T43: NAME -> [field names] in declaration order
        self.sigs = {}  # T48: fn name -> (param types, return type)
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
                if name in RESERVED:  # T122: bebop.bin exits 99 on such a fn
                    raise SyntaxError('reserved word used as a fn name: ' + name)
                self.expect('(')
                params = []
                ptypes = []  # T48 census: declared param types ('i64', 'str', '[i64]', 'ref T', NAME)
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
            elif v == 'struct':  # T43: `struct NAME { f: T, ... }` -> field order
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
            elif v == 'return':  # T99: `return e;` = one b to the epilogue
                self.next()
                items.append(('return', self.cmp()))
            elif v == 'break':   # T99: `break;` = one b to the loop exit
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
        if self.at('{') and v in self.structs:  # T43 struct literal
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
        while self.at('.'):  # T43 field access: index of f in the FIRST struct (bebop emit_field_access)
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
    def __init__(self, fns, ctors, structs=None, first_struct=None):
        self.fns = fns
        self.loop_arrs = []  # T43: per-iteration array literals of the running while bodies
        self.structs = structs or {}
        self.first_struct = first_struct
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
                    env[r[1]] = self.ev(r[2], env)
                else:
                    env[it[1]] = self.ev(r, env)
                val = 0
            elif it[0] == 'while':
                # the compiler skips the reset when a body `let` rebinds an OUTER name to a
                # bare literal (construct c34): then nothing is released
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
            arr[i] = self.ev(e[3], env)
            return 0
        if t == 'arr':
            a = Arr(self.ev(x, env) for x in e[1])
            if self.loop_arrs:
                self.loop_arrs[-1].append(a)
            return a
        if t == 'str':
            return e[1]
        raise RuntimeError('bad node %r' % (t,))

    def builtin_or_call(self, name, args):
        if name in self.fns:
            return self.call(name, args)
        if name in self.ctors:
            return ('ctor', self.ctors[name], args)
        if name == 'zeros':
            # T118 (2026-09-05): the 256 MB seed arena is a capacity; bebop.bin exits 80
            # when a zeros() crosses x28. Mirror it (arena minus the ~64 KiB the loader
            # and argv take): the fuzzer classifies rc 80 == 80 as TRAP-OK.
            self.arena_cells = getattr(self, 'arena_cells', 0) + max(args[0], 0)
            if self.arena_cells > (256 << 20) // 8 - 8192:
                raise SystemExit(80)
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
        if name == 'crc32x':
            import zlib, struct
            c = args[0][args[1]:args[1] + args[2]]
            return zlib.crc32(b''.join(struct.pack('<q', ((x + (1 << 63)) % (1 << 64)) - (1 << 63)) for x in c))
        if name == 'crc32':
            import zlib
            return zlib.crc32(bytes(x & 255 for x in args[0][:args[1]]))
        if name == 'clz':
            return 64 - (args[0] & 0xFFFFFFFFFFFFFFFF).bit_length()
        if name == 'clock_ms' or name.startswith('sys_'):
            return 0  # ponytail: stubs; real fs/mmap syscalls are out of the fuzzed surface
        raise NameError('unknown fn %s' % name)


def run(src):
    p = Parser(src)
    p.program()
    it = Interp(p.fns, p.ctors, p.structs, getattr(p, 'first_struct', None))
    if p.fns['main'][0]:  # fn main(argc, argv): argv[2:] = the args after the program path, as the seed passes them
        argv = [b'seed', sys.argv[1].encode()] + [a.encode() for a in sys.argv[2:]]
        return it.call('main', [len(argv), argv])
    return it.call('main', [])


def expand_use(src, seen=None):
    # T47 one-level `use "path"` expansion; T47b (2026-09-06): recursive, path-deduplicated,
    # dependencies first, every use line replaced by a comment -- the same shape as
    # bebop.bp's use_scan.
    if seen is None: seen = set()
    out = []; pre = []
    for line in src.split('\n'):
        if line.startswith('use "') and line.rstrip().endswith('"'):
            path = line.strip()[5:-1]
            if path not in seen:
                seen.add(path)
                real = path
                if path.startswith('cas://sha256:'):  # T80: content-addressed module, verified by digest
                    import hashlib
                    real = '.bcas/%s.bp' % path[13:]
                    if hashlib.sha256(open(real, 'rb').read()).hexdigest() != path[13:]: raise SystemExit(88)
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
