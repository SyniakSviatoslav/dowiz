# R1-C — Kernel/Service Correctness, Compute & Storage — Gap Analysis (2026-07-16)

> Cluster C of the sovereign roadmap R1 pass. Anchors owned: **S1–S9, D1, D6, D7, M8, V2, V3, V5,
> F31–F40, E21–E30** (per ARCHITECTURE.md §2/§3/§6 + STRATEGIC-VECTORS-LOCKED).
> Every claim below carries file:line evidence or an honest NOT BUILT. Grounded in direct reads of
> `kernel/src/*`, `engine/src/*`, `tools/*`, `deploy/*`, `.github/workflows/*`, `Dockerfile`,
> `.gitleaks.toml`, plus test runs (kernel **337 passed / 0 failed**, engine **47 passed / 0 failed**,
> offline, 2026-07-16) and git archaeology. Branch tip at analysis time: `55b576b0b`.
>
> **SCOPE RULE (ARCHITECTURE.md §0, restated once, applies to EVERY S/V anchor below):**
> every gate/policy in this cluster is a **canonical-repo DEV-TIME fence** — a blocking CI/pre-commit
> control on the operator's own build — **never a runtime control on hubs**. At runtime every hub is
> a sovereign Hydra (M5/M9/M11) and may override any of it. Where an anchor says "blocking CI gate"
> read "blocks merge into the canonical repo", nothing more.

---

## 0. Headline findings (read these first)

1. **The CI rewrite silently regressed V3.** Commit `b10a7bfe3` ("ci(security): Tier-0 C gitleaks
   gate") added a gitleaks CI job; commit `f9ab28ff1` ("drop ALL JS/TS; rewire CI to telemetry+eqc")
   rewrote `.github/workflows/ci.yml` to just two jobs (`telemetry-selftest`, `eqc-proofs`) —
   **no gitleaks job, no cargo-test job, no zero-OCI/supply-chain job** survive. `.gitleaks.toml`
   still exists (repo root, 63 lines) and `scripts/check-zero-oci.sh:8` still *says* "the
   SBOM/scan/sign half runs in CI (see .github/workflows/ci.yml — supply-chain job)" — that job
   does not exist. The kernel's 337 tests and the engine's 47 run **nowhere in CI**. This single
   fact breaks S9's "blocking CI gate", S3's gitleaks half, V3 entirely, and V5-C's premise.
2. **The event_log double-hash (P0-A2) is still present and is a dedup-correctness bug, not just
   perf.** `kernel/src/event_log.rs:284` (`commit_after_decide`) computes `event_id()` on the
   event **before** prev-chaining; `append` (`event_log.rs:253-258`) rebinds `prev` to the tip
   *then* hashes. So the duplicate check at `:286` tests a **different id** than what gets stored.
   Concrete failure: replay a zero-`prev` event onto a non-empty log → `contains()` misses,
   `decide` re-runs, a **second** event is committed. The existing test
   `dup_event_is_idempotent_no_state_change` (`event_log.rs:375`) only covers the genesis case
   (empty log), so it stays green. Fix = bind `prev` before the dedup check and hash once;
   P0-A2's stated acceptance ("byte-identical event IDs") is **insufficient** — it needs the RED
   replay-on-non-empty-log test.
3. **SYNTHESIZED-BLUEPRINT P0-A status: A1 OPEN, A2 OPEN (and under-specified, see #2), A3
   REGRESSED-OPEN (config exists, gate dropped), A4 OPEN (both halves), A5 OPEN (and its
   referenced `eqc-proofs/lambda_max_of_d.rs` pattern does not exist anywhere in dowiz — no
   `eqc-proofs/` dir; eqc proofs are emitted ephemerally in the CI job), A6 OPEN (zero `.tf`
   files; and its `backend "pg"`-on-pgrust idea is at risk, see §D1/E26), A7 process-only.**
   Nothing from Cluster A has been implemented since the blueprint was written.
4. **M8's only *existing* metrics pipeline exfiltrates.** The bash telemetry bridge
   (`tools/telemetry/lib.sh:146` `resource_sample()` → `tg_send`) ships host load/mem/disk JSON to
   the **Telegram Bot API** — a remote third party. It is operator-directed self-reporting, so
   compliant in spirit, but it is **unsigned** (F40 NOT BUILT) and not typed/filtered (F32 partial).
   No Rust typed CPU/GPU per-process metrics exist anywhere (`kernel/src/telemetry.rs` is trigram
   pattern-surfacing, not process metrics).
5. **Ground-truth drift confirmed:** project memory says "kernel 152 tests"; actual is **337
   passing** (385 `#[test]` fns incl. feature-gated). Re-verify counts from `cargo test`, never
   memory (the ROADMAP-GROUND-TRUTH rule, re-proven).

---

## 1. Per-anchor gap table

### S1 — Runtime: zero-OCI static binaries + systemd; microVM when fleet>5
- **CURRENT:** PARTIALLY BUILT.
  - Zero-OCI: `Dockerfile:53` final stage `FROM scratch`, single static `native-spa-server` binary
    + SPA dist + CA certs only. Gate script `scripts/check-zero-oci.sh` (exit 1 on nginx base,
    warns if no scratch stage) exists and passes — but is **not wired into current CI** (see §0.1).
  - systemd: one hardened unit exists — `deploy/pgrust.service` (`NoNewPrivileges`,
    `ProtectSystem=strict`, `CapabilityBoundingSet=` empty, `EnvironmentFile=/etc/pgrust/pgrust.env`).
    **But** `ExecStart=/usr/local/bin/pgrust` references a binary that is **not installed on the
    host** (verified: `pgrust-binary-not-installed`). No unit exists for `native-spa-server` or any
    node binary. The kernel crate is `cdylib+rlib` only — **there is no node binary at all yet**.
  - microVM: `kernel/src/isolation/microvm.rs` = fail-closed KVM capability probe
    (`SandboxTier::{WasmComponent, NativeProcessRequiresKvm}`, refuses native adapters without
    KVM). Actual VMM launch (Firecracker/jailer) explicitly marked `innovate:` follow-up
    (`microvm.rs:14-17`). Fleet size = 1, so the `fleet>5` trigger is not met — probe-only is
    honest for now.
- **TARGET:** all services as static binaries under systemd units with EnvFile; zero-OCI gate
  blocking in CI; microVM launcher behind the existing probe, activated at fleet>5.
- **GAP:** wire check-zero-oci into CI; ship systemd units for every real service (currently only
  pgrust, whose binary is absent); note Dockerfile's builder stages still use node/pnpm (apps/web
  survives the JS-drop) — acceptable, runtime stage is what zero-OCI governs.

### S2 — Topology: modular monolith; microVM per P1
- **CURRENT:** BUILT-by-construction. Single host, separate crates (`kernel/`, `engine/`,
  `tools/*` — explicitly *not* a cargo workspace, per `Dockerfile:44` comment), engine depends on
  kernel by path (`engine/Cargo.toml`). No service mesh, no k8s. microVM = probe only (S1).
- **GAP:** none structural. Optional build-hygiene question (workspace vs not) — leave as-is
  unless a DECART comparison says otherwise.

### S3 — Secrets: systemd EnvFile + never in-repo + gitleaks; ADR-020 LICENSE mismatch
- **CURRENT:** PARTIALLY BUILT / REGRESSED.
  - EnvFile pattern: `deploy/pgrust.env` (explicit "NO SECRETS here" header) + unit's
    `EnvironmentFile=` — pattern GREEN.
  - `.gitleaks.toml` exists (repo root) with `useDefault = true` + a **broad allowlist**
    (`scripts/`, `tools/`, `docs/`, `e2e/`, `.github/workflows/` all excluded — documented as
    test-only-key dirs, verified 2026-07-12; re-verify periodically, the surface is wide).
  - **CI gate: GONE** (dropped in `f9ab28ff1`, see §0.1). Real secrets live on disk in untracked
    `.env` (per project memory: OPENROUTER/GOOGLE/JWT/COURIER_PII/VAPID — use, never commit).
  - ADR-020 mismatch (LICENSE Apache-2.0 vs AGPLv3 target) + force-push scrub: documented OPEN in
    ARCHITECTURE.md §8; P10 decision 2026-07-16 says origin already points at scrubbed tip, no
    force-push needed. Owned by cluster E legally; the *gitleaks half* is this cluster's.
- **GAP:** restore the gitleaks CI job (cherry-pick from `b10a7bfe3`); planted-secret RED test.

### S4 — API: gRPC/protobuf internal + REST edge RECOMMENDED; GraphQL client-edge only
- **CURRENT:** NOT BUILT — and currently **premature by construction**. Zero tonic/prost/protobuf
  anywhere (grep of all `.toml`/`.rs`; the only hits were "mono**tonic**" false positives). The
  legacy JS API (`apps/api`) was deleted in the JS-drop; the only server today is
  `tools/native-spa-server` (static SPA, no API). There are not yet two in-host services to speak
  gRPC between. GraphQL: absent everywhere = compliant.
- **TARGET:** when the node binary / hub services exist, internal service-to-service = gRPC/proto,
  edge = REST. RECOMMENDED not mandated; hub may open any port (M5).
- **GAP:** a written `.proto` contract for the first internal boundary (event-sync / node control)
  + a DECART report for the tonic/prost dep **before** the first internal API is built any other
  way. No code gap today because no internal API exists.

### S5 — Errors: fail-closed Result always
- **CURRENT:** LARGELY BUILT in kernel: `money.rs` all `Result` (checked_add/mul, range-checked
  i128→i64, cross-currency add is `Err` — `money.rs:70-87,94-112,119-132`); `order_machine.rs`
  `TransitionError` enum + `assert_transition`/`fold_transitions` (`order_machine.rs:123-153`);
  event_log `DecideRejected`, no partial commit (`event_log.rs:289-292`); isolation gate
  fail-closed (`microvm.rs`). **Deviation:** engine integrator fail-closes by **panic**
  (`field_frame.rs:55-68` `assert_stable`), not Result — deliberate (a divergent dt must never
  reach the integrator) but should be recorded as a named exception to "Result always".
- **GAP:** document the panic-as-fail-closed exception; audit `unwrap_or(0)` at
  `money.rs:218` (`estimate_order_total` swallows a tax overflow into 0 — display-mirror only,
  server-authoritative, but it is a silent-default in money-adjacent code; worth a deliberate
  comment or propagation).

### S6 — Deploy: single-env; V5-C local re-exec
- **CURRENT:** single-env is trivially true (Fly/staging scripts deleted with the JS-drop;
  `scripts/` now holds only build/verify tooling). **V5-C local re-exec verifier: NOT BUILT** —
  no independent-context re-execution harness exists anywhere.
- **GAP:** build the local re-exec check (re-run `cargo test` in a clean checkout of the diff,
  emit RED|GREEN with rationale) and require it on red-line paths (V5-C). Dev-time, per SCOPE RULE.

### S7/S8/D7 — Observability: local tracing + typed CPU/GPU metrics; OTel opt-in local-only
- **CURRENT:** SPLIT.
  - Local tracing: BUILT for the dowiz kernel — `tracing = "0.1"` + `tracing-subscriber`
    (env-filter) as real deps (`kernel/Cargo.toml`), spans on hot paths
    (`order_machine.rs:144-151` `fold_transitions` span). Note: `KERNEL-OBSERVABILITY-DECART-2026-07-15.md`
    **rejected** tracing *for bebop rust-core* (empty-import wasm gate) and chose C-ABI counter
    exports + ring-buffer upgrade path — not a contradiction (different crates, different
    constraints) but S7 should name which surface gets which mechanism.
  - Typed CPU/GPU per-process metrics: **NOT BUILT in Rust.** `kernel/src/telemetry.rs` is trigram
    pattern-surfacing over tool-outcome tokens (self-improvement loop), not process metrics. The
    only resource sampling is bash host-level (`tools/telemetry/lib.sh:146` `resource_sample()` —
    load/mem/disk via /proc; `bench_run()` wall-ms + peak-RSS). No GPU exists on the host; "GPU
    metrics" today = typed `None`.
  - OTel: NOT BUILT anywhere (grep clean) — which *is* the M8-compliant default (opt-in only).
- **AMBIGUITY (flag):** ARCHITECTURE.md never defines S7 vs S8 separately — they appear only as
  the joint line "Observability (S7/S8/D7/M8)". Proposed split for the roadmap: S7 = tracing
  spans/events, S8 = typed numeric metrics. Needs operator confirmation or a canon edit.
- **GAP:** a Rust typed-metrics module (per-process CPU from `/proc/self/stat`, GPU as typed
  Option) + local sink through the already-built pure `spool.rs` state machine + strict type
  filter on log lines (F32).

### S9 — Money: integer + event-sourcing + saga-compensation; canonical-repo blocking CI gate
- **CURRENT:** ONE THIRD BUILT.
  - Integer money: GREEN — `money.rs` full integer minor-units, currency-tagged `Money` (M5 guard),
    overflow-safe, 25+ tests incl. RED overflow cases.
  - Event-sourcing: PARTIAL — a generic content-addressed event log with decide-gate exists
    (`event_log.rs` `commit_after_decide`), and `fold_transitions` is a replay reducer, **but
    money is not evented**: there is no money ledger-entry event type, and the log itself has the
    dedup bug (§0.2) plus a non-durable `MemEventStore` only (the `PgEventStore` is an `innovate:`
    marker, `event_log.rs:16-20`).
  - Saga-compensation: **NOT BUILT.** `order_machine.rs:64-78` — `InDelivery → [Delivered]` only;
    `Delivered/Rejected/Cancelled/PickedUp` all terminal with no compensating transitions; grep for
    saga/reversal/compensating-credit across kernel+engine+tools = zero. `money.rs` has no reversal
    primitive. Exactly as P0-A4 diagnosed; still open.
  - Blocking CI gate: **ABSENT** — no cargo-test job in CI at all (§0.1), so even the built money
    invariants are not CI-enforced.
- **GAP:** P0-A4 (compensation states + reversal primitive + drift-gate deliberate re-key — the
  golden-signature gate at `order_machine.rs:390` will go RED and must be re-keyed *explicitly*),
  money-as-events wiring, and the CI gate itself. This is the red-line domain: V5-C verifier
  required on the diff.

### D1 — DB: native vectorless DEFAULT; hub MAY promote pgrust
- **CURRENT:** DEFAULT is BUILT and clean: `retrieval/memory_store.rs` `InMemoryStore` (std-only,
  deterministic `snapshot_root` at `:36,85,207`), retrieval stack native (`bm25.rs`, `ppr.rs`,
  `index.rs`, `diffusion.rs`, `recall.rs` — no vector DB, no embeddings dep). pgrust adapter is
  feature-gated real code (`kernel/Cargo.toml` `pgrust = ["dep:sqlx", "dep:tokio"]`, opt-in
  `migrate()` DDL) — **but has never run against a real pgrust server**: the binary is not
  installed, and `internal-retrieval-living-memory-blueprint.md:88-90` records pgrust upstream as
  "~67% compat, extensions incomplete". Living-memory tier/TTL schema (tier col, demote-never-
  delete) is blueprint-only.
- **GAP:** none for the canonical default (compliant as-is). pgrust = E26's gap (below). SCOPE
  RULE: pgrust promotion is a hub's free choice, not a canon requirement.

### D6 — Patterns: Trait-as-Port | content-addressing | eqc | deny-by-default | event-sourcing | closure-criterion | DECART-gate
- **CURRENT:** the strongest anchor in the cluster.
  - Trait-as-Port: GREEN — `EventStore` (`event_log.rs:160`), `BlockStore` (`backup.rs:29`),
    `MemoryStore` (`retrieval/memory_store.rs`), engine `gpu` feature with **honest Err stub**
    (`engine/Cargo.toml` gpu = [], `bridge.rs::gpu::new_gpu` returns Err — the boundary is real,
    the adapter waits for network to cache wgpu).
  - Content-addressing sha3: GREEN — in-tree zero-dep SHA3-256 w/ FIPS-202 KAT
    (`event_log.rs:30-125,333`), CDC chunker + dedup + exact-restore backup organ (`backup.rs`).
  - eqc VERIFIED-BY-MATH: GREEN as harness — `tools/eqc/` (Python sympy → emitted Rust proof
    programs, commit `c7c1e0f5`) + the live `eqc-proofs` CI job (ci.yml:24-58) that regenerates a
    proof, compiles it with rustc and asserts GREEN. **But no persistent `eqc-proofs/` directory
    exists** — P0-A5's referenced `eqc-proofs/lambda_max_of_d.rs` is not in this repo (flag: the
    blueprint cites a pattern-file that doesn't exist; the I-FINAL proof needs a decided home,
    likely bebop-repo).
  - deny-by-default: PARTIAL in this repo (isolation gate fail-closed; capability enforcement is
    bebop's cluster).
  - event-sourcing: PARTIAL (see S9).
  - closure-criterion + DECART-gate: PROCESS-ONLY — real DECART reports exist
    (`KERNEL-OBSERVABILITY-DECART-2026-07-15.md`, rsa-triage `innovate:` marker in
    `kernel/Cargo.toml`) but nothing automated checks that a new dep carries one.
- **GAP:** a cheap CI lint: new `[dependencies]` line in a diff without a DECART doc reference
  fails the canonical build (dev-time fence).

### M8 / F31 / F32 / F39 — local-only typed metrics, no surveillance
- **CURRENT:** see S7/S8. F31 (per-process CPU+GPU) NOT BUILT; F32 (strict type-filter per log
  line) PARTIAL (tracing env-filter is level/target filtering, not a typed schema; bash telemetry
  emits ad-hoc JSON); F39 (no remote Grafana by default) VACUOUSLY GREEN (no Grafana exists —
  nothing enforces the default, but nothing violates it).
- **TENSION (flag):** the Telegram bridge ships metrics off-host today (§0.4). Resolution that
  keeps both truths: classify Telegram self-report as **F40 operator-opt-in self-reporting** —
  which then *requires* the F40 signed envelope (ML-DSA, reusing the bebop2 stack) and an explicit
  opt-in marker, or it stays a letter-violation of M8's "NEVER exfiltrated".
- **F36 (claim-latency anomaly alert) / V5-B:** NOT BUILT — zero hits for claim-latency anywhere
  in repo or tooling.
- **F40 (signed-envelope self-report):** NOT BUILT — telemetry→Telegram is unsigned; ML-DSA
  signing exists only in bebop2 (other cluster).

### V2 — Tech stack as law (+ DECART escape)
- **CURRENT:** LAW IS WRITTEN AND CODE LARGELY OBEYS IT. Rust/WASM kernel core (`Cargo.toml`
  cdylib+rlib, `wasm` feature), Trait-as-Port + sha3 (D6 above), GPU behind port only (engine
  `gpu = []` honest stub; wgpu absent from cache — W21 documented ceiling), zero-OCI excludes k8s
  (Dockerfile scratch + gate script), no GraphQL, NO-COURIER-SCORING asserted in code comments
  (`event_log.rs:22-23`). Permanent-rejections list lives in ARCHITECTURE.md §1 +
  SYNTHESIZED-BLUEPRINT §5.
- **GAP (dev-time, SCOPE RULE):** enforcement is honor-system — no CI check catches a k8s manifest,
  a GraphQL server dep, or an undocumented new dep. The D6 DECART-lint covers the practical hole.

### V3 — Governance gates as blocking CI (+ falsifiable reinstatement triggers)
- **CURRENT:** REGRESSED-OPEN. Present in CI: telemetry selftest, Markov health smoke, eqc proof
  job, plus `safety-floor.yml` (runs `.claude/hooks/verify-safety-floor.sh` on every push — one
  genuine survivor) and `skill-security.yml`/`heartbeat-monitor.yml`/`visual.yml` (peripheral).
  Absent: gitleaks (dropped, §0.1), cargo-test, i18n gate, IDOR gate, OTP gate, dormant bebop
  wires (those live as pre-commit/CI in bebop-repo `feat/logic-governance`, not dowiz).
  Trigger-discipline exemplar EXISTS and is good: the rsa/RUSTSEC-2023-0071 triage
  (`kernel/Cargo.toml` `innovate:` block — named condition: "`cargo tree -i rsa` shows a real
  path OR a patched rsa release ships").
- **AMBIGUITY (flag):** V3 names i18n/IDOR/OTP gates whose **target surface was deleted** with
  apps/api (`otp.ts`, owner routes are gone; `apps/web` SPA survives). Restoring them verbatim is
  impossible; they must be **re-scoped** to the current surfaces (i18n → apps/web locale
  completeness; IDOR/OTP → carried as dormant wires that re-arm when a server API surface
  reappears, each with a written reinstatement trigger in the rsa-triage form).
- **GAP:** restore gitleaks + add cargo-test (kernel+engine, `--offline` with vendored/cached
  deps) + zero-OCI job; write the re-scoped trigger docs for i18n/IDOR/OTP; port the dormant
  bebop wire checks that apply to dowiz files.

### V5 — Verification: claim-latency stat + V1-B verifier on red-line; VERIFIED-BY-MATH stays
- **CURRENT:** V5-B claim-latency: NOT BUILT (no logging of diff-landing→GREEN-claim time
  anywhere). V5-C independent re-exec verifier: NOT BUILT. VERIFIED-BY-MATH: alive (eqc CI job;
  RED→GREEN test style pervasive in kernel).
- **GAP:** claim-latency ledger (append per commit: commit-ts, first-green-claim-ts, delta;
  anomaly flag on deltas like the recorded 52s-on-1610-line-diff pattern) + the V5-C local
  re-exec harness required on money/orders/auth diffs. Both dev-time fences (SCOPE RULE).

### E21–E25 — Compute/GPU
- **E21 GPU offline/behind-port:** BUILT as boundary — engine `gpu` feature + honest Err stub;
  no GPU dep in any default graph (verified in Cargo.toml comments + W20). GREEN within the
  documented W21 ceiling (wgpu uncached offline; trigger = network `cargo add wgpu`).
- **E22 Modal scale-to-zero:** NOT BUILT — pricing verified ($0.001097/s H100) but no
  `SplatReconstructionJob`-style port, no adapter, no budget ceiling code.
- **E23 webgl/webgpu feature-gated:** NOT BUILT — engine features contain only `gpu`; no
  `webgl`/`webgpu`/`splat` features yet (P1-B2 spec exists in SYNTHESIZED §3).
- **E24 SIMD f64x4:** PARTIAL — `kernel/src/householder.rs:28-62` has a runtime-detected AVX2
  FMA dot-product kernel (`_mm256_fmadd_pd`, `is_x86_feature_detected!("fma")`, scalar fallback).
  Missing: the struct-of-arrays batch lane (N Kalman filters across couriers), softmax
  SIMD-reduction — the P1 items from SYSTEMS §6.6.
- **E25 NUMA core-pinning:** NOT BUILT — no core_affinity/hwloc anywhere. Candidate crates
  (web-verified 2026-07-16): `core_affinity` (simple cross-platform pinning) vs `hwlocality`
  (hwloc bindings, real NUMA topology + memory binding). Either is a NEW DEP ⇒ DECART report
  required per D6/V2. Single-socket Hetzner host today: measure before adopting (the falsifiable
  comparison V2 demands).

### E26–E30 / F37 / F38 — Storage
- **E26 pgrust = backup/fallback:** SPEC + ADAPTER ONLY. `deploy/pgrust.{service,env,toml}` +
  feature-gated sqlx `PgStore` exist; the pgrust **binary is not installed** and has never been
  exercised. ops-reliability plan (operator decision: "pgrust одразу") predates the SCOPE-RULE
  re-scope to backup/fallback — the plan doc should be marked superseded on that point.
- **E27/F38 COLD zstd:** OPERATIONAL, NOT TOOLED — real archives exist
  (`/root/.backups/cold/{buckets-c,claude-projects,state-db}-2026-07-16.tar.zst` +
  `state-db-preprune-1236.db`), produced by terminal ops. No in-repo archiver, no restore-verify
  harness (the 3-2-1-1-0 "0 errors on restore-verify" leg is manual).
- **E28 event-replay:** PARTIAL — the pure replay machinery exists (`fold_transitions`,
  content-addressed EventLog) but there is no durable store to replay *from* (MemEventStore only)
  and the dedup bug (§0.2) must be fixed before any durable adapter persists wrong ids.
- **E29 sha3 cache:** GREEN in-memory — `backup.rs` BlockStore/BackupOrgan (dedup + bit-exact
  restore, Verified-by-Math tests); no disk adapter yet (`MemStore` only, trait seam ready).
- **E30/F37 deep-clean cron:** TOOL BUILT AND VERIFIED — `tools/deep-clean` (rusqlite+std;
  vacuum/clean/prune/all, dry-run default, hard deny-list `main.rs:22`, commit `37f2bf2a5`
  **confirmed on this branch's ancestry**, 46-line prune diff). Scheduling = Hermes host cronjobs
  (daily --commit --days 7 + weekly dry-run audit) — **outside the repo**; no systemd timer unit
  in-repo, so a fresh hub cannot reproduce the schedule from the canon.

### F33–F35 — Cost
- **F33 TokenBucket GPU-budget throttle:** NOT BUILT in Rust (the old TS token-bucket rate-limiter
  is atticked per ops-reliability inventory). 
- **F34 Modal H100 billing + budget ceiling:** NOT BUILT (see E22).
- **F35 tiny-model-on-edge:** NOT BUILT in dowiz; the adjacent model-tier routing compute lives in
  the third repo (hermes-kernel, HK05) and is dev-tooling, explicitly NOT a product feature
  (SYNTHESIZED §2 Cluster C caveat) — do not conflate.

---

## 2. Ambiguities / underspecifications (honest list)

1. **S7 vs S8 are never individually defined** — only ever the joint line. Proposed: S7=tracing,
   S8=typed metrics. Needs canon confirmation.
2. **V3's i18n/IDOR/OTP gates target deleted code** (apps/api dropped). Must be re-scoped, not
   "restored" (each with a written reinstatement trigger).
3. **P0-A2's acceptance criteria are too weak** — "hash once" alone can preserve the dedup bug.
   The fix must chain `prev` *before* the duplicate check; acceptance needs the
   replay-on-non-empty-log RED test.
4. **P0-A1's framing is imprecise for field_frame.rs** — the per-step cost there is an O(n)
   stencil + **two fresh Vec allocations per step** (`field_frame.rs:143-144`), not an O(n³)
   eigendecomposition. The eigendecomposition-recompute gap is real for `spectral.rs` consumers
   (harmonic/hydraulic/markov surfaces) — cache keyed by `snapshot_root`
   (`retrieval/memory_store.rs:36`) applies there. For the 5-point Neumann grid Laplacian the
   eigenbasis is closed-form (DCT modes) — the "cache" may be analytic, not numeric. Roadmap both
   precisely: (a) allocation-free `step()` via buffer reuse, (b) spectral-decomposition cache with
   snapshot_root invalidation.
5. **P0-A5 cites `eqc-proofs/lambda_max_of_d.rs` which does not exist** in dowiz (no eqc-proofs/
   dir anywhere). The I-FINAL proof needs a decided home (bebop-repo consensus path vs dowiz
   tools/eqc emitted-proof dir) before it can be built "alongside the existing pattern".
6. **P0-A6's `backend "pg"`-on-pgrust is at risk:** OpenTofu's pg backend requires **Postgres
   advisory locks** (state locking; per-database global mechanism, sequence in the public schema)
   — pgrust is ~67% compatible with incomplete extension support and isn't installed. Falsifier
   before building: prove `pg_advisory_lock` works on pgrust, else use stock Postgres or a local
   file backend for tofu state. Sources: [OpenTofu pg backend](https://opentofu.org/docs/language/settings/backends/pg/),
   [OpenTofu state locking](https://opentofu.org/docs/language/state/locking/). The libvirt
   provider itself (`dmacvicar/libvirt`) is the standard module path and host KVM is already
   probed by `kernel/src/isolation/microvm.rs`.
7. **M8 vs Telegram telemetry** (§0.4) — needs the F40 signed-envelope + explicit-opt-in
   resolution to be letter-compliant.
8. **`estimate_order_total` swallows tax overflow to 0** (`money.rs:218`) — display-mirror only,
   but a silent default in money-adjacent code; decide deliberately.
9. **F39 is vacuous** — nothing enforces "no remote Grafana"; it's true because nothing exists.
   Fine, but don't count it as a built control.

---

## 3. Build phases (ordered; every anchor of this cluster lands in exactly one primary phase)

> Dependency spine: **you cannot verify anything until CI tells the truth (K1) → the red-line
> domain must be correct before it is persisted (K2) → budgets and alerts need a typed metrics
> substrate (K3) → compute optimizations need the budget/throttle rails (K4) → durable storage
> must not persist a buggy event id, and IaC state needs a verified backend (K5).**

### Phase K1 — CI TRUTH FLOOR (dev-time fences restored)
- **Anchors:** V3, V5-B, S3(gitleaks half), S1(zero-OCI gate wiring), S6(V5-C harness), S9(CI-gate
  half), V2(enforcement lint), D6(DECART-dep lint), F36(ledger half).
- **Scope:** add to `.github/workflows/ci.yml`: (1) `cargo test --offline` kernel+engine (vendored
  deps), (2) gitleaks job restored from `b10a7bfe3`, (3) `scripts/check-zero-oci.sh` job,
  (4) DECART-dep lint (new `[dependencies]` line without a DECART doc ⇒ RED), (5) claim-latency
  ledger appender (commit-ts → green-claim-ts). Build the V5-C local re-exec harness (independent
  clean-checkout re-run, RED|GREEN + rationale). Write re-scoped reinstatement-trigger docs for
  i18n/IDOR/OTP in the rsa-triage form. Fix the stale comment in `check-zero-oci.sh:8`.
- **Falsifiable done-test:** a planted secret, a failing kernel test, an nginx `FROM` line, and an
  un-DECARTed dep each independently turn CI RED on a probe branch; the claim-latency ledger has
  one entry per new commit; SCOPE RULE note present in every gate's header.

### Phase K2 — MONEY-LAW CLOSURE (red-line correctness before persistence)
- **Anchors:** S9(core), S5, D6(event-sourcing), E28(replay half), plus P0-A2 and P0-A4.
- **Scope:** (1) event_log fix — bind `prev` before dedup check, hash once
  (`event_log.rs:275-294`), with the RED replay-on-non-empty-log test; (2) FSM compensation states
  + transitions (`order_machine.rs:64-78`) with **deliberate** golden-signature drift-gate re-key
  (`order_machine.rs:390`); (3) `money.rs` reversal/compensating-credit primitive; (4) money
  ledger-entry event type committed through `commit_after_decide`; (5) document the engine
  panic-as-fail-closed exception; decide the `money.rs:218` silent-default. Runs under K1's V5-C
  verifier (first real consumer — red-line diff).
- **Falsifiable done-test:** cancel-after-confirm reaches a terminal compensated state whose
  ledger entries net to exactly zero; the replay RED test fails on pre-fix code and passes after;
  drift-gate re-key is a named, reviewed diff (not a silent constant change).

### Phase K3 — TYPED LOCAL OBSERVABILITY (M8 made real, not vacuous)
- **Anchors:** M8, S7, S8, D7, F31, F32, F36(alert half), F39, F40, V5-B(anomaly consumption).
- **Scope:** Rust typed-metrics module (per-process CPU from `/proc/self`, GPU as typed
  Option-none until hardware exists); strict typed log-line schema + filter (reject-on-untyped);
  local JSONL sink through a `spool.rs` file adapter (crash-safe, matching the pure state machine
  already tested); claim-latency anomaly alert (consumes K1's ledger); ML-DSA **signed envelope**
  for the Telegram self-report + explicit operator-opt-in marker for ANY remote sink (Telegram,
  future OTel — local sink default); document S7/S8 split + per-surface mechanism (dowiz kernel =
  tracing; bebop-core = C-ABI counters per its DECART).
- **Falsifiable done-test:** metrics accrue locally with zero network; an untyped log line is
  provably rejected; killing the process mid-write loses zero acked spool records; the Telegram
  report carries a signature that verifies against the operator key and fails on 1-bit corruption;
  grep proves no metric leaves the host without the opt-in marker set.

### Phase K4 — COMPUTE BUDGET & CACHE (E-compute + P0-A1, throttles on K3 rails)
- **Anchors:** E21, E22, E23, E24, E25, F33, F34, F35, D6(content-addressed cache), P0-A1.
- **Scope:** (1) spectral-decomposition cache keyed by `snapshot_root`, invalidate-on-topology-
  change, + allocation-free `FieldFrame::step` (reused buffers) — the two precise halves of
  P0-A1 (§2.4); (2) heavy one-shot ops (spectral decomp, backup, re-index) routed through
  `spool.rs` with a MAX_SUBSTEPS-style bounded drainer (`engine/loop_.rs:25` pattern);
  (3) Rust TokenBucket (F33) driving a GPU/spend budget ceiling; (4) Modal adapter behind a job
  port (E22/F34: per-second billing, scale-to-zero, mandatory-teardown watchdog, degrade-closed
  monthly ceiling) — feature-gated, offline default = honest Err stub exactly like `gpu`;
  (5) `webgl`/`webgpu` cargo features (empty, `default = []` unchanged) (E23); (6) SoA f64x4/FMA
  batch lane for N-courier Kalman + softmax reduction, extending the proven
  `householder.rs` runtime-detection pattern (E24); (7) NUMA/core-pinning behind a trait port,
  crate chosen by DECART bake-off (`core_affinity` vs `hwlocality`), adopted only on a measured
  win (E25/F35 scheduling). Every new dep ⇒ DECART report (K1 lint enforces).
- **Falsifiable done-test:** unchanged topology ⇒ decomposition-count counter stays 0 across
  1000 steps and a snapshot_root change makes it exactly 1; default `cargo build` dependency
  graph byte-identical to today; TokenBucket property test (never exceeds budget under
  concurrency); Modal stub returns Err offline and the ceiling degrade-closes in a fake-billing
  test; SIMD lane bit-identical to scalar reference.

### Phase K5 — DURABLE STORAGE & IaC SPINE (persist only what K2 made correct)
- **Anchors:** D1(confirm), E26, E27, E28(durable half), E29, E30, F37, F38, S1(systemd units +
  microVM launcher gate), S2(confirm), S4(contract), S6(deploy unit).
- **Scope:** (1) disk `BlockStore` adapter (content-addressed file store) behind `backup.rs`'s
  trait; (2) COLD `zstd` archiver + **restore-verify** subcommand added to `tools/deep-clean`
  (or sibling), replacing terminal-only ops (E27/F38, 3-2-1-1-0's "0 errors" leg automated);
  (3) durable `EventStore` adapter (file-JSONL first; `PgEventStore` per the `event_log.rs:16`
  innovate marker once pgrust is proven) — **strictly after K2's dedup fix**; (4) pgrust
  install+smoke as backup/fallback (E26; SCOPE RULE: promotion stays a hub choice) incl. the
  advisory-lock falsifier (§2.6); (5) in-repo systemd units + timers: native-spa-server unit,
  deep-clean timer (E30 — schedule becomes reproducible from canon, not Hermes-host-only);
  (6) `opentofu/` first step: one `dmacvicar/libvirt` `libvirt_domain` module reproducing one
  KVM microVM (gated on `microvm.rs` probe), state backend = stock Postgres or local file until
  the pgrust advisory-lock falsifier passes (P0-A6, de-risked); (7) S4: write the first internal
  `.proto` contract (event-sync/node-control) + tonic DECART report — implementation lands with
  the mesh cluster's node binary, the *contract* is this cluster's deliverable.
- **Falsifiable done-test:** `kill -9` mid-append loses zero acked events and replay reproduces
  the exact tip id; COLD archive → restore-verify is byte-identical (backup.rs round-trip on the
  real archive); `tofu apply` creates one microVM from a clean checkout and `tofu plan` is then
  empty; `systemctl list-timers` shows deep-clean from the in-repo unit; the .proto compiles and
  a golden-encoding test pins the wire bytes.

**Coverage check:** S1(K1,K5) S2(K5) S3(K1) S4(K5) S5(K2) S6(K1,K5) S7(K3) S8(K3) S9(K1,K2)
D1(K5) D6(K1,K2,K4) D7(K3) M8(K3) V2(K1,K4) V3(K1) V5(K1,K2,K3) F31-32(K3) F33-35(K4) F36(K1,K3)
F37-38(K5) F39-40(K3) E21-25(K4) E26-30(K5, E28 also K2). No anchor deferred outside a phase.

---

## 4. Evidence appendix (quick refs)

| Claim | Evidence |
|---|---|
| double-hash + dedup-id divergence | `kernel/src/event_log.rs:253-258` vs `:284-292` |
| happy-path-only FSM | `kernel/src/order_machine.rs:64-78`; drift-gate `:390` |
| no money reversal | `kernel/src/money.rs` (checked_add only, `:70-87`) |
| laplacian per-step + allocs | `engine/src/field_frame.rs:139-156` |
| MAX_SUBSTEPS drainer pattern | `engine/src/loop_.rs:25,68-77` |
| CI has no gitleaks/cargo-test | `.github/workflows/ci.yml` (2 jobs only); regression commit `f9ab28ff1`; prior gate `b10a7bfe3` |
| zero-OCI Dockerfile | `Dockerfile:53` (`FROM scratch`); gate `scripts/check-zero-oci.sh` |
| gitleaks config alive | `.gitleaks.toml` (repo root) |
| systemd EnvFile pattern | `deploy/pgrust.service`, `deploy/pgrust.env`; binary absent on host |
| microVM probe | `kernel/src/isolation/microvm.rs` |
| tracing in kernel | `kernel/Cargo.toml` (tracing, tracing-subscriber); span at `order_machine.rs:144` |
| FMA SIMD | `kernel/src/householder.rs:28-62` |
| deep-clean verified | `tools/deep-clean/src/main.rs`; commit `37f2bf2a5` on branch ancestry |
| eqc harness + CI job | `tools/eqc/eqc.py` (commit `c7c1e0f5`); `ci.yml:24-58` |
| COLD archives real | `/root/.backups/cold/*.tar.zst` (host, not repo) |
| test truth | kernel 337/0, engine 47/0 (cargo test --offline, 2026-07-16) |
| OpenTofu pg advisory locks | opentofu.org/docs/language/settings/backends/pg/ |
| NUMA crates | crates.io/crates/core_affinity; docs.rs/hwlocality |
