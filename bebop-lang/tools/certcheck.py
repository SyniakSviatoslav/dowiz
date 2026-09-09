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

With an obligation given, the 15.1 binding is enforced: sha256 over the
obligation's canonical text must equal the certificate's `x` line, which is what
stops a valid certificate being replayed against a DIFFERENT obligation.
Without it the derivation is still checked, but the certificate is not tied to
anything -- so a gate must always pass the obligation.
"""
import hashlib
import sys


def die(msg):
    sys.stderr.write('certcheck: %s\n' % msg)
    raise SystemExit(2)


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


def check(cert_path, obl_path=None):
    clauses, order = {}, []
    nvars = nclauses = None
    oid = None
    derived_empty = False
    steps = 0

    with open(cert_path) as f:
        for lineno, raw in enumerate(f, 1):
            w = raw.split()
            if not w or w[0] == '%':
                continue
            tag = w[0]
            if tag == 'p':
                nvars, nclauses = int(w[1]), int(w[2])
            elif tag == 'c':
                cid = int(w[1])
                lits = [int(x) for x in w[2:]]
                if lits and lits[-1] == 0:
                    lits = lits[:-1]
                if cid in clauses:
                    die('line %d: duplicate clause id %d' % (lineno, cid))
                clauses[cid] = lits
                order.append(cid)
            elif tag == 'r':
                cid = int(w[1])
                rest = [int(x) for x in w[2:]]
                try:
                    z = rest.index(0)
                except ValueError:
                    die('line %d: malformed r line (no 0 terminator)' % lineno)
                lits, hints = rest[:z], [h for h in rest[z + 1:] if h != 0]
                if not hints:
                    die('line %d: derived clause %d has NO HINTS. Hints are mandatory '
                        'in this format -- an unhinted step would force the checker to '
                        'run unit propagation, which is the thing hints exist to avoid.'
                        % (lineno, cid))
                if cid in clauses:
                    die('line %d: duplicate clause id %d' % (lineno, cid))
                # --- the whole check, and it is a loop ---
                assign = {}
                for l in lits:                      # assume the negation of the claim
                    assign[abs(l)] = (l < 0)
                conflict = False
                for h in hints:
                    hc = clauses.get(h)
                    if hc is None:
                        die('line %d: clause %d cites antecedent %d, which is not '
                            'present (deleted too early, or a forward reference)'
                            % (lineno, cid, h))
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
                            'the hint order does not produce a resolution chain'
                            % (lineno, cid, h))
                    if not unassigned:
                        conflict = True             # falsified: the chain closes
                        break
                    if len(unassigned) > 1:
                        die('line %d: clause %d antecedent %d is not unit under the '
                            'assumption (%d literals unassigned) -- with hints this is '
                            'a malformed certificate, not a reason to search'
                            % (lineno, cid, h, len(unassigned)))
                    u = unassigned[0]
                    assign[abs(u)] = (u > 0)
                if not conflict:
                    die('line %d: clause %d -- the hint chain ended without a conflict'
                        % (lineno, cid))
                clauses[cid] = lits
                order.append(cid)
                steps += 1
                if not lits:
                    derived_empty = True
            elif tag == 'd':
                for x in w[1:]:
                    clauses.pop(int(x), None)
            elif tag == 'x':
                oid = w[1]
            else:
                die('line %d: unknown record %r' % (lineno, tag))

    if nvars is None:
        die('no `p` header')
    if not derived_empty:
        die('the empty clause was never derived -- this certificate does not '
            'refute anything, so it proves nothing')
    if oid is None:
        die('no `x <oid>` line: the certificate is not bound to an obligation and '
            'could be replayed against any of them (15.1)')

    bound = 'UNBOUND (no obligation given -- a gate must always pass one)'
    if obl_path:
        with open(obl_path) as f:
            want = hashlib.sha256(canonical_text(f.read().split('\n'))).hexdigest()
        if want != oid:
            die('OID MISMATCH: certificate binds %s, obligation hashes to %s.\n'
                '  This is exactly the replay 15.1 exists to stop: a certificate that '
                'checks, filed against the wrong obligation.' % (oid, want))
        bound = 'bound to %s' % obl_path

    print('certcheck: OK')
    print('  resolution steps verified   %d (all hinted; no propagation, no search)' % steps)
    print('  empty clause derived        yes')
    print('  oid                         %s' % oid)
    print('  binding                     %s' % bound)
    return 0


if __name__ == '__main__':
    a = sys.argv[1:]
    if not a:
        die('usage: certcheck.py <cert> [<obligation.obl>]')
    raise SystemExit(check(a[0], a[1] if len(a) > 1 else None))
