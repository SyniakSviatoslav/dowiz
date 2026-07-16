# BLUEPRINT W19 — Kalman/trigram integration into decide/loop

## WHY
`kernel/src/kalman.rs` (456L, full n-D predict+update, T2-α) and `kernel/src/trigram.rs`
(138L, bigram+trigram, T2-β) exist but are STRANDED (ORGANISM-STATUS: "11/11 organs
stranded" — math present, not consumed). The decide/loop must actually use them: Kalman for
state estimation in the order/trust fold; trigram for recurring-pattern surfacing in the
self-improvement loop.

## WHAT (acceptance)
- `decide`/`fold` Law path calls `kalman::KalmanFilter::predict`+`update` for a courier/trust
  state estimate (extends `geo::ema_next` scalar steady-state — documented in kalman.rs header).
- Self-improvement loop calls `trigram::count` over its tool-outcome token stream and surfaces
  top-k recurring triples (deterministic ranking, lex tie-break).
- Both wired fail-closed: missing observation → Kalman holds prior; empty token stream → 0 trigrams.

## RED→GREEN
- RED: `decide`/`loop` never references `kalman`/`trigram` (grep: 0 usages in engine/telemetry).
- GREEN:
  (a) kernel test: a `decide` step with a noisy observation yields a Kalman-filtered estimate
      closer to truth than the raw observation (variance-reduction gate, mirrors bebop2 BP-21).
  (b) loop test: a known token sequence returns the expected top-1 trigram (deterministic).

## FILES (Owns — disjoint from W17/W18)
- Modify: `kernel/src/lib.rs` (wire modules into a `cortex` facade if absent),
  `kernel/src/decide.rs` or `domain.rs` (Kalman call), `telemetry/*.rs` (trigram call)
- Test: `kernel/src/tests.rs` (Kalman-in-decide) + `kernel/src/trigram.rs` existing 4 tests extended

## RISKS
- Don't re-implement geo/ema in the loop — consume kernel `kalman` (it generalises ema_next).
- Trigram must stay zero-dep (std HashMap only, per trigram.rs header).

## NON-GOALS
- No ML/N-gram model training. Deterministic counting only.
