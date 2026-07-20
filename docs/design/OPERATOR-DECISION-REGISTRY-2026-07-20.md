# Operator Decision Registry — consolidated outstanding rulings (2026-07-20)

Compiled from the authoritative docs so the operator can ratify in one pass. Every entry cites
its source. "Default" = the doc's recommended/recorded default if the operator does not override.
"RED-LINE" = touches money / auth / RLS / migration surfaces — per the autonomy gate, even after
a high-level ruling, each concrete change still needs per-change confirmation.

NOT re-asked (already RULED in `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md §9`):
O1 hardware posture→certify tested-with list · O2 SMPL→drop · O3 web/ JS→vendor `<model-viewer>`
static asset · O4 asset-format→invest conversion · O5 streaming→superseded by concurrency-arch ·
O6 proactive-alert allowlist→mirror classical · O7 courier liveness→not now · O8 voice-data→local-first.

================================================================================
TIER A — LAUNCH DECISIONS (C) — gate the first real order (M1). Source:
  DOWIZ-STRATEGIC-REGRET-MINIMIZATION-SYNTHESIS-2026-07-20.md §3 + item 5.
================================================================================
C1 [RED-LINE: money+auth] Ordering-surface channel.
    Q: web storefront (0% code since JS drop, ~months) vs Telegram-first ordering
       (messenger.rs already builds deep-links, ~weeks)?
    Blocks: M1 first real order (surface a). Default: evaluate Telegram-first (shortens path).

C2 [RED-LINE: auth/capability] Production admission roster.
    Q: server boots with EMPTY capability roster and rejects everything by design — decide
       enrollment tooling + trusted-anchor roster as DEPLOY CONFIG (not compile-time).
    Blocks: M1 (order admitted). Default: move roster+provider+currency to deploy config + enroll tool.

C3 [RED-LINE: persistence/migration] Durability spine.
    Q: persistence is `Mutex<HashMap>` — restart erases every order. Decide backend + format
       versioning + replication reservation as ONE design.
    Blocks: M1 (order survives). Default: wired persistence + versioned format + off-node snapshot reserved.

C4 [RED-LINE: money] Payment rail.
    Q: only `NoOpPaymentAdapter` exists; real adapter crate absent. Real provider (vendor/
       geography/fee model) vs cash-on-delivery for the pilot?
    Blocks: M1 (order paid). Default: cash-on-delivery for pilot OR name provider.

C5 [RED-LINE: distribution] Courier delivery client.
    Q: courier crate has no `[[bin]]`; nothing installable on a phone. Decide delivery-client
       distribution for the pilot.
    Blocks: M1 (order delivered). Default: ship a minimal installable courier bin.

================================================================================
TIER B — P75–P96 WAVE (OD-1..OD-15). Source: MASTER-STATUS-LEDGER-2026-07-19.md §5.
================================================================================
OD-1  GCRA lock-free swap on `token_bucket` (3.6×@8t benched; security primitive; low real contention).
      Blocks: P90. Default: NOT shipped; Mutex+clock-hoist stands.
OD-2  Push/merge `perf/contention-bench-2026-07-18` to remote/main.
      Blocks: P90, P80 (contended benches). Default: stays local (push-after-milestone precedent).
OD-3  [RED-LINE: crypto] Resolve bebop C3 ungated-keygen red state (or explicit `--no-verify`
      ruling for the bus patch). Blocks: ENTIRE bebop lane (commit freeze), P85, P90.
      Default: bus patch stays a file; branch commit-frozen.
OD-4  Push `a857cd71a` (slot_arena) + whole unpushed local main line above `4b30c9b4c` (P57–P74 wave).
      Blocks: I2; repo safety. Default: stays local.
OD-5  Execute P91.0 (comment-only false-FIPS-203 claim removal in `kem.rs` header) ahead of P91.1.
      Blocks: P91. Default: header keeps falsely claiming FIPS-203 (trap stays armed).
OD-6  P85 closure path: real 3-model review vs recorded retroactive sign-off.
      Blocks: D-9 NTT wire-in, P91.1. Default: quarantine holds (no wire-in, no Montgomery).
OD-7  [RED-LINE: money/FSM] D-1 golden state-digest regression gate (would become P84).
      Blocks: P84. Default: not proposed.
OD-8  [RED-LINE: scoring] D-2 `reputation.rs` — delete or event-source (courier-scoring divergence).
      Blocks: P76 scope note. Default: undecided; fix trivial once ruled.
OD-9  D-3/D-9 `pq_kem` NTT wire-in (triple-gated: P82 bench evidence AND P85 complete AND sign-off).
      Blocks: D-9. Default: not wired.
OD-10 D-4 PPR determinism relaxation — standing default REJECTED; record so never adopted silently.
      Blocks: nothing. Default: rejection stands.
OD-11 GPU field-state decision (operator-owned, P38 §4.2).
      Blocks: P86, P87 (build), all W5 GPU work. Default: nothing starts.
OD-12 D-93-A privacy fork: plaintext `ReceiverID` vs blinded recipient tag.
      Blocks: P93. Default: blueprint records BOTH; no default taken.
OD-13 D-93-C broadcast/multicast: per-recipient signed copies vs wildcard-sentinel defer.
      Blocks: P93. Default: blueprint must resolve or defer-with-reason.
OD-14 P92 proceed ruling after D-BENCH measure-first gate (+ arrange mandatory independent
      adversarial review for M1 and the fast-path).
      Blocks: P92, M1. Default: NO-GO if the bench doesn't clear; review gate is DoD-blocking.
OD-15 Restore + commit the 11 recovered `docs/research/` files from the scratchpad `recovered/` dir.
      Blocks: source integrity of the whole ledger. Default: files remain only in scratchpad + transcripts.

================================================================================
TIER C — SOVEREIGN-ARCHITECTURE O-SERIES (gating subset). Source:
  MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md §3. (O2/O6/O10–O17 are mechanical with
  safe defaults in BLUEPRINT-P02 — accept/override only.)
================================================================================
O1   D5/D8 define (candidates in root DECISIONS.md on a colliding numbering) or renumber whole D-series.
     Blocks: every "147"/"146" anchor-count claim across all docs. Default: BLUEPRINT-P02 diff.
O3   F44 dispute/escrow mechanism (only spec contradicts M12 NO-COURIER-SCORING + M6 zero-dep).
     Blocks: Phase 14. Default: operator-gated arbiter capability OR staked Schelling voting.
O4   F48 merge semantics: content-address-only vs CRDT for per-hub graph sync.
     Blocks: Phase 14. Default: CRDT fenced out of money/order; open for knowledge-wiki.
O5   D2/iroh — land the crate for real, or amend canon to "quinn primary + named unlock trigger".
     Blocks: Phase 9. Default: canon currently claims iroh exists; it does not.
O7   E1/F41 "hub-ring" — ratify consistent-hash reading (literal star-hub contradicts M7 no-SPOF).
     Blocks: Phase 13. Default: two words, no formal spec until ratified.
O9   V1-B verifier isolation bar: fresh worktree vs separate machine vs different model family.
     Blocks: Phase 6. Default: one sentence of canon closes it.
O19  I-FINAL proof home: bebop consensus path vs dowiz `tools/eqc` (cited file doesn't exist at either).
     Blocks: Phase 13 (F46 closure). Default: exists at a third, legacy path.
O18a graphics-unlock (network `cargo add wgpu` succeeds) — external/environment-gated (RED/403 as of 2026-07-16).
     Blocks: P17, P15 E13-gpu. Default: stays external-gated.
O18b model-weights-unlock (llama.cpp CPU tier — GGUF fetch + local server) — GREEN-ish on this host;
     needs only a DECART report + operator go, NOT an external trigger.
     Blocks: P15 E13-cpu. Default: highest-leverage UNBLOCKED item — actionable now.
O8   F10 max sub-hub recursion depth — numeric value only. Default: any safe constant.

================================================================================
TIER D — PARKED RED-LINE (item 5 of regret-minimization synthesis). Sat 10–18 days.
================================================================================
D-money  money-leg settlement scope — parked, red-line. Decide or defer-with-date.
D-fuel    Wasmtime fuel policy — parked. Decide or defer-with-date.
D-batch   gated 53× event-log batching (I) — parked. Decide or defer-with-date.

================================================================================
PRODUCT-SURFACE OPEN DIMENSIONS (the 3 of 8 not yet covered)
================================================================================
Per operator: 5 of 8 product-surface dimensions covered = design (P38b + ARCHITECTURE),
checkout (P69+P72), auth (item 23/P37/P49), voice control (P64), local agent (item 21 →
P40+P41). The 3 OPEN ones need their own ruling/blueprint before build. Likely candidates
from the 2026-07-20 synthesis themes: (i) spatial AR storefront rendering, (ii) offline
resilience, (iii) concurrency architecture (or media/comms, or intent interface). Operator to
confirm which 3 and rule each.

================================================================================
HOW TO ANSWER
================================================================================
For each Tier A/B/C/D entry, reply with your ruling. Conventions:
- "accept default" (or "a") → I record the doc default and proceed.
- A specific choice → I record it.
- For RED-LINE items (C1–C5, OD-3, OD-7, OD-8): a high-level ruling is noted, but each concrete
  code change still gets per-change confirmation before commit (autonomy gate).
- For the 3 open product-surface dimensions: name them + rule each (blueprint-first, per discipline).

After ratification I will: write the rulings into DECISIONS.md + the owning blueprints, and where a
ruling unblocks a dowiz-side, ungated, buildable item, proceed to build+verify+commit on
`autopilot/roadmap-exec-2026-07-20` (never push without explicit command).
