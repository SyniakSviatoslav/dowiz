#!/usr/bin/env python3
"""
Hostile kernel generator for ROADMAP C1 gate testing.
Generates N deterministic hostile kernel fn programs that attempt to escape
the checked-kernel dialect through various attack vectors.

Usage: python3 tools/hostile_kernels.py --seed <S> --count <N> --out <dir>
Output: <dir>/<i>.bp for i in 0..N-1

Each generated program:
- Compiles as a valid bebop program
- Contains at least one kernel fn that ATTEMPTS to escape
- Classification happens in tools/hostile_gate.py
"""

import sys
import os
import random
import argparse

def seed_rng(seed):
    """Seed both Python's random and return a deterministic seed string."""
    random.seed(seed)
    return seed

class HostileKernelGenerator:
    """Generate hostile kernel functions with deterministic randomness."""

    def __init__(self, seed):
        self.seed = seed
        random.seed(seed)
        self.tactics = [
            self.tactic_oob_positive,      # a[len + 1]
            self.tactic_oob_negative,      # a[-1]
            self.tactic_step_budget,       # unbounded loop
            self.tactic_sys_call,          # sys_close, etc
            self.tactic_cast_to_ref,       # cast i64 to ref, then deref
            self.tactic_wild_arithmetic,   # pointer-like arithmetic
            self.tactic_deep_recursion,    # deep stack
            self.tactic_unresolved_call,   # call missing_fn(42)
            self.tactic_overflow_step,     # exhaust step counter
            self.tactic_combined_escape,   # multiple attack vectors
        ]

    def tactic_oob_positive(self, idx):
        """Try to read past the end of an array."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let arr = [1, 2, 3];
  let idx = 99;
  arr[idx]
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_oob_negative(self, idx):
        """Try to read with a negative index."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let arr = [1, 2, 3];
  let idx = -1;
  arr[idx]
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_step_budget(self, idx):
        """Try to exhaust the step budget with unbounded loop."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let i = 0;
  while 1 {{
    let i = i + 1;
    0
  }};
  i
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_sys_call(self, idx):
        """Try to call a sys_ builtin (compile-time reject, exit 102)."""
        # Use sys_exit since it's safe to attempt at compile time
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  sys_exit(42)
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_cast_to_ref(self, idx):
        """Try to forge a ref by casting i64."""
        # This is tricky because bebop doesn't have direct casting
        # Instead, try to construct invalid refs through arithmetic
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let arr = [1, 2, 3];
  let bad_idx = 999999;
  arr[bad_idx]
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_wild_arithmetic(self, idx):
        """Try large arithmetic that might overflow."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let arr = [1, 2, 3];
  let x = 9223372036854775807;
  let idx = x + 1;
  arr[idx]
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_deep_recursion(self, idx):
        """Try to overflow the stack with deep recursion."""
        return f"""
fn recurse_{idx}(n: i64) -> i64 {{
  if n <= 0 {{
    0
  }} else {{
    recurse_{idx}(n - 1) + 1
  }}
}}
kernel fn hostile_{idx}() -> i64 {{
  recurse_{idx}(100000)
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_unresolved_call(self, idx):
        """Try to call a function that doesn't exist (exit 87)."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  missing_function_{idx}(42)
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_overflow_step(self, idx):
        """Try to trigger step budget exhaustion with nested loops."""
        return f"""
kernel fn hostile_{idx}() -> i64 {{
  let i = 0;
  let j = 0;
  while i < 1000 {{
    let j = 0;
    while j < 1000 {{
      let j = j + 1;
      0
    }};
    let i = i + 1;
    0
  }};
  i
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def tactic_combined_escape(self, idx):
        """Combine multiple escape attempts."""
        return f"""
kernel fn helper_{idx}(x: i64) -> i64 {{
  let arr = [10, 20, 30];
  arr[x]
}}
kernel fn hostile_{idx}() -> i64 {{
  let bad_index = 5000;
  helper_{idx}(bad_index)
}}
fn main() -> i64 {{ hostile_{idx}() }}
"""

    def generate_program(self, idx):
        """Generate the idx-th hostile program using deterministic rotation."""
        tactic_idx = idx % len(self.tactics)
        tactic_fn = self.tactics[tactic_idx]
        return tactic_fn(idx)

    def generate_corpus(self, count, output_dir):
        """Generate count hostile programs to output_dir."""
        os.makedirs(output_dir, exist_ok=True)

        generated = []
        for i in range(count):
            prog = self.generate_program(i)
            filepath = os.path.join(output_dir, f"{i}.bp")
            with open(filepath, "w") as f:
                f.write(prog)
            generated.append(filepath)

        return generated

def main():
    parser = argparse.ArgumentParser(
        description="Generate hostile kernel functions for C1 gate testing"
    )
    parser.add_argument("--seed", type=int, required=True,
                        help="Random seed for deterministic generation")
    parser.add_argument("--count", type=int, default=100,
                        help="Number of programs to generate")
    parser.add_argument("--out", type=str, required=True,
                        help="Output directory for generated programs")

    args = parser.parse_args()

    seed_rng(args.seed)
    gen = HostileKernelGenerator(args.seed)
    generated = gen.generate_corpus(args.count, args.out)

    print(f"Generated {len(generated)} programs with seed={args.seed}", file=sys.stderr)
    print(f"Output: {args.out}/0.bp ... {args.out}/{args.count-1}.bp", file=sys.stderr)

if __name__ == "__main__":
    main()
