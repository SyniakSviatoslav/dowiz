#!/usr/bin/env python3
"""certgen.py -- UNTRUSTED bridge producer for Bebop's certificate format.

  *** THIS FILE IS OUTSIDE THE TRUST ROOT AND IS MEANT TO BE DELETED. ***

  It exists because of one scheduling fact: with Lean demoted to a cross-check
  oracle (operator, 2026-09-09), NOTHING in the tree can produce a certificate,
  so F6's checker alone is half a system. A producer is outside the trust root
  BY CONSTRUCTION -- nothing it says is believed, only what the checker
  re-derives -- so it may be written in any language and needs no verification.
  That is the whole argument for it being python, and the whole argument for
  deleting it the day `bebop.bp`'s own producer emits this format.

  It is NOT a dependency: it shells out to the `cadical` that already ships
  inside the Lean toolchain (`lean/bin/cadical`, CaDiCaL 2.1.2 measured
  2026-09-09), and the artifacts it writes are checked by a checker that never
  runs it. If this file and cadical both vanish, every certificate already
  committed still checks.

Format produced (docs: /root/s30/outC4/CERT-FORMAT-DESIGN.md; operator choices
11.3, 12.1, 13.3, 14.3, 15.1+15.3):

  TEXT IS NORMATIVE. The sha256 of 15.1 is over the text, because the hash must
  cover the artifact a checker actually reads -- hash a binary cache instead and
  the audited form is unverified. No binary is emitted here; a cache, if one is
  ever added, carries this text's sha256 and is re-derived, never trusted.

  HINTS ARE MANDATORY. Every derived clause names its antecedents in resolution
  order, so checking is a linear loop with no unit propagation, no watched
  literals and no search. That is the difference between a small checker and a
  re-implemented solver, and it is why the certificate is allowed to be larger.

    % <comment>                       header lines, not hashed
    p <nvars> <nclauses>              CNF dimensions
    c <id> <lit>* 0                   an input clause from the bit-blaster
    r <id> <lit>* 0 <hint-id>* 0      derived clause + antecedents, in order
    d <id>*                           deletion (lets a checker free)
    x <oid>                           the sha256 of the obligation this discharges

Obligation language (a DAG, one node per line, args refer to smaller ids only,
so ONE forward pass checks well-formedness and acyclicity together):

    % comment
    obligation <name>
    <id> var <name>                   a fresh 64-bit variable
    <id> const <decimal>              a 64-bit constant (two's complement)
    <id> add|sub|mul|and|or|xor <a> <b>
    <id> not <a>
    <id> shl|lshr <a> <k>             k a literal shift amount 0..63
    <id> eq <a> <b>                   a boolean node
    prove <id>

  There is deliberately NO sdiv/srem. Nothing in the tree can bit-blast a
  64-bit division -- `tcheck.bp` has no divider -- and a producer that emits an
  obligation no checker can discharge is worse than one that refuses. `mul` is
  here because `bb_mul64` works and its cost is known (33,777 clauses for one
  symbolic 64x64 multiply, measured 2026-09-14); a divider has neither.

Usage:
    tools/certgen.py <obligation.obl> <out.cert> [--cadical <path>]
Environment:
    BEBOP_CADICAL   path to the cadical binary (else --cadical, else PATH)
"""
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile

W = 64  # Bebop's only scalar width


def die(msg):
    sys.stderr.write('certgen: %s\n' % msg)
    raise SystemExit(2)


# ------------------------------------------------------------ obligation text

def canonical_text(lines):
    """The bytes the oid is taken over. Pinned deliberately and early, because
    every oid and every Merkle root depends on it and it cannot change later
    without invalidating them: LF only, comments and blank lines REMOVED, no
    leading/trailing space, single spaces between tokens."""
    out = []
    for l in lines:
        l = l.strip()
        if not l or l.startswith('%'):
            continue
        out.append(' '.join(l.split()))
    return ('\n'.join(out) + '\n').encode()


def oid_of(lines):
    return hashlib.sha256(canonical_text(lines)).hexdigest()


def parse(path):
    with open(path) as f:
        lines = f.read().split('\n')
    nodes, order, name, goal = {}, [], None, None
    for i, raw in enumerate(lines, 1):
        t = raw.strip()
        if not t or t.startswith('%'):
            continue
        w = t.split()
        if w[0] == 'obligation':
            name = w[1] if len(w) > 1 else 'anon'
            continue
        if w[0] == 'prove':
            goal = int(w[1])
            continue
        nid, op, args = int(w[0]), w[1], w[2:]
        if nid in nodes:
            die('line %d: duplicate node id %d' % (i, nid))
        for a in args:
            if re.fullmatch(r'-?\d+', a) and op in ('add', 'sub', 'mul', 'and', 'or', 'xor', 'not', 'eq'):
                if int(a) >= nid:
                    die('line %d: node %d refers to %s -- args must be STRICTLY smaller '
                        '(this is what makes one forward pass enough)' % (i, nid, a))
        nodes[nid] = (op, args)
        order.append(nid)
    if goal is None:
        die('no `prove <id>` line')
    if goal not in nodes:
        die('prove names node %d which is not defined' % goal)
    return name, nodes, order, goal, lines


# --------------------------------------------------------------- bit-blasting

class CNF:
    def __init__(self):
        self.n = 0
        self.cl = []
        self.true = self.fresh()
        self.cl.append([self.true])          # a constant-true literal

    def fresh(self):
        self.n += 1
        return self.n

    def add(self, *lits):
        self.cl.append(list(lits))

    def const(self, b):
        return self.true if b else -self.true

    def gate_and(self, a, b):
        o = self.fresh()
        self.add(-a, -b, o); self.add(a, -o); self.add(b, -o)
        return o

    def gate_or(self, a, b):
        o = self.fresh()
        self.add(a, b, -o); self.add(-a, o); self.add(-b, o)
        return o

    def gate_xor(self, a, b):
        o = self.fresh()
        self.add(-a, -b, -o); self.add(a, b, -o); self.add(a, -b, o); self.add(-a, b, o)
        return o

    # --- constant-folding gates, used ONLY by the mul/sub path -------------
    # gate_and/gate_or/gate_xor above are deliberately left alone, so every
    # obligation that predates `mul` emits a byte-identical CNF (checked by
    # regenerating arith_add.cert and comparing).  A folded gate costs no
    # variable and no clause, which is what makes a shift-and-add multiplier
    # affordable here for the same reason it does in selfhost/tcheck.bp.
    def fand(self, a, b):
        T, F = self.true, -self.true
        if a == F or b == F: return F
        if a == T: return b
        if b == T: return a
        if a == b: return a
        if a == -b: return F
        return self.gate_and(a, b)

    def for_(self, a, b):
        T, F = self.true, -self.true
        if a == T or b == T: return T
        if a == F: return b
        if b == F: return a
        if a == b: return a
        if a == -b: return T
        return self.gate_or(a, b)

    def fxor(self, a, b):
        T, F = self.true, -self.true
        if a == F: return b
        if b == F: return a
        if a == T: return -b
        if b == T: return -a
        if a == b: return F
        if a == -b: return T
        return self.gate_xor(a, b)

    def addc(self, a, b, cin):
        """Ripple-carry add, low 64 bits, explicit carry-in. The carry OUT of
        bit 63 is never computed: a wrapping 64-bit add discards it."""
        out = []
        for k in range(W):
            axb = self.fxor(a[k], b[k])
            out.append(self.fxor(axb, cin))
            if k < W - 1:
                cin = self.for_(self.fand(a[k], b[k]), self.fand(cin, axb))
        return out

    def mul(self, a, b):
        """Shift and add, low 64 bits only -- the same structure as
        selfhost/tcheck.bp's bb_mul64 and folded the same way: a CONST_FALSE
        multiplier bit contributes no gates at all, and column j of row `bit`
        comes from a[j - bit], so the partial products that would only have fed
        bits 64..127 are never built."""
        F = -self.true
        acc = [F] * W
        for bit in range(W):
            sel = b[bit]
            if sel == F:
                continue
            row = [F] * bit + [self.fand(a[j], sel) for j in range(W - bit)]
            acc = self.addc(acc, row, F)
        return acc


def blast(nodes, order, cnf):
    """Each 64-bit term becomes a list of 64 literals, LSB first."""
    bits = {}
    for nid in order:
        op, args = nodes[nid]
        if op == 'var':
            bits[nid] = [cnf.fresh() for _ in range(W)]
        elif op == 'const':
            v = int(args[0]) & ((1 << W) - 1)
            bits[nid] = [cnf.const((v >> k) & 1) for k in range(W)]
        elif op in ('and', 'or', 'xor'):
            a, b = bits[int(args[0])], bits[int(args[1])]
            g = {'and': cnf.gate_and, 'or': cnf.gate_or, 'xor': cnf.gate_xor}[op]
            bits[nid] = [g(a[k], b[k]) for k in range(W)]
        elif op == 'not':
            bits[nid] = [-l for l in bits[int(args[0])]]
        elif op == 'add':
            a, b = bits[int(args[0])], bits[int(args[1])]
            out, carry = [], cnf.const(0)
            for k in range(W):                      # ripple-carry, 64 full adders
                s = cnf.gate_xor(cnf.gate_xor(a[k], b[k]), carry)
                c1 = cnf.gate_and(a[k], b[k])
                c2 = cnf.gate_and(cnf.gate_xor(a[k], b[k]), carry)
                carry = cnf.gate_or(c1, c2)
                out.append(s)
            bits[nid] = out
        elif op in ('shl', 'lshr'):
            a, k = bits[int(args[0])], int(args[1])
            if not 0 <= k < W:
                die('node %d: shift amount %d out of range' % (nid, k))
            z = cnf.const(0)
            bits[nid] = ([z] * k + a[:W - k]) if op == 'shl' else (a[k:] + [z] * k)
        elif op == 'sub':
            a, b = bits[int(args[0])], bits[int(args[1])]
            bits[nid] = cnf.addc(a, [-l for l in b], cnf.const(1))
        elif op == 'mul':
            a, b = bits[int(args[0])], bits[int(args[1])]
            bits[nid] = cnf.mul(a, b)
        elif op == 'eq':
            a, b = bits[int(args[0])], bits[int(args[1])]
            acc = cnf.const(1)
            for k in range(W):
                acc = cnf.gate_and(acc, -cnf.gate_xor(a[k], b[k]))
            bits[nid] = [acc]                        # a boolean node: one literal
        else:
            die('node %d: unknown op %r' % (nid, op))
    return bits


# ---------------------------------------------------------------------- driver

def find_cadical(argv):
    if '--cadical' in argv:
        p = argv[argv.index('--cadical') + 1]
    else:
        p = os.environ.get('BEBOP_CADICAL') or shutil.which('cadical') or \
            '/root/s30/outC4/lean/bin/cadical'
    if not (os.path.isfile(p) and os.access(p, os.X_OK)):
        die('cadical not found at %r.\n'
            '  It ships inside the Lean toolchain at <lean>/bin/cadical (CaDiCaL 2.1.2\n'
            '  measured 2026-09-09). Set BEBOP_CADICAL or pass --cadical.\n'
            '  This is a PRODUCER dependency only: no checker and no committed\n'
            '  certificate needs cadical to exist.' % p)
    return p


def main(argv):
    if len(argv) < 2:
        die(__doc__.strip().split('Usage:')[1].strip())
    obl, out = argv[0], argv[1]
    cad = find_cadical(argv)
    name, nodes, order, goal, raw = parse(obl)
    oid = oid_of(raw)

    cnf = CNF()
    bits = blast(nodes, order, cnf)
    g = bits[goal]
    if len(g) != 1:
        die('prove names node %d, which is a 64-bit term, not a boolean '
            '(use an `eq` node)' % goal)
    cnf.add(-g[0])                       # negate the goal: UNSAT == the goal is valid
    inputs = list(cnf.cl)

    tmp = tempfile.mkdtemp(prefix='certgen.', dir=os.environ.get('BEBOP_TMP') or None)
    dim, prf = os.path.join(tmp, 'p.cnf'), os.path.join(tmp, 'p.lrat')
    with open(dim, 'w') as f:
        f.write('p cnf %d %d\n' % (cnf.n, len(inputs)))
        for c in inputs:
            f.write(' '.join(map(str, c)) + ' 0\n')

    r = subprocess.run([cad, '--lrat=true', '--binary=false', '-q', dim, prf],
                       capture_output=True, text=True)
    # cadical: 20 = UNSAT (the goal is valid), 10 = SAT (it is not)
    if r.returncode == 10:
        die('obligation %s is NOT valid -- cadical found a model. No certificate '
            'written. (A producer that emitted one anyway would be exactly the '
            'thing the checker exists to catch.)' % name)
    if r.returncode != 20:
        die('cadical exited %d, expected 20 (UNSAT) or 10 (SAT):\n%s'
            % (r.returncode, (r.stderr or r.stdout)[:400]))
    if not os.path.exists(prf) or os.path.getsize(prf) == 0:
        die('cadical reported UNSAT but wrote no proof to %s -- refusing to emit a '
            'certificate with no derivation' % prf)

    derived, deleted = [], []
    with open(prf) as f:
        for line in f:
            w = line.split()
            if not w:
                continue
            if len(w) > 1 and w[1] == 'd':
                deleted.append((int(w[0]), [int(x) for x in w[2:] if x != '0']))
                continue
            cid = int(w[0])
            rest = [int(x) for x in w[1:]]
            z = rest.index(0)
            derived.append((cid, rest[:z], [h for h in rest[z + 1:] if h != 0]))

    with open(out, 'w') as f:
        f.write('%% bebop certificate, text form (NORMATIVE -- the oid below is over the\n'
                '%% obligation text, and this file is what a checker reads).\n'
                '%% obligation: %s\n' % name)
        f.write('%% produced by tools/certgen.py, an UNTRUSTED bridge producer, via %s\n'
                '%% Nothing here is believed; the checker re-derives every step.\n'
                % os.path.basename(cad))
        f.write('p %d %d\n' % (cnf.n, len(inputs)))
        for i, c in enumerate(inputs, 1):
            f.write('c %d %s 0\n' % (i, ' '.join(map(str, c))))
        for cid, lits, hints in derived:
            body = ' '.join(map(str, lits))
            f.write('r %d%s 0 %s 0\n' % (cid, (' ' + body) if body else '',
                                         ' '.join(map(str, hints))))
        for _, ids in deleted:
            if ids:
                f.write('d %s\n' % ' '.join(map(str, ids)))
        f.write('x %s\n' % oid)

    print('certgen: %s -> %s' % (obl, out))
    print('  oid            %s' % oid)
    print('  cnf            %d vars, %d input clauses' % (cnf.n, len(inputs)))
    print('  derivation     %d derived clauses (all HINTED), %d deletion lines'
          % (len(derived), len([1 for _, i in deleted if i])))
    print('  bytes          %d' % os.path.getsize(out))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
