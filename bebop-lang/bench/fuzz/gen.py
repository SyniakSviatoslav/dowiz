#!/usr/bin/env python3
"""gen.py --seed N [--out F]: one random well-typed .bp program (T39 fuzzer).

Generates ONLY the surface bpref.py documents (its docstring is the grammar):
fn / let / let-in / let-chain / if / while / match-literal / enum / arrays /
zeros / calls / 1-arg direct recursion / comparisons / arithmetic / bitwise /
shifts, i64-only. Caps: <=128 binds per fn (params + unique locals),
<=14 params, <=8 array binds per fn, <=511-element literal arrays, no
allocation inside while bodies (L8), no str literals / ++ (R3.x d), no
clock_ms (R3.x e), no unary - / !, no return / break.

Every array has a power-of-two "mask" (len >= 8) and every index is a literal
< 8, a loop var whose bound <= 8, or `((E) & mask)`, so no run is ever
out of bounds. Loops are `let i = 0; while i < K {...}` with K <= 6 and the
loop var never assigned elsewhere, so every run terminates. Recursion is
direct, 1-arg, guarded by `n < 1`, entered with `(E & 7)`, and `n` is never
assigned inside the recursive fn, so depth <= 7.
"""
import argparse
import random
import sys

ARITH = ['+', '-', '*', '/', '%']
BITS = ['&', '|', '^', '<<', '>>', '>>>']
CMP = ['==', '!=', '<', '>', '<=', '>=']
CTORS = ['none', 'some', 'many']
BIG = [2**63 - 1, 2**63, 2**64 - 1, 2**32, 2**31 - 1, 4095, 4096, 65535, 65536,
       1000000000000, 10054667875361132632, 9223372036854775807]


class Fn:
    def __init__(self, name, params, rec=False):
        self.name = name
        self.params = params  # [(name, is_arr)]
        self.rec = rec
        self.text = ''


class Ctx:
    """Per-fn generation state."""

    def __init__(self, g, fn, callees, budget, heavy=False):
        self.g = g
        self.r = g.r
        self.fn = fn
        self.callees = callees
        self.scalars = [p for p, a in fn.params if not a]
        self.arrays = {p: 7 for p, a in fn.params if a}  # name -> mask
        self.loopvars = []
        self.binds = set(p for p, _ in fn.params)
        self.budget = budget  # max binds this fn may reach (<=128)
        self.loop_depth = 0
        self.nstmt = 0
        self.ntmp = 0
        self.heavy = heavy

    # ---- helpers ----
    def can_bind(self, name):
        return name in self.binds or len(self.binds) < self.budget

    def bind(self, name):
        self.binds.add(name)

    def new_scalar(self):
        n = 'v%d' % len([s for s in self.scalars if s.startswith('v')])
        while n in self.binds:
            n += 'x'
        return n

    def assignable(self):
        # the recursion parameter is never assigned: `f(n - 1)` guarded by
        # `n < 1` only bounds the depth (<= 7) if n keeps its entry value
        fixed = self.loopvars + ([self.fn.params[0][0]] if self.fn.rec else [])
        return [s for s in self.scalars if s not in fixed]

    def lit(self):
        r = self.r.random()
        if r < 0.70:
            return str(self.r.randint(0, 20))
        if r < 0.85:
            return str(self.r.randint(0, 5000))
        if r < 0.90:                                  # T99 hex literal
            return hex(self.r.randint(0, 1 << 20))
        if r < 0.95:                                  # T99 negative literal
            return '-' + str(self.r.randint(1, 5000))
        return str(self.r.choice(BIG))

    def index(self, name, simple=False, d=1):
        mask = self.arrays[name]
        r = self.r.random()
        lv = self.loopvars  # bounds <= 6 < 8 <= every array length
        if r < 0.4:
            return str(self.r.randint(0, min(mask, 7)))
        if r < 0.7 and lv:
            return self.r.choice(lv)
        # depth flows through: an index used to restart at expr(2), and a
        # chain of a[..a[..a[..]]] resets recursed past Python's frame limit
        return '((%s) & %d)' % (self.expr(max(d + 1, 2), simple), mask)

    # ---- expressions ----
    def expr(self, d, simple=False):
        """simple=True: no commas, braces or parens-in-payload hazards (match arms)."""
        r = self.r.random()
        if d >= 4:
            r *= 0.6
        if d >= 8 or r < 0.25:
            # hard leaf at d>=8: the 0.6 damping alone is supercritical when
            # scalars/arrays are empty (lit 42% vs binop 58% x 2 children)
            if self.scalars and self.r.random() < 0.5:
                return self.r.choice(self.scalars)
            return self.lit()
        if r < 0.45 and self.scalars:
            return self.r.choice(self.scalars)
        if r < 0.55 and self.arrays:
            a = self.r.choice(list(self.arrays))
            return '%s[%s]' % (a, self.index(a, simple, d))
        if r < 0.74:
            return self.binop(d, simple)
        if r < 0.78:                                  # T99 unary - / !
            return '%s(%s)' % (self.r.choice(['-', '!']), self.expr(d + 1, simple))
        if r < 0.85:
            return '(if %s then %s else %s)' % (self.expr(d + 1, simple), self.expr(d + 1, simple), self.expr(d + 1, simple))
        if r < 0.90 and not simple:
            return self.letin(d)
        if r < 0.94 and not simple:
            return self.match(d)
        if r < 0.99 and not simple and self.callees:
            return self.call(d)
        return self.lit()

    def binop(self, d, simple):
        k = self.r.random()
        op = self.r.choice(ARITH if k < 0.5 else BITS if k < 0.8 else CMP)
        a = self.expr(d + 1, simple)
        b = self.expr(d + 1, simple)
        # parenthesize sometimes -> exercises both precedence paths and the folds
        if self.r.random() < 0.5:
            a = '(%s)' % a
        if self.r.random() < 0.5:
            b = '(%s)' % b
        return '%s %s %s' % (a, op, b)

    def letin(self, d):
        k = self.r.random()
        if k < 0.35 and self.assignable():
            v = self.r.choice(self.assignable())
            if self.can_bind('_'):
                self.bind('_')
                return '(let _ = %s = %s in %s)' % (v, self.expr(d + 1), self.expr(d + 1))
        if k < 0.6 and self.arrays and self.can_bind('_'):
            self.bind('_')
            a = self.r.choice(list(self.arrays))
            return '(let _ = %s[%s] = %s in %s)' % (a, self.index(a, False, d), self.expr(d + 1), self.expr(d + 1))
        t = 't%d' % self.r.randint(0, 3)
        if not self.can_bind(t):
            return self.lit()
        rhs = self.expr(d + 1)
        self.bind(t)
        saved = self.scalars
        self.scalars = self.scalars + [t]  # visible only inside the body (untaken-branch safety)
        body = self.expr(d + 1)
        self.scalars = saved
        return '(let %s = %s in %s)' % (t, rhs, body)

    def match(self, d):
        c = self.r.choice(CTORS)
        payload = None
        if self.r.random() < 0.7:
            # paren-free payload: emit_match finds the payload end with skip_to(')')
            payload = self.r.choice([self.lit(), self.r.choice(self.scalars) if self.scalars else self.lit()])
            if self.r.random() < 0.5:
                payload += ' %s %s' % (self.r.choice(ARITH), self.lit())
        arms = []
        mv = 'm%d' % self.r.randint(0, 1)
        order = CTORS[:]
        self.r.shuffle(order)
        for name in order:
            if name != c and self.r.random() < 0.3:
                continue
            if payload is not None and name == c and self.can_bind(mv) and self.r.random() < 0.7:
                self.bind(mv)
                saved = self.scalars
                self.scalars = self.scalars + [mv]
                body = self.expr(d + 1, simple=True)
                self.scalars = saved
                arms.append('%s(%s) => %s' % (name, mv, body))
            else:
                arms.append('%s => %s' % (name, self.expr(d + 1, simple=True)))
        self.g.uses_enum = True
        scrut = c if payload is None else '%s(%s)' % (c, payload)
        return 'match %s { %s }' % (scrut, ', '.join(arms))

    def call(self, d):
        f = self.r.choice(self.callees)
        args = []
        for p, is_arr in f.params:
            if is_arr:
                if self.arrays and (self.loop_depth or self.r.random() < 0.7):
                    args.append(self.r.choice(list(self.arrays)))
                elif not self.loop_depth:
                    args.append('[%s]' % ', '.join(self.expr(d + 2) for _ in range(8)))
                else:
                    return self.lit()  # no allocation inside while bodies (L8)
            elif f.rec:
                args.append('((%s) & 63)' % self.expr(d + 1))  # D11-D: recursion depth to 63 (127 timed the python oracle out)
            else:
                args.append(self.expr(d + 1))
        return '%s(%s)' % (f.name, ', '.join(args))

    # ---- statements ----
    def stmt(self, depth):
        r = self.r.random()
        self.nstmt += 1
        asg = self.assignable()
        if r < (0.6 if self.heavy else 0.30):
            new = self.r.random() < (0.9 if self.heavy else 0.5) or not asg
            v = self.new_scalar() if new else self.r.choice(asg)
            if not self.can_bind(v):
                return '%s;' % self.expr(0)
            rhs = self.expr(0)
            self.bind(v)
            if v not in self.scalars:
                self.scalars.append(v)
            return 'let %s = %s;' % (v, rhs)
        if r < 0.40 and self.loop_depth and len(self.arrays) < 8 and len(self.binds) < self.budget and self.r.random() < 0.5:
            # D11-D: an array literal INSIDE a loop body (T43 frame-heap reset path)
            a = 'a%d' % len(self.arrays)
            n = self.r.choice([8, 8, 16])
            rhs = '[%s]' % ', '.join(self.expr(1) for _ in range(n))
            self.bind(a)
            self.arrays[a] = n - 1
            return 'let %s = %s;' % (a, rhs)
        if r < 0.40 and not self.loop_depth and len(self.arrays) < 8 and len(self.binds) < self.budget:
            a = 'a%d' % len(self.arrays)
            k = self.r.random()
            if k < 0.5:
                n = self.r.choice([8, 8, 16, 32])
                rhs = '[%s]' % ', '.join(self.expr(1) for _ in range(n))
                mask = n - 1
            elif k < 0.9:
                n = self.r.choice([8, 16, 64, 256, 512])
                rhs = 'zeros(%d)' % n
                mask = n - 1
            else:
                n = 511
                rhs = '[%s]' % ', '.join(self.lit() for _ in range(n))
                mask = 255
            self.bind(a)
            self.arrays[a] = mask
            return 'let %s = %s;' % (a, rhs)
        if r < 0.55 and self.arrays:
            a = self.r.choice(list(self.arrays))
            s = '%s[%s] = %s' % (a, self.index(a), self.expr(0))
            if self.r.random() < 0.5 and self.can_bind('_'):
                self.bind('_')
                return 'let _ = %s;' % s
            return '%s;' % s
        if r < 0.65 and asg:
            v = self.r.choice(asg)
            return '%s %s= %s;' % (v, self.r.choice(ARITH), self.expr(0))
        if r < 0.72 and asg and self.can_bind('_'):
            self.bind('_')
            return 'let _ = %s = %s;' % (self.r.choice(asg), self.expr(0))
        if r < 0.88 and depth < 2 and self.loop_depth < 2:
            return self.loop(depth)
        return '%s;' % self.expr(0)

    def loop(self, depth):
        i = 'i%d' % len(self.loopvars)
        if not self.can_bind(i):
            return '%s;' % self.expr(0)
        self.bind(i)
        # D11-D (2026-09-05): one loop in four runs 100..3000 iterations; its var is
        # NOT registered as an index candidate (indices stay masked), so the widened
        # loop exercises frame-heap resets (T43), arena growth and trap paths.
        big = self.r.random() < 0.25 and not self.loop_depth
        k = self.r.randint(60, 150) if big else self.r.randint(1, 6)  # bpref budget (40 s): 200 still gave 18 timeouts/60
        self.g.loop_bound[i] = k
        self.scalars.append(i)
        if not big:
            self.loopvars.append(i)
        self.loop_depth += 1
        body = [self.stmt(depth + 1) for _ in range(self.r.randint(1, 4))]
        # T99: `break;` only as the LAST statement of a loop body -- a `let` after it
        # would be dead in bebop (fn-scoped symbol never assigned) but unbound in bpref
        if self.r.random() < 0.15:
            body.append('break;')
        step = self.r.choice(['let %s = %s + 1;' % (i, i), '%s += 1;' % i, 'let _ = %s = %s + 1;' % (i, i)])
        if step.startswith('let _') and not self.can_bind('_'):
            step = '%s += 1;' % i
        if step.startswith('let _'):
            self.bind('_')
        body.append(step)
        if self.r.random() < 0.7:
            body.append('0')
        self.loop_depth -= 1
        cond = self.r.choice(['%s < %d' % (i, k), '%s < %d' % (i, k), '%s != %d' % (i, k), '%s <= %d' % (i, k - 1)])
        ind = '  ' * (depth + 1)
        return 'let %s = 0;\n%swhile %s {\n%s\n%s};' % (i, ind, cond, '\n'.join(ind + '  ' + b for b in body), ind)

    def body(self, nstmts):
        lines = [self.stmt(0) for _ in range(nstmts)]
        # T99: `return e;` only as the last statement before the tail (same reason)
        if not self.fn.rec and self.r.random() < 0.12:
            lines.append('return %s;' % self.expr(0))
        if self.fn.rec:
            n = self.fn.params[0][0]
            base = self.expr(1)
            recur = '%s(%s - 1)' % (self.fn.name, n)
            tail = self.r.choice(['%s + %s' % (self.expr(1), recur), '%s * 2 + %s' % (recur, self.expr(1)),
                                  '(%s) - %s' % (recur, self.expr(1)), '(if %s then %s else %s)' % (self.expr(1), recur, self.expr(1))])
            lines.append('if %s < 1 then %s else %s' % (n, base, tail))
        else:
            lines.append(self.expr(0))
        return lines


class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.seed = seed
        self.uses_enum = False
        self.loop_bound = {}

    def program(self):
        r = self.r
        nfn = r.choice([0, 0, 1, 1, 2, 3, 4])
        fns = []
        for k in range(nfn):
            rec = r.random() < 0.25
            if rec:
                params = [('n', False)]
            else:
                np = r.choice([1, 1, 2, 2, 3, 4, 9, 13, 14]) if r.random() < 0.85 else r.randint(0, 14)
                params = [('p%d' % j, r.random() < 0.25) for j in range(np)]
                if sum(1 for _, a in params if a) > 8:
                    params = [(p, a if j < 8 else False) for j, (p, a) in enumerate(params)]
            fns.append(Fn('f%d' % k, params, rec))
        total = r.randint(3, 30)
        # main gets ~half the statements, helpers share the rest (DAG: f_k calls f_j, j>k)
        out = []
        budgets = []
        for k, f in enumerate(fns):
            heavy = r.random() < 0.10
            budgets.append(r.randint(70, 120) if heavy else r.randint(12, 40))
        for k in range(len(fns) - 1, -1, -1):
            f = fns[k]
            cx = Ctx(self, f, fns[k + 1:], budgets[k], budgets[k] > 60)
            n = max(0, total // (2 * max(1, len(fns))) + r.randint(-1, 2))
            if budgets[k] > 60:
                n += r.randint(30, 60)
            lines = cx.body(n)
            sig = ', '.join('%s: %s' % (p, '[i64]' if a else 'i64') for p, a in f.params)
            f.text = 'fn %s(%s) -> i64 {\n%s\n}' % (f.name, sig, '\n'.join('  ' + l for l in lines))
        heavy = r.random() < 0.10
        cx = Ctx(self, Fn('main', []), fns, r.randint(70, 120) if heavy else r.randint(12, 40), heavy)
        n = max(2, total - sum(1 for _ in fns) * 2) // 2 + r.randint(0, 3)
        if heavy:
            n += r.randint(30, 60)
        lines = cx.body(n)
        out.append('// gen.py --seed %d' % self.seed)
        out.append('fn main() -> i64 {\n%s\n}' % '\n'.join('  ' + l for l in lines))
        out.extend(f.text for f in fns)
        if self.uses_enum:
            out.append('enum opt { %s }' % ', '.join(CTORS))
        return '\n'.join(out) + '\n'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--seed', type=int, required=True)
    ap.add_argument('--out')
    a = ap.parse_args()
    src = Gen(a.seed).program()
    if a.out:
        open(a.out, 'w').write(src)
    else:
        sys.stdout.write(src)


if __name__ == '__main__':
    main()
