// selfhost/readme.md — Status: 2026-09-04 CURRENT
// bebop.bp = the self-hosting compiler + CLI (compile/size/version/self_check),
//   built by itself to a byte-exact fixpoint: ./seed/build/seed bebop.bin compile bebop.bp gen2.bin
// std/*.bp = gate sources (each `fn main() -> i64` returns one fold; twin in bench/vs_rust/std_tests/,
//   oracle in bench/oracles/<name>.py, gate line in bench/vs_rust/std_golden.sh — law L17).
// expr_compile.bp RETIRED 2026-09-04 (ROADMAP T45): moved to selfhost/attic/; bebop.bp owns every builtin incl. sys_clone/futex/atomic_add.
// Surface, laws, task stack: ../ROADMAP.md, ../AGENTS.md, ../docs/SESSION-HANDOFF.md.
