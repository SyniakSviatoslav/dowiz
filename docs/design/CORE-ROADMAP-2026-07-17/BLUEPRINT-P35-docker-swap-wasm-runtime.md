# BLUEPRINT P35 — Docker-swap: register DK-01..10, finish DK-04/05/06/08 (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). This phase IS `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §10.5.2's **P35** — the registration-and-completion home for the zero-OCI runtime subsystem
> (DK-01..DK-10). Sibling of `BLUEPRINT-P34-mesh-kernel-wiring.md` and
> `BLUEPRINT-P34B-mesh-remaining-halves.md`; independent lane, no ordering dependency on the
> mesh wiring (§10.5.2: "runs parallel to P34"). Source blueprints reused, not re-derived:
> `docs/design/docker-swap/BLUEPRINTS-DOCKER-SWAP.md` + `DOCKER-SWAP-PLAN.md`; DK-07 is
> DECIDED by `docs/design/microvm-isolation/ADR-NO-SANDBOX-AGENT-GOVERNANCE.md` and is cited,
> not reopened.
>
> **Headline ground-truth finding of this pass:** §10.5.2's registration gap (DoD-1) is
> **already closed** — `CORE-ROADMAP-INDEX.md:153` carries the P35/DK row — and DK-04/DK-05
> are further along than the PARTIAL status recorded: DK-04's server is a real tested axum
> crate WITH a committed systemd unit, and DK-05's native deployment config + RED gate + an
> isolation-model README already exist. The honest remaining build surface is: **DK-06's
> actual microVM launch** (the probe's own comment still admits "the next unit"), **DK-08's
> SBOM/sign supply chain** (zero syft/cosign hits in either repo), one **stale hub/per-node
> wording fix** in DK-05's README (co-owned with P34B V-1a), and **DK-10's ledger
> aggregation**.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `f9b2eb9bb` and `bebop-repo` `main` @ `e56ba6a`
(= `openbebop/main`). Bebop paths relative to `/root/bebop-repo/`, dowiz paths relative to
`/root/dowiz/`.

| # | Claim | Fresh `file:line` (this pass) | Inherited claim | Status |
|---|---|---|---|---|
| 1 | Registration (DoD-1) | `docs/design/CORE-ROADMAP-INDEX.md:153`: "docker-swap/ \| P35 \| DK-01..10; DK-01/02/03/07 DONE with real `wasip2` component + deny-by-default WASI host — previously zero index reference despite working code" | §10.5.2 DoD-1: index registration required; "falsified by grep finding no DK-* reference" | **ALREADY DONE** — the falsifier grep now passes; §5 keeps it as a pinned row, not new work |
| 2 | DK-01 toolchain DONE | `bebop2/tooling/build-wasm-component.sh` exists (bebop-repo); `bebop2/ports/telegram/Cargo.toml:9` cites it, `:15-16` carries the `[target.wasm32-wasip2]` config | §10.5.2: DK-01 DONE | **MATCH** — the *supply-chain half* of DK-01's original spec (SBOM/sign) was always DK-08's wave; still open (row 8) |
| 3 | DK-02 component DONE | `bebop2/ports/telegram/Cargo.toml:6`: "DK-02 — Telegram Notify port as a wasm32-wasip2 component. Zero-ambient-authority: only the `notify-telegram` host import (Scope Order::Notify)" | §10.5.2: DK-02 DONE, `bebop2/ports/telegram/src/lib.rs` | **MATCH** |
| 4 | DK-03 host DONE | `bebop2/wasm-host/Cargo.toml:6`: "DK-03 — wasmtime host mapping Scope->WASI imports, deny-by-default"; wasmtime OPT-IN (`:19-26`: default build carries NO wasmtime, compiles to a deny-by-default stub returning `WasmRuntimeDisabled`); `src/lib.rs` = 297 lines | §10.5.2: DK-03 DONE, "Scope→WASI deny-by-default host (wasmtime feature-gated)" | **MATCH exact** |
| 5 | DK-07 DECIDED | `docs/design/microvm-isolation/ADR-NO-SANDBOX-AGENT-GOVERNANCE.md` — ACCEPTED 2026-07-13; agent-governance = pure text/voice policy filter, executes nothing untrusted, "there is nothing to sandbox"; grep-proof falsifier recorded in its §Open-items | §10.5.2: DK-07 resolved by this ADR | **MATCH — cited, closed, NOT reopened** (anti-scope) |
| 6 | 🔴 DK-06 still probe-only | `kernel/src/isolation/microvm.rs` (177 lines): `kvm_available` `:52`, `can_accept_native_adapter` `:61`, `register_adapter` fail-closed `:76`; the comment `:14-16` still reads "today we only probe host capability. The actual VMM launch (jailer, seccomp, guest kernel, network tap) is the next unit"; tests `r1_kvm_unavailable_on_this_host` `:125` … `r4_cannot_accept_native_adapter_without_kvm` `:156`. This host: `ls /dev/kvm` → No such file (probe correctly false, fail-closed posture live) | §10.5.2 DoD-2: "that comment being still-true is the fail condition" | **CONFIRMED still true** — DK-06 is P35's one substantial build unit (§3.1). Note: the refusal HALF of DK-06's RED is already tested (r1-r4); only the launch half is missing |
| 7 | DK-04 further than PARTIAL | `tools/native-spa-server/` — real axum crate (`Cargo.toml`: "DK-04: native-Rust static SPA server (axum) replacing the nginx container. Zero-OCI runtime artifact"), with `tests/` (in-repo RED tests for SPA fallback / cache headers / security headers / compression, per its own manifest note); `deploy/native-spa-server.service` systemd unit committed | §10.5.2 DoD-4: "promoted to a deployed serving path (proven by one staging deploy) or descoped with a dated note. No third state" | **DRIFT in the DoD's own premise** — "staging deploy" targets Fly, which was DECOMMISSIONED (CORE-ROADMAP-STANDARD §1: deployment target = decentralized local nodes). §3.2 re-anchors the DoD honestly |
| 8 | 🔴 DK-08 fully open | `grep -rn "syft\|cosign\|sbom" {dowiz,bebop-repo}/.github/workflows/` → 0 hits in both repos | §10.5.2 DoD-3: "zero hits exist in any workflow file today" | **MATCH — still zero** (§3.3) |
| 9 | DK-05 substantially DONE | `deploy/pgrust.{service,toml,env}` (systemd, hardened directives, `rls.cross_tenant = "deny"`); `deploy/check-no-docker.sh` — a real RED gate ("Exits non-zero if ExecStart references docker/podman/nerdctl/containerd"); `deploy/README.md` — the isolation-model decision text (process model + app-level RLS gate; microVM = opt-in defense-in-depth, "NOT the default and NOT required by DK-05") | §10.5.2 DoD-5: "decision note written, cross-referencing P34B DoD-4" | **MOSTLY DONE** — the note exists in substance; two gaps: `README.md:4` calls pgrust "the per-node source-of-truth" (the exact hub/per-node conflation P34B DoD-4 forbids — correction co-owned with P34B V-1a) and the P34B cross-reference is absent (§3.4) |
| 10 | DK-09 dev-only | `BLUEPRINTS-DOCKER-SWAP.md:86-92` — dev ergonomics, its own RED column says "N/A (dev)" | — | registration-only; no DoD beyond the index row (§1) |
| 11 | Node/Docker cruft adjacency (P36's, cited not owned) | bebop-repo root still has `package.json`, `docker-compose.sovereign.yml`, `Dockerfile.sovereign`; `scripts/build-unikernel.sh` REFERENCES the compose file (verify-before-delete catch) | §10.5.2 P36 DoD-4 owns the purge | **boundary pinned** — P35 deletes nothing in bebop-repo; the compose-file reference is handed to P36 as a named verify-before-delete fact (§7) |
| 12 | P36 cross-wire (toolchain, not same build) | the P36 `no_std` regression target is `wasm32-unknown-unknown` for `bebop2-core`; DK components are `wasm32-wasip2` — "related toolchain, not the same build" | §10.5.2 P35 dependency note, verbatim | **MATCH** — no P35 item gates on P36 |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — what P35 owns and what it deliberately does NOT own

**P35's single sentence:** finish the zero-OCI runtime subsystem to its own blueprint's bar —
an actual microVM boots a workload under a fail-closed KVM gate (DK-06), the build emits an
SBOM and signed artifacts (DK-08), DK-04/DK-05 get their honest closure notes against the
post-Fly deployment reality, and DK-10's proofs land as permanent ledger rows — with
DK-01/02/03/07 cited as done and the index row (already live) pinned.

**P35 owns (build items §3):**

| Item | §10.5.2 DoD | Content |
|---|---|---|
| K-1 | DoD-2 (DK-06) | Firecracker launch unit: one microVM boots one workload in CI; fail-closed refusal path (already tested r1-r4) preserved and extended |
| K-2 | DoD-3 (DK-08) | CI supply chain: syft SBOM + cosign signature over the two real zero-OCI artifacts (wasip2 component, native-spa-server binary); zero-OCI assertion |
| K-3 | DoD-4 (DK-04) | The dated decision note re-anchoring DK-04's serving target post-Fly + a falsifiable local serve-proof |
| K-4 | DoD-5 (DK-05) | Close DK-05's note: fix the hub/per-node wording (co-owned P34B V-1a), add the P34B cross-reference |
| K-5 | (DK-10) | Aggregate the DK RED-suite into permanent regression-ledger rows; fill the two missing proof gaps it surfaces |

**P35 does NOT own (anti-scope, binding — each with its owner):**

- **DK-07 is decided.** The no-sandbox-for-agent-governance ADR (§0 row 5) closes it; per
  §10.5.2's anti-scope this blueprint cites it and moves on. Any re-litigation is a
  stop-and-flag event.
- **No Kubernetes** (standing rejection, restated verbatim from §10.5.2).
- **No rewrite of DK-01/02/03 working code** "to modernize it" (§10.5.2 anti-scope; standard
  item 19 cuts both ways).
- **P36 owns the bebop-repo Node/Docker purge** — P35 deletes nothing there; §0 row 11's
  compose-reference fact is handed over, not acted on.
- **P34B owns the per-node storage decision** — K-4 consumes its §2.1 ruling for one wording
  fix; the decision itself is not restated here.
- **AGENT's sandboxed tool ports** consume the wasm-host; their port implementations are P40's
  scope. ECOSYSTEM/OPS deployment topology consumes DK-06; the topology itself is §10.5.5's.
- **Dokploy/deploy-layer decision** (BLUEPRINTS-DOCKER-SWAP.md:81-82's decision-point):
  MOOT post-Fly-decommission — no Dokploy remains in the target topology; recorded here as
  resolved-by-events, no work item.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/isolation/microvm.rs — the ONLY new production types P35 introduces ──

/// Pinned Firecracker release the launch unit drives. Bumping is a reviewed,
/// deliberate commit (same pin discipline as P34's OPENBEBOP_CI_PIN).
pub const FIRECRACKER_VERSION: &str = "v1.10.1";

/// SHA-256 pins for the guest boot artifacts (binary blobs, fetched in CI,
/// verified before use — a hash mismatch is a hard Err, never a warning).
/// Values filled at K-1 implementation time from the built/downloaded artifacts.
pub const GUEST_KERNEL_SHA256: &str = "<pinned-at-implementation>";
pub const GUEST_ROOTFS_SHA256: &str = "<pinned-at-implementation>";

/// Outcome of one microVM workload run. The ONLY success witness is the
/// workload's own marker read back over the vsock/serial channel — a VM that
/// boots but never writes the marker is a failure (no boot-equals-success lie).
#[derive(Debug, PartialEq, Eq)]
pub enum MicroVmRun {
    /// Marker bytes the guest workload wrote (must equal WORKLOAD_MARKER).
    Completed { marker: Vec<u8> },
    /// KVM absent/unusable — the fail-closed branch (register_adapter's posture).
    RefusedNoKvm,
    /// Boot or workload failure, with the collected console tail for diagnosis.
    Failed { console_tail: String },
}

/// The exact bytes the guest /init writes to prove code EXECUTED inside the VM.
pub const WORKLOAD_MARKER: &[u8] = b"dowiz-microvm-workload-ok-v1";

/// Launch a Firecracker microVM with the pinned kernel/rootfs and run the
/// marker workload. MUST consult `kvm_available()` first and return
/// `RefusedNoKvm` without touching the VMM binary when false — the launch path
/// may never weaken the r2/r4 fail-closed proofs.
pub fn run_workload(timeout: core::time::Duration) -> MicroVmRun;
```

No other new types: DK-08 is workflow YAML + two shell gates; K-3/K-4 are decision notes.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 K-1 — DK-06: an actual microVM boots and runs a workload (DoD-2)

**Spec** = §2's types. Staged honestly — the unit ends at "one VM, one workload, one marker";
jailer/seccomp-profiles/network-tap are NAMED next units (the `:14-16` comment shrinks to name
them, it does not vanish by wordsmithing).

**(a) The launch path** (`microvm.rs`, behind a new `microvm-launch` cargo feature so the
default kernel build gains zero new surface): spawn the pinned Firecracker binary with an API
socket, configure boot-source (pinned kernel + rootfs, SHA-256-verified per §2), machine-config
(1 vCPU, 128 MiB), start, read the guest marker over the serial/vsock channel, enforce
`timeout`. The guest rootfs's `/init` is a ~10-line static shell/binary that writes
`WORKLOAD_MARKER` and powers off.

**(b) RED→GREEN.** RED exists by construction on this dev host: no `/dev/kvm` →
`run_workload` returns `RefusedNoKvm` (asserted in a new test
`r5_launch_refuses_without_kvm`, sibling of r1-r4 — green HERE, and it pins the refusal
contract). The launch-success test `r6_microvm_boots_and_runs_marker_workload` is
`#[ignore]`d locally and runs in a NEW CI job on a KVM-capable runner (GitHub-hosted Linux
runners expose `/dev/kvm`):

```yaml
  microvm-launch:   # dowiz .github/workflows/ci.yml — additive job, DK-06 DoD
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo setfacl -m u:$USER:rw /dev/kvm   # runner exposes KVM; make it usable
      - run: bash kernel/src/isolation/fetch-guest-artifacts.sh   # pinned URLs + SHA-256 verify
      - run: cargo test -p dowiz-kernel --features microvm-launch -- --ignored r6_microvm
```

Honest divergence note (same class as P34 §3.1c's): this job fetches pinned binary artifacts —
a deliberate, recorded exception to the P01 offline floor, justified because guest kernels are
not cargo artifacts; the SHA-256 pins keep it reproducible. Named hardening (not P35-blocking):
build the rootfs in-repo with mkosi and vendor it.

**Adversarial (designed to break K-1):** (i) *marker-forgery teeth* — run the harness against
a deliberately-empty rootfs (no `/init` marker write): the run must return `Failed`, proving
success requires guest EXECUTION, not merely VMM exit-0; (ii) *pin teeth* — corrupt one byte
of the downloaded rootfs: the SHA-256 gate must hard-fail before any launch; (iii) *fail-closed
regression guard* — r2/r4 re-run unchanged with the feature ON, proving the launch path added
no bypass around `register_adapter`'s refusal.

### 3.2 K-3 — DK-04: the dated decision note + a local serve-proof (DoD-4)

§10.5.2's DoD-4 assumed a Fly staging deploy that no longer exists (§0 row 7). Re-anchored
decision, recorded here as the dated note DoD-4 demands:

> **DK-04 decision (2026-07-18): RETAINED as the node-local static asset server.** Its serving
> target is the P38 WebGPU/WASM bundle (the SPA it was built for is decommissioned with
> `apps/web`); promotion to a live serving path is GATED ON P38's first emitted bundle —
> a named handoff, not a silent deferral. Until then the falsifiable state is: crate + tests
> green, systemd unit committed, and a **local serve-proof** green.

The serve-proof (`tools/native-spa-server/tests/serve_proof.rs`, or extending its existing
tests): boot the binary against a fixture asset dir; assert (i) SPA fallback (unknown path →
index), (ii) CSP header present, (iii) compression negotiated — the three RED conditions of
the original DK-04 blueprint, now pinned as one named test. **Adversarial:** request
`../../etc/passwd`-style traversal → 404/refused, never file contents (the classic static-server
break; if a traversal test already exists in its suite, pin it by name in the ledger instead of
duplicating).

### 3.3 K-2 — DK-08: SBOM + signature over the real zero-OCI artifacts (DoD-3)

Spec: a CI workflow (dowiz `.github/workflows/ci.yml` additive job + a bebop-side equivalent
for the wasip2 component, routed per repo rules) that:

1. Builds the two real artifacts: `native-spa-server` release binary (dowiz) and the
   DK-02 telegram component via `bebop2/tooling/build-wasm-component.sh` (bebop).
2. Runs `syft` over each → SPDX-JSON SBOM, uploaded as a CI artifact.
3. Signs each with `cosign sign-blob` (keyless, GitHub OIDC) → `.sig` + certificate uploaded;
   a `cosign verify-blob` step in the SAME job proves the signature verifies (sign-then-verify,
   no unverified artifact claimed signed).
4. Asserts zero-OCI: the job greps its own workspace for produced OCI layouts
   (`find . -name index.json -path "*oci*"` empty) and reuses the `deploy/check-no-docker.sh`
   pattern as `scripts/ci-zero-oci.sh` — build steps reference no
   docker/podman/nerdctl/containerd.

RED→GREEN: RED is §0 row 8's zero-hits state (the falsifier grep IS the red); GREEN = both
SBOMs + verified signatures present as artifacts of a green run. **Adversarial:** *verify
teeth* — the verify step run once against a tampered blob (1 byte flipped post-sign) must
fail; recorded in the PR (proving verification checks content, not file presence). Scan
(Trivy/Grype) is the original blueprint's second wave — named follow-up with trigger (first
external dependency added to either artifact), not silently dropped.

### 3.4 K-4 — DK-05: close the decision note (DoD-5)

Two-line closure, both edits in `deploy/README.md`: (i) fix `:4` — pgrust is the **hub-tier**
store, not "the per-node source-of-truth" (this is P34B V-1a's second edit; ONE owner executes
it — coordinated in §10, not done twice); (ii) append the cross-reference §10.5.2 DoD-5
demands: hub deployment (this dir) vs per-node storage
(`BLUEPRINT-P34B-mesh-remaining-halves.md` §2.1) vs hub tenant-schema
(`BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md`) — three documents, three concerns, one
sentence each. Falsifier: `grep -n "P34B" deploy/README.md` non-empty AND
`grep -n "per-node source-of-truth" deploy/README.md` empty. The existing `check-no-docker.sh`
RED gate and hardened unit are cited as already satisfying DK-05's mechanical half — no new
code.

### 3.5 K-5 — DK-10: the RED-suite becomes permanent ledger rows

DK-10's aggregation, mapped to what exists vs what K-1..K-3 add: WR-01 component-trap +
no-ambient-authority (DK-02/03 — existing wasm-host deny-by-default tests), microVM
fail-closed (r1-r5) + launch (r6), zero-OCI (ci-zero-oci.sh + check-no-docker.sh), native-node
(the serve-proof + pgrust systemd unit). Work item: verify each named proof exists and runs in
CI (the wasm-host trap test is the one to CONFIRM by name — if the WR-01 out-of-scope-import
trap lacks a named test, K-5 adds it bebop-side as `wr01_out_of_scope_import_traps`), then
append the ledger rows (§5). No new machinery — DK-10 was always an aggregation gate.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 4.1 Hazard-safety as math (item 6)

- **Untrusted code escaping to ambient authority:** unreachable by construction on the WASM
  path — a component's reachable syscall surface IS its granted import set (wasm-host
  deny-by-default: no capability → no host import → the unsafe call is *unrepresentable in the
  instance*, not filtered at runtime); on the native path, `register_adapter` refuses
  `native-process` without KVM (`microvm.rs:76-85`) — the fallback-to-unisolated state has no
  code path (r2/r4 pin it).
- **"Boots-but-lies" success:** `MicroVmRun::Completed` requires the guest-written marker
  (§2) — a VMM that exits 0 without executing the workload cannot produce it (§3.1
  adversarial-i proves the distinguisher has teeth).
- **Supply-chain artifact swap:** SHA-256 pins gate the guest artifacts before launch; cosign
  verify gates the emitted artifacts after build; both fail hard (a mismatch is `Err`, never a
  log line).
- **Sandbox-theater inversion (DK-07):** the ADR's argument is itself a hazard-safety-as-math
  argument — isolating code that executes nothing untrusted adds attack/ops surface for zero
  reachability reduction; preserved by citation, enforced by its grep falsifier.

### 4.2 Schemas designed for scaling (item 8)

- **microVM tier:** scaling axis = concurrent untrusted-non-WASM adapters per hub; the K-1
  unit is 1 VM (128 MiB, 1 vCPU); the stated break point is adapter-count × 128 MiB vs hub
  RAM — pool/jailer design is the named next unit, triggered by the FIRST real dev-agent-tier
  adapter (none exists today; building a pool now would be scaffold).
- **SBOM:** O(dependency count); both artifacts are near-leaf (axum tree; a wasip2 component
  with one import) — no break point within this architecture's horizon.
- **Component count:** one component today (telegram). The host's Scope→import mapping is
  per-component O(1); the axis is component count and it is nowhere near a limit.

### 4.3 Isolation (11), mesh awareness (12), rollback vocabulary (13), living memory (15)

- **Isolation:** K-1 lives behind `microvm-launch` (default build unchanged — zero new deps in
  the default kernel closure); a Firecracker defect's blast radius is the feature-gated test +
  the CI job. DK-08's jobs are additive; failure blocks artifact publication, never the test
  floor. The isolation boundary NAMES itself: this whole phase is the isolation-boundary
  subsystem.
- **Mesh awareness:** node-local phase — no wire formats, no transport. The wasm-host feeds
  AGENT's tool ports (named consumer); DK-06 feeds hub deployment. Payload budgets: N/A
  (nothing crosses the mesh here).
- **Rollback (item-13 vocabulary, used precisely):** P35 claims **Self-Termination /
  unrepresentable-state** on both runtime paths (no-import-no-call; no-KVM-no-launch) — this
  is the phase's load-bearing property. Snapshot-Re-entry and Self-Healing are NOT claimed
  (a microVM run is stateless from the host's view; redundancy math lives elsewhere).
  Mechanical rollback: delete the feature-gated launch code + 2 CI jobs + revert two README
  edits — the DK-01/02/03 done-set is untouched by P35.
- **Living memory (item 15):** N/A in the data sense — no stored state with temporal access
  patterns is introduced; SBOMs/signatures are immutable CI artifacts. Claimed as a reasoned
  exemption, not skipped.

### 4.4 Linux-discipline verdict framework (item 9)

K-1 = **EXTENDS** (real VMM control is new machinery for this repo — justified by DK-06's
threat model: untrusted non-WASM code has no other home); K-2 = **REINFORCES** (the repo's
gate culture extended to provenance; sign-then-verify mirrors RED→GREEN); K-3/K-4 =
**ALREADY-EQUIVALENT** (honest-status documentation discipline); the fail-closed KVM posture =
**ALREADY-EQUIVALENT** (the kernel's own degrade-closed doctrine, already tested r1-r4).
DK-09's OrbStack/Podman guidance = **DOES-NOT-TRANSFER** to CI/prod (dev-machine-only, per its
own blueprint) — named to close the set.

### 4.5 Non-contradiction constraints (sequencing, hard)

Independent lane — no P35 item waits on P34/P34B/P36, and nothing here may serialize them
(§10.5.2). The single coordination point: K-4's README edit overlaps P34B V-1a's second edit —
exactly ONE of the two executes it (§10 assigns it to P34B's U1 if that lands first, else K-4;
the falsifier grep is shared, so double-execution is detectable and idempotent-by-content).
The P36 cross-wire stays informational: wasip2 ≠ wasm32-unknown-unknown (§0 row 12) — a P36
no_std fix neither unblocks nor blocks any DK build.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Sharpens §10.5.2's P35 DoD-1..5 (kept 1:1; DoD-1 pinned as done; DoD-4 re-anchored with its
premise drift stated):

| Item | §10.5.2 | RED (fails before) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|---|
| K-0 | DoD-1 | (was: no DK reference in the index) | **already green** (§0 row 1) | `grep -n "DK-" docs/design/CORE-ROADMAP-INDEX.md` non-empty — pinned, re-run at close-out |
| K-1 | DoD-2 | `microvm.rs:14-16` comment true; no launch path; no KVM CI job | `r6_microvm_boots_and_runs_marker_workload` green in the `microvm-launch` CI job; `r5` green locally; r1-r4 unchanged; the `:14-16` comment rewritten to name jailer/seccomp/tap as next units | falsified by the old comment surviving verbatim, by r6 passing without the marker equality, or by any weakening of r2/r4 |
| K-2 | DoD-3 | zero syft/cosign hits (§0 row 8) | SBOM + verified signature artifacts for BOTH artifacts on a green run; zero-OCI assertion green; tamper-teeth recorded | `grep -rn "syft\|cosign" .github/workflows/` non-empty in both repos; falsified by a signature claimed without the verify step |
| K-3 | DoD-4 | DoD-4's two allowed states both unreachable honestly (staging target gone) | the §3.2 dated decision recorded + `serve_proof` test green + traversal adversarial green | falsified by promotion claimed without a P38 bundle, or the note absent from this file/its close-out commit |
| K-4 | DoD-5 | README wording conflates hub/per-node; no P34B cross-ref | both edits landed | `grep -n "per-node source-of-truth" deploy/README.md` empty AND `grep -n "P34B" deploy/README.md` non-empty |
| K-5 | (DK-10) | DK proofs scattered, unledgered; WR-01 trap test unconfirmed by name | every §3.5 proof named, CI-run, ledgered | falsified by any DK-10 row whose named test does not exist in the tree |

Permanent regression rows (item 17), dowiz `docs/regressions/REGRESSION-LEDGER.md`:
(1) "P35 microVM fail-closed + launch — guardrail: r1-r6, `microvm-launch` CI job";
(2) "P35 zero-OCI supply chain — guardrail: SBOM+cosign job, `ci-zero-oci.sh`";
(3) "P35 native static serve-proof — guardrail: `serve_proof` + traversal adversarial".
Bebop ledger (created by P36 R-0): (4) "P35/DK-02 WR-01 out-of-scope-import trap — guardrail:
`wr01_out_of_scope_import_traps`". Ledger ratchet rule applies verbatim.

---

## 6. Benchmark plan (item 10) — measure the isolation tax, build no new harness

1. `microvm/boot_to_marker` — wall-clock from Firecracker spawn to marker read, measured in
   the KVM CI job (expectation to falsify, not assert: the ~125-200 ms class the ADR cites;
   the number is recorded, not assumed). This is THE decision-relevant number: it prices the
   microVM tier against the WASM tier for future adapter-placement decisions.
2. `wasm_host/instantiate_component` — component instantiation cost under the `wasm` feature
   (criterion, bebop-side) — the WASM tier's half of the same comparison.
3. `native_spa/first_byte` — serve-proof latency on the fixture dir (sanity floor only).
   Baselines recorded in the respective `BENCH_HISTORY.md`s; regression gating = P-H's
   deliverable, same dependency note as P34 §6. No GPU, no load-rig — these are placement
   prices, not throughput marketing.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.2 P35 (charter; anti-scope
restated §1) · `docs/design/docker-swap/BLUEPRINTS-DOCKER-SWAP.md` + `DOCKER-SWAP-PLAN.md`
(DK design source — reused for design, not status) ·
`docs/design/microvm-isolation/ADR-NO-SANDBOX-AGENT-GOVERNANCE.md` (DK-07, decided — cited
per anti-scope) + `MICROVM-ISOLATION-RESEARCH-2026-07-13.md` (threat-model grounding) ·
`docs/design/CORE-ROADMAP-INDEX.md:153` (DoD-1's live registration row) ·
`BLUEPRINT-P34B-mesh-remaining-halves.md` (§2.1 storage ruling consumed by K-4; shared README
edit coordinated §4.5/§10) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (pin discipline + honest
CI-divergence-note pattern reused §3.1) · `docs/regressions/REGRESSION-LEDGER.md`. Handoff TO
P36: §0 row 11 (build-unikernel.sh references docker-compose.sovereign.yml — verify before any
deletion). Memory: `docker-swap-arc-2026-07-13` (WAVE0 DONE lineage) ·
`ops-reliability-arc-2026-07-13` (degrade-closed doctrine K-1 preserves) ·
`ecosystem-strategy-arc-2026-07-13` · `rust-native-bare-metal-decision-2026-07-14` ·
`environment-and-ops-facts-2026-07-16` (Fly decommission context for §3.2's re-anchor) ·
`cross-branch-todo-map-2026-07-10` (repo routing for the bebop-side SBOM job + WR-01 test).
Supersedes: the PARTIAL/gap framing of DK-04/DK-05 and the "unregistered" framing of DoD-1
(§0 rows 1, 7, 9 are newer); the DK design source remains authoritative for design.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): one capability model at every tier —
  Scope→WASI imports (WASM), KVM-gated registration (native), signed provenance (build): the
  same grant-before-use shape, three substrates; no second isolation vocabulary introduced.
- **P6 CAUSE-AND-EFFECT** (determinism as law): pinned VMM version + SHA-256'd guest artifacts
  ⇒ the launch test is reproducible bit-for-bit in its inputs; success is witnessed by a
  deterministic marker, never inferred from exit codes.
- **P7 GENDER** (paired creation, no self-certification): artifacts do not certify
  themselves — syft (independent tool) describes them, cosign + a separate verify step
  attests them, and the marker-forgery adversarial proves the launch test cannot rubber-stamp
  a non-executing VM.

(P1/P3/P4/P5 are not load-bearing for this runtime subsystem and are not claimed
decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 12 rows live-verified; DoD-1 found already-closed; DK-04/05 status corrected; DoD-4 premise drift surfaced |
| 2 DoD | §5 — RED→GREEN + falsifiers; §10.5.2 DoD-1..5 kept 1:1, DoD-4 re-anchored with the drift stated, none weakened |
| 3 spec/event-driven TDD | §2 spec-first; §3.1's r5/r6 RED-first; success asserted on the marker event, not end-state exit codes |
| 4 predefined types/consts | §2 — `FIRECRACKER_VERSION`, artifact SHA pins, `MicroVmRun`, `WORKLOAD_MARKER`, `run_workload` signature |
| 5 adversarial/breaking tests | §3.1 marker-forgery + pin teeth + fail-closed guard; §3.2 traversal; §3.3 tamper-verify teeth |
| 6 hazard-safety as math | §4.1 — unrepresentable-import, no-KVM-no-launch, marker-witness, pin gates |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — VM-pool break point with named trigger; SBOM/component axes stated |
| 9 Linux discipline | §4.4 — five verdicts incl. one DOES-NOT-TRANSFER |
| 10 benchmarks+telemetry | §6 — boot-to-marker as the placement price, measured not assumed |
| 11 isolation/bulkhead | §4.3 — feature-gated launch path, default build untouched, blast radii named |
| 12 mesh awareness | §4.3 — node-local phase, consumers named, N/A budgets stated |
| 13 rollback/self-heal vocabulary | §4.3 — Self-Termination claimed with mechanisms; the other two explicitly NOT claimed |
| 14 error-propagation gates | §3.3's zero-OCI + no-docker gates; §3.1's SHA gates; K-5's test-by-name lint |
| 15 living memory | §4.3 — reasoned exemption (no temporal data introduced) |
| 16 tensor/spectral + eqc reuse | **Explicit N/A, not decorative**: runtime isolation and supply-chain provenance contain no closed-form math organ. Reasoned exemption per "where applicable" |
| 17 regression ledger | §5 — four named permanent rows |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §3.2/§3.4 close by citation not construction; §3.3 reuses check-no-docker's pattern; §3.5 adds only the one missing named test; DK-01/02/03 code untouched |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repos: `/root/dowiz` (kernel isolation, deploy/, native-spa-server, dowiz CI) and
`/root/bebop-repo` (wasip2 component, wasm-host, bebop CI; push to `openbebop`, never
`origin`). K-items are independent — fan out freely; only the K-4/P34B-U1 README edit needs
the coordination check below.

1. **T1 (K-1; dowiz).** Read `kernel/src/isolation/microvm.rs` in full first. Add the
   `microvm-launch` feature + §2's types/consts + `run_workload` + `r5` (green locally,
   RefusedNoKvm on this no-KVM host) + `r6` (`#[ignore]`, marker equality). Write
   `fetch-guest-artifacts.sh` (pinned URLs + SHA-256 verify, hard-fail on mismatch) and the
   `microvm-launch` CI job (§3.1 YAML). Run the three adversarials once, record outputs in the
   PR. Acceptance: r1-r6 policy per §5 K-1 row; the `:14-16` comment now names
   jailer/seccomp/tap as next units. Do NOT add jailer/network — scope ends at the marker.
2. **T2 (K-2; both repos).** Dowiz job: build `native-spa-server --release` → syft SPDX →
   cosign sign-blob → cosign verify-blob → upload artifacts → `scripts/ci-zero-oci.sh`.
   Bebop job (separate commit, pushed to `openbebop`): same chain over the DK-02 component
   built via `bebop2/tooling/build-wasm-component.sh`. Run the tamper-teeth once, record.
   Acceptance: §5 K-2 row.
3. **T3 (K-3; dowiz).** Add `serve_proof` (SPA-fallback + CSP + compression) + the traversal
   adversarial to `tools/native-spa-server/tests/` (extend the existing suite — check first
   whether any of the four assertions already exist by name; pin rather than duplicate).
   The decision note is §3.2 of THIS file — reference it in the commit message.
4. **T4 (K-4; dowiz).** CHECK FIRST: if P34B U1 already landed the `deploy/README.md:4` fix
   (`grep -n "per-node source-of-truth" deploy/README.md` empty), only append the
   cross-reference sentence; otherwise land both edits (§3.4). Acceptance: both K-4 greps.
5. **T5 (K-5; both repos).** Confirm each §3.5 proof exists by name (searching the wasm-host
   test mod for the WR-01 trap; if absent, add `wr01_out_of_scope_import_traps` bebop-side:
   instantiate the telegram component with an EMPTY grant set, assert instantiation/call
   traps). Append the four ledger rows (§5; bebop row goes in the ledger P36 R-0 creates —
   coordinate as in P34B U5). Re-run the K-0 pin grep. Push both repos; fetch before every
   push, never force.

**Stop-and-flag conditions (do not improvise past these):** (i) any impulse to reopen DK-07
or add sandboxing to agent-governance (decided — cite the ADR and stop); (ii) any
Kubernetes/orchestrator suggestion (standing rejection); (iii) r6 "passing" without the marker
byte-equality, or any weakening of r2/r4 to make the launch path easier; (iv) unpinned guest
artifacts or a skipped SHA verify ("it's just CI" is not a reason); (v) deleting anything in
bebop-repo's root (P36's scope — hand findings over, act on none); (vi) modernizing/rewriting
the DK-01/02/03 working code.
