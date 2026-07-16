# BLUEPRINT — Phase 1: CI TRUTH FLOOR (dev-time fences)

> Roadmap: `R2-MERGED-PHASE-ROADMAP.md` Phase 1 (Wave 0, no dependencies, parallel-safe with P2/P3/P4/P5).
> Anchors owned: **V2, V3, V5, S3, S6, D6, E2, E3, E40, E52, E58, E62** (R2 §5 anchor map).
> Evidence base: `R1-C-kernel-service-compute-storage-gap-analysis.md` + `R1-E-ecosystem-community-business-legal-growth-gap-analysis.md`.
> Every current-state claim below carries a file:line or commit citation from those reports — none re-derived.
>
> ### SCOPE RULE BANNER (read before anything else — ARCHITECTURE.md §0, lines 23–27)
> **Everything in this phase is a canonical-repo DEV-TIME fence.** It is a blocking CI / pre-commit
> control on *the operator's own build and merge into the canonical repo* — it is **NOT** a runtime
> control over any autonomous hub. At runtime every hub is a sovereign Hydra (M5/M9/M11): it may
> fork this repo, delete these workflows, self-gate locally via `eqc`, ignore the upstream gate, and
> replicate-or-reject any artifact. **No gate in this blueprint may be read as a "global control."**
> Where a job header says "blocks merge", it means *blocks merge into the canonical repo*, nothing more.
> This banner is reproduced verbatim in the header of every workflow job this phase adds, so a future
> reader can never misread a red CI run as a limit on hub autonomy.

---

## 0. Thesis (one sentence)

Until CI tells the truth, every downstream "GREEN" in this repo is unverifiable — so before any product
math, crypto, or mesh work is trusted, the canonical repo's own dev-time floor must independently turn
RED on a planted secret, a failing kernel test, an `nginx` base image, an un-signed-off commit, and an
un-DECARTed dependency, and stay GREEN only on a clean signed diff.

---

## 1. Current-state evidence (file:line grounded)

**1.1 CI regressed to two jobs.** `.github/workflows/ci.yml` at HEAD has exactly **two jobs**:
`telemetry-selftest` and `eqc-proofs` (the eqc math-proof job at `ci.yml:24-58`). Commit `b10a7bfe3`
("ci(security): Tier-0 C gitleaks gate") had *added* a gitleaks CI job; commit `f9ab28ff1` ("drop ALL
JS/TS; rewire CI to telemetry+eqc") **silently dropped it** along with everything else (R1-C §0.1,
evidence appendix "CI has no gitleaks/cargo-test"). Survivors outside `ci.yml`: `safety-floor.yml`
(runs `.claude/hooks/verify-safety-floor.sh` on push — one genuine survivor) plus peripheral
`skill-security.yml` / `heartbeat-monitor.yml` / `visual.yml` (R1-C V3).

**1.2 Kernel 337 + engine 47 tests run NOWHERE in CI.** Verified offline this session: kernel **337
passed / 0 failed**, engine **47 passed / 0 failed** (R1-C header + appendix "test truth"). No
`cargo test` job exists, so even the built money invariants (`money.rs`, 25+ RED overflow tests) are
not CI-enforced (R1-C S9).

**1.3 Secrets floor half-gone.** `.gitleaks.toml` still exists (repo root, 63 lines, `useDefault =
true`, broad allowlist covering `scripts/ tools/ docs/ e2e/ .github/workflows/`) but the **CI gate is
gone** (dropped in `f9ab28ff1`). `gitleaks` and `cargo-audit` binaries ARE installed
(`/usr/bin/gitleaks`) — only the CI wiring is missing (R1-C S3; R1-E E2). The EnvFile pattern
(`deploy/pgrust.env` "NO SECRETS" header + unit `EnvironmentFile=`) is GREEN (S3/E40 secrets half).

**1.4 CONTRIBUTING.md makes a false claim.** `CONTRIBUTING.md:17` states *"CI rejects commits without
a valid Signed-off-by"* — **no DCO check exists** in any workflow (R1-E D3 gap 1; R1-C §0.3). By
VERIFIED-BY-MATH this is an unshipped RED: the doc asserts a gate that does not exist. The fix is to
**make the claim true**, not delete it (R2 §1.3).

**1.5 No supply-chain floor.** No `vendor/` dir, no in-repo `.cargo/config.toml`; `cargo-audit` is run
manually (the rsa triage); no `deny.toml`. bebop is ahead here — it vendors `advisories/` (advisory-db)
and ships `deny.toml`; dowiz has neither (R1-E E3). The one exemplary artifact is the
rsa/RUSTSEC-2023-0071 triage: an `innovate:` marker at `kernel/Cargo.toml:31` with a named revisit
condition ("`cargo tree -i rsa` shows a real path OR a patched rsa release ships") — **this is E53, the
canonical suspension/waiver form to reuse, not reinvent** (R1-E D3, E53; R1-C V3).

**1.6 No DECART-dep lint (D6/V2/E62).** Real DECART reports exist
(`KERNEL-OBSERVABILITY-DECART-2026-07-15.md`, the rsa marker) but **nothing automated** checks that a
new `[dependencies]` line carries one; V2's tech-stack-as-law is honor-system — no CI check catches a
k8s manifest, a GraphQL dep, or an undocumented new dep (R1-C D6, V2; R1-E E62).

**1.7 NO-COURIER-SCORING has no dowiz-side CI wire (E58).** The law is asserted in kernel code
(`event_log.rs:22-23` comments; references in `domain.rs`, `wasm.rs`, `native-spa-server`) and enforced
by bebop dormant pre-commit guards (fc1805f) — but **the dowiz-side CI grep guard is missing** (R1-E
E58).

**1.8 V5-B and V5-C are NOT BUILT.** No claim-latency ledger anywhere (zero hits for `claim-latency` in
repo/tooling — R1-C V5, R1-E E47); no independent-context re-execution harness (V5-C) exists (R1-C S6, V5).

**1.9 License metadata inconsistent.** `kernel/Cargo.toml` has **no `license` field**;
`tools/async-spool/Cargo.toml` and `tools/native-spa-server/Cargo.toml` say `license = "MIT"` —
conflicting with the AGPLv3-only policy (LICENSE is already full AGPLv3, flipped at `ac1caba40`
2026-07-14). Either an intentional per-tool permissive carve-out that must be documented in `NOTICE`,
or a bug (R1-E D3 gap 2, §0.1). No SPDX headers anywhere.

**1.10 Stale zero-OCI comment + unwired gate.** `scripts/check-zero-oci.sh:8` says the SBOM/scan/sign
half "runs in CI (see `.github/workflows/ci.yml` — supply-chain job)" — **that job does not exist**
(R1-C §0.1). The gate script itself works (exit 1 on nginx base, `Dockerfile:53` = `FROM scratch`,
passes) but is not wired into CI (R1-C S1).

**1.11 V3's i18n/IDOR/OTP gates target DELETED surfaces.** V3 names i18n/IDOR/OTP gates whose target
code was removed with `apps/api` (`otp.ts`, owner routes gone; `apps/web` SPA survives). Restoring them
verbatim is impossible; they must be **re-scoped**, each with a written reinstatement trigger in the
E53 form (R1-C V3, §2.2). The i18n *rebuild* itself is Phase 16 — Phase 1 writes only the trigger docs.

---

## 2. Target-state design (concrete job + script sketches)

All new jobs are added to `.github/workflows/ci.yml` (extend, do not replace — keep `telemetry-selftest`
and `eqc-proofs`). Each job header carries the SCOPE-RULE banner as a leading comment. Jobs, names, and
commands (sketch — the blueprint does NOT create these files):

```yaml
# .github/workflows/ci.yml  (jobs added by Phase 1; SCOPE RULE banner atop each)
jobs:
  cargo-test:                    # S9/V3/E2 — the 337+47 tests, offline, vendored
    steps:
      - run: cargo test --offline --manifest-path kernel/Cargo.toml
      - run: cargo test --offline --manifest-path engine/Cargo.toml
      # deps come from committed vendor/ + .cargo/config.toml source-replacement (E3)

  gitleaks:                      # S3/E40/E2 — recovered from b10a7bfe3
    steps:
      - run: gitleaks detect --config .gitleaks.toml --redact --exit-code 1

  dco-check:                     # E52 — makes CONTRIBUTING.md:17 TRUE
    steps:                       # every commit in the PR range needs Signed-off-by
      - run: |
          for sha in $(git rev-list origin/main..HEAD); do
            git log -1 --format=%B "$sha" | grep -qE '^Signed-off-by: .+ <.+@.+>' \
              || { echo "::error::$sha missing Signed-off-by"; exit 1; }
          done

  supply-chain:                  # E3/S3/S1 — makes check-zero-oci.sh:8 TRUE by name
    steps:
      - run: cargo audit --deny warnings   # waivers via advisories/ + deny.toml
      - run: cargo deny check              # deny.toml (cloned from bebop pattern)
      - run: scripts/check-zero-oci.sh     # zero-OCI: reject nginx/non-scratch base

  decart-dep-lint:               # D6/V2/E62 — new dep without a DECART doc => RED
    steps:
      - run: scripts/decart-dep-lint.sh origin/main HEAD

  no-courier-scoring:            # E58 — dowiz-side wire of the reputation red-line
    steps:
      - run: |
          ! git grep -nEi 'courier[_-]?(score|rating|reputation|rank)' -- \
            'kernel/**' 'engine/**' 'tools/**' ':!**/*NO-COURIER-SCORING*'

  claim-latency-ledger:          # V5-B — appender only (anomaly consumer = Phase 8)
    steps:
      - run: scripts/claim-latency-append.sh   # one JSONL entry per commit

  v5c-reexec:                    # S6/V5-C — UNSIGNED here; Phase 6 adds ML-DSA K/V
    if: red-line paths touched   # money.rs / order_machine.rs / event_log.rs / auth
    steps:
      - run: scripts/v5c-reexec.sh origin/main HEAD   # RED|GREEN + rationale JSON
```

**2.1 `cargo-test` (S9, V3, E2).** Runs kernel + engine suites `--offline`. Requires the vendoring
decision (2.4) so CI has no network. Falsifier: a planted `assert!(false)` in any kernel test → RED.

**2.2 `gitleaks` (S3, E40, E2).** Cherry-pick the job body from `b10a7bfe3`; point at the existing
`.gitleaks.toml`. `--exit-code 1` makes a planted secret RED. Keep the allowlist under periodic review
(R1-C S3 flags the surface is wide).

**2.3 `dco-check` (E52).** Iterates `origin/main..HEAD`, requires `Signed-off-by:` on each commit.
This is the single job that turns `CONTRIBUTING.md:17` from a false claim into a true one — its
existence is itself an acceptance criterion.

**2.4 `supply-chain` (E3, S3, S1).** One job named `supply-chain` so `check-zero-oci.sh:8`'s reference
becomes true by construction (and the comment is reconciled to the final job name in the same PR —
1.10). Contains: `cargo audit` with waivers vendored under `advisories/` (bebop pattern) + a new
`deny.toml` (cloned from bebop) whose waivers use the **E53 rsa-triage form** (named owner + checkable
revisit condition — reuse, do not invent a new form); plus `scripts/check-zero-oci.sh` for the
zero-OCI/nginx check. Vendoring decision: `cargo vendor` → committed `vendor/` + `.cargo/config.toml`
source-replacement, **or** a documented registry-cache policy — pick one and write it down (R1-E E3).

**2.5 `decart-dep-lint` (D6, V2, E62).** `scripts/decart-dep-lint.sh` diffs `Cargo.lock` / the
`[dependencies]` blocks between base and head; if a dependency is added and no `docs/**/*DECART*.md` (or
an `innovate:`/`decart:` marker in the touching Cargo.toml) references it → RED. This is the cheap CI
lint R1-C D6 and R1-E E62 both call for.

**2.6 `no-courier-scoring` (E58).** Grep guard rejecting reputation/scoring identifiers in
kernel/engine/tools (excluding the assertion sites). Ports the dowiz-side wire that today lives only as
bebop dormant guards.

**2.7 `claim-latency-ledger` (V5-B).** `scripts/claim-latency-append.sh` appends one JSONL entry per
commit to `docs/ledger/claim-latency.jsonl` — `{commit_sha, authored_ts, ci_observed_green_ts, delta_s,
diff_loc}`. Phase 1 builds **only the appender**; the anomaly detector (the 52s-on-1610-line-diff flag)
is Phase 8's consumer (R2 P8; R1-C V5 "ledger half in 1"). Emit the ledger as a CI artifact (avoids a
bot commit-back loop) or commit via a dedicated ledger bot — decide in migration.

**2.8 `v5c-reexec` (S6, V5-C).** `scripts/v5c-reexec.sh` (or a small std-only Rust tool) checks out the
diff into a **clean, independent worktree**, re-runs `cargo test --offline` for kernel+engine, and emits
`RED|GREEN` + a rationale JSON. **Unsigned in Phase 1** — Phase 6 wraps this exact runner with ML-DSA
key_K/key_V signatures and a merge gate (R2 §1 merge note: "Phase 1 builds it unsigned, Phase 6 adds
signatures on top"). Gate it on red-line paths (money/orders/event_log/auth) per SCOPE-RULE dev-time.

**2.9 License metadata + docs (E52, D3-doc-half).** Set `license = "AGPL-3.0-or-later"` in
`kernel/Cargo.toml`; **rule on the two MIT tool crates** (2.4 operator note in §6) — either document the
carve-out in `NOTICE` or flip them to AGPL. Fix `check-zero-oci.sh:8` to name the real `supply-chain`
job. Write the re-scoped **reinstatement-trigger docs** for i18n/IDOR/OTP in the E53 form (i18n →
`apps/web`/`web/` locale completeness, armed by Phase 16; IDOR/OTP → dormant, re-arm when a server API
surface reappears).

---

## 3. Migration steps (in order)

1. **Vendoring decision first** (unblocks offline `cargo-test`): run `cargo vendor`, commit `vendor/` +
   `.cargo/config.toml` source-replacement, OR write the registry-cache policy doc. (E3)
2. **Add `deny.toml` + `advisories/`** cloned from the bebop pattern; encode the rsa/RUSTSEC-2023-0071
   waiver in the **E53 form** (do not invent a new waiver shape). (E3, E53)
3. **Restore `gitleaks` job** from `b10a7bfe3` against the existing `.gitleaks.toml`. (S3, E40, E2)
4. **Add `cargo-test` job** (kernel+engine, `--offline`). (S9, V3, E2)
5. **Add `supply-chain` job** (`cargo audit` + `cargo deny` + `check-zero-oci.sh`); **then fix
   `check-zero-oci.sh:8`** to reference the now-real job name. (E3, S1, S3)
6. **Add `dco-check` job**; only after it is green, `CONTRIBUTING.md:17` is true. (E52)
7. **Add `decart-dep-lint`** + `scripts/decart-dep-lint.sh`. (D6, V2, E62)
8. **Add `no-courier-scoring`** grep guard. (E58)
9. **Add `claim-latency-ledger`** appender + `docs/ledger/claim-latency.jsonl`. (V5-B)
10. **Build `v5c-reexec`** harness (unsigned) + wire on red-line paths. (S6, V5-C)
11. **License metadata**: `kernel/Cargo.toml` `license`; rule + document the two MIT tool crates. (E52)
12. **Reinstatement-trigger docs** for i18n/IDOR/OTP in the E53 form. (V3)
13. **Stamp the SCOPE-RULE banner** into every added job header. (ARCHITECTURE §0)
14. **Prove on a probe branch** (§4), capture the RED runs as CI artifacts, then merge.

Ordering rationale: vendoring/deny.toml precede the test/audit jobs that consume them; the
`supply-chain` job must exist before its stale comment can be truthfully fixed; `dco-check` must be
green before the CONTRIBUTING claim is made true.

---

## 4. Acceptance criteria (falsifiable, numbered)

On a throwaway **probe branch**, each of the following must hold independently:

1. **Planted secret → RED.** Commit a fake AWS-key-shaped string in a non-allowlisted path → the
   `gitleaks` job fails; removing it → green. (S3/E40/E2)
2. **Failing kernel test → RED.** Insert `assert!(false)` into one kernel test → `cargo-test` fails;
   revert → green. (S9/V3)
3. **nginx base image → RED.** Change `Dockerfile` final stage `FROM scratch` to `FROM nginx` → the
   `supply-chain` job's `check-zero-oci.sh` step fails. (S1)
4. **Commit without `Signed-off-by` → RED.** Push a commit lacking the trailer → `dco-check` fails.
   (E52)
5. **Un-DECARTed new dep → RED.** Add a `[dependencies]` line with no DECART doc / marker →
   `decart-dep-lint` fails; add the DECART reference → green. (D6/V2/E62)
6. **Courier-scoring identifier → RED.** Introduce `courier_score` in kernel/engine/tools →
   `no-courier-scoring` fails. (E58)
7. **Clean signed commit → GREEN.** A diff with a valid `Signed-off-by`, no secret, no failing test,
   scratch base, no un-DECARTed dep, no scoring identifier → all jobs green.
8. **Ledger grows by one per commit.** After the probe commits, `docs/ledger/claim-latency.jsonl` has
   exactly one new entry per new commit. (V5-B)
9. **V5-C RED on planted failure.** `v5c-reexec` re-executes the diff in a **clean checkout** and emits
   `RED` on a planted red-line test failure, `GREEN` on a clean red-line diff. (S6/V5-C)
10. **DCO job exists (doc truth).** `grep` finds a `dco-check` job in `ci.yml` → `CONTRIBUTING.md:17` is
    now a true statement. (E52)
11. **License metadata consistent.** `kernel/Cargo.toml` carries `license = "AGPL-3.0-or-later"`; the two
    MIT tool crates are either documented in `NOTICE` or flipped. (E52)
12. **Stale comment fixed.** `scripts/check-zero-oci.sh:8` references a CI job that actually exists.
13. **Reinstatement-trigger docs exist** for i18n/IDOR/OTP in the E53 form (named owner + checkable
    re-arm condition), not blind restorations. (V3)
14. **SCOPE-RULE banner present** in every added job header — a reader cannot misread a red run as a
    runtime hub control.

---

## 5. Dependencies & sequencing

- **Depends on:** nothing. Phase 1 is Wave-0, starts immediately (R2 §2/§3).
- **Parallel-safe with:** Phases 2, 3, 4, 5 (all Wave-0). No shared mutable surface — Phase 1 touches
  `.github/workflows/`, `scripts/`, `deny.toml`, `advisories/`, `docs/ledger/`, license metadata, and
  the reinstatement docs; the other Wave-0 phases touch canon docs (P2), crypto (P3), kernel math (P4),
  routing wiring (P5).
- **Phase 1 unblocks downstream:** every "GREEN" claim in the roadmap is unverifiable until this lands —
  R1-C names it "Phase 1 for the whole roadmap." Concrete hard consumers: **Phase 6** wraps the Phase-1
  `v5c-reexec` runner with ML-DSA signatures (do NOT re-architect the harness in P6 — it must be the same
  runner); **Phase 7** (money-law) is the first real consumer of the V5-C verifier on a red-line diff;
  **Phase 8** consumes the Phase-1 claim-latency ledger for anomaly alerting; **Phase 11** relies on the
  `decart-dep-lint` to keep the default `cargo build` graph byte-identical; **Phase 16**'s i18n gate is
  *hosted by* Phase 1's CI (the trigger doc written here arms it); **Phase 18**'s public flip has a hard
  precondition of "gitleaks CI green."

---

## 6. Operator decisions touching this phase (R2 §4 — noted, NOT resolved)

Per R2 §4, **no O1–O19 item is assigned to Phase 1** — this phase is engineering, not a ruling gate.
Two adjacent decisions must nonetheless be surfaced (the blueprint flags them; it does not resolve them):

- **P1-local ruling — the two MIT tool crates.** `tools/async-spool` and `tools/native-spa-server`
  declare `license = "MIT"` against an AGPLv3-only repo (R1-E D3 gap 2). Phase 1's "fix license metadata"
  step cannot silently pick one: the operator must decide **carve-out (document in `NOTICE`)** vs **flip
  to AGPL**. This is not in the §4 O-list but is a genuine operator call embedded here. *Not resolved by
  this blueprint.* (Note: the `kernel/Cargo.toml` value itself is settled — `AGPL-3.0-or-later`, since
  LICENSE already flipped at `ac1caba40` — so only the two tool crates need the ruling.)
- **O17 downstream precondition.** Phase 1's `gitleaks` job going green is a **hard precondition** for
  **O17 (public-flip go, Phase 18)** and for E-4's flip readiness (R1-E Phase E-4 dependencies). O17
  itself is a one-way-door operator decision in Phase 18 — Phase 1 only *supplies* the precondition, it
  does not approach the door.

Explicitly **out of scope for Phase 1** (owned elsewhere, do not resolve here): D5/D8 canon count (O1,
Phase 2 — Phase 1 changes no "147" arithmetic); E10≡E36 (O2, Phase 2); the ADR-020 file and canon §8/S3
stale-line corrections (Phase 2 docs work). Phase 1 fixes only *executable* truth (CI jobs, license
metadata, one stale script comment); canon-prose truth is Phase 2.

---

*Blueprint P01 complete. Sources: R1-C (kernel/service/compute/storage gap), R1-E (ecosystem/legal/growth
gap), ARCHITECTURE.md §0 SCOPE RULE + anchor defs, STRATEGIC-VECTORS-LOCKED (E-anchor cluster defs),
R2-MERGED-PHASE-ROADMAP §1/§2/§3/§4/§5. This document plans work; it writes no CI or code.*
