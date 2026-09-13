#!/usr/bin/env python3
"""theorem_census.py -- measure kernel-checked theorems in the Bebop codebase.

This instrument enumerates every `theorem` declaration in .bp sources and determines
whether each has been kernel-checked. A theorem is kernel-checked only when:
  (a) a Core term is DERIVED from that theorem's source text, and
  (b) the kernel checker has been invoked on it and observed acceptance.

Since no elaborator exists in this tree to derive Core terms from .bp theorems,
the numerator is structurally locked at 0 today.

Output:
  Line 1: gate line: `theorems: <kernel_checked>/<declared>`
  Line 2: diagnostic line: `theorem_bridge: <status>`
  Lines 3+: for each UNCHECKED theorem: <file>:<line>: <declaration>

Exit codes:
  0 = success, all theorems are kernel-checked (only possible after elaborator exists)
  1 = failure, at least one theorem is unchecked (gate is RED)
  2 = NOT MEASURED -- cannot find sources
"""

import os
import re
import subprocess
import sys


def find_theorems(root_dirs):
    """Enumerate every `theorem` declaration in .bp files under root_dirs.

    Returns: list of (file_path, line_number, declaration_text)
    Raises: OSError if root_dirs don't exist
    """
    theorems = []

    for root_dir in root_dirs:
        if not os.path.isdir(root_dir):
            raise OSError(f"source directory not found: {root_dir}")

        for dirpath, dirnames, filenames in os.walk(root_dir):
            for filename in sorted(filenames):
                if not filename.endswith('.bp'):
                    continue

                filepath = os.path.join(dirpath, filename)
                try:
                    with open(filepath, 'r') as f:
                        lines = f.readlines()
                except (IOError, OSError):
                    continue

                for lineno, line in enumerate(lines, 1):
                    # Match: theorem <name> : <type> := <proof>
                    match = re.match(r'^\s*theorem\s+(\w+)\s*:', line)
                    if match:
                        decl_text = line.rstrip()
                        theorems.append((filepath, lineno, decl_text))

    return theorems


# Where Core fixtures live. NOTE bench/kernel_neg/ is the kernel's NEGATIVE corpus --
# tools/battery.sh asserts `kernel_neg: 0 accepted of 21` and `kernel_parity: 28/28`
# against it. This instrument only READS it and must never write there; a positive
# theorem corpus, when one exists, gets its own directory.
FIXTURE_DIRS = ['bench/kernel_neg']

PROVENANCE_RE = re.compile(r'theorem-src:\s*(\S+:\d+)')


def scan_provenance(fixture_dirs):
    """Map 'file:line' of a .bp theorem -> the fixture path claiming to be derived from it.

    A fixture claims provenance by carrying a `theorem-src: <file>:<line>` marker in its
    own text. Nothing infers the link from a filename.
    """
    found = {}
    for d in fixture_dirs:
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            if not name.endswith('.core'):
                continue
            path = os.path.join(d, name)
            try:
                text = open(path).read()
            except (IOError, OSError):
                continue
            m = PROVENANCE_RE.search(text)
            if m:
                found[m.group(1)] = path
    return found


def kernel_accepts(fixture_path):
    """True only when tools/kcheck.py actually checks this fixture and accepts it.

    Provenance alone is a claim; this is the half that makes it evidence. Any failure to
    run the checker counts as NOT accepted -- an instrument that cannot reach the kernel
    must not award a pass.
    """
    if not os.path.exists('tools/kcheck.py'):
        return False
    d = os.path.dirname(fixture_path) or '.'
    try:
        r = subprocess.run([sys.executable, 'tools/kcheck.py', '--corpus', d],
                           capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.SubprocessError):
        return False
    if r.returncode != 0:
        return False
    base = os.path.basename(fixture_path)
    for line in r.stdout.splitlines():
        if base in line and 'accepted' in line:
            return True
    return False


def measure_theorems(bp_root_dirs):
    """Measure kernel-checked theorems.

    Returns: (kernel_checked_count, total_declared, unchecked_list, bridge_status)
             where kernel_checked_count is always 0 (no elaborator exists),
             and unchecked_list is list of (file, line, declaration)
    Raises: OSError if sources cannot be accessed
    """

    # Find all theorem declarations
    try:
        declared = find_theorems(bp_root_dirs)
    except OSError as e:
        raise OSError(f"cannot find theorem sources: {e}")

    # The numerator is DERIVED, never asserted. A theorem counts as kernel-checked
    # only when both halves hold:
    #   (a) PROVENANCE -- some fixture declares, in its own text, that it was derived
    #       from THAT theorem, via a `theorem-src: <file>:<line>` marker; and
    #   (b) ACCEPTANCE -- tools/kcheck.py actually checks that fixture and accepts it.
    #
    # Matching a fixture by FILENAME was the bug this instrument was built to expose and
    # then briefly reproduced: a file named `bad.core` containing the literal text
    # "THIS IS NOT A PROOF, IT IS GARBAGE" made `theorem bad : 1 + 1 = 3 := refl` count
    # as proved. A name is not a proof. Provenance has to be claimed by the artifact.
    #
    # No fixture in this tree carries the marker, because no elaborator emits one -- so
    # this computes 0 today. It computes 0; it does not return a hardcoded 0. The day an
    # elaborator emits a fixture with real provenance, this number moves on its own.
    provenance = scan_provenance(FIXTURE_DIRS)
    kernel_checked = 0
    unchecked = []
    for filepath, lineno, decl_text in declared:
        key = '%s:%d' % (filepath, lineno)
        fixture = provenance.get(key)
        if fixture and kernel_accepts(fixture):
            kernel_checked += 1
        else:
            unchecked.append((filepath, lineno, decl_text))

    if not provenance:
        bridge_status = (
            "ABSENT -- no .bp theorem is elaborated to a Core term. No fixture under %s "
            "carries a `theorem-src: <file>:<line>` marker, so nothing can raise the "
            "numerator above 0. A name match against a .core filename is NOT a proof: a "
            "file reading \"THIS IS NOT A PROOF\" satisfied an earlier version of this "
            "instrument." % ', '.join(FIXTURE_DIRS)
        )
    else:
        bridge_status = "PRESENT -- %d fixture(s) claim provenance from a .bp theorem" % len(provenance)

    return kernel_checked, len(declared), unchecked, bridge_status


def main():
    # Directories to search for theorems
    bp_root_dirs = ['samples', 'selfhost', 'bench']

    try:
        kernel_checked, total_declared, unchecked_list, bridge_status = measure_theorems(
            bp_root_dirs
        )
    except OSError as e:
        # NOT MEASURED
        print(f"theorems: NOT MEASURED -- {e}")
        sys.exit(2)

    # Print gate line
    print(f"theorems: {kernel_checked}/{total_declared}")

    # Print diagnostic line (explains why numerator is 0)
    print(f"theorem_bridge: {bridge_status}")

    # Print unchecked theorems
    for filepath, lineno, decl_text in unchecked_list:
        print(f"{filepath}:{lineno}: {decl_text}")

    # Exit 1 if any theorem is unchecked (RED), 0 if all are checked (GREEN)
    if unchecked_list:
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == '__main__':
    main()
