#!/usr/bin/env python3
"""certcheck_census.py -- measure certificate checker soundness.

This instrument verifies that `tools/certcheck.py` correctly REJECTS corrupted
certificates (checker_neg) and counts ACCEPTED valid certificates (cert_checked).

A certificate is CHECKED only when `certcheck.py` is actually invoked on it
and returns 0 (acceptance). Merely having a .cert file does not count.

Output:
  Line 1: `cert_checked: <accepted>/<total_valid>`
  Line 2: `checker_neg: <wrongly_accepted>/<total_corrupted>`
  Lines 3+: for each wrongly-accepted corruption: <file> <corruption-type>

Exit codes:
  0 = success, all corrupted certificates are rejected
  1 = failure, at least one corrupted certificate was wrongly accepted (UNSOUND)
  2 = NOT MEASURED -- cannot find certificates or obligations
"""

import os
import subprocess
import sys


def run_certcheck(cert_path, obl_path=None):
    """Run certcheck.py and return True if it accepts (rc=0), False if rejects.

    Returns None if certcheck.py crashes or cannot run.
    """
    try:
        cmd = [sys.executable, 'tools/certcheck.py', cert_path]
        if obl_path:
            cmd.append(obl_path)
        result = subprocess.run(cmd, capture_output=True, timeout=5)
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        return None


def find_valid_certificates(base_dirs):
    """Find all valid (cert, obligation) pairs in cert_pos directories.

    Returns: list of (cert_path, obl_path)
    """
    certs = []
    for base_dir in base_dirs:
        pos_dir = os.path.join(base_dir, 'cert_pos')
        if os.path.isdir(pos_dir):
            for f in sorted(os.listdir(pos_dir)):
                if f.endswith('.cert'):
                    cert_path = os.path.join(pos_dir, f)
                    obl_path = cert_path.rsplit('.', 1)[0] + '.obl'
                    if os.path.exists(obl_path):
                        certs.append((cert_path, obl_path))
    return certs


def find_corrupted_certificates(base_dirs):
    """Find deliberately corrupted test certificates in cert_neg directories.

    Returns: list of (cert_path, obl_path or None)
    """
    neg_certs = []
    for base_dir in base_dirs:
        neg_dir = os.path.join(base_dir, 'cert_neg')
        if os.path.isdir(neg_dir):
            for f in sorted(os.listdir(neg_dir)):
                if f.endswith('.cert'):
                    cert_path = os.path.join(neg_dir, f)
                    # Check for corresponding obligation
                    obl_path = cert_path.rsplit('.', 1)[0] + '.obl'
                    if os.path.exists(obl_path):
                        neg_certs.append((cert_path, obl_path))
                    else:
                        neg_certs.append((cert_path, None))
    return neg_certs


def measure_certificates():
    """Measure certificate checking.

    Returns: (cert_checked, total_valid, checker_neg, total_corrupted, unsound_list)
    Raises: OSError if sources cannot be found
    """
    base_dirs = ['bench']

    # Find all valid certificates (in cert_pos/)
    valid_certs = find_valid_certificates(base_dirs)

    if not valid_certs:
        raise OSError("no valid certificates found in bench/cert_pos/")

    # Count accepted certificates
    cert_checked = 0
    cert_total = len(valid_certs)
    for cert_path, obl_path in valid_certs:
        result = run_certcheck(cert_path, obl_path)
        if result is True:
            cert_checked += 1

    # Find test negatives (corrupted certificates that SHOULD be rejected)
    neg_certs = find_corrupted_certificates(base_dirs)

    if not neg_certs:
        # No test negatives found; measure returns 0 corrupted
        return cert_checked, cert_total, 0, 0, []

    # Count wrongly-accepted corrupted certificates
    unsound = []
    for cert_path, obl_path in neg_certs:
        result = run_certcheck(cert_path, obl_path)
        if result is True:
            # This is a corruption that was ACCEPTED (unsound!)
            unsound.append((cert_path, 'wrongly-accepted'))

    checker_neg_total = len(neg_certs)
    checker_neg_accepted = len(unsound)

    return cert_checked, cert_total, checker_neg_accepted, checker_neg_total, unsound


def main():
    try:
        cert_checked, cert_total, checker_neg_accepted, checker_neg_total, unsound = measure_certificates()
    except OSError as e:
        print(f"cert_checked: NOT MEASURED -- {e}")
        sys.exit(2)

    # Print gate lines
    print(f"cert_checked: {cert_checked}/{cert_total}")
    print(f"checker_neg: {checker_neg_accepted}/{checker_neg_total}")

    # Print unsound certificates
    for cert_path, corruption_type in unsound:
        print(f"{cert_path} {corruption_type}")

    # Exit 1 if any corrupted certificate was wrongly accepted
    if unsound:
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == '__main__':
    main()
