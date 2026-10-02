# Roadmap open-items verification — live git ground truth (2026-07-20 autopilot pass)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


Branch: `autopilot/roadmap-exec-2026-07-20`. Method: every claim below is verified with
`git merge-base --is-ancestor <branch> main` + `git log --grep`, NOT taken from the ledger.

## A. Items the ledger claims MERGED → CONFIRMED REAL (all 10)
| Item | branch | branch exists | merged to main |
|---|---|---|---|
| P75 | feat/p75-ci-bench-gate | YES | MERGED (eed8908cb) |
| P77 | feat/p77-spool-spine | YES | MERGED (8514bd01b) |
| P79 | feat/p79-causal-spectral | YES | MERGED (d20fa3bd7) |
| P80 | feat/p80-kernel-benches | YES | MERGED (b12281189) |
| P81 | feat/p81-engine-benches | YES | MERGED (7627cd7df) |
| P83 | feat/p83-span-metrics | YES | MERGED (4b8749fee) |
| P88 | feat/p88-gpu-atomicity-policy | YES | MERGED (587e4c264) |
| P89 | feat/p89-field-eigenmodes | YES | MERGED (98cad8bf0) |
| P91 | feat/p91-kem-ring-fix | YES | MERGED (e4715a067) |
| P96 | feat/p96-eta-live-speed | YES | MERGED (b6172cb3c) |

## B. Genuinely OPEN — no dedicated branch, gated (verified: `git branch -a | grep` = none)
| Item | Gate | Why not buildable from dowiz now |
|---|---|---|
| P76 | bebop C3 freeze | bebop-side bus-lock/test-ungate; transport in /root/bebop-repo |
| P78 | bebop C3 freeze | bebop complexity fixes |
| P82 | bebop C3 freeze | bebop bench expansion |
| P85 | operator (OD-6) | NTT `--no-verify` remediation, quarantines D-9 wire-in |
| P86 | operator (OD-11) | GPU SlotArena, build gated on P38 §4.2 GPU decision |
| P87 | operator (OD-11) | GPU 2-bit mask, same gate |
| P90 | operator (OD-1/2/4) | contention fixes done on local branch; push rulings outstanding |
| P92 | bebop C3 + M1 + review gate | mesh fast-path; hard-prereq M1 |
| P93 | bebop C3 + M1 | store-and-forward transcript/replay |
| P94 | after P92 | in-memory ScopeMask |
| P84 | operator (OD-7) | reserved; money/FSM red-line, not proposed |
| P95 | NO-GO (precondition P95-C1 unmet) | its only wired caller `gov_recall` is dead; cost paid 0×/day |
| M1 | bebop C3 freeze | open RFC-5705 exporter bug; in /root/bebop-repo `insecure-tls` |

## C. What this pass actually changed (committed, not pushed)
- `40efcc168` telemetry/topics: flush stdout before `process::exit` (12 tests green).
- `f25796044` docs: corrected stale P95 "just build it" claim → HOLD/NO-GO, evidence-cited.
- `ROADMAP-EXEC-CLOSEOUT-DOUBT-CHECK-2026-07-20.md`: 2-question ritual + Q2 risk log.
- this file: verified open-items map.

## D. Honest verdict
The roadmap is NOT 100% done. The P01–P96 core build is ~95% MERGED-and-verified (section A
confirmed by real git, not the ledger). The residual (section B) is entirely:
  operator decisions (OD-1..OD-14 / O1..O8, cannot be made by an agent),
  bebop-side items frozen behind C3/P85 (different repo, in-flight `exec/sg-item*` branches),
  GPU-gated P86/P87 (OD-11 outstanding),
  P95 NO-GO (no caller),
  2026-07-20 product-surface synthesis (awaiting operator review → blueprint → build).
There is no open, ungated, unowned, buildable dowiz-side item left to "finish" — every such
item either already MERGED (A) or is gated (B). Claiming 100% would be fake-green.
