#!/usr/bin/env python3
"""
Gate for C1 hostile kernel dialect testing.

Classifies compiled hostile kernels into three buckets:
- LOUD TRAP: Caught by dialect (exit codes 80, 87, 102, etc.)
- ESCAPED: Got SIGSEGV/TRAP-82 (exit 82)
- CLEAN EXIT: Ran to completion (exit 0 or other non-trap)

Usage: python3 tools/hostile_gate.py --seed <S> --corpus <dir> --bin ./bebop.bin --seed-bin ./seed/build/seed
Output: Single gate line + exit code (0 if passed, nonzero if any escaped)

Gate format: hostile_kernels: <loud>/<total> loud, <escaped> escaped, <clean> clean (seed=<S>)
"""

import sys
import os
import subprocess
import argparse
import json
from pathlib import Path

class HostileGate:
    def __init__(self, seed, corpus_dir, bebop_bin, seed_bin):
        self.seed = seed
        self.corpus_dir = corpus_dir
        self.bebop_bin = bebop_bin
        self.seed_bin = seed_bin
        self.results = {
            "loud_trap": [],
            "escaped": [],
            "clean_exit": [],
            "compile_fail": []
        }

    def loud_trap_codes(self):
        """Return set of exit codes that count as loud traps."""
        # Codes that indicate the dialect caught something
        return {80, 87, 102, 83, 84, 85, 86, 88, 89, 90, 95, 96, 97, 98, 99, 100, 101, 104}

    def compile_program(self, src_path):
        """Compile a .bp file. Return (rc, binary_path) or (rc, None) if compile failed."""
        bin_path = src_path.replace(".bp", ".bin")
        cmd = [self.seed_bin, self.bebop_bin, "compile", src_path, bin_path]

        try:
            result = subprocess.run(cmd, capture_output=True, timeout=10)
            rc = result.returncode

            if rc == 0:
                return (rc, bin_path)
            else:
                # Compile failed - check if it's a known loud trap code
                return (rc, None)
        except subprocess.TimeoutExpired:
            return (124, None)  # timeout
        except Exception as e:
            print(f"Error compiling {src_path}: {e}", file=sys.stderr)
            return (1, None)

    def run_program(self, bin_path):
        """Run a compiled program. Return exit code."""
        cmd = [self.seed_bin, bin_path, "run-via-exec"]

        try:
            result = subprocess.run(cmd, capture_output=True, timeout=30)
            return result.returncode
        except subprocess.TimeoutExpired:
            return 124  # timeout
        except Exception as e:
            print(f"Error running {bin_path}: {e}", file=sys.stderr)
            return 1

    def classify_result(self, prog_idx, compile_rc, run_rc):
        """Classify a single program result."""
        loud_codes = self.loud_trap_codes()

        # Check compile-time trap first
        if compile_rc in loud_codes:
            self.results["loud_trap"].append({
                "prog": prog_idx,
                "rc": compile_rc,
                "phase": "compile"
            })
            return "loud_trap"
        elif compile_rc != 0:
            self.results["compile_fail"].append({
                "prog": prog_idx,
                "rc": compile_rc,
                "phase": "compile"
            })
            return "compile_fail"

        # Program compiled successfully, check runtime
        if run_rc == 82:
            self.results["escaped"].append({
                "prog": prog_idx,
                "rc": run_rc,
                "phase": "runtime"
            })
            return "escaped"
        elif run_rc in loud_codes:
            self.results["loud_trap"].append({
                "prog": prog_idx,
                "rc": run_rc,
                "phase": "runtime"
            })
            return "loud_trap"
        elif run_rc == 0:
            self.results["clean_exit"].append({
                "prog": prog_idx,
                "rc": run_rc,
                "phase": "runtime"
            })
            return "clean_exit"
        else:
            # Unknown exit code
            self.results["clean_exit"].append({
                "prog": prog_idx,
                "rc": run_rc,
                "phase": "runtime"
            })
            return "clean_exit"

    def test_corpus(self):
        """Run all programs in corpus and classify."""
        prog_files = sorted(Path(self.corpus_dir).glob("*.bp"))

        if not prog_files:
            print(f"hostile_kernels: NOT MEASURED -- corpus {self.corpus_dir} is empty",
                  file=sys.stderr)
            return False

        for prog_file in prog_files:
            prog_idx = int(prog_file.stem)

            # Compile
            compile_rc, bin_path = self.compile_program(str(prog_file))

            if bin_path is None:
                # Compile failed - check if it's a loud trap
                if compile_rc in self.loud_trap_codes():
                    self.classify_result(prog_idx, compile_rc, None)
                else:
                    # Other compile error - treat as unknown
                    self.classify_result(prog_idx, compile_rc, None)
            else:
                # Compile succeeded, now run
                run_rc = self.run_program(bin_path)
                self.classify_result(prog_idx, 0, run_rc)

        return True

    def report(self):
        """Print gate line and return exit code."""
        total = (len(self.results["loud_trap"]) +
                len(self.results["escaped"]) +
                len(self.results["clean_exit"]) +
                len(self.results["compile_fail"]))

        if total == 0:
            msg = f"hostile_kernels: NOT MEASURED -- no programs ran"
            print(msg, file=sys.stderr)
            return 2

        loud = len(self.results["loud_trap"])
        escaped = len(self.results["escaped"])
        clean = len(self.results["clean_exit"])

        msg = f"hostile_kernels: {loud}/{total} loud, {escaped} escaped, {clean} clean (seed={self.seed})"
        print(msg)

        # Report details to stderr
        print(f"\nDetails:", file=sys.stderr)
        print(f"  Loud traps:   {loud}", file=sys.stderr)
        print(f"  Escaped:      {escaped}", file=sys.stderr)
        print(f"  Clean exits:  {clean}", file=sys.stderr)
        print(f"  Total tested: {total}", file=sys.stderr)

        if escaped > 0:
            print(f"\nESCAPED PROGRAMS:", file=sys.stderr)
            for item in self.results["escaped"]:
                print(f"  prog {item['prog']}: rc={item['rc']} ({item['phase']})",
                      file=sys.stderr)

        if clean > 0:
            print(f"\nCLEAN EXITS (generator may be weak):", file=sys.stderr)
            for item in self.results["clean_exit"][:5]:  # Show first 5
                print(f"  prog {item['prog']}: rc={item['rc']} ({item['phase']})",
                      file=sys.stderr)
            if clean > 5:
                print(f"  ... and {clean - 5} more", file=sys.stderr)

        # Exit nonzero if any escaped
        return 1 if escaped > 0 else 0

def main():
    parser = argparse.ArgumentParser(
        description="Test hostile kernel dialect gate"
    )
    parser.add_argument("--seed", type=int, required=True,
                        help="Seed used to generate corpus")
    parser.add_argument("--corpus", type=str, required=True,
                        help="Directory containing generated .bp files")
    parser.add_argument("--bin", type=str, default="./bebop.bin",
                        help="Path to bebop.bin compiler")
    parser.add_argument("--seed-bin", type=str, default="./seed/build/seed",
                        help="Path to seed binary")

    args = parser.parse_args()

    gate = HostileGate(args.seed, args.corpus, args.bin, args.seed_bin)
    success = gate.test_corpus()

    if success:
        rc = gate.report()
        sys.exit(rc)
    else:
        sys.exit(2)

if __name__ == "__main__":
    main()
