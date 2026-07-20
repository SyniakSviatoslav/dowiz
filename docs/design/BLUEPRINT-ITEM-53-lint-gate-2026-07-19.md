# BLUEPRINT — Item 53: `lint-gate` — clippy + fmt (+ miri-required promotion) Contribution Gates

- **Date:** 2026-07-19 · **Tier:** roadmap §J (fourth wave) · **Status:** BLUEPRINT v1 (planning
  artifact, no code). **LOW priority, LAST in this arc, blocks nothing** — sequenced behind items
  50–52 by explicit RULING (roadmap:851, synthesis §2.5).
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §J item 53 (lines 851–866);
  `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` §2.5 (priority RULING + escalation trigger);
  `docs/audits/hardening/CHECKLIST.md`; `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (CI-gate style).
- **Ground-truth cited (branch `main`, verified in-tree this session):** `.github/workflows/ci.yml`
  (the full current job list — no clippy/fmt/miri); `rust-toolchain.toml` (components pin);
  `/root/dowiz/CLAUDE.md` (the NO-workspace build model — load-bearing for §3).
- **Upstream:** item 52 (`miri-gate`) — "miri-required" is just promoting item 52's job to a required
  status check; item 14 (`rust-toolchain.toml` components pin — already met).
- **Downstream / trigger:** ADR-0020 public-flip authorization promotes this item to a pre-flip
  BLOCKER (§7.1).

---

## 1. Scope / goal

When dispatched, add ONE cheap CI job, `lint-gate`, that runs `cargo clippy --deny warnings` +
`cargo fmt --check` **per standalone crate** (the repo has no workspace, §3), plus the meta-step of
promoting item 52's `miri-gate` to a required status check ("miri-required"). This is the "proof of
PR" contribution surface for a future open-source flip — deliberately LAST because that surface is
not authorized to exist yet (§2.2).

**Non-goals:** NOT built before items 50–52 (explicit sequencing RULING, roadmap:851); NOT a new lint
philosophy or a wall of `#![deny(...)]` attributes across the tree (that is a separate cleanup);
NOT the branch-protection server-side flip itself (owed, G5, out of a workflow file's reach, §7.2).

## 2. Current-state grounding

### 2.1 None of the triad exists in CI (verified, exhaustive)

The complete job list in `.github/workflows/ci.yml` this session: `telemetry-selftest` (:19),
`eqc-proofs` (:44), `claim-latency-ledger` (:63), `v5c-reexec` (:88), `cargo-test` (:128),
`bench-regression` (:160), `gitleaks` (:184), `dco-check` (:210), `supply-chain` (:232),
`zero-dep-gate` (:272), `decart-dep-lint` (:296), `no-courier-scoring` (:309),
`no-pub-raw-matrix-hash` (:330), `fence-check` (:348), `regression-digest` (:362),
`firewall-agent-loop` (:377), `mesh-adapter` (:398), `toolchain-bump-gate` (:434),
`hardening-gate` (:488), `kani-gate` (:528). **Zero `cargo clippy` / `cargo fmt` / `cargo miri`
occurrences.** The roadmap's grounded baseline (roadmap:852–856) is confirmed exactly.

### 2.2 Open-sourcing is NOT imminent — the urgency premise is absent

The raw-prompt urgency ("any PR is an attack vector") presumes an open contribution surface. Per the
synthesis (§2.5) and roadmap (856–857): ADR-0020 is Accepted (AGPLv3 landed) but the public flip +
EUTM are **explicitly operator-gated and unauthorized**. There is no external PR surface to defend
yet, so the LOW-priority / LAST ruling is the grounded call — not laziness, a correct prioritization.

### 2.3 Both lint components are already pinned — the job is cheap to add

`rust-toolchain.toml:7` — `components = ["rustfmt", "clippy"]` (verified) — so a `lint-gate` job
needs no toolchain change; the binaries are present under the pinned `1.96.1` channel
(`rust-toolchain.toml:6`). The synthesis's "costs minutes to add" (roadmap:858–860) is grounded.

### 2.4 The NO-workspace build model shapes the job (critical, §3)

Per `/root/dowiz/CLAUDE.md` ("Build model — CRITICAL"): **there is NO root `Cargo.toml` / cargo
workspace**; each crate is standalone and you MUST `cd` into the crate dir. `cargo clippy` /
`cargo fmt` at the repo root would resolve nothing (or the wrong graph — the documented false-green
trap). So `lint-gate` MUST iterate the crates explicitly, mirroring how `cargo-test`/`hardening-gate`
already `cd kernel && …`, `cd engine && …` per crate.

## 3. Implementation plan (when dispatched)

1. **Enumerate the crate set.** The standalone crates (per CLAUDE.md crate map): `kernel/`,
   `engine/`, plus the agent lane (`agent-facade`, `agent-loop`, `agent-adapters`, `llm-adapters`),
   `mesh-adapter/`, `wasm/`, `agent-governance-wasm/`, `apps/courier/`, and `tools/*`. The job runs
   the lint pair inside each crate dir. (Executor confirms the live crate list from the tree at
   dispatch time — do not hardcode a stale list; CLAUDE.md warns the Repowise index is pre-`drop js`.)
2. **The gate script `scripts/lint-gate.sh`.** For each crate dir:
   `( cd "$crate" && cargo fmt --check && cargo clippy --all-targets -- --deny warnings )`.
   `--deny warnings` makes any clippy warning a hard failure; `fmt --check` makes any format
   divergence a hard failure. Aggregate non-zero exits → job RED.
3. **CI job `lint-gate`** in `.github/workflows/ci.yml`, slotted after `miri-gate` (item 52) /
   `kani-gate` (ci.yml:528). Checkout; the pinned toolchain (rustfmt+clippy already in the pin, §2.3
   — no extra install step); `bash scripts/lint-gate.sh`. Cheap, offline (`--offline` where the
   crate's deps are vendored/fetched, matching hardening-gate's P6 discipline).
4. **miri-required = a branch-protection promotion, not new machinery.** "Promote item 52's job to a
   required check" is a **server-side branch-protection setting**, not a workflow edit — a workflow
   file cannot mark itself required. Item 53's workflow contribution is only the clippy/fmt job; the
   "required" half is recorded as the same owed G5 step §7.2 names.
5. **Advisory-until-required caveat, inherited from item 14.** Every `ci.yml` gate is advisory until
   marked a required status check in branch protection (server-side, G5-owed) — item 53 inherits this
   (roadmap:861–862). Record it in the job's comment header so it is not mistaken for enforced.
6. **Escalation-trigger comment.** The job header records the named trigger (§7.1) verbatim so a
   future reader knows when this LOW item jumps the queue.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

Item 53 is CI machinery; the "tests" are its proven RED path (the item-6/P7 layer):

1. **Oracle — `N/A`** (a lint gate has no algorithmic oracle; clippy/rustfmt ARE the references).
2. **dudect — `N/A`.**
3. **Debug cross-check — `N/A`.**
4. **Assembly spot-check — `N/A`.**
5. **The gate's own RED-path demonstration (P7, in the PR):**
   - a **planted clippy warning** (e.g. an unused variable or a `clippy::needless_return`) in one
     crate turns `lint-gate` RED; removing it → green;
   - a **planted fmt divergence** (a mis-indented line) turns `lint-gate` RED; `cargo fmt` → green;
   - a clean tree is green;
   - the per-crate iteration is proven: a warning planted in a NON-`kernel` crate (e.g. `engine`)
     also turns the job RED — proving the job does not silently lint only the first crate (guards
     against the NO-workspace false-green trap, §2.4).

## 5. Falsifiable acceptance criteria

- A single clippy warning anywhere in any linted crate fails the job (`--deny warnings` proven per
  crate, not just kernel).
- A single unformatted line fails the job.
- The clean tree passes with zero toolchain-install steps beyond the pinned components (proving §2.3).
- The job header states, in words a reviewer can find, (a) that it is advisory until required in
  branch protection, and (b) the ADR-0020 escalation trigger.
- The job runs at repo scope but lints each crate in its own dir — no `cargo clippy` invoked at the
  workspace-less root (structural review of the script).

## 6. Dependency gates (honest)

| Gate | Status | Effect |
|---|---|---|
| Sequencing behind items 50–52 | explicit RULING | LAST by design; blocks nothing (roadmap:851). Do not dispatch before 50–52 without an operator override of the sequencing ruling. |
| rustfmt + clippy pinned | **MET** (`rust-toolchain.toml:7`) | job needs no toolchain install. |
| Item 52 `miri-gate` exists | **NOT MET yet** (item 52 unbuilt) | "miri-required" promotion has nothing to promote until item 52 lands; the clippy/fmt half is independent of item 52. |
| Existing tree is clippy-clean under `--deny warnings` | **UNVERIFIED — likely OPEN** (§7.3) | if the current tree has latent clippy warnings, the job goes RED on day one; landing it may require a pre-pass to clear warnings (a real, possibly non-trivial task — flag, do not assume clean). |
| Branch-protection "required" flip | **OPEN (G5, server-side)** | out of a workflow file's reach; owed step. |

## 7. Operator / executor decision points (flagged)

1. **The named escalation trigger (operator-owned).** The moment the operator authorizes public-flip
   *preparation* (ADR-0020's gate), item 53 **jumps the queue to a pre-flip BLOCKER**, alongside the
   ADR-recommended all-origin-refs gitleaks sweep (roadmap:862–864). Until that trigger fires, LOW is
   the grounded priority. This is an operator decision; the blueprint only records the trigger.
2. **Branch-protection "required" flip (operator/admin, server-side).** Marking `lint-gate` (and the
   promoted `miri-gate`) as required status checks is a GitHub branch-protection change, not a code
   change — an admin action the workflow cannot self-apply (G5-owed, roadmap:861–862). Flag for the
   operator; do not claim enforcement from a merged workflow alone.
3. **Latent-warning pre-pass scope (executor, at dispatch).** Whether the current tree already passes
   `cargo clippy --deny warnings` per crate is UNVERIFIED here (running clippy is out of this
   planning task's scope). If it does not, landing item 53 requires first clearing the warnings — a
   task whose size the executor must measure at dispatch, not assume. Recommend a dry `clippy` sweep
   as the first step of the item, ledgered.
4. **Clippy lint-level policy.** `--deny warnings` denies ALL default clippy warnings; whether to add
   `--warn clippy::pedantic` or selectively `--allow` noisy lints is a policy call. Recommend the
   default lint set + `--deny warnings` (minimal, matches "cheap job"); expansions are a later item.
