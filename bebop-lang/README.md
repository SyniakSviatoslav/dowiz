Status: 2026-09-05 CURRENT (T119)

# bebop-lang

A self-hosting, integer-only language for AArch64 with no C in the toolchain: a 1.5 KB
assembly loader (`seed/seed.S`) runs `bebop.bin`, which compiles `.bp` source to raw
machine words; `bebop.bin` is itself compiled from `bebop.bp` and reaches a byte-exact
fixpoint. The project's goal and its measured state live in `ROADMAP.md` (the single
source of truth), the laws of work in `AGENTS.md`.

- Language: `docs/LANGUAGE.md`. Exit codes: `docs/TRAPS.md`.
- Try it: `./seed/build/seed ./bebop.bin compile bench/vs_rust/kernels/k1.bp /tmp/k1.bin && ./seed/build/seed /tmp/k1.bin` prints `500000500000`.
- Reference semantics: `python3 tools/bpref.py file.bp` (the oracle the fuzzer compares against).
- Gates: `bench/vs_rust/std_golden.sh` (91 std gates with python oracles in `bench/oracles/`),
  `bench/vs_rust/construct_parity.sh` (frozen word streams + values), `bench/vs_rust/invariants.sh`
  (ABI zones, fntab map, branch census), `bench/fuzz/fuzz.sh N START` (generator vs oracle),
  `bench/vs_rust/pool_parity.sh` (threads), `bench/vs_rust/bench_pinned.sh` (K1-K4 vs Rust, pinned),
  `bench/tq_sqlite/run.sh` (tensor query vs sqlite), `bench/substrate_spike/run.sh`.
- Rebuilding the compiler: `./seed/build/seed ./bebop.bin compile bebop.bp gen2.bin`, then
  `gen2.bin compile bebop.bp gen3.bin`, then `gen3.bin compile bebop.bp gen4.bin`; gen3 == gen4
  is the fixpoint (three generations once codegen changes). Every compiler change is one commit
  with that fixpoint and the whole battery (AGENTS.md L1-L17, ROADMAP decisions D8-D11).

Layout: `bebop.bp` compiler · `seed/` loader · `selfhost/std/*.bp` gated modules (+ `prelude/`,
`attic/`) · `bench/` gates, oracles, fuzzer, benchmarks · `tools/` bpref, census, check_abi,
mutate_gate · `docs/` analyses, handoff, journal (`docs/exp.journal`, one line per experiment).
