# GROUND-TRUTH — dowiz local `main` final state (2026-07-19)

> Authoritative re-baseline after the autopilot landing wave. Supersedes
> GROUND-TRUTH-2026-07-17 (that doc was stale — main had moved through P57–P74
> by the time it was written). Read THIS for current truth; pasted "pending" todos
> from compacted sessions are hypotheses, not facts.

## What is on local `main` (verified this session)

### Harness A3/A4 (the real remaining harness work)
- `feat/harness-a3a4-fix` → main `5ef8fbb78`.
- A4: dead concurrency cap → `WorkerSlots` counting semaphore (try_acquire → typed
  `DispatchError::Busy`, degrade-closed refusal; slot guard released on thread completion).
- A3: unbounded `MemStore` cache → `BoundedStore<S>` LRU (default cap 1024), needed
  `BlockStore::remove` (additive default-noop + real impls for MemStore/FileBlockStore).
- VERIFIED: llm-adapters 21 pass; kernel 866→894 pass; `cargo tree -p dowiz-kernel` = NO
  http-client (kernel stayed HTTP-free).

### Product/CI branches landed this wave (all `--no-ff`, all gated green by the
pre-commit hook's `cargo test` + gitleaks + firewall)
- harness A3/A4, p34, p71, p79, p80, p81, p83, p88, p89, p96, p01, p75, p77,
  p72-v3, contention-bench, p91 (ML-KEM ring), p47 (payment rail), reconcile-redline,
  p06 (took main's CLOSED HybridSigner — see below).

### Final test evidence (fresh, this session)
- kernel: **894 passed / 0 failed** (3 ignored)
- engine: **121 passed / 0 failed**
- ci-truth (tools/ci-truth): builds clean; HybridSigner = main's CLOSED variant
  (commit `58987d79d`, e2e GREEN).
- RED-line grep-gates still green: `payment_capability::red_line_no_real_provider_references`,
  `wallet::no_card_data_in_wallet`, kernel firewall (no http-client).

## Conflicts resolved this wave (root-cause, not assumptions)
- `kernel/Cargo.toml` `[[bench]]` block: p80's 8-entry expansion + contention's
  `[[bench]] name="contention"` — both additive, merged (kept p80's full block + appended
  contention entry).
- `kernel/benches/criterion.rs`: p77's `bench_spool_drain`/`bench_spine_build` + p89's
  `bench_field_eigen` — both registered in `criterion_group!` (no drop). Repaired a
  merge-induced dropped `spine` import.
- `kernel/src/lib.rs` (p72-v3): kept HEAD's real `pub mod` decls (wallet/hub_provisioning/
  span_metrics/hub_supervisor/landing) that p72-v3 predated; dropped the empty branch side.
- `kernel/src/ports/customer.rs`: removed a merge-duplicated `use crate::vendor::VendorId;`,
  restored `use crate::rng::Rng;` (used 12× in file).
- `.gitignore` (reconcile-branch): kept both sides' additions (`.worktrees/` + ci-truth
  v1-sigverify telemetry jsonl).

## p06 decision (explicit)
`feat/p06-v1-real-signer` was NOT merged wholesale: it carried 81 conflict hunks against
main's already-CLOSED `HybridSigner` (commit `58987d79d`, independent 3-model-verified
GREEN). Its only genuine delta vs main was native signing telemetry (SIG_TAG_K/V,
`record_telemetry`, JSONL sink). To avoid destabilizing the verified-green signing gate,
the merge took main's CLOSED signer (HEAD) for `ci-truth/src/{main,v1}.rs`. **p06's
telemetry delta is DEFERRED** — re-port it as an additive module once desired, don't
re-litigate the signer.

## Unmerged-branch final audit (2026-07-19, definitive)
Re-checked every unmerged branch with `git log main..<branch>` (NOT `main...branch`, which
falsely inflates via main's later files):
- **22 branches fully contained in main** (main..branch empty) → redundant junk. Safe to delete:
  all `recover/*` (except the 2 stash-* below), `*-snapshot-*`, `pq-crypto-tier1`, `kalman-organ`,
  `markov-attractor-signal`, `agent-capability-boost`, `decentralized-pq-protocol`,
  `remove-legacy-thin-layer`, `rw-02/03`.
- **`docs-research-2026-07-19`** (9 commits) → PROVEN REDUNDANT: merge brought only 3 conflicted
  files (README.md + Q-SERIES + SYNTHESIS blueprints); all 3 were the branch's STALE versions that
  main already superseded via the P75–P96 waves. `git diff HEAD` after resolving = empty → no unique
  content. Aborted (no empty merge).
- **`recover/stash-1-2994e6c8`** (77 commits) + **`recover/stash-2-93919edd`** (40 commits) → unique
  commits exist, BUT they are `git stash` recovery branches from the SUPERSEDED `feat/sovereign-core-
  phase-zero` arc (dated 2026-07-06/07). Operator rule: NEVER auto-merge `recover/*`. Left unmerged;
  operator decision.

**Conclusion: zero actionable product/CI branches remain unmerged. Core roadmap is COMPLETE on
`origin/main` (d8004a3c7). The only not-done items are operator-EXTERNAL: GitHub public-flip (P18),
secrets-scrub, bebop frozen-lane (C3/P85) in the bebop repo, and cleanup of the 24 redundant local
branches (deletion is operator hygiene, not required for roadmap completion).**

## Graphics/GPU state (added 2026-07-19)
The **O18a graphics-unlock** landed — `wgpu 30.0.0` is now cached and in `kernel/Cargo.lock`;
`kernel/src/render/gpu.rs` builds a real headless wgpu context under `feature="gpu"` (P38). The
eigenvector solve landed too (`spectral::eigh`/`topk_symmetric`, `03ac0fefe`, Phase-28). **Still
blueprint-only:** the `engine/` field-UI GPU render loop (engine `gpu = []` remains empty) and the
entire `living-interface-2026-07-16/` arc (R-SON audio, R-LM viz, R-VENDOR brand→GPU) — see
MASTER-ROADMAP §20 and `equations-knowledge-base-2026-07-19/SYNTHESIS-2026-07-19.md`. W21's "wgpu
uncached" premise is obsolete. O18a (graphics network-gate) and P06 (key_V crypto) are two distinct
operator decisions; both have landed, independently.

## Dashboard
- Local `main` HEAD after wave: `5a97e1f6f` (p06 merge).
- Kernel 894 / engine 121 / ci-truth green. 0 failures.
- Push to `origin/main`: authorized, executed per operator word.
