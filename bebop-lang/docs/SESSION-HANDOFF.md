# SESSION HANDOFF — 2026-09-02 (resume in ONE read)

Repo: /root/dowiz/bebop-lang (git@github.com:SyniakSviatoslav/dowiz.git, branch main)
HEAD: 37fd6ea "docs: token economy & tiered routing protocol". Everything pushed.

## How to resume (cheap, in order)
1. `/tmp/opencode/ctx` — orient pack (git state, corpus hashes, 23-gate status).
2. Read the active SPEC card below and execute it (Flash tier) or extend it (Pro tier).
3. Never re-read whole files: use `tb h/s`, graphify, mempalace, cached embeds.

## Where we are
- bebop self-hosted compiler: fixpoint bebop.bin (dfaf06c3), 23 std_golden gates
  23/23. Gate list: checksum sort rng base64 sha256 crc32 hex hv spectral(2038)
  csr bt cache wht haar ntt rev store petri lsm holo(2766693490590679850)
  scoord(2010131) sgamma(3550431) tb(1111000).
- ROADMAP: N6, SS-15, SS-16 DONE. NEXT: SS-17 seigtime (SPEC below), then
  SS-18, then SS-1..14 in dependency order, N3 ring-VSA, N7, N8, SME/SVE2,
  final ROADMAP status. User directive: continue until roadmap finished.
- Journal: docs/exp.journal up to 1788288207. Next: 1788288208+.

## Bebop discipline (LAWS, non-negotiable)
- T0 expected-vs-got on every run; one-line journal H:DID:GOT:VERDICT.
- L8: no allocations inside while bodies. L11: entry identity for .bin runs.
- R3.x emitter/runtime defects (workarounds, see ROADMAP + journal):
  (a) `a*b<<c` fast-path — parenthesize (1788288190);
  (b) `>>` splits by operand origin: array-loaded→LSR, literal/local→ASR —
  &-mask for logical (1788288193); oracle mirrors must shift unsigned;
  (c) loop-shaped while+compare+conditional-store miscompile — unroll or
  branch-free multiply-select stores (1788288197);
  (d) str literals and ++ concat SEGFAULT in .bin runtime; argv strings fine
  — .bp programs must be str-free: argv + cells + arithmetic (1788288206).
- Fold discipline: each gate = ONE i64 fold == python mirror bit-exact;
  one run is proof once frozen; ×3 only at first freeze.
- std_golden gate recipe: embed machinery verbatim from
  /tmp/opencode/spectral_machinery.bp (cached slice, sha256 c184416666fe,
  regenerate from selfhost/std/spectral.bp if it drifts); ≤8 array binds/fn;
  gate main returns fold; cp selfhost/std/<g>.bp → bench/vs_rust/std_tests/;
  add gate() line to std_golden.sh; journal; ROADMAP; commit+push.
- Spectral topk: need UNIQUE |lambda| in the top-k block (bipartite graphs
  freeze power iteration at mixed fixed points — self-loops fix; equal-lambda
  pairs harmless, equal-|lambda| different-lambda fatal) (1788288204).

## Toolstack (always on)
- rtk (git/ls output compression); rtk plugin auto-activates after restart.
- tb = `tb` wrapper at ~/.local/bin/tb (PATH via ~/.profile; auto-recompiles
  from selfhost/std/tb.bp if missing/stale): tb h <path> (crc32), tb ctx j r g,
  tb s <needle> <path> (line numbers), tb c (stdin compressor), no args =
  self-test fold 1111000. Gate 23 in std_golden.
- graphify (uv tool): query/path/explain on graphify-out/graph.json;
  update . after code changes. mempalace: search/wake-up; re-mine journal
  after commits. snap (/tmp/opencode/snap): text→PNG for vision models only
  (current model cannot read images).
- Token economy: docs/TOKEN-ECONOMY.md (binding; loaded as global
  instruction after restart). Tier routing: Pro = reasoning/planning/
  debugging/synthesis; Flash (flash-exec agent) = execution of SPEC cards.

## ACTIVE SPEC — SS-17 seigtime (eigentime: time = spectral iteration)
Design (Pro-approved): eigentime(rp,ci,vv,n,x,ax,px,p2x): LCG seed
(-7046029254386353131, x[j]=frac+frac-2^32) → normalize_fp(x,n) → power
iteration spmv_fp+normalize, after each step detect cycle vs history px
(period 1) / p2x (period 2) → t = iterations to enter cycle → +16 absorb
iterations verifying membership (stable bit) → return t*100 + per*10 + ab
(per=1 if e1 else 2). Slow clock = C8+I (ring+selfloop, ring_dense); fast
clock = J8 (all-ones, ones_dense). Claims: e_slow > e_fast (time-scale
separation), ab=1 both (absorbing → energy-efficient state).
PHASE A (measure, no expectations): build seigtime.bp with embed
(fp_mul..csr_from_dense from spectral_machinery.bp), run, report raw
e_slow/e_fast (expected unknown; ratio 0.805 → e_slow likely 100-400).
PHASE B (Pro): python mirror of eigentime (ss17_oracle.py) → match BP
bit-exact; fix fold widths from measured values:
fold = e_slow*W1 + e_fast*W2 + ab_s*W3 + ab_f*W4 + order*10 + (per bits).
PHASE C: gate "seigtime" in std_golden.sh (gate 24), journal 1788288208+,
ROADMAP SS-17 DONE, commit+push.

## Scratch layout (/tmp/opencode)
spectral_machinery.bp (embed cache) | tb.bin + tb_test.bin | ctx | snap +
snapenv/ | ss15_oracle.py ss15_fold.py ss16_oracle.py ss17 (new) | sg_*/tp1
probes (stale, safe to delete).
