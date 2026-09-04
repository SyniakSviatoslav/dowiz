# Oracle for gate `ordfsm` (T66): thin wrapper -- the oracle is PRODUCTION Rust
# (crates/dowiz-core) driven by bench/oracles/rust/src/bin/ordfsm.rs; cargo builds
# it on first use. Prints the bin's last line; exits non-zero if cargo fails.
import pathlib, subprocess, sys
r = subprocess.run(["cargo", "run", "--release", "-q", "--bin", "ordfsm"],
                   cwd=pathlib.Path(__file__).resolve().parent / "rust", capture_output=True, text=True)
if r.returncode:
    sys.stderr.write(r.stderr); sys.exit(1)
print(r.stdout.strip().splitlines()[-1])
