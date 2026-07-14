# ROADMAP + GROUND TRUTH — dowiz & bebop (product) & dowiz-pq — 2026-07-14 UPDATE

> **Supersedes** `ROADMAP-GROUND-TRUTH-2026-07-11.md`. That doc is now STALE:
> `dowiz` `origin/main` is 640 commits behind the active canonical-stack work
> and a `git merge-tree` shows a conflict in `.agents/tmp/check-jobs.mjs`, so
> `main` is a frozen anchor, not the integration branch. Canonical-stack code
> lives on feature branches. This doc re-verifies every claim against live disk
> (grep/git/cargo test) on 2026-07-14. Memory-first + push-plans-first still apply.

## 0. GROUND TRUTH — verified this turn (2026-07-14)

### 0.1 Repo topology (re-verified)
| Repo | Active branch | Tests (this turn) | Status |
|---|---|---|---|
| /root/dowiz | `feat/kernel-fsm-graph-analysis` | kernel **106 green**, engine **17 green** | DONE; branch pushed to origin |
| /root/dowiz-pq | `feat/pq-crypto-tier1` | **178 tests** (ML-DSA/KEM/codesign/hybrid/x25519) | DONE; committed + pushed `da99e4e4`; only `target/` untracked (build artifact) |
| /root/bebop-repo | `feat/logic-governance` | workspace (verifying — see §0.2) | 4 files modified, uncommitted, upstream 0/0 |

### 0.2 Open WAVE1 artifact — bebop `feat/logic-governance`
Four files carry real uncommitted work (97 ins / 35 del), NOT fmt-only as the
2026-07-11 memory claimed:
- `crates/bebop/src/algebra.rs` (+5/−1)
- `crates/bebop/src/orthogonality.rs` (+25/−5)
- `crates/bebop/src/renormalizer.rs` (+85/−10)
- `rust-core/src/lib.rs` (+17/−2)

**Action (autopilot):** verify `cargo test --workspace` green, then commit +
push. Fenced from main-merge (see §2).

### 0.3 dowiz `origin/main` is stale — DO NOT force-merge
`feat/kernel-fsm-graph-analysis` is **640 commits ahead** of `origin/main`
(`7c5e816b`, "master build sequence updated", dated ~2026-07-11). A
`git merge-tree` against origin/main reports a conflict in
`.agents/tmp/check-jobs.mjs`. Conclusion: `main` is an ancient anchor; the
canonical stack has shipped across many feature branches. **Merging to main is
an operator-gated red-line-free but high-risk integration step — not auto-run.**

## 1. DONE (verified, re-checked 2026-07-14)
- **WAVE0 docker-swap** — `48b32840`, native SPA server, zero-OCI gate, microVM fail-closed. Pushed.
- **WAVE2/WAVE3/WAVE4 kernel** — `kernel/` Rust→WASM: decide/fold Law, integer money, ChannelLedger, full G11 backend (`server/`), web storefront calling REAL kernel, no money-tween. 65→106 tests across the FSM work.
- **FSM graph analysis** (`feat/kernel-fsm-graph-analysis`, this turn's capstone): `has_cycle` (DFS), `cyclomatic_number` (μ), `topological_order` (Kahn), `reachable` (BFS bitmask), `spectral_radius` (ρ, power iteration), `fsm_graph_report()` aggregate + `fsm_graph_report_js` surface. 96→106 kernel tests green, wasm32 release green. **3 commits pushed.**
- **Tier-0 A/B/C/D** — money-tween resolved in canonical storefront; degrade-storm ratchet; gitleaks CI hard-fail; OG+QR+?ch= attribution. All verified.
- **Tier-3 venue/claim plumbing** — `0295f30a`, G11 GREEN plumbing in place (external order still operator-owned).
- **RW-* kernel authority** — channel.js deleted (RW-02), estimate_order_total (RW-03), cart/geo/order-machine consolidated into kernel.
- **MESH-06 event_log** (`a0d19627`) — content-addressed per-node log, commit-after-decide.
- **PQ crypto tier-1** (`dowiz-pq` `da99e4e4`) — ML-DSA-65/KEM-768 zero-dep, KAT bit-exact, 178 tests. Pushed.
- **bebop WAVE1 MESH** (`ded4985`) — 751 tests, logic-governance + PQ proto-crypto; 4 CI compile-fence gates. (4 files since extended — see §0.2.)

## 2. OPEN OPERATOR GATES (not auto-executed — red-line / human decision)
1. **Merge canonical branches → `main`.** main is 640 commits stale with a
   tooling conflict. High-risk integration; operator sign-off required.
2. **MESH-12 genesis-policy decision.** Loader + self-cert built; policy enum
   default `Unspecified`, fail-closed. HUMAN enum choice pending.
3. **Tier-1 P1/P7/P8 execution** (prod OG/demo, remote-history scrub, branch
   prune) — red-line, needs operator approval.
4. **Re-author the ~20 missing 2026-07-11 research/design reports** OR accept
   them as UNVERIFIED. Operator decision #4 from the original roadmap; still open.
5. **Tier-2 quality bars / Tier-3 external G11 order** — external, not code.

## 3. PARALLEL-SAFE vs SEQUENTIAL (structure before code)
- **PARALLEL-SAFE (can run now, own branch):** the bebop `feat/logic-governance`
  commit+push (Wave this turn); dowiz-pq `target/` gitignore; any new
  feature-branch work (field-ui-engine, rust-engine-rewrite, dowiz-interfaces).
- **SEQUENTIAL GATES (operator):** main merge (§2.1), MESH-12 (§2.2), Tier-1
  (§2.3). bebop protocol work stays PARKED until dowiz carries it (invariant).

## 4. INVARIANT
Build DOWN from the first real order, not UP from the protocol. Gates are
falsifiable conditions, not calendar dates. Ground truth (grep/git/cargo) always
outranks this plan.

*Generated 2026-07-14. Re-verify before trusting any DONE line.*
