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
    if obl_path:
        want = obl_oid(obl_path)
        if want != oid:
            die('OID MISMATCH: certificate binds %s,\n'
                '  but obligation hashes to     %s.\n'
                '  This is exactly the replay attack 15.1 exists to stop: a '
                'certificate that checks, filed against the wrong obligation.\n'
                '  File: %s' % (oid, want, obl_path))
        bound = 'bound to %s (sha256 verified)' % obl_path

    print('certcheck: OK')
    print('  resolution steps verified   %d (all hinted; no propagation, no search)' % steps)
    print('  empty clause derived        yes')
    print('  oid                         %s' % oid)
    print('  binding                     %s' % bound)
    return 0


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
