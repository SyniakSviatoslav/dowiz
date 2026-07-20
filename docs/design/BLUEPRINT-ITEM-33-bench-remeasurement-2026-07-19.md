# BLUEPRINT — Item 33: Bench Ground-Truth Re-Measurement (close the `_cur.json` partial-run gap)

- **Date:** 2026-07-19 · **Arc:** §H Deterministic AI Inference (items 33–44) · **Tier:** Tier-0-class
  (zero prerequisites) · **Status:** BLUEPRINT v1 — planning artifact, NO code, NO Cargo.toml edits.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* (safety and
  predictability over speed). For this item it manifests as: **an unreproducible number is not
  actionable** (the ground-truth-over-proxy rule) — re-measure before believing.
- **Sources read this session:** `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` §1.2 (the
  GROUNDED-names / UNVERIFIED-numbers verdict); `RAW-PROMPT-4-*.md` part 2 (the telemetry claims);
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §H item 33 (lines 495–510); the live
  `kernel/benches/baseline.json` (keys verified: `fold_transitions/5_hops` = 4.2674 ns:27,
  `empirical_identify/20k_samples` = 112560:4, `empirical_identify/end_to_end_20k` = 883950:5,
  `absorbing/fundamental_matrix_16` = 26655:2); `kernel/benches/_cur.json` — **absent in a clean
  worktree** (gitignored transient run-output; its absence IS the partial-run failure surface item
  33 must close); `CLAUDE.md` bench-regression section (`kernel/benches/bench_track.py`, criterion
  A/B verdict, per-crate `cd`).
- **Dependency gate:** ZERO prerequisites. **NOT gated on item 34.** Ready to start the instant the
  operator dispatches §H. Produces no code the rest of the arc depends on — pure measurement +
  a tooling gap-close.

---

## 1. Problem + non-goals

**Problem.** The raw-prompt telemetry report (`RAW-PROMPT-4` part 2) makes five specific perf
claims — every component *name* is real, **no number matches a committed artifact in its claimed
context** (synthesis §1.2, verified both repos, benches + docs + git + saved JSON):

| Raw-prompt claim | Grounding verdict (synthesis §1.2) |
|---|---|
| bebop wire/loop_cycle **+30%** | not found; real bus-fix evidence is deadlock-elimination, `portkey/publish_fanout_8subs` ≈ 303–334 ns (no +30% throughput number recorded) |
| ML-DSA-65 verify **3.02× @ N=64** | not found; the only real "3.66×@8-threads" is the GCRA lock-free bucket — a different primitive |
| `fold_transitions/5_hops` **+16.6%** regression | not found; committed history shows it *improving* (5.59→4.27 ns) and the 07-18 audit calls this bench **NOISE-BOUND at ±40% CI** |
| `empirical_identify/20k_samples` **+14.3%** regression | not found |
| engine **"123 passed"** | REFUTED for engine (121/116/112/117 across docs, never 123); "123" is a real count for **`bebop-proto-cap`** (`WAVE-CLOSEOUT-P57-P74-2026-07-19.md:36`, P65) — a **cross-wired attribution** |
| **MISSING** `fundamental_matrix_16` | real tracker semantics: a partial run leaves keys out of `_cur.json`, and `native-trackers` scores MISSING as RED (`worst = threshold+1 → exit 1`) |

The verdict is **re-measure, not fix** (synthesis §1.2 verdict b): confirm or refute each claim
against committed baselines, and close the tooling hole that lets an *incomplete run* masquerade
as a *regression*.

**Non-goals.** (a) NOT "fix the +16.6% regression" — no regression is confirmed to exist yet.
(b) NOT a new bench harness — `bench_track.py` + criterion's A/B verdict already exist. (c) NOT a
separate-core or scheduler change — those are Q2-rejected (§4). (d) NOT bebop-repo work beyond
reading its committed bench artifacts for the reconciliation.

## 2. Back-of-envelope (why this is cheap and bounded)

`baseline.json` is the finite key set (≈ dozens of `<group>/<n>` ids across kernel + engine).
A full criterion pass of the tracked set is minutes-scale on one runner; `bench_track.py` already
runs criterion **twice on the same runner** (merge-base vs HEAD) and gates on criterion's own
statistical verdict — so "confirm/refute a regression" is a mechanical A/B, not a judgement call.
The only real work is (i) enumerating **all** baseline keys so zero are skipped, (ii) reconciling
the two perf branches (`perf/contention-bench-2026-07-18`, merged `8c865805b`; bebop
`perf/bus-contention-2026-07-18`), and (iii) closing the partial-run gap.

## 3. Implementation plan

1. **Enumerate the full tracked key set.** Parse every key in `kernel/benches/baseline.json` (and
   the engine + bebop committed baselines). This is the authoritative "must-run" list.
2. **Run the full set, both repos, per-crate.** `cd kernel && cargo bench` / `cd engine && cargo
   bench` (never `cargo -p` — the no-workspace false-green trap). Emit a `_cur.json` that contains
   **every** baseline key — zero omissions.
3. **Per-bench delta table vs `baseline.json`.** For each key: baseline mean, measured mean,
   delta%, and criterion's own A/B verdict (improved / no-change / regressed) with its CI. A delta
   inside criterion's noise band (e.g. `fold_transitions` at ±40% CI) is recorded as **NOISE**, not
   a regression — this is the falsifiable discriminator.
4. **Adjudicate each raw-prompt number: CONFIRMED or REFUTED.** Each of the six rows in §1 gets an
   explicit verdict with **the reproducing command** (CONFIRMED) or the searched-and-absent record
   (REFUTED). The "123 passed" row is resolved by naming the cross-wired suite (`bebop-proto-cap`).
5. **Close the `_cur.json` partial-run gap (the real deliverable).** Today an incomplete run
   silently drops keys → MISSING → RED, indistinguishable from a deleted bench. Add a
   **key-set-completeness assertion** to the tracker path: before scoring, assert `keys(_cur.json) ⊇
   keys(baseline.json)`; a missing key is reported as **`INCOMPLETE-RUN(<key>)`** (operator error,
   re-run) distinctly from **`DELETED-BENCH(<key>)`** (a real removal, RED). MISSING can then only
   mean a genuinely removed bench.
6. **File follow-up tickets for any *confirmed* regression only.** Fix shape is **static data
   layout first** (Q2 resolution): item 3's `order_machine` const-adjacency (`[u16; 12]` compile-
   time bitmask, roadmap line 49) is the named fix shape for a real `fold_transitions` regression.
   **Separate-core stays rejected** — `core_pinning.rs` is a deliberate no-op seam (single-socket
   host ⇒ no locality; cross-core adds host-scheduler nondeterminism, the opposite of the ruling).

## 4. Required proofs (5-point hardening-checklist mapping)

Item 33 is **measurement + a tracker gap-close**, not a new algorithmic hot path, so it does not
register a new HOT-PATHS row. The checklist maps by analogy:

- **1 (oracle):** N/A — the "oracle" here is the committed `baseline.json` itself; criterion's A/B
  is the differential.
- **2 (dudect) / 4 (asm) / 5 (kani):** N/A.
- **3 (debug/differential):** the key-set-completeness assertion (step 5) is the differential —
  `_cur.json` keys checked against `baseline.json` keys every run.
- **P7 (re-execute, never presence-check):** load-bearing here — the results doc must carry the
  **reproducing command per number**, not an asserted figure. A number without a command is treated
  as UNVERIFIED, exactly as the raw-prompt numbers were.

## 5. Falsifiable acceptance criteria

1. A dated results doc exists with a **per-bench delta vs `baseline.json` for EVERY baseline key**
   (zero keys unaddressed).
2. Each of the six §1 raw-prompt claims is marked **CONFIRMED (with reproducing command)** or
   **REFUTED (searched-and-absent record)** — none left ambiguous.
3. A **full-key `_cur.json` run recorded with ZERO MISSING rows** (proves the run was complete).
4. The tracker now distinguishes `INCOMPLETE-RUN(k)` from `DELETED-BENCH(k)`: a deliberately
   truncated run (drop one key) reports `INCOMPLETE-RUN`, and a deliberately deleted bench reports
   `DELETED-BENCH` → RED. **RED→GREEN demonstration required** for both paths.
5. Any *confirmed* regression has a follow-up ticket filed with the static-data-layout-first fix
   shape (item 3 const-adjacency named); **zero** tickets filed for noise-band deltas.

## 6. Dependency gate + operator-decision-needed

- **Gate:** zero prerequisites; NOT gated on item 34; can run fully in parallel with item 34's
  spec work.
- **Operator-decision-needed:** **none.** This is pure measurement + a deterministic tracker fix.
  The one judgement — "is a delta a regression or noise?" — is delegated to criterion's statistical
  A/B verdict, not to opinion, so no operator ruling is required.
