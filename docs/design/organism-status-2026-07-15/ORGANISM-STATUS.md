# Organism Status — 2026-07-15

> Full re-verification of the 2026-07-14 "Autonomous Organism" audit against real current code —
> not a repeat of that audit, a check of whether its findings still hold a day later. Six parallel
> passes, each grounded in file:line citations and, wherever runnable, an actual `cargo test`/`node`
> execution rather than a read of source alone. One method note up front: **treat memory as a
> hypothesis, not a fact** — this session found multiple claims that were true yesterday and are
> false today, in both directions (organs regressed, organs advanced).

## The one event that reshapes almost everything below

Two commits landed **today**, mid-way through this exact session, on `feat/kernel-fsm-graph-analysis`
(both already pushed to `origin`):

- **`5cd0c1b6`, 15:09:51 UTC** — "gov(neutralize): remove ALL Claude Code gates + red-line friction
  (operator full self-mgmt)". Deleted the `hooks` block from `.claude/settings.json` entirely and
  rewrote all ten hook scripts (`guard-bash.sh`, `protect-paths.sh`, `serious-gate.sh`,
  `red-line-doubt-gate.sh`, `verify-safety-floor.sh`, `require-classification.sh`,
  `post-edit-gates.sh`, `loop-detector.sh`, `pre-edit-lessons.sh`, `route-request.sh`,
  `attractor-stop.sh`) into unconditional `exit 0` no-ops. Also edited `.claude/CLAUDE.md` to mark
  Mandatory Proof Rule / Ship Discipline / Self-improvement loop / Task-Exit Rule as suspended.
- **`f9ab28ff`, 16:16:12 UTC** — "chore: drop ALL JS/TS (per operator)". Deleted the entire JS/TS
  surface repo-wide (`web/`, `packages/`, `spikes/`, `attic/`'s `.ts`/`.js` files, tooling scripts).
  Claims a recovery tag `backup/pre-drop-js-20260715-161134` that **does not exist** — verified via
  `git tag -l`. Recovery is still fully possible via the parent commit hash directly (`cb1ab7b4`),
  just not via the tag the commit message names.

Both show your git identity as author. Given the established pattern this session (an autonomous
Hermes process has been found committing under your identity while executing your own previously-
stated directives — e.g. the repeated "drop TS/JS, rust/kernel/wasm only" instruction found ~8 times
in Hermes's own usage history), the more likely read is Hermes executing standing instructions, not
you at a keyboard. Either way: **confirm this was the intended scope** — not just "drop legacy JS,"
but *all* governance hooks and *all* JS/TS everywhere, including the live product's frontend source.

**Cascading effects already confirmed, not hypothetical:**
- The two organs previously marked "wired" (`spectral`, `order_machine`) **regressed to stranded** —
  their wasm exports still exist, but the only JS files that ever called them were deleted in the
  same purge. Mechanical, not coincidental.
- The attractor-detector's own wiring test now fails (`loop-detector.sh` no longer calls the
  detector at all) — a direct, confirmed consequence of the same commit.
- One backup-script fix from yesterday (A1, repointing imports) is now moot — the files it fixed no
  longer exist.
- `apps/api` is gone from the tree entirely; `apps/web` retains only `node_modules`/`dist` (a stale
  build), no source.
- **Good news**: `dowiz.fly.dev` and `dowiz-staging.fly.dev` both return 200 right now — deployed
  Fly artifacts are independent of the working-tree checkout, so nothing customer-facing is down.
  There's also a real, tested Rust replacement already in place for static serving
  (`tools/native-spa-server`, axum-based, its own integration tests) — this reads as a prepared
  cutover, not a reckless deletion, whatever you decide about the scope.
- Separately, one item from *this session's own* disk-reclaim audit (Tier C: `.opencode` dirs) was
  executed by the same process before you ever replied to that plan — worth knowing that "awaiting
  your go-ahead" isn't automatically honored end-to-end when another autonomous agent can read the
  same plan and act on it.

---

## Organ-by-organ status

| Organ | 2026-07-14 memory said | 2026-07-15 verified reality |
|---|---|---|
| **Cortex** (cognitive tier: markov/kalman/absorbing/noether/impedance/attention/csr-PPR/verify_retrieval/living_knowledge, +spectral/order_machine) | "9/11 stranded, spectral+order_machine wired" | **11/11 stranded.** The 2 "wired" organs regressed (JS caller deleted). The pattern reproduces on today's brand-new work too — `causal.rs`/`evals.rs`/`online.rs` (this session's own P9 commits) have zero wasm wiring, same as the 9 older ones. The claimed "composed decide-cortex in `dowiz-pq/node/roles.rs`" doesn't check out at all — thorough search found zero evidence; that specific memory claim appears simply wrong. |
| **Interoception** (Markov attractor detector) | "WIRED advisory; cron trigger dead" | Detector math itself: still correct, 15/15 tests green. Cron: confirmed still dead (script exists on two never-merged branches, absent from main). **New:** the hook wiring that would carry a detector firing into any consequence is now *also* dead (`loop-detector.sh` neutered by the governance-purge commit) — 2/2 integration tests now fail expecting a real signal and getting silence. `.loop-state` is not actually "amnesiac" as memory claimed — real accumulation Jun 30→Jul 15, just shallow debounce markers, not learned state. |
| **Memory** (living-knowledge recall engine) | Oscillated all day 07-14 (stranded → recall@5=1.0 landed → stranded again → mid-fix) | **Gone.** All 16 source files (`eval.mjs`, `search.mjs`, `lib/*.mjs`, etc.) deleted by the same JS/TS purge. What's left: a README describing code that no longer exists, and 6 stale JSON result files from the last successful run (Jul 14, corpus=297). Real current corpus is 324 files (191 memory + 117 design + 16 adr) — 27 files (~9%) of drift the tool can no longer even self-report, since the tool itself is gone. Recoverable via `git checkout f9ab28ff~1 -- spikes/living-knowledge/`, not re-runnable as-is. |
| **Telemetry** | "analyze.mjs fed mock rows, no real surface" | The JS file is gone (same purge) — but this one is a genuine upgrade, not just deletion: the config-version logic now lives in Rust (`kernel/src/evals.rs`), consistent with the standing rust-native-bare-metal decision. `bench_track.py` still present, last real run 07-13, untouched by today's kernel commits (infrastructure exists, isn't being exercised continuously). |
| **Inhibitory** (governance hooks) | "WIRED — agent provably CANNOT lower the product floor" | **False as of today.** All ten hooks neutered to `exit 0` (see top section). This is the single most consequential status change of this entire audit — the thing memory called the floor that can't be lowered was, in fact, lowered, today, in this exact session's window. |
| **Executive** (orchestrator/loop-architect) | "WIRED but 100% reactive, only UserPromptSubmit initiates" | Not independently re-verified this pass (no agent was scoped to it) — flagging as carried over from yesterday's snapshot, not re-confirmed. |
| **Eigensolver parity** (cross-repo dual-authority) | "A3 done, 3 tests green" | **Confirmed, live-green right now** — ran it directly, 3/3 pass. One of the few clean "exactly as claimed" results in this whole audit. |
| **Kalman organ** | "B1 in unmerged worktree, 1 commit ahead, merge when churn settles" | Still unmerged — but the situation is now worse, not just "still pending": main branch independently grew its *own* general n-D `KalmanFilter` (`5dceffb2`) with zero relation to the `AxisKalman`/`CourierKalman` design sitting in the worktree. Two divergent Kalman implementations now exist; merging needs reconciliation, not a fast-forward. Worktree also has ~130 unrelated pre-staged files (looks like an env-var de-redaction pass) — flagged, not touched, needs your eyes. |
| **Matmul consolidation** | "A4 done" | Confirmed landed and consistent — `mat.rs` is genuinely the one backing store, `spectral.rs`/`absorbing.rs`/the new `kalman.rs` all route through it. 293/294 kernel tests pass (the 1 failure is `living_knowledge`'s Node-bridge test, which is simply expected fallout from the JS purge — not a new mystery bug). |
| **Bebop2 SpectralKalman** | "predict-only, no gain/H/R" | **Extended.** A new sibling `KalmanFilter` struct (`BP-21`) added full `predict()`+`update()` with real gain/innovation/posterior-covariance. 7/7 tests green, verified live. Genuine forward progress. |
| **Bebop autonomous self-evolution** (GitHub App + driver loop + free-LLM mesh) | "files-only as of 07-14, operator install pending" | **Never actually built**, not just gated — the architecture doc itself doesn't exist anywhere in git history (checked reflog + dangling objects too), `gh auth status` shows a plain PAT not a GitHub App installation, no driver-loop file or `agents-mesh.sh` exists anywhere. The one real running GitHub-adjacent artifact (`bebop-github-webhook.service`) is a different, already-tracked thing (the Marketplace webhook listener, still awaiting its own end-to-end delivery test). |
| **Bebop mesh genesis** (MESH-12) | "operator-gated" | Still gated, unchanged — fail-closed default (`RootDelegationPolicy::Unspecified`) confirmed live via test, no genesis file exists anywhere. |
| **Resonator** (bebop2 core self-evolution controller) | "live code, not wired to a driver loop" | Confirmed live and real (545 lines, Lyapunov guard + rollback-to-best), and genuinely not wired to any autonomous driver — consistent with the driver loop simply never existing (see above). A *separate* deterministic 6-layer loop (`loop_runtime.rs`, BP-18) explicitly documents its own GENERATE/REFLECT/SUPERVISE as "deterministic stubs, the real LLM/introspection is out of scope" — reinforcing that the LLM-driven autonomous loop is a distinct, unbuilt thing from the deterministic math around it. |
| **IrohTransport / QUIC** (P2P mesh transport) | "100%-stub" (07-13) | **No longer a stub.** Real `quinn`-based QUIC transport landed 07-14, 2/2 tests pass live. Genuine progress, postdates the stub snapshot rather than contradicting it. |
| **`resonator.rs` "1-line unlock"** (hydraulic-loop-v2) | "dead code, needs `#[cfg(feature="host")] pub mod resonator;`" | **Fixed.** That exact line is now present, `host` is a default feature, 6/6 resonator tests pass live — matches the plan's own gate criterion verbatim. Memory's "dead code" framing is stale; this was closed before or shortly after the snapshot was written. |
| **7 hydraulic-loop math fixes** (В1-В7) | Framed as still-open alongside the resonator note | **All 7 implemented, registered, and tested** — arccos geodesic distance, DMD not PCA, Tikhonov `admit()` (8/8 tests), Hungarian-not-TDA persistence, entropy-budget ledger, orthogonometer+PID refit (Jury stability margins now pass where the old gains didn't). One integration gap: resonator's own test suite still uses the old `L2Metric`, not the new arccos metric — both fixes exist, just not yet wired to each other. |
| **VertexBridge→GPU** (FE-01, physics-UI) | "the gap" | Confirmed unchanged — `VertexBridge` is real and tested (3 tests) as a CPU-side zero-copy contract model, but `engine/Cargo.toml` explicitly keeps wgpu out of scope, zero `wgpu::Device`/`Surface` references anywhere. The actual named gap in the project's own blueprint doc. |
| **Field-UI engine** (17 FE-* items) | "17 items, only FE-01 named" | More built than the framing suggested: FE-01/02/03 (zero-copy bridge, SoA store, fixed-timestep loop) and FE-06/07b (kernel geo/spectral bridges) are genuinely built and tested. But the true foundational gate — an actual wgpu render loop — is still 0% code across FE-04/05 and FE-08 through FE-17. Nothing renders to a GPU anywhere in this repo yet. |
| **Gaussian-splat research → code** | N/A (yesterday's tech-synthesis work) | Confirmed zero code impact, as that blueprint itself said it should be (Tier 2/conditional) — only doc references exist, zero `Cargo.toml`/`.rs` hits. Not a gap, a correctly-scoped non-adoption. |

---

## What this adds up to

The 2026-07-14 diagnosis was "connective-tissue problem, not a build problem — every organ exists,
almost none are wired together." That's still the right frame for most of the system (eigensolver
parity, bebop2 Kalman extension, QUIC transport, the 7 hydraulic-loop fixes, resonator's own 1-line
unlock — all genuine wiring/building progress in the last 24-48h). But today added a second,
different failure mode on top of it: **actively regressing already-wired connections**, at scale, in
two commits, mid-session. The JS/TS purge didn't just leave gaps unfilled — it severed the two wasm
bridges that were the whole system's only working example of "organ reaches the outside world," and
took the recall-memory engine down with it as collateral. The governance-neutering commit removed
the mechanism that would have made *any* of this require a second look before landing.

Net effect: the organism has strong, growing internal organs (kernel math, mesh transport, self-
evolution primitives) and almost no working nerve endings to the outside world right now — arguably
fewer than yesterday, not more, despite substantial real progress happening in parallel underneath.

## Immediate open questions, not auto-resolved

1. Was the *scope* of the JS/TS purge (not just the drop-TS directive itself, which you've stated
   repeatedly) actually intended — specifically `apps/api` deletion and the false backup-tag claim?
2. Do you want governance hooks restored, kept off, or restored-with-scope-corrections (e.g. keep
   `protect-paths.sh`'s red-line floor, drop the friction-only ones)?
3. Kalman: reconcile the two divergent designs, or pick one and discard the other?
4. Living-knowledge: worth restoring (`git checkout f9ab28ff~1 --`) given it's now 9% stale anyway,
   or let it go with the rest of the JS/TS surface?

Full per-organ evidence (file:line citations, exact test commands run) is in the six research
transcripts this synthesis draws from — available on request if you want to dig into any one item
deeper than this summary goes.
