#!/usr/bin/env python3
"""certcheck.py -- REFERENCE checker for Bebop's certificate format.

  *** ALSO OUTSIDE THE TRUST ROOT. This is an executable SPECIFICATION, not
      the checker the tree will trust. ***

  The checker the tree trusts is `tcheck.bp` (ROADMAP F6), in Bebop, in-tree,
  zero dependency. This file exists so the format is checkable TODAY -- before
  `tcheck.bp` is written -- and so that `tcheck.bp` has something concrete to
  agree with. Two independent checkers of the same file is the property worth
  having; when the Bebop one lands, this stays only as the differential twin.

  It is deliberately written as the SMALLEST algorithm that suffices, because
  its job is to demonstrate the format's central claim:

      HINTS MAKE CHECKING A LOOP.

  Every derived clause names its antecedents in resolution order, so the check
  is: assume the negation of the claimed clause, walk the named antecedents in
  order, each must be unit (extend the assignment) or falsified (conflict --
  step verified). No unit propagation over the whole database, no watched
  literals, no occurrence lists, no search. That is why the Bebop checker can be
  tens of functions rather than a solver, and it is the entire reason the
  format pays 2-5x in size for hints.

Usage:
    tools/certcheck.py <cert> [<obligation.obl>]
    tools/certcheck.py --verify-root <root_hex> <cert> [<obl1.obl> ...]
    tools/certcheck.py --merkle <obl1.obl> [<obl2.obl> ...]

With an obligation given, the 15.1 binding is enforced: sha256 over the
obligation's canonical text must equal the certificate's `x` line, which is what
stops a valid certificate being replayed against a DIFFERENT obligation.
Without it the derivation is still checked, but the certificate is not tied to
anything -- so a gate must always pass the obligation.

--verify-root: check a certificate AND verify that the Merkle root over all
    provided obligations matches the expected root. This is the 15.3 gate.

--merkle: compute and print the Merkle root over the given obligation files
    (diagnostic; does not check certificates).

Merkle root construction (15.3):
    SHA-256 of the sorted concatenation of sha256(canonical_text obl_i) for
    each obligation. Single-element: root = sha256(sha256(text)).
    Empty: root = 0x0000...0000 (64 zeros).

THE GOAL CHECK (added 2026-09-14; `selfhost/tcheck.bp:1225` had it, this file did
not).  Walking the hint chains proves the certificate refutes SOME CNF.  The `x`
line proves the checker was handed SOME obligation text.  NEITHER proves the CNF
encodes that text -- so until now a certificate built from the clauses and the
resolution chain of one obligation and the `x` line of another replayed
perfectly and printed `certcheck: OK` (bench/cert_neg/n05_wrong_clauses.cert).
docs/RESEARCH-VERIFICATION-2026-09-09.md:196 already named the requirement: "A
certificate is worthless unless the checker also verifies that the CNF it
refutes ENCODES the VC: that is why the bit-blaster is inside the trusted
checker rather than in the solver."

So the checker carries its OWN bit-blaster (below), re-blasts the obligation it
was handed, and compares the result POSITIONALLY with the certificate's input
(`c`) clauses: same count, same order, same literals.  The blaster is a copy,
not an import: `tools/certgen.py` is an untrusted producer scheduled for
deletion, and a checker that imports the producer's encoding checks nothing.
Both mirror the normative definition in `selfhost/tcheck.bp` (bb_and/bb_or/
bb_xor/bb_maj/bb_addc64/bb_sub64/bb_mul64/bb_eq64), folding cases included and
in the same order.  Exit 3 on mismatch -- the same code tcheck.bp uses.

THE WIDTH BOUND (same round).  A clause may carry at most MAX_CLAUSE_WIDTH=64
literals, which is the certificate path's clause stride in `tcheck.bp`
(`verify_cert` sets `n_cl[2] = 64`; `add_input:305` and `add_derived:337`
`sys_exit(2)` rather than store past it).  A checker that silently accepted a
65-literal clause where the trusted one refuses it is a checker that disagrees
with the artifact it is the twin of (bench/cert_neg/n06_clause_too_wide.cert).
Exit 2, the code tcheck.bp uses for it -- REFUSED, never truncated: a truncated
clause is a DIFFERENT clause, and a chain checked against it is a chain checked
against the wrong formula.

Exit codes:
    0  accepted
    2  malformed / unverifiable certificate (bad chain, no empty clause, no `x`,
       oid mismatch, clause too wide)
    3  the certificate does not encode the obligation it is bound to
"""
import hashlib
import re
import sys

W = 64                  # Bebop's only scalar width
MAX_CLAUSE_WIDTH = 64   # tcheck.bp's certificate-path clause stride (n_cl[2])
MAX_HINTS = 1024        # tcheck.bp add_derived:337 -- the hint array's real bound


def die(msg, rc=2):
    sys.stderr.write('certcheck: %s\n' % msg)
    raise SystemExit(rc)


def canonical_text(lines):
    """MUST match tools/certgen.py exactly: LF only, comments and blank lines
    removed, single spaces between tokens. Pinned early because every oid and
    every Merkle root depends on it."""
    out = []
    for l in lines:
        l = l.strip()
        if not l or l.startswith('%'):
            continue
        out.append(' '.join(l.split()))
    return ('\n'.join(out) + '\n').encode()


def obl_oid(obl_path):
    """sha256 of the canonical text of an obligation file (15.1 oid)."""
    with open(obl_path) as f:
        return hashlib.sha256(canonical_text(f.read().split('\n'))).hexdigest()


def merkle_root(obl_paths):
    """Compute a SHA-256 Merkle root over the obligation oids (15.3).

    Construction: each leaf is sha256(canonical_text(obl)) as bytes.
    The root is sha256(leaf_0 || leaf_1 || ... || leaf_n).
    Empty list: 64 zero hex digits.
    Single file: sha256(sha256(canonical_text)) as hex.
    """
    if not obl_paths:
        return '0' * 64
    leaves = []
    for p in obl_paths:
        with open(p) as f:
            ct = canonical_text(f.read().split('\n'))
        leaves.append(hashlib.sha256(ct).digest())
    # sort leaves for determinism (binary sort by byte value)
    leaves.sort()
    return hashlib.sha256(b''.join(leaves)).hexdigest()


# =============================================================================
# THE BIT-BLASTER -- the checker's own copy of the NORMATIVE encoding.
#
# `selfhost/tcheck.bp` DEFINES it (operator ruling 2026-09-14: the trusted
# artifact fixes the encoding and the untrusted producer is made to match).
# This is an independent transcription of the same definition, deliberately NOT
# an import of tools/certgen.py: that file is an untrusted producer scheduled
# for deletion, and a checker that asks the producer what the clauses should be
# has asked the wrong party.  Method-for-function correspondence, folding cases
# included and in the same order:
#   gate_and  <- bb_and  + bb_and_fold     gate_or  <- bb_or  + bb_or_fold
#   gate_xor  <- bb_xor  + bb_xor_fold     gate_maj <- bb_maj
#   addc/add64 <- bb_addc64/bb_add64       sub64    <- bb_sub64
#   mul64     <- bb_mul64                  eq64     <- bb_eq64
# =============================================================================

class CNF(object):
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
        T, F = self.true, -self.true
        if a == F or b == F: return F
        if a == T: return b
        if b == T: return a
        if a == b: return a
        if a == -b: return F
        o = self.fresh()
        self.add(-a, -b, o); self.add(a, -o); self.add(b, -o)
        return o

    def gate_or(self, a, b):
        T, F = self.true, -self.true
        if a == T or b == T: return T
        if a == F: return b
        if b == F: return a
        if a == b: return a
        if a == -b: return T
        o = self.fresh()
        self.add(a, b, -o); self.add(-a, o); self.add(-b, o)
        return o

    def gate_xor(self, a, b):
        T, F = self.true, -self.true
        if a == F: return b
        if b == F: return a
        if a == T: return -b
        if b == T: return -a
        if a == b: return F
        if a == -b: return T
        o = self.fresh()
        self.add(-a, -b, -o); self.add(a, b, -o); self.add(a, -b, o); self.add(-a, b, o)
        return o

    def gate_maj(self, a, b, c):
        T, F = self.true, -self.true
        if c == F: return self.gate_and(a, b)
        if c == T: return self.gate_or(a, b)
        if a == F: return self.gate_and(b, c)
        if a == T: return self.gate_or(b, c)
        if b == F: return self.gate_and(a, c)
        if b == T: return self.gate_or(a, c)
        if a == b: return a
        if a == -b: return c
        if a == c: return a
        if a == -c: return b
        if b == c: return b
        if b == -c: return a
        o = self.fresh()
        self.add(a, b, -o); self.add(a, c, -o); self.add(b, c, -o)
        self.add(-a, -b, o); self.add(-a, -c, o); self.add(-b, -c, o)
        return o

    def addc(self, a, b, cin):
        out = []
        for k in range(W):
            axb = self.gate_xor(a[k], b[k])
            out.append(self.gate_xor(axb, cin))
            if k < W - 1:
                cin = self.gate_maj(a[k], b[k], cin)
        return out

    def add64(self, a, b):
        return self.addc(a, b, -self.true)

    def sub64(self, a, b):
        t = self.add64(a, [-l for l in b])
        one = [self.const(1 if k == 0 else 0) for k in range(W)]
        return self.add64(t, one)

    def mul64(self, a, b):
        F = -self.true
        acc = [F] * W
        for bit in range(W):
            sel = b[bit]
            if sel == F:
                continue
            row = [F] * bit + [self.gate_and(a[j], sel) for j in range(W - bit)]
            acc = self.add64(acc, row)
        return acc

    def eq64(self, a, b):
        acc = self.true
        for k in range(W):
            acc = self.gate_and(acc, -self.gate_xor(a[k], b[k]))
        return acc


def obl_parse(path):
    """Parse an obligation DAG.  Args refer to STRICTLY smaller ids, so one
    forward pass checks well-formedness and acyclicity together."""
    with open(path) as f:
        lines = f.read().split('\n')
    nodes, order, name, goal = {}, [], None, None
    binop = ('add', 'sub', 'mul', 'and', 'or', 'xor', 'not', 'eq')
    for i, raw in enumerate(lines, 1):
        t = raw.strip()
        if not t or t.startswith('%'):
            continue
        w = t.split()
        if w[0] == 'obligation':
            name = w[1] if len(w) > 1 else 'anon'
            continue
        if w[0] == 'prove':
            if len(w) < 2:
                die('%s line %d: `prove` with no node id' % (path, i), 3)
            goal = int(w[1])
            continue
        if len(w) < 2 or not re.fullmatch(r'\d+', w[0]):
            die('%s line %d: not a node line: %r' % (path, i, t), 3)
        nid, op, args = int(w[0]), w[1], w[2:]
        if nid in nodes:
            die('%s line %d: duplicate node id %d' % (path, i, nid), 3)
        for a in args:
            if re.fullmatch(r'-?\d+', a) and op in binop:
                if int(a) >= nid:
                    die('%s line %d: node %d refers to %s -- args must be '
                        'STRICTLY smaller' % (path, i, nid, a), 3)
        nodes[nid] = (op, args)
        order.append(nid)
    if goal is None:
        die('%s: no `prove <id>` line -- nothing to encode' % path, 3)
    if goal not in nodes:
        die('%s: prove names node %d which is not defined' % (path, goal), 3)
    return name, nodes, order, goal


def blast(nodes, order, cnf, path):
    """Each 64-bit term becomes a list of 64 literals, LSB first.  Variables are
    allocated 64 at a time in node order, exactly as bb_alloc_word does."""
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
            bits[nid] = cnf.add64(bits[int(args[0])], bits[int(args[1])])
        elif op == 'sub':
            bits[nid] = cnf.sub64(bits[int(args[0])], bits[int(args[1])])
        elif op == 'mul':
            bits[nid] = cnf.mul64(bits[int(args[0])], bits[int(args[1])])
        elif op in ('shl', 'lshr'):
            a, k = bits[int(args[0])], int(args[1])
            if not 0 <= k < W:
                die('%s: node %d: shift amount %d out of range' % (path, nid, k), 3)
            z = cnf.const(0)
            bits[nid] = ([z] * k + a[:W - k]) if op == 'shl' else (a[k:] + [z] * k)
        elif op == 'eq':
            bits[nid] = [cnf.eq64(bits[int(args[0])], bits[int(args[1])])]
        else:
            # A checker that cannot re-derive the encoding must REFUSE, not wave
            # the certificate through: "unknown to me" is not "correct".
            die('%s: node %d uses op %r, which this checker cannot bit-blast, so '
                'it cannot verify that the certificate\'s clauses encode this '
                'obligation.' % (path, nid, op), 3)
    return bits


def blast_obligation(obl_path):
    """Return the input-clause list a conforming certificate for this obligation
    MUST carry, in order: the blasted CNF plus the NEGATED goal as the last
    clause (UNSAT == the goal is valid)."""
    name, nodes, order, goal = obl_parse(obl_path)
    cnf = CNF()
    bits = blast(nodes, order, cnf, obl_path)
    g = bits[goal]
    if len(g) != 1:
        die('%s: prove names node %d, which is a 64-bit term, not a boolean '
            '(use an `eq` node)' % (obl_path, goal), 3)
    cnf.add(-g[0])
    return name, cnf.n, list(cnf.cl)


def goal_check(cert_path, obl_path, input_clauses, nvars, nclauses):
    """POSITIONAL comparison of the certificate's `c` clauses against the blast
    of the obligation it is bound to.  Mirrors tcheck.bp's ob_cmp (:1177): same
    count, same length, same literals in the same order.  The producer and the
    checker emit gates in the same sequence, which is what the alignment bought;
    a set comparison would be weaker and slower for no gain."""
    name, want_nvars, want = blast_obligation(obl_path)
    if nvars is not None and nvars != want_nvars:
        die('GOAL CHECK FAILED: the `p` header declares %d variables but the '
            'obligation %s blasts to %d.\n'
            '  The certificate does not encode the obligation it is bound to.'
            % (nvars, obl_path, want_nvars), 3)
    if len(input_clauses) != len(want):
        die('GOAL CHECK FAILED: the certificate carries %d input clauses, but '
            'obligation %s (%s) blasts to %d.\n'
            '  The oid proves this checker was handed the right TEXT. It does '
            'NOT prove the clauses encode that text -- this check does, and it '
            'says they do not.'
            % (len(input_clauses), obl_path, name, len(want)), 3)
    for i, (got, exp) in enumerate(zip(input_clauses, want)):
        lineno, lits = got
        if lits != exp:
            die('GOAL CHECK FAILED at input clause %d (line %d of %s):\n'
                '  certificate: %s\n'
                '  obligation:  %s\n'
                '  The certificate refutes a DIFFERENT formula from the one '
                'obligation %s (%s) encodes. Walking the hint chain proves a '
                'refutation of SOME CNF; only this check ties that CNF to the '
                'claim in the `x` line.'
                % (i + 1, lineno, cert_path,
                   ' '.join(map(str, lits)), ' '.join(map(str, exp)),
                   obl_path, name), 3)
    if nclauses is not None and nclauses != len(want):
        die('GOAL CHECK FAILED: the `p` header declares %d input clauses but '
            'the obligation blasts to %d' % (nclauses, len(want)), 3)
    return len(want)


def _format_clause(clause, assign):
    """Format a clause under the current assignment for diagnostics."""
    parts = []
    for l in clause:
        v, want = abs(l), (l > 0)
        if v in assign:
            val = assign[v]
            tag = 'T' if val else 'F'
            parts.append('%d=%s' % (l, tag))
        else:
            parts.append('%d=U' % l)
    return '{' + ', '.join(parts) + '}'


def _context_line(cert_path, lineno, raw_lines=None):
    """Return the source line for better error context."""
    if raw_lines and lineno <= len(raw_lines):
        return '  --> %s:%d: %s' % (cert_path, lineno, raw_lines[lineno - 1].rstrip())
    return '  --> %s:%d' % (cert_path, lineno)


def check(cert_path, obl_path=None, raw_lines=None):
    clauses, order = {}, []
    nvars = nclauses = None
    oid = None
    derived_empty = False
    steps = 0
    last_line = 0
    input_clauses = []          # (lineno, lits) for every `c` line, in file order
    seen_derived = False        # the input block is CLOSED by the first `r` line

    with open(cert_path) as f:
        for lineno, raw in enumerate(f, 1):
            last_line = lineno
            w = raw.split()
            if not w or w[0] == '%':
                continue
            tag = w[0]
            if tag == 'p':
                if len(w) < 3:
                    die('line %d: malformed p header -- expected "p <nvars> <nclauses>", '
                        'got %d tokens' % (lineno, len(w)))
                nvars, nclauses = int(w[1]), int(w[2])
                if nvars < 0 or nclauses < 0:
                    die('line %d: p header has negative dimensions '
                        '(nvars=%d, nclauses=%d)' % (lineno, nvars, nclauses))
            elif tag == 'c':
                if len(w) < 2:
                    die('line %d: malformed c line -- expected at least "c <id>"' % lineno)
                cid = int(w[1])
                lits = [int(x) for x in w[2:]]
                if lits and lits[-1] == 0:
                    lits = lits[:-1]
                _width_check(lineno, cid, lits, 'input')
                if seen_derived:
                    die('line %d: input clause %d appears AFTER a derived (`r`) '
                        'line. The input block is what the goal check compares '
                        'against the obligation; a `c` line smuggled in after it '
                        'would be an unchecked axiom, which is the same hole the '
                        'goal check exists to close.' % (lineno, cid))
                input_clauses.append((lineno, lits))
                if cid in clauses:
                    die('line %d: duplicate clause id %d '
                        '(first seen at clause %d in the certificate)' % (lineno, cid, cid))
                clauses[cid] = lits
                order.append(cid)
            elif tag == 'r':
                if len(w) < 3:
                    die('line %d: malformed r line -- expected at least '
                        '"r <id> ... 0 ..."' % lineno)
                cid = int(w[1])
                rest = [int(x) for x in w[2:]]
                try:
                    z = rest.index(0)
                except ValueError:
                    die('line %d: derived clause %d has no 0 terminator '
                        'separating literals from hints' % (lineno, cid))
                lits, hints = rest[:z], [h for h in rest[z + 1:] if h != 0]
                seen_derived = True
                if len(hints) > MAX_HINTS:
                    # The trusted checker SILENTLY TRUNCATED hint lists until
                    # 1fa8df8; the fix there was to refuse loudly at 1024. A twin
                    # that quietly walks a 2000-hint chain the trusted checker
                    # would have refused is not a twin.
                    die('line %d: derived clause %d cites %d antecedents, past '
                        'the hint bound of %d. selfhost/tcheck.bp refuses here '
                        '(add_derived:337); silently walking a longer chain is '
                        'how the trusted checker came to TRUNCATE hint lists.'
                        % (lineno, cid, len(hints), MAX_HINTS))
                _width_check(lineno, cid, lits, 'derived')
                if not hints:
                    die('line %d: derived clause %d has NO HINTS. Hints are mandatory '
                        'in this format -- an unhinted step would force the checker to '
                        'run unit propagation, which is the thing hints exist to avoid.'
                        % (lineno, cid))
                if cid in clauses:
                    die('line %d: duplicate clause id %d '
                        '(first seen earlier in the certificate)' % (lineno, cid))
                # --- the whole check, and it is a loop ---
                assign = {}
                for l in lits:                      # assume the negation of the claim
                    assign[abs(l)] = (l < 0)
                conflict = False
                for h in hints:
                    hc = clauses.get(h)
                    if hc is None:
                        die('line %d: clause %d cites antecedent %d, which is not '
                            'present (deleted too early, or a forward reference).\n'
                            '  Known clause ids: %s\n'
                            '  %s'
                            % (lineno, cid, h, _known_ids(clauses),
                               _context_line(cert_path, lineno)))
                    unassigned = []
                    satisfied = False
                    for l in hc:
                        v, want = abs(l), (l > 0)
                        if v not in assign:
                            unassigned.append(l)
                        elif assign[v] == want:
                            satisfied = True
                            break
                    if satisfied:
                        die('line %d: clause %d antecedent %d is already SATISFIED -- '
                            'the hint order does not produce a resolution chain.\n'
                            '  Antecedent %s under current assignment\n'
                            '  %s'
                            % (lineno, cid, h,
                               _format_clause(hc, assign),
                               _context_line(cert_path, lineno)))
                    if not unassigned:
                        conflict = True             # falsified: the chain closes
                        break
                    if len(unassigned) > 1:
                        die('line %d: clause %d antecedent %d is not unit under the '
                            'assumption (%d literals unassigned) -- with hints this is '
                            'a malformed certificate, not a reason to search.\n'
                            '  Unassigned: %s\n'
                            '  %s'
                            % (lineno, cid, h, len(unassigned),
                               ', '.join(str(u) for u in unassigned),
                               _context_line(cert_path, lineno)))
                    u = unassigned[0]
                    assign[abs(u)] = (u > 0)
                if not conflict:
                    die('line %d: clause %d -- the hint chain ended without a conflict. '
                        'Each hint must be either unit (extend assignment) or falsified '
                        '(close chain). Walked %d hints without reaching a conflict.\n'
                        '  %s'
                        % (lineno, cid, len(hints),
                           _context_line(cert_path, lineno)))
                clauses[cid] = lits
                order.append(cid)
                steps += 1
                if not lits:
                    derived_empty = True
            elif tag == 'd':
                for x in w[1:]:
                    clauses.pop(int(x), None)
            elif tag == 'x':
                if oid is not None:
                    die('line %d: duplicate x line (previous oid: %s)' % (lineno, oid))
                oid = w[1]
                if len(oid) != 64:
                    die('line %d: oid must be exactly 64 hex characters, '
                        'got %d: %s' % (lineno, len(oid), oid))
            else:
                die('line %d: unknown record type %r -- valid types are '
                    'p, c, r, d, x' % (lineno, tag))

    if nvars is None:
        die('no `p` header -- certificate must start with "p <nvars> <nclauses>"')
    if nclauses is not None and steps > nclauses:
        die('declared %d clauses but verified %d derivation steps -- '
            'certificate appears truncated or malformed' % (nclauses, steps))
    if not derived_empty:
        die('the empty clause was never derived -- this certificate does not '
            'refute anything, so it proves nothing.\n'
            '  A valid refutation must derive the empty clause [] via resolution.')
    if oid is None:
        die('no `x <oid>` line: the certificate is not bound to an obligation and '
            'could be replayed against any of them (15.1).\n'
            '  Every certificate must end with "x <sha256hex>" binding it to one '
            'obligation.')

    bound = 'UNBOUND (no obligation given -- a gate must always pass one)'
    encodes = ('NOT CHECKED (no obligation given -- the clauses are tied to '
               'nothing)')
    if obl_path:
        want = obl_oid(obl_path)
        if want != oid:
            die('OID MISMATCH: certificate binds %s,\n'
                '  but obligation hashes to     %s.\n'
                '  This is exactly the replay attack 15.1 exists to stop: a '
                'certificate that checks, filed against the wrong obligation.\n'
                '  File: %s' % (oid, want, obl_path))
        bound = 'bound to %s (sha256 verified)' % obl_path
        # THE GOAL CHECK. The oid above proves only that the right TEXT was
        # handed over; this proves the clauses the chain refutes encode it.
        n_enc = goal_check(cert_path, obl_path, input_clauses, nvars, nclauses)
        encodes = ('yes -- %d input clauses re-blasted and matched positionally'
                   % n_enc)

    print('certcheck: OK')
    print('  resolution steps verified   %d (all hinted; no propagation, no search)' % steps)
    print('  empty clause derived        yes')
    print('  oid                         %s' % oid)
    print('  binding                     %s' % bound)
    print('  clauses encode obligation   %s' % encodes)
    return 0


def _width_check(lineno, cid, lits, kind):
    """REFUSE, never truncate. A truncated clause is a DIFFERENT clause, and a
    resolution chain checked against it is a chain checked against the wrong
    formula. 64 is tcheck.bp's certificate-path clause stride (verify_cert sets
    n_cl[2]=64); add_input:305 and add_derived:337 sys_exit(2) at the same
    boundary, so this is the twin agreeing with the trusted artifact."""
    if len(lits) > MAX_CLAUSE_WIDTH:
        die('line %d: %s clause %d carries %d literals, past the clause stride '
            'of %d.\n'
            '  The trusted checker (selfhost/tcheck.bp) REFUSES here rather than '
            'storing a prefix, and so does this one: a truncated clause is a '
            'different clause.' % (lineno, kind, cid, len(lits),
                                   MAX_CLAUSE_WIDTH))


def _known_ids(clauses):
    """Return a compact string of known clause ids for error messages."""
    ids = sorted(clauses.keys())
    if len(ids) <= 10:
        return str(ids)
    return str(ids[:5]) + ' ... (%d total)' % len(ids)


def main():
    args = sys.argv[1:]
    if not args:
        die('usage:\n'
            '  certcheck.py <cert> [<obligation.obl>]\n'
            '  certcheck.py --verify-root <root_hex> <cert> [<obl1.obl> ...]\n'
            '  certcheck.py --merkle <obl1.obl> [<obl2.obl> ...]')

    if args[0] == '--merkle':
        obl_paths = args[1:]
        if not obl_paths:
            die('--merkle requires at least one obligation file')
        for p in obl_paths:
            try:
                open(p)
            except FileNotFoundError:
                die('obligation file not found: %s' % p)
        root = merkle_root(obl_paths)
        print('merkle root: %s' % root)
        print('  obligations: %d' % len(obl_paths))
        return 0

    if args[0] == '--verify-root':
        if len(args) < 3:
            die('--verify-root requires <root_hex> <cert> [<obl1.obl> ...]')
        expected_root = args[1]
        cert_path = args[2]
        obl_paths = args[3:]
        if not obl_paths:
            die('--verify-root requires at least one obligation file')
        for p in obl_paths:
            try:
                open(p)
            except FileNotFoundError:
                die('obligation file not found: %s' % p)
        actual_root = merkle_root(obl_paths)
        if actual_root != expected_root:
            die('MERKLE ROOT MISMATCH:\n'
                '  expected: %s\n'
                '  actual:   %s\n'
                '  This means the obligation set does not match the gate root.' %
                (expected_root, actual_root))
        # verify each certificate against its obligation
        n_ok = 0
        for obl_path in obl_paths:
            oid = obl_oid(obl_path)
            # look for a matching certificate (convention: <obl>.cert)
            cert_candidate = obl_path.rsplit('.', 1)[0] + '.cert'
            import os
            if os.path.exists(cert_candidate):
                with open(cert_candidate) as f:
                    lines = f.readlines()
                rc = check(cert_candidate, obl_path, raw_lines=lines)
                n_ok += 1
        print('merkle root: %s' % actual_root)
        print('  obligations: %d' % len(obl_paths))
        print('  certificates verified: %d' % n_ok)
        return 0

    # default mode: single certificate check
    cert_path = args[0]
    obl_path = args[1] if len(args) > 1 else None
    if obl_path:
        try:
            open(obl_path)
        except FileNotFoundError:
            die('obligation file not found: %s' % obl_path)
    with open(cert_path) as f:
        lines = f.readlines()
    return check(cert_path, obl_path, raw_lines=lines)


if __name__ == '__main__':
    raise SystemExit(main())
