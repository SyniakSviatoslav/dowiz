# Wave Closeout — P57–P74 (2026-07-19)

All four waves merged into `main` and verified with real `cargo test` runs
(no blind trust). Main HEAD: `1d2e3d279`.

## Test counts (final, on main)
- `kernel` default features: **859 passed, 0 failed, 3 ignored**
- `kernel --features pq`: **1009 passed, 0 failed, 3 ignored**
- `engine`: **112 passed** (intent/friction/voice)
- `apps/courier`: **build green**

## W1 — P57–P62 (kernel core surfaces)
| Blueprint | Commit | Notes |
|-----------|--------|-------|
| P57 canvas/text-input | `81594dbea` | engine text_input/text_scope |
| P58 a11y-mirror | `546af7f50` | engine + wasm |
| P59 capability-cert-chain | `69345a364` | kernel/src/capability_cert.rs (26 tests) |
| P60 payment-adapter-core | `6f2d33961` | NO card-data type (minor-int money) |
| P61 notification-fabric | `8eabb042b` | kernel/src/ports/notification |
| P62 catalog-multivendor | `422b45c95` | kernel/src/catalog |

## W2 — P63, P64, P69–P72
| Blueprint | Commit | Notes |
|-----------|--------|-------|
| P63 floor-parity spike | `f2e90e7bf` | engine/tests (4 passed) |
| P64 intent/friction/voice | `5b97275a7` | engine (112 passed) |
| P69 customer-storefront | `a2119307d` | kernel (746 passed) |
| P70 owner-surface | `7d6a4eaef` | kernel (30 passed) |
| P71 courier-surface | `e8d189ff7` | apps/courier (build green) |
| P72 foodcourt-checkout | `c37b2f3a8` | kernel (15 passed) + over-refund bug fixed |
| fix | `6109a339b` | owner_surface OrderItem currency/vendor_id reconcile |

## W3 — P65–P68
| Blueprint | Commit | Notes |
|-----------|--------|-------|
| P65 dispatch-orchestrator | `bae2134` | **cross-repo**: `bebop-repo/bebop2/proto-cap/src/dispatch.rs` (per blueprint); `cargo test -p bebop-proto-cap` 123 passed |
| P66 data-wallet | `76df96199` | kernel/src/wallet/ (offline drafts, no tombstones, no card-data) |
| P67 hub-provisioning | `76df96199` | kernel/src/hub_provisioning.rs |
| P68 hub-supervisor | `76df96199` | kernel/src/hub_supervisor.rs (pq-gated; §4-B firewall) |

## W4 — P73, P74
| Blueprint | Commit | Notes |
|-----------|--------|-------|
| P73 dowiz-org-landing | `60facb847` | kernel/src/landing/ + json_api bot-pack (Rust/wgpu, TS ban) |
| P74 moderation-reports-blocklist | `60facb847` | kernel/src/moderation.rs + blocklist.rs (no scoring, no quality variant) |

## Merge topology
- W3 integrated as one commit `76df96199` (P66+P67+P68) → merge `1dd35b98a`.
- W4 integrated as one commit `60facb847` (P73+P74) → merge `1d2e3d279`.
- All subagent WIP was rescued from main-pollution into isolated worktrees before merge
  (the shared working-tree hazard from 2026-07-16 was avoided structurally).

## Open items (not part of P57–P74)
- `docs/research/OPUS-*.md` + `docs/design/*SYNTHESIS*.md` + `BLUEPRINT-P92`: untracked,
  produced by subagents, NOT committed (awaiting operator decision).
- Canon-diffs P38-rev, P39-rev: handled separately.
