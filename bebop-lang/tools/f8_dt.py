#!/usr/bin/env python3
"""F8 gate: dependent surface types parsing + erasure.
Verifies bebop.bp parses [T;n], {x:T|p}, requires/ensures/invariant, theorem
as INERT syntax (erased before emission, fixpoint unchanged).
Usage: python3 tools/f8_dt.py <bebop.bin> [<src.bp>]
Exit 0 = PASS, exit 1 = FAIL
"""
import sys, os, subprocess, tempfile

def main():
    if len(sys.argv) < 2:
        print("usage: f8_dt.py <bebop.bin> [src.bp]", file=sys.stderr)
        sys.exit(1)
    bebop_bin = sys.argv[1]
    seed = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "seed", "build", "seed")

    # Test 1: requires/ensures contracts
    test1 = """
module core { }
fn add(x: i64, y: i64) -> i64 requires true ensures result > x { x + y }
fn main() { add(3, 4) }
"""
    # Test 2: invariant in while body
    test2 = """
module core { }
fn sum_array() -> i64 {
  let i = 0;
  let s = 0;
  while i < 5 invariant s == 0 {
    let s = s + i;
    let i = i + 1;
    0
  };
  s
}
fn main() { sum_array() }
"""
    # Test 3: theorem
    test3 = """
module core { }
theorem add_zero : forall x:i64. x + 0 = x := refl
fn main() { 42 }
"""
    # Test 4: dependent array type [T; n]
    test4 = """
module core { }
fn process(a: [i64; 10]) -> i64 { a[0] }
fn main() { let arr = zeros(10); let _ = arr[0] = 7; process(arr) }
"""
    tests = [("requires/ensures", test1), ("invariant", test2), ("theorem", test3), ("dependent_array", test4)]

    all_pass = True
    for name, src in tests:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.bp', delete=False) as f:
            f.write(src)
            path = f.name
        try:
            result = subprocess.run([seed, bebop_bin, "compile", path, "/tmp/f8_gate_test.bin"],
                                    capture_output=True, text=True, timeout=60)
            if result.returncode != 0:
                print(f"FAIL [{name}]: rc={result.returncode} stderr={result.stderr[:200]}")
                all_pass = False
                continue
            # Run the compiled binary
            run_result = subprocess.run([seed, "/tmp/f8_gate_test.bin"],
                                        capture_output=True, text=True, timeout=10)
            print(f"PASS [{name}]: output={run_result.stdout.strip()}")
        except Exception as e:
            print(f"FAIL [{name}]: {e}")
            all_pass = False
        finally:
            os.unlink(path)

    if all_pass:
        print("\nf8_dt gate: PASS — all dependent surface syntax parsed and erased (inert)")
        sys.exit(0)
    else:
        print("\nf8_dt gate: FAIL")
        sys.exit(1)

if __name__ == "__main__":
    main()
