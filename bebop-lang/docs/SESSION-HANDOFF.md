# SESSION HANDOFF — 2026-09-04 (session 8; resume in ONE read)

Status: 2026-09-05 CURRENT (rewritten at every session close; task bodies now live in HISTORY.md, the ledger in TASKS.md)

Repo: /root/dowiz/bebop-lang (git@github.com:SyniakSviatoslav/dowiz.git, branch main)
HEAD: see `git log --oneline | head -3`; every commit message carries the full gate evidence.

## Where we are
- bebop.bin fixpoint 104b6291 (T96 step 1) — when codegen changes the
  fixpoint test is THREE generations (gen3 == gen4; gen2 != gen3 by construction).
- Battery green: std_golden 91/91, construct 31/31 (re-frozen with word deltas),
  parity 9/9+1skip, run_all ok=91, pool_parity 5/5 (real threads, no ptrace
  skip), invariants GREEN, fuzz 150/150 (seeds 6000+).
- Closed today: T42 (all but (b)), T45, T42(a) C precedence, T96 step 1, T55
  SPIKE, T97/T63 pinned report, T100 sqlite gate, decision D8, two analyst
  reports (docs/SPEEDUP-ANALYSIS.md, docs/ROADMAP-AUDIT-2026-09-04.md).
- Measured (bench/vs_rust/REPORT-pinned.md, bench/tq_sqlite/RESULT.md): K1 3.0 ms
  (Rust twin 2.41), K4 13 ms (2.85); tensor query 4.0 us vs sqlite C-API 55 us.

## Open decisions for the operator
1. T42(b): `>>` = ASR changes 8 gate folds (rng hv spectral lsm holo seigtime
   srepl r3x) — `>>` stays LOGICAL until decided (tools/prec_switch.sh).
2. The analyst's delete/re-scope list (docs/SPEEDUP-ANALYSIS.md §5): T25/T26/T35
   bank ABI, T55 K1-K4 rung, T57/T58/T74, T60, T98 as k1/k2, T92-T95/T84/T85
   for speed, T15 "8-12x". Nothing re-scoped yet.

## Next (D6/D8 order, plan P1-P10 in docs/SPEEDUP-ANALYSIS.md §5)
- T96 step 2: pop2() peephole (push a; P; push b -> P with rd=1) in
  emit_binop_plain/emit_cmp_op + dead while-tail literal; patch script pattern
  lives in the session scratch (t96s2.py) — re-derive from the journal if lost.
- P2 temporaries in x1-x7, P3 cmp fusion, P4 small frames + register args (K2),
  P5 madd/shift peephole (K4), P7 sdiv/isqrt in tq/tdg, P8 T98 as nn4 sharding,
  P9 incremental-substrate curve, P10 .becache in cli_compile.
- T99 return/break after T43; T43 L8 lift; T47 `use`.

## Discipline (unchanged, see AGENTS.md L1-L17 + memory notes)
- Single writer for bebop.bp; every codegen step = own commit with fixpoint +
  battery + construct FREEZE=1 word deltas + census --freeze with the delta stated.
- Stream retractions must respect fntab[3660] (last label / rd=0 barrier, reset
  per fn); localise layout bugs with objdump diffs between two compiler variants.
- Harness scripts take BEBOP_BIN/BEBOP_TMP; invariants.sh still hardcodes
  ./bebop.bin for check_abi/census — promote first.
- fuzz.sh at J>=3 gives transient COMPILEFAIL rc=90 (seed open) — rerun standalone.
