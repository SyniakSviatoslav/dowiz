#!/usr/bin/env python3
"""F8 gate: dependent surface types parsing + erasure.
Tests whether bebop.bp REFUSES invalid syntax in dependent-type positions.
- Positive tests: valid forms that compile and run (discarded unread by compiler today)
- Negative tests: garbage in requires/ensures and theorem positions (should be rejected)
Truth: compiler discards unrecognized syntax without diagnostic (a loud-failure violation).
Ratchet: track count of garbage-accepting positions; FAIL if count rises (regression).
Usage: python3 tools/f8_dt.py <bebop.bin> [<src.bp>]
Exit 0 = PASS (ratchet holds), exit 1 = FAIL
"""
import sys, os, subprocess, tempfile

def main():
    if len(sys.argv) < 2:
        print("usage: f8_dt.py <bebop.bin> [src.bp]", file=sys.stderr)
        sys.exit(1)
    bebop_bin = sys.argv[1]
    seed = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "seed", "build", "seed")

    # Ratchet: count of syntactic positions where garbage is ACCEPTED (should be 0).
    # Compiler accepts without error: requires/ensures (positions 1-2) and theorem (positions 1-2).
    # Recorded 2026-09-13 from the ROADMAP F8 defect: bebop.bin silently discards garbage.
    RATCHET_VALUE = 4  # four garbage probes accepted (should be rejected per bebop.bp's role as parser)

    # Positive tests: valid forms that COMPILE and RUN (discarded unread, not parsed)
    test1 = """
module core { }
fn add(x: i64, y: i64) -> i64 requires true ensures result > x { x + y }
fn main() { add(3, 4) }
"""
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
    test3 = """
module core { }
theorem add_zero : forall x:i64. x + 0 = x := refl
fn main() { 42 }
"""
    test4 = """
module core { }
fn process(a: [i64; 10]) -> i64 { a[0] }
fn main() { let arr = zeros(10); let _ = arr[0] = 7; process(arr) }
"""
    positive_tests = [("requires/ensures", test1), ("invariant", test2), ("theorem", test3), ("dependent_array", test4)]

    # Negative tests: GARBAGE syntax in dependent-type positions.
    # These SHOULD be rejected but are currently ACCEPTED (compiler discards them unread).
    # Forms: requires/ensures position and theorem position, one main + one backup per.
    neg1 = """
module core { }
fn add(x: i64, y: i64) -> i64 requires @@@ %%% not_a_thing ensures 1 2 3 ][ { x + y }
fn main() { add(3, 4) }
"""
    neg2 = """
module core { }
fn mul(x: i64, y: i64) -> i64 requires garbage ensures JUNK @#$% !@# { x * y }
fn main() { mul(2, 3) }
"""
    neg3 = """
module core { }
theorem this_is_utter_nonsense_and_should_not_parse ][ @@@
fn main() { 42 }
"""
    neg4 = """
module core { }
theorem invalid_garbage with bad syntax @@@ { this will not parse }
fn main() { 99 }
"""
    negative_tests = [("requires/ensures garbage 1", neg1), ("requires/ensures garbage 2", neg2),
                      ("theorem garbage 1", neg3), ("theorem garbage 2", neg4)]

    positive_pass = 0
    negative_accepted = 0  # Count of garbage-accepting probes that compile without error

    # Run positive tests (they should compile and run)
    for name, src in positive_tests:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.bp', delete=False) as f:
            f.write(src)
            path = f.name
        try:
            result = subprocess.run([seed, bebop_bin, "compile", path, "/tmp/f8_gate_test.bin"],
                                    capture_output=True, text=True, timeout=60)
            if result.returncode != 0:
                print(f"DISCARDED (compile failed) [{name}]: rc={result.returncode}")
                continue
            # Run the compiled binary
            run_result = subprocess.run([seed, "/tmp/f8_gate_test.bin"],
                                        capture_output=True, text=True, timeout=10)
            print(f"DISCARDED (not parsed, but runs) [{name}]: output={run_result.stdout.strip()}")
            positive_pass += 1
        except Exception as e:
            print(f"DISCARDED (error) [{name}]: {e}")
        finally:
            os.unlink(path)

    # Run negative tests (garbage should be REJECTED, but today it's ACCEPTED)
    for name, src in negative_tests:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.bp', delete=False) as f:
            f.write(src)
            path = f.name
        try:
            result = subprocess.run([seed, bebop_bin, "compile", path, "/tmp/f8_gate_test.bin"],
                                    capture_output=True, text=True, timeout=60)
            if result.returncode != 0:
                # GOOD: compiler rejected this garbage
                print(f"REJECTED [{name}]: compile exit {result.returncode}")
            else:
                # BAD: compiler accepted garbage
                print(f"ACCEPTED (discarded unread, SHOULD BE REJECTED) [{name}]: no diagnostic")
                negative_accepted += 1
        except Exception as e:
            print(f"ERROR [{name}]: {e}")
        finally:
            os.unlink(path)

    print()
    # Report honestly: 0 forms genuinely parsed, N forms accept garbage
    print(f"f8_dt: {positive_pass} discarded (not parsed), {negative_accepted} garbage-accepting positions (ratchet {RATCHET_VALUE})")

    # Ratchet check: FAIL if the garbage-accepting count rises above the ratchet.
    #
    # The `f8_dt gate:` line below is a CONTRACT with tools/battery.sh, which asserts
    # `line f8_dt.log '^f8_dt gate:' 'PASS'` (:62) -- drop the prefix or the word and the
    # battery reports MISSING and goes red. What changed on 2026-09-13 is the MEANING of
    # that PASS: it used to claim "all dependent surface syntax parsed and erased (inert)",
    # which was false -- bebop.bp has no code for requires/ensures/theorem at all, the
    # parser DISCARDS them, and the gate could not tell a parser from a shredder. PASS now
    # says only "the ratchet holds": the known-bad state has not got worse.
    if negative_accepted > RATCHET_VALUE:
        print(f"RATCHET BROKEN: garbage-accepting positions rose to {negative_accepted} (was {RATCHET_VALUE})")
        print(f"f8_dt gate: FAIL -- a new syntactic position accepts garbage with no diagnostic")
        sys.exit(1)
    elif negative_accepted < RATCHET_VALUE:
        print(f"RATCHET IMPROVED: garbage-accepting positions fell to {negative_accepted} (lower RATCHET_VALUE in tools/f8_dt.py)")
        print(f"f8_dt gate: FAIL -- improvement not recorded; lower RATCHET_VALUE to {negative_accepted}")
        sys.exit(1)
    else:
        print(f"ratchet holds at {RATCHET_VALUE}")
        print(f"f8_dt gate: PASS -- ratchet holds at {RATCHET_VALUE}. NOT an erasure claim: "
              f"{positive_pass} forms are DISCARDED unread (0 parsed) and {negative_accepted} "
              f"positions ACCEPT garbage a parser would reject")
        sys.exit(0)

if __name__ == "__main__":
    main()
