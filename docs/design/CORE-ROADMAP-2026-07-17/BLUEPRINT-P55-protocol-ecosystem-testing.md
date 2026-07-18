# BLUEPRINT P55 — Protocol/ecosystem-wide systematic testing: regression-class taxonomy, feature-matrix discipline, property/mutation testing, network-condition injection, friction-span map (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). P55 is **Part 2 of a
> 3-part parallel effort**:
>
> - **P54** — LLM/agent-specific verification (adversarial probes, tokenizer-artifact evals,
>   money-arithmetic trust). Sibling, NOT this file's scope. At this pass its file
>   (`BLUEPRINT-P54-llm-agent-verification-harness.md`) is **mid-write by the sibling agent**
>   (6 lines on disk, mtime minutes ago) — cited by name, content not relied on.
> - **P55 (this file)** — systematic testing/verification of the WHOLE deterministic
>   protocol/ecosystem: dowiz kernel+engine, bebop2 crates, openbebop CI, and the cross-repo
>   `kernel-rlib` closure. Classical methods, logical-consistency checks, compliance-with-claims
>   checks, property-based + mutation testing, cross-environment/network matrices, friction-point
>   instrumentation.
> - **P56 (expected name)** — shared storage / cross-platform / feedback-loop infrastructure.
>   **PENDING — no file on disk this pass** (`find docs -name "*P56*"` → 0). Cited by expected
>   name only; where P55 needs its surface (result storage, cross-device runners) the interim
>   answer is named inline (§4.6) so P55 does not block on it.
>
> **The pattern that names this phase (one sentence, three-plus instances, all found THIS
> session):** *real, working-when-written code silently rots because nothing keeps testing it* —
> (1) bebop's no_std wasm32 build went RED through a CI gate that ran and blocked nothing
> (P36 §0 row 4); (2) the `kernel-rlib` cross-repo spine went RED with zero gate anywhere
> (P34 §0 rows 12–13); (3) `insecure-tls` shipped default-on with no fence and calcified
> (P36 §0 row 6); (operational sibling, same day) (4) the disk-cleanup automation memory claimed
> was scheduled had silently stopped existing (`BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` §0 —
> "designed once, silently stopped"). P55's job is to make catching this failure class
> SYSTEMATIC and repo-wide, instead of per-blueprint-manual — the RED→GREEN + adversarial-case
> discipline every blueprint this session practiced by hand, turned into standing machinery.

---

## 0. Ground truth — every claim live-verified this pass (standard §2 item 1)

Verified 2026-07-18 on `dowiz` `main` (clean tree, HEAD per session snapshot `f9b2eb9bb`) and
`bebop-repo` `main`. Paths relative to `/root/dowiz/` unless prefixed `bebop:` =
`/root/bebop-repo/`.

### 0.1 The three regressions this design must have caught (re-cited, owned elsewhere)

| # | Regression | Cite (fresh) | Owner |
|---|---|---|---|
| 1 | no_std wasm32 build RED (E0425, `at_rest.rs:74`) — **the CI gate EXISTED (`bebop:.github/workflows/ci.yml:48-49`) and fired RED after the commit was already on main; direct pushes bypass it** | `BLUEPRINT-P36-bebop-remediation.md` §0 rows 1–5, §3.1 | P36 R-1 |
| 2 | `cargo test -p bebop-delivery-domain --features kernel-rlib` RED (E0004: `Refunding`/`CompensatedRefund` not covered in `from_order_status`) — **the feature is CI-gated NOWHERE in either repo** (`grep kernel-rlib .github/workflows/` → 0 both sides) | `BLUEPRINT-P34-mesh-kernel-wiring.md` §0 rows 12–13 | P34 W-1 |
| 3 | `bebop:bebop2/proto-wire/Cargo.toml:50` `default = ["insecure-tls"]` — a deliberate-but-unfenced "temporary" default that calcified | `BLUEPRINT-P36` §0 rows 6–8, §3.2 | P36 R-2 |

P55 does NOT re-fix any of these (anti-scope §1). P55 owns the *generalized mechanism* that
turns each one's CLASS into a standing RED.

### 0.2 What already exists that P55 extends (reuse-first, item 19)

| Asset | Cite (fresh this pass) | Status |
|---|---|---|
| **proptest is ALREADY a dowiz-kernel dev-dependency** — `proptest = "1.11"`, pinned 1.11.0 in `Cargo.lock`, in real use: a 400-case reconciliation property suite | `kernel/Cargo.toml:89`; `kernel/src/ports/payment.rs:623` (`use proptest::prelude::*`), `:644-649` (`ProptestConfig::with_cases(400)`, `b3_reconciliation_folded_eq_fold_derived`) | LIVE — the in-repo template for §3.2 |
| Dev-deps are already fenced from prod paths | `kernel/tests/firewall_p47.rs:14` — "`no-dev` so a dev-only proptest/criterion never masks a real prod-path violation" | LIVE |
| bebop side has a RECORDED proptest-adoption trigger, deliberately unfired | `bebop:bebop2/proto-wire/src/sync_pull.rs:1038-1041` — hand-rolled xorshift64 property fuzzing; "upgrade trigger: if we later need shrinking of arbitrary structured inputs, adopt proptest (dev-only, guarded by ci-core-no-ccrypto.sh)" | LIVE — §5 DECART rules on whether the trigger now fires |
| **Deterministic chaos/fault-injection harness is BUILT** (P-H W-H1): `ChaosSite`/`FaultInjection`/`Trigger`/`FaultPlan`, seeded PCG64, compiled out of release (`#[cfg(any(test, feature = "chaos"))]`), `DelayResponse { virtual_ms }` already models delay via virtual time | `kernel/src/chaos.rs` (621 lines; enums at `:33-63`), `kernel/src/lib.rs:126-127` | LIVE — §3.3's Tier-1 substrate |
| Zero-peer operation is PROVEN at the domain layer | `bebop:bebop2/delivery-domain/src/intake.rs:408` `ac6_solo_island_full_flow_no_peers` (full lifecycle, zero peers) — F50/mesh-real AC-6 | LIVE (modulo regression #2 — it does not compile until P34 W-1) |
| Transport is swappable behind Trait-as-Port | `docs/design/ARCHITECTURE.md:15` (M6 LOCK: "Transport swappable (iroh/quinn/HTTP/stdio) behind Trait"), `:30`, `:48`; live impls `bebop:bebop2/proto-wire/src/{stdio_transport,iroh_transport,wss_transport}.rs` | LIVE — §3.3's mock seam |
| P24 telemetry design: `SiteAgg` aggregates + SPSC ring + drainer + explained-anomaly capsule; closed u16 site table; `telemetry-off` gate for wasm/no_std | `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` §3.1–3.4, §4 | DESIGNED — §3.4 names NEW sites only, zero new mechanism |
| P27 fault-isolation: `CircuitBreaker` primitive, two-pole doctrine, grep-gates-in-CI idiom, §6 Fault-Containment rule | `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` §3.2, §5, §6 | DESIGNED — P55 tests ON TOP of it, never redesigns it (anti-scope §1) |
| dowiz CI: 12 jobs incl. `cargo-test` (kernel+engine, **default features only**), `bench-regression` (baseline-gated), `v5c-reexec` (independent re-exec on red-line paths), `supply-chain`, `decart-dep-lint`, grep fences | `.github/workflows/ci.yml` (read in full this pass) | LIVE — §0.3 names what it does NOT cover |
| bebop CI: `rust-test` (workspace default), `rust-hardened` (`--no-default-features` core+proto-wire, wasm32 build), `sovereign-guards` (13 fence scripts incl. "Kernel layer fence (proto-cap ⊥ dowiz-kernel)") | `bebop:.github/workflows/ci.yml:14-31,36-53,60-100` | LIVE — the fence constrains §3.4 (bebop crates cannot import dowiz-kernel's ring) |
| The planted-`assert!(false)` discipline — a manual, one-shot mutation-test precedent | `.github/workflows/ci.yml` `cargo-test` job header: "a planted assert!(false) in any kernel test goes RED on every push" | LIVE precedent §3.2's mutation row systematizes |
| Regression ledgers exist: dowiz live; bebop's created by P36 R-0 | `docs/regressions/REGRESSION-LEDGER.md` (ratchet rule + guardrail-type vocabulary incl. `chaos`, `grep-CI-gate`); `BLUEPRINT-P36` §0 row 14 | LIVE / OWNED-P36 |
| Off-box result storage exists and is live | `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` §0: rclone remote `hetzner:dowiz` (Hetzner S3, 141 objects, 13.07 GiB) — operator instruction: ship heavy results there, not local disk | LIVE — §4.6 |

### 0.3 The uncovered configuration space (the gap, enumerated live)

Full `[features]` inventory, both workspaces (every `Cargo.toml` grepped this pass):

| Crate | Features (default first) | Exercised by ANY CI lane today? |
|---|---|---|
| `dowiz-kernel` | `default=["std"]`; `std`, `wasm`, `chaos`, `pgrust`, `pq` (`kernel/Cargo.toml`) | **default only** (`cargo-test` job). `--no-default-features` (no-std embedding): NO. `wasm`: NO. `pq` (107 KAT tests, GROUND-TRUTH-2026-07-17): NO. `chaos` as feature-lane: NO (reachable via `cfg(test)` implicitly). `pgrust`: NO |
| `dowiz-engine` | `default=[]`; `gpu`, `webgl`, `webgpu`, `splat` (all EMPTY by design — boundary flags) | default only; the empty flags still need a `cargo check --features X` lane so a future dep addition can't silently break them |
| `dowiz wasm/` | `default=[]` | NO lane |
| `agent-adapters` | `default=[]`; `wasmtime-fuel` | NO lane |
| `bebop2-core` | `default=["std","host"]`; `dangerous_deterministic`, `test_keygen`, `ceremony` | default + `--no-default-features` (rust-hardened). Feature singletons: NO |
| `bebop-proto-wire` | `default=["insecure-tls"]` (regression #3); `iroh`, `insecure-test` | default + `--no-default-features`. Post-P36-flip the opt-in lane is P36 §3.2(b)'s |
| `bebop-proto-cap` | `anu`, `ceremony` | default only |
| `bebop-delivery-domain` | `default=[]`; **`kernel-rlib`** | **NOTHING** (regression #2) — owned by P34 W-1's CI job; P55's matrix subsumes-by-inclusion, never duplicates |
| `bebop-mesh-node` | `default=[]`; `kernel-rlib` | NOTHING |
| `bebop-wasm-host` | `default=[]`; `wasm` (wasmtime) | default only |

**This table IS the finding:** ~14 named non-default configurations exist; CI exercises 3.
Every un-exercised row is a regression-#2-shaped accident waiting (the P36 header's own words:
"ungated (or unenforced) build paths rot silently").

### 0.4 Environment probes (this pass, this host)

- `crates.io` egress: **HTTP 403 live** (`curl -sI https://crates.io/api/v1/crates/proptest` →
  `HTTP/2 403`, Heroku) — consistent with the P15 §9 recorded probe. GitHub-hosted CI runners
  have normal egress (existing jobs run `cargo fetch`/`cargo audit`/gitleaks). This asymmetry
  drives both DECARTs in §5.
- `cargo mutants` → "no such command" (not installed; cannot be installed from this box while
  the 403 stands).
- `tc` present (`/usr/sbin/tc`); **`sch_netem` kernel module present**
  (`modinfo sch_netem` → `/lib/modules/6.8.0-134-generic/kernel/net/sched/sch_netem.ko.zst`,
  "Network characteristics emulator qdisc"); `ip`/`iptables` present; running as root — network
  namespaces (`ip netns`) available.

### 0.5 Target functions for property-based testing (signatures re-verified live)

| Function | Live signature (this pass) |
|---|---|
| `order_machine::assert_transition` | `kernel/src/order_machine.rs:139` `pub fn assert_transition(from: OrderStatus, to: OrderStatus) -> Result<(), TransitionError>`; `OrderStatus` = 12 variants incl. P07's `Refunding`/`CompensatedRefund` (`:8-25`) |
| `domain::compute_order_total` | `kernel/src/domain.rs:129` `pub fn compute_order_total(subtotal: i64, tax_rate: f64, price_includes_tax: bool, fee: Option<i64>) -> Result<i64, String>` — checked_add throughout, non-negative asserted (module lives in `domain.rs`, not `money.rs` — the dispatch brief's `money::` prefix corrected here) |
| `money` ledger laws | `kernel/src/money.rs:164` `reversed_leg`, `:185` `ledger_append`, `:230` `ledger_sum` |
| `claim_machine::assert_transition` | `bebop:bebop2/proto-cap/src/claim_machine.rs:85` `pub fn assert_transition(from: ClaimStatus, to: ClaimStatus) -> Result<(), ClaimError>` |
| `matcher::assign` | `bebop:bebop2/proto-cap/src/matcher.rs:63` `pub fn assign(order: &Order, candidates: &[Courier], max: usize) -> Vec<CourierKey>` |
| Dual legality tables (drift hazard, P34 §0 row 18) | kernel table `order_machine.rs:139` vs wire mirrors `bebop:bebop2/proto-cap/src/event_dict.rs:75` (`allowed_next`) / `:90` (`assert_status_transition`) + `bebop:bebop2/delivery-domain/src/lib.rs:148` `assert_transition_local` (all three re-verified live) — parity sweep OWNED by P34 §3.2; P55 adds the sequence-level property on top (§3.2) |

Ground truth is non-discussible; everything below builds on this section only.

---

## 1. Scope — what P55 owns and deliberately does NOT own

**P55's single sentence:** one standing, repo-wide testing discipline — a regression-class
taxonomy with a named CI mechanism per class, a curated feature-matrix that re-exercises every
named configuration in both repos, property-based + mutation testing over the deterministic
core, a two-tier network/environment injection harness, and the friction-span map P24's
mechanism instruments — so that "proven once" can never again silently become "broken now."

**Owns (build items §3):** M-1 gate-liveness ratchet + auditor (RC-1) · M-2 feature-matrix
lanes + coverage drift-gate (RC-2) · M-3 secure-defaults auditor generalized to every crate
(RC-3) · M-4 scheduled-automation heartbeat (RC-4) · PB-1..PB-5 property suites · MT-1
mutation-testing job · NET-1/NET-2 network-condition tiers · SPAN-1 friction-site table.

**Does NOT own (anti-scope, binding):**

- **The three concrete fixes** — P36 R-1/R-2 and P34 W-1 own them; P55 cites, never
  double-fixes. P34 W-1's `kernel-rlib` CI job becomes one ROW in M-2's matrix, not a fork.
- **P27's fault-isolation primitives** (`CircuitBreaker`, bulkheads, supervision policy) —
  P55 injects faults AGAINST them and asserts their declared poles; it never redesigns them.
- **P24's telemetry mechanism** (ring, drainer, capsule, tiers) — §3.4 names NEW measurement
  sites only, consumed through P24's own site-table discipline.
- **The chaos harness's internals** (`kernel/src/chaos.rs`) — extended with new `ChaosSite`
  variants via its own closed-set review rule, not rebuilt.
- **LLM/agent verification** — P54's, in flight. Anything probabilistic/model-shaped is out;
  P55's whole surface is deterministic code and deterministic gates.
- **Shared storage/feedback-loop infrastructure** — P56's (pending). §4.6 names the interim.
- **No foreign-runtime testing framework** — no Python hypothesis, no JS harnesses. Rust-native
  only (`proptest`, `cargo-mutants`), each behind a real DECART (§5), per the standing
  rust-native-bare-metal decision.
- **Branch-protection settings changes are operator acts** (never-bypass-human-gates) — M-1
  PREPARES them; the operator executes. P36 R-1b already owns the two bebop contexts; M-1
  generalizes the audit, not the act.

---

## 2. Predefined types & constants (item 4 — named BEFORE implementation)

```
# ── scripts/feature-matrix.txt (NEW — M-2's checked-in lane manifest; one lane per line:
#    <repo> <command>) — CURATED named configurations, not a powerset (§3.1-M2 rationale).
dowiz   cargo test  --manifest-path kernel/Cargo.toml --offline
dowiz   cargo build --manifest-path kernel/Cargo.toml --offline --no-default-features
dowiz   cargo test  --manifest-path kernel/Cargo.toml --offline --features pq
dowiz   cargo test  --manifest-path kernel/Cargo.toml --offline --features chaos
dowiz   cargo check --manifest-path kernel/Cargo.toml --offline --features wasm
dowiz   cargo check --manifest-path engine/Cargo.toml --offline --features gpu,webgl,webgpu,splat
dowiz   cargo check --manifest-path wasm/Cargo.toml   --offline
dowiz   cargo check --manifest-path agent-adapters/Cargo.toml --offline            # wasmtime-fuel: EXEMPT(net-dep), see exemptions
bebop   cargo test --workspace
bebop   cargo build -p bebop2-core --no-default-features --target wasm32-unknown-unknown
bebop   cargo test -p bebop-proto-wire --no-default-features
bebop   cargo test -p bebop-proto-wire --features insecure-tls                     # post-P36 flip: the opt-in lane (P36 §3.2b)
bebop   cargo test -p bebop-delivery-domain --features kernel-rlib                 # = P34 W-1's job, INCLUDED not duplicated
bebop   cargo test -p bebop-mesh-node --features kernel-rlib
bebop   cargo check -p bebop-wasm-host --features wasm                             # wasmtime tree: check-only on the weekly pass
```

```bash
# ── scripts/ci-feature-coverage.sh (NEW — M-2's drift gate) ──
# Enumerates every feature of every workspace crate via `cargo metadata`
# (both repos). Exits 1 if any feature name appears in NO line of
# scripts/feature-matrix.txt AND NO line of scripts/feature-matrix-exemptions.txt
# (each exemption: <crate> <feature> <dated reason>). A NEW feature without a
# lane or a dated exemption is a named CI RED — regression-class RC-2 becomes
# unrepresentable-silently.
```

```bash
# ── scripts/ci-secure-defaults.sh (NEW — M-3, generalizes P36's ci-no-insecure-default.sh) ──
# For EVERY crate in BOTH workspaces: resolved default-feature closure via
# `cargo metadata` (not a text grep — rename/indirection-proof, same reasoning
# as P36 §2). Exits 1 on any default-enabled feature matching
#   (?i)(insecure|dangerous|test_keygen|deterministic)
# unless listed in scripts/secure-default-allowlist.txt with a dated
# justification. Red-proof obligation (§6): run pre-P36-flip → MUST flag
# proto-wire's live incident.
```

```bash
# ── scripts/ci-gate-liveness.sh (NEW — M-1's auditor: the gate for the gates) ──
# For each repo: `gh api repos/<owner>/<repo>/branches/main/protection` →
# required_status_checks.contexts must be a SUPERSET of the workflow's job list
# (parsed from .github/workflows/ci.yml). Empty/missing protection = RED with
# the exact contexts to add printed (the operator's one-command fix, prepared
# per never-bypass-human-gates). Catches RC-1 at the meta level: a job that
# exists but doesn't block is a finding, permanently.
```

```rust
// ── kernel/src/chaos.rs — NET-1's ChaosSite extension (closed-set review per its own rule) ──
pub enum ChaosSite {
    // …existing four variants (chaos.rs:33-45) unchanged…
    /// Transport port boundary, send half (Trait-as-Port M6 seam).
    TransportSend,
    /// Transport port boundary, receive half.
    TransportRecv,
}
// FaultInjection reused as-is: DelayResponse{virtual_ms} = latency; a Trigger
// of Probability(p) over StoreSyncFail-shaped Err = loss; CorruptPayload = the
// corrupt-frame cell. Partition = Always-fire on both halves. NO new fault types.
```

```
# ── NET-2 lane constants (named, tunable, claim-latency style) ──
NETNS_PREFIX        = "p55"        # every netem qdisc lives ONLY inside an ip-netns; never on a host interface
NET_PROFILE_GOOD    = (no qdisc)
NET_PROFILE_LATENCY = delay 200ms 50ms distribution normal
NET_PROFILE_LOSSY   = loss 5% delay 50ms
NET_PROFILE_ABSENT  = loss 100%    # partition; domain-layer zero-peer already proven by AC-6
STEADY_STATE_METRIC = delivered-order count + ledger_sum == 0 residue (measurable output, not internals)
```

New external dependencies: **zero** in any shipped graph. `proptest` extension = existing
dev-dep. `cargo-mutants` = a CI-runner-installed dev BINARY, never a manifest entry (§5).

---

## 3. Design — the four mechanisms, the property/mutation suites, the injection tiers, the span map

### 3.1 The regression-class taxonomy and its per-class mechanism (the dispatch's item 1)

Derived strictly from §0.1's real instances — no invented classes:

| Class | Definition | This session's instance | Mechanism (build item) |
|---|---|---|---|
| **RC-1 Advisory gate** | A check exists and runs, but its RED blocks nothing (enforcement topology, not detection) | no_std wasm32: gate at `bebop ci.yml:48-49` fired RED after the push was already on main | **M-1**: required-checks ratchet (P36 R-1b executes the two bebop contexts; the same operator act is PREPARED for dowiz `main` covering its 12 jobs) + `ci-gate-liveness.sh` run on the weekly pass so protection-vs-workflow drift is a standing RED, not a one-time setting |
| **RC-2 Unexercised configuration** | A named feature/flag/target combination proven once, then never rebuilt; rot is invisible because no lane compiles it | `kernel-rlib` E0004; §0.3's table shows ~11 more silent rows today | **M-2**: `scripts/feature-matrix.txt` lanes (§2) — cheap lanes (`check`/core `test`) on every push; the FULL matrix on a weekly scheduled workflow (`schedule:` cron, both repos) — "periodically re-run EVERY named feature-flag combination across EVERY crate" made literal. Plus `ci-feature-coverage.sh`: a feature with no lane and no dated exemption is RED at introduction time |
| **RC-3 Unsafe default** | A security-relevant posture rides a `default = [...]` nobody re-decides; "temporary" calcifies | `insecure-tls` default-on, 4+ days | **M-3**: `ci-secure-defaults.sh` over EVERY crate's resolved default closure, both repos, allowlist-with-dated-reason for legitimate hits. Sits in bebop's `sovereign-guards` and a new dowiz fence job — the established fence idiom (13 scripts already, `bebop ci.yml:60-100`), one more sibling, not a new system |
| **RC-4 Silently-stopped automation** | A scheduled job (cron, workflow) stops existing/running and nothing notices its absence | disk-ops §0: memory's "deep-clean tool + cronjobs" — not scheduled today | **M-4**: every P55 scheduled job appends one heartbeat line to a JSONL ledger (GapWire §4.3's heartbeat-ledger pattern, P27 §3.4 — reused); the weekly pass flags any registered job whose last heartbeat is older than 2× its period. Absence-detection for the detectors themselves |

**Logical-consistency + compliance-with-claims checks (the operator's classical-methods ask),
mapped to existing organs rather than invented:** claims-cite-a-test =
`bebop scripts/ci-claim-live-test.sh` (live); independent re-execution = dowiz `v5c-reexec`
(live); doc-claim drift = P36 R-1c's append-only correction discipline; structural invariants =
the grep-fence family (both repos). P55 adds exactly one new consistency check:
**M-2's coverage gate is itself a consistency check between the manifest space (what Cargo says
exists) and the verification space (what CI actually exercises)** — the formal statement of
"proven implies still-being-proven."

### 3.2 Property-based + mutation testing for the deterministic core (item 2 of the dispatch)

The core functions (§0.5) are pure, total-or-typed-Err, no-I/O — exactly the QuickCheck-family
target shape: state properties as executable specifications, generate adversarial inputs,
shrink failures to minimal counterexamples (Claessen & Hughes, *QuickCheck: A Lightweight Tool
for Random Testing of Haskell Programs*, ICFP 2000 —
https://www.cse.chalmers.se/~rjmh/QuickCheck/; Rust lineage: proptest, Hypothesis-style
per-value `Strategy` shrinking — https://github.com/proptest-rs/proptest). The in-repo template
is the existing 400-case B3 suite (`ports/payment.rs:644-649`) — every suite below copies its
shape (a `proptest!` block, named config, one property per fn):

| ID | Target | Properties (each falsifiable) |
|---|---|---|
| PB-1 | `order_machine::assert_transition` + `apply_event` (`domain.rs:256`) | (a) exhaustive 12×12 agreement with `allowed_next` (144 cases — plain exhaustive test, cheaper than proptest, kept alongside); (b) proptest over arbitrary event SEQUENCES: folding any sequence through `apply_event` either errors or lands in a reachable state — no illegal state is ever constructible; (c) terminal states admit no successor; (d) fold determinism: same sequence twice ⇒ identical final `Order` |
| PB-2 | `domain::compute_order_total` + `money` ledger | (a) result is always ≥ 0 or `Err` — never a wrapped/negative i64 (generators include `i64::MAX`-adjacent subtotals and fees: the overflow arms at `domain.rs:137-143` become reachable-and-tested, not just written); (b) `price_includes_tax=true` ⇒ total == subtotal + fee exactly; (c) monotone in `fee`; (d) ledger zero-sum: for any earn entry, `ledger_append(reversed_leg(e))` ⇒ `ledger_sum == 0` (`money.rs:164,185,230`) |
| PB-3 | `claim_machine::assert_transition` (bebop) | same shape as PB-1(a)/(c) over `ClaimStatus` |
| PB-4 | `matcher::assign` (bebop) | (a) output ⊆ candidates, no duplicates; (b) `len ≤ max`; (c) determinism: identical inputs ⇒ identical output; (d) the documented tie-break is the ONLY order-sensitivity — permuting candidates yields the same SET (and the same sequence if the tie-break is total); (e) NO-COURIER-SCORING invariance: assign must be insensitive to any field the red-line forbids ranking on (pins the E58 invariant as an executable property, not only a grep) |
| PB-5 | Cross-table sequence parity | for any legal kernel transition sequence, the wire mirror (`event_dict.rs` tables) accepts the mapped sequence and vice versa — the SEQUENCE-level layer on top of P34 §3.2's per-pair parity sweep (cited, not duplicated). Lives in the `kernel-rlib` lane, so it also keeps regression #2's gate meaningful |

**Mutation testing (does the suite catch deliberately-introduced bugs):** grounded in DeMillo,
Lipton & Sayward, *Hints on Test Data Selection: Help for the Practicing Programmer*, IEEE
Computer 11(4), 1978 (the coupling effect: tests catching simple seeded faults catch complex
ones) — and in SQLite's standing practice of test-the-tests discipline as the price of "100%
MC/DC" claims (https://sqlite.org/testing.html). Tool: `cargo-mutants`
(https://mutants.rs/, github.com/sourcefrog/cargo-mutants) — DECART §5.2. **MT-1**: a weekly
scheduled GitHub-Actions job per repo, scoped by `--file` to the deterministic core
(`order_machine.rs`, `domain.rs`, `money.rs`, `event_log.rs`; bebop: `claim_machine.rs`,
`matcher.rs`, `signed_frame.rs` non-crypto arms), advisory-exit-0 for the first month writing a
`mutants-score.jsonl` ledger row (the claim-latency posture, `ci-truth` precedent), promoted to
a gated minimum-caught-fraction once the baseline is known — thresholds earn their teeth from
measurement, not guesses (P27 §4's own probe discipline). The existing planted-`assert!(false)`
one-shot (§0.2) is the manual precedent this systematizes.

### 3.3 Network/environment condition injection — two tiers, one DECART (item 3 of the dispatch)

The operator's ask: test under good / absent / latency network, across environments. Grounding:
the Principles of Chaos Engineering (https://principlesofchaos.org/) — define steady state as
measurable OUTPUT, hypothesize it holds, inject real-world events (severed connections,
latency), try to DISPROVE; and Jepsen's nemesis practice — fault schedules (partitions via
iptables, process kill, clock skew) driven against a running system while checking documented
guarantees (https://www.infoq.com/articles/jepsen/,
https://asatarin.github.io/testing-distributed-systems/). Two tiers because the repo's own
canon splits here: `chaos.rs` is deliberately hermetic ("no wall-clock, no real sleep, no real
network" — its module doc), while real-stack behavior (QUIC retransmission, TCP timeouts,
congestion response in iroh/quinn) is invisible to any mock by construction.

- **NET-1 (Tier 1 — deterministic, default, every CI run): `ChaosTransport` at the M6 port.**
  A decorator over the transport Trait (exactly `ChaosStore`'s shape over `EventStore` —
  `chaos.rs`'s Seam A generalized to its second port), with the two new `ChaosSite` variants
  (§2). Cells: latency = `DelayResponse{virtual_ms}` (virtual time, already built); loss =
  `Probability(p)` triggers; partition = `Always`; corruption = `CorruptPayload`. Assertions
  are exact and reproducible from `(seed, plan)`: e.g. under 100% loss the intake edge's
  behavior must equal AC-6's solo-island behavior (`intake.rs:408`) — the offline-first F12
  canon promoted from "a test that exists" to "the steady-state oracle every degraded-network
  cell is compared against." Runs in plain `cargo test`, no root, no OS coupling.
- **NET-2 (Tier 2 — real-stack, weekly scheduled, Linux-only, netns-scoped): tc/netem.**
  `sch_netem` verified present (§0.4). Harness: `scripts/netem-lane.sh` creates a `p55-*`
  network namespace pair + veth, applies one NET_PROFILE (§2) INSIDE the namespace only (never
  a host interface — blast-radius bulkhead per P27 §3.3's own table), runs the two-node
  proto-wire round-trip/sync tests across it, tears down in a trap. Assertions are
  steady-state-statistical, not bit-exact (netem loss is probabilistic — stated honestly):
  under LATENCY the flow completes with round-trip spans (§3.4) reflecting ≥ the injected
  delay; under LOSSY it completes within a bounded retry budget; under ABSENT the node behaves
  as an island (AC-6 oracle again) and P27's breaker/timeout poles engage as declared —
  NET-2 is precisely the fault-injection done-check P27 §3.6 requires for containment claims,
  executed against a real network stack. Reference: tc-netem(8),
  https://man7.org/linux/man-pages/man8/tc-netem.8.html.
- **Cross-platform/multi-device matrix (scoped honestly):** what is buildable NOW: the weekly
  pass already spans x86_64-linux (runners) × wasm32-unknown-unknown (hardened lane) ×
  no_std — three real targets. An `aarch64-unknown-linux-gnu` cross-`cargo check` lane is
  added to the weekly pass (couriers' phones are ARM; check-only until a real runner exists).
  Real multi-DEVICE execution (phone-class hardware, GPU variation) requires P56's runner
  infrastructure — named as P56's surface, with the matrix manifest (§2) already shaped to
  accept a target column so P56 plugs in without redesign. No pretend coverage is claimed.

### 3.4 Friction-point spans — WHAT gets instrumented, P24's mechanism unchanged (item 4)

P24 owns the how (SiteAgg 3-atomic aggregate + anomaly-ring event + explained capsule, §3.1;
closed u16 site table, F32 discipline). P55 contributes the named protocol sites. One hard
constraint found in ground truth: **`sovereign-guards` enforces proto-cap ⊥ dowiz-kernel**
(`bebop ci.yml` kernel fence) — bebop crates can NEVER import the kernel's ring. Therefore
every bebop-side span is measured from the CALLER at the P34 `dowiz-mesh-adapter` boundary
(dowiz-side, fence-clean) or inside `delivery-domain[kernel-rlib]` (which already depends on
dowiz-kernel legally, §0.2). bebop2-core no_std/wasm builds keep C-ABI counters only (P24
§3.4's `telemetry-off` rule — inherited, restated, not re-decided).

| Span (site-table name) | Measures | Instrumentation point (verified) |
|---|---|---|
| `wire.frame_verify` | signature-verification time, hybrid | around `SignedFrame::verify` (`bebop:bebop2/proto-cap/src/signed_frame.rs:258`; classical `:208` / pq `:229`) — measured from the adapter caller |
| `cap.verify_chain` | capability-delegation-chain verification time | around `roster::verify_chain` (`bebop:bebop2/proto-cap/src/roster.rs:252`) — adapter caller |
| `wire.roundtrip` | send→ack wall time per envelope, per transport impl | transport call sites in the adapter / NET-2 harness (stdio/iroh/wss impls, §0.2) — the cell where NET-2's injected delay must reappear, closing the injection→measurement loop |
| `law.admit_and_fold` | full WIRE→LAW→MONEY admission time | around `IntakeEdge::admit_and_fold` (`bebop:delivery-domain/src/intake.rs:234`), inside the kernel-rlib module set |
| `law.commit_after_decide` | decide→insert→tip commit time | `kernel/src/event_log.rs:366` — kernel-side, direct |
| `match.assign` | matcher assignment time vs candidate count | around `matcher::assign` (`bebop:bebop2/proto-cap/src/matcher.rs:63`) — adapter caller; the candidate count rides the capsule so latency anomalies are explainable against load, P24 §4.2 discipline |
| `money.order_total` | total-computation time (µs-scale; aggregate-only, no ring events — P24's `min_flag_ns` floor makes this explicit) | `kernel/src/domain.rs:129` call sites |
| `sync.pull_verify` | anti-entropy pull verification time | around `sync_pull` verify (`bebop:bebop2/proto-wire/src/sync_pull.rs:338`) — adapter/NET-2 harness caller |

Existing `tracing` spans on the kernel fold path (`order_machine.rs:160-161`) stay — P08 S7's
half; P24's rule that the two surfaces don't merge is inherited.

---

## 4. Cross-cutting obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as structure (item 6)
RC-1 recurrence requires simultaneously defeating a required check AND the weekly
gate-liveness audit; RC-2 requires adding a feature while editing the matrix manifest AND the
exemption file (the coverage gate makes "silent" impossible — the failure mode left is
*deliberate*, which is reviewable); RC-3 requires renaming past a resolved-metadata regex
inside a required job; chaos machinery in production stays unrepresentable
(`cfg(any(test, feature = "chaos"))` — inherited); NET-2 cannot touch host traffic by
construction (netns-only, falsifier: `tc qdisc show` on host interfaces is unchanged during a
lane run, asserted in the harness itself).

### 4.2 Schemas & scaling (item 8)
`feature-matrix.txt` scales linearly in named lanes (~16 today); its breaking axis is
combinatorial pairwise interaction — the named upgrade trigger for a `cargo-hack`-style
powerset tool is "a bug escapes through a PAIR of features neither lane exercises alone" or
>~15 features on one crate (recorded; today's per-crate max is 5). `mutants-score.jsonl` and
heartbeat ledgers are append-only JSONL with the standing rotation discipline (P24 §3.5's
capsule-rotation model). Weekly-pass wall-clock is the real budget: recorded per §6 and capped
by lane pruning, never by silently skipping (a pruned lane needs a dated exemption row —
same rule as features).

### 4.3 Isolation / mesh awareness / rollback vocabulary / living memory (items 11-13, 15)
Isolation: every mechanism is additive CI/dev-plane; zero product hot-path changes except
span instrumentation, which rides P24's proven ≤1% overhead gate (its W2a bench). Mesh: all
of P55 is canonical-repo dev-time fencing — the SCOPE RULE banner (dowiz `ci.yml`, verbatim
in every job this phase adds): no gate here is a runtime control on sovereign hubs. Rollback:
P55 claims **Self-Termination** only in the weak structural sense (unlisted feature / unsafe
default / unprotected gate are unrepresentable-without-RED); every artifact is a plain
`git revert`; Self-Healing NOT claimed. Living memory: the ledgers (mutants-score, heartbeat,
regression rows) are the class's memory instrument; `internal-retrieval-living-memory-arc`
cross-referenced for their eventual tiering, not a v1 dependency.

### 4.4 Linux-discipline verdicts (item 9)
M-1 = **EXTENDS** (enforcement topology; same justification P36 §4.4 recorded — a
demonstrated advisory-CI failure); M-2 = **ALREADY-EQUIVALENT** (the kernel's own allmodconfig/
randconfig culture: build every config, not the default one) formalized for cargo features;
M-3 = **REINFORCES** (deny-by-default doctrine applied to manifest defaults); PB/MT =
**EXTENDS** (test-the-tests: the suite's own strength becomes measured, SQLite-style);
NET-1/2 = **REINFORCES** (the harness respects the hermetic/real split instead of blending
them). Nothing GAP or DOES-NOT-TRANSFER.

### 4.5 Non-contradiction / sequencing (hard)
- P34 W-1 and P36 R-1/R-2 land INDEPENDENTLY of P55; M-2's manifest includes their lanes from
  day one so landing order cannot leave a hole. If P55 lands first, two matrix rows are
  simply RED with known owners — honest state, not a conflict.
- M-3's red-proof (§6) must run BEFORE P36's R-2 flip to capture the live-incident proof; if
  R-2 already landed, the red-proof runs against a scratch revert of the manifest line
  (recorded as such).
- M-1's operator act for bebop = P36 R-1b's act (one act, two blueprints cite it); the dowiz
  act is new, prepared separately.
- P54/P56 files are in flight — P55 does NOT edit `CORE-ROADMAP-INDEX.md` or the SOVEREIGN
  roadmap this pass (three parallel writers, one hot file — the fan-out rule keeps shared
  integration points for the lead; registration is the lead's single pass afterward).

### 4.6 Where results live (operator instruction, disk-ops)
GH-scheduled runs: artifacts via `upload-artifact` (existing idiom). Locally-run heavy passes
(NET-2, any local mutation run once egress allows): outputs land in the scratch dir and are
`rclone move`d to `hetzner:dowiz/ci-results/` (bucket live, §0.2) — never accumulated on `/`
(the 90%-disk incident's lesson, `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` §1-2). P56 may
supersede this path; until then it is the named interim.

---

## 5. DECART — the two adoption decisions (item 19; new-integration honesty)

### 5.1 proptest (property-based testing)

| Candidate | Native fit | Falsifiable | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|
| Python `hypothesis` / any foreign-runtime harness | ✗ violates the Rust-native execution rule outright | — | — | — | — | REJECT (anti-scope §1) |
| Hand-rolled xorshift fuzzing everywhere (extend `sync_pull.rs`'s pattern) | ✓ zero-dep | ✓ but NO shrinking — a failing 60-op sequence stays a 60-op haystack | low | none | — | REJECT as the general mechanism; KEEP where it already works (proto-wire stays as-is — its own comment's terms honored) |
| **proptest, extend in dowiz-kernel + adopt dev-only in proto-cap** ← | ✓ Rust-native, ALREADY a vetted dowiz dev-dep (`Cargo.toml:89`, 1.11.0 in lock, real 400-case suite live) — dowiz-side this is NOT a new dependency at all | ✓ shrinking gives minimal counterexamples; every PB property is a falsifiable executable spec | small | dowiz: zero new. proto-cap: one NEW dev-dep — **the crate's own recorded trigger fires**: PB-3/PB-4 inputs are structured (`Order`/`Courier`/`ClaimStatus`), exactly the "shrinking of arbitrary structured inputs" condition `sync_pull.rs:1038-1041` named; dev-only, `ci-core-no-ccrypto.sh` guards the shipped graph, `firewall_p47.rs:14`'s no-dev discipline mirrored | trivial (delete dev-dep + test mods) | **ADOPT** |

**Probe (strongest case against):** crates.io egress is 403 from this box (§0.4) — can
proto-cap even resolve the new dev-dep locally? Mitigation, verified-then-honest: proptest
1.11.0 + its tree already sit in the local cargo cache (dowiz-kernel builds green offline with
it); `cargo add proptest@1.11 --dev --offline` in proto-cap is expected to resolve from cache —
**checked at implementation as T3's first step; if it fails offline, the bebop-side suites run
CI-only until egress, recorded, and the dowiz-side suites (PB-1/PB-2, the money-critical ones)
are unaffected either way.**

### 5.2 cargo-mutants (mutation testing)

| Candidate | Native fit | Falsifiable | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|
| Hand-rolled mutation script (sed operator-swaps + rebuild loop) | ✓ zero-dep | weakly — naive mutants are mostly equivalent/uncompilable; the tool problem IS mutant generation quality | deceptively high | none | — | REJECT (re-implements a mature tool badly; reuse-first) |
| Confuse it with the chaos harness ("we already inject faults") | — | — | — | — | — | REJECT the conflation explicitly: `chaos.rs` injects RUNTIME faults to test the PRODUCT's containment; mutation testing injects SOURCE faults to test the TEST SUITE's strength. Different subject under test; both stay |
| **cargo-mutants as a scheduled CI dev-binary** ← | ✓ Rust-native cargo subcommand (mutants.rs, sourcefrog/cargo-mutants; Thoughtworks Radar-listed), zero source-tree changes required, NEVER a Cargo.toml entry — a runner-installed binary like gitleaks/cargo-audit already are in these workflows | ✓ its whole output is a falsifiability measurement: surviving mutants = named unconstrained behavior; DeMillo-Lipton-Sayward coupling effect grounds the method | one weekly job per repo; runtime bounded by `--file` scoping to the deterministic core | installed on GH runners (egress exists there — §0.4's asymmetry); LOCAL runs blocked by the 403 until the operator installs the binary out-of-band (stated, not hidden) | trivial (delete the workflow job) | **ADOPT (scheduled, advisory→gated per §3.2 MT-1)** |

**Probe:** mutation runs are slow (each mutant = a build+test cycle) and noisy with equivalent
mutants. Mitigation is structural: `--file`-scoped to ~7 pure-logic files where mutants are
cheap to build and equivalence is rare; advisory month first so the gate threshold is a
measured number; `--timeout-multiplier` bounds hangs (A8's lesson: no subprocess without a
deadline, P27 §1.3 class 3).

---

## 6. DoD — falsifiable, RED-first (items 2, 5, 17)

| Item | RED (provable before) | GREEN (passes after) | Falsifier / teeth (run once, recorded in PR) |
|---|---|---|---|
| M-1 | `gh api …/branches/main/protection` → no required contexts (both repos today — regression #1's enabling condition) | contexts ⊇ workflow job lists (operator-executed); `ci-gate-liveness.sh` green on the weekly pass | teeth: temporarily point the script at a branch with no protection → RED with the exact missing contexts printed |
| M-2 | §0.3's table: ~11 configurations exercised by nothing; `cargo test -p bebop-delivery-domain --features kernel-rlib` RED live | all matrix lanes green (or RED with a named owner — P34/P36 rows); coverage gate green | teeth 1: add a scratch feature `p55_teeth = []` to any Cargo.toml → `ci-feature-coverage.sh` MUST exit 1; teeth 2: delete a matrix line for a live feature → same |
| M-3 | run `ci-secure-defaults.sh` against pre-P36-flip proto-wire → MUST exit 1 (the live incident detected, not a hypothetical) | exits 0 post-flip with an empty-or-justified allowlist | teeth: add `(?i)dangerous` to any crate's default in a scratch branch → RED |
| M-4 | no heartbeat exists for any scheduled job (disk-ops §0's finding class) | every P55 scheduled job writes heartbeat rows; staleness check green | teeth: back-date one heartbeat row 3 periods → flagged |
| PB-1..5 | properties don't exist; PB-2's overflow arms untested beyond fixed cases | `cargo test` green incl. property suites, 400-case config each | teeth (mutation-by-hand, one each): weaken one table arm / drop one `checked_add` in a scratch branch → the property MUST fail and shrink to a minimal case |
| MT-1 | `cargo mutants` not installed anywhere; suite strength unmeasured | weekly job green, `mutants-score.jsonl` accumulating; month-2: threshold gate | teeth: plant one survivable mutant by hand (delete an assertion in a scratch branch), run scoped mutants → it must appear in the survivors list |
| NET-1 | no transport-boundary chaos seam (`ChaosSite` has 4 variants, none transport) | `ChaosTransport` + 2 new sites; all four cells (good/latency/loss/absent) asserted deterministically from `(seed, plan)`; ABSENT cell equals the AC-6 oracle | teeth: run the ABSENT cell with the AC-6 assertion inverted → MUST fail (proves the oracle binds) |
| NET-2 | no real-stack degraded-network lane exists | `netem-lane.sh` weekly: 4 profiles × 2-node round-trip green; host qdisc untouched (asserted in-harness) | teeth: under LATENCY profile, `wire.roundtrip` span p50 must be ≥ injected delay — if instrumentation reports less, the injection→measurement loop is broken and the lane is RED |
| SPAN-1 | none of §3.4's 8 sites exist in P24's table | sites registered per P24's closed-set discipline; capsules carry them; ≤1% overhead bench holds (P24 W2a gate reused) | teeth: NET-2's latency-reappearance check doubles as SPAN-1's |

**Regression-ledger rows (item 17):** one row per mechanism in
`docs/regressions/REGRESSION-LEDGER.md` (dowiz) and the P36-created bebop ledger, guardrail
types from the ledger's existing vocabulary (`CI-gate`, `grep-CI-gate`, `chaos`, `bench-gate`,
`unit/integration`). Ratchet rule verbatim: red→green proof before any "done".

---

## 7. Benchmark plan (item 10)
(1) Span overhead: P24 W2a's ≤1% instrumented-vs-not bench, re-run with §3.4's sites armed —
the gate is inherited, the number re-measured. (2) Matrix cost: weekly-pass wall-clock minutes
recorded once in the landing PR and tracked as a heartbeat-ledger field (so lane-growth cost is
a visible number, §4.2). (3) MT-1 runtime per repo recorded; scoping adjusted only via dated
manifest change. (4) NET-2 baseline: round-trip p50/p95 per profile recorded on first run —
these become the steady-state numbers later runs are compared against (principlesofchaos.org's
"measurable output" made concrete).

## 8. Links to docs & memory (item 7)
Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) · `BLUEPRINT-P36-bebop-
remediation.md` (regressions #1/#3, fence idiom, R-1b operator act, bebop ledger) ·
`BLUEPRINT-P34-mesh-kernel-wiring.md` (regression #2, kernel-rlib lane, parity sweep) ·
`BLUEPRINT-FAULT-ISOLATION-…-2026-07-17.md` P27 (primitives under test, containment
done-check discipline, grep-gate idiom) · `BLUEPRINT-NATIVE-TELEMETRY-…-2026-07-17.md` P24
(span mechanism, site table, overhead gate) · `BLUEPRINT-P-H-ops-telemetry.md` + `kernel/src/
chaos.rs` (chaos substrate) · `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` (RC-4 instance,
`hetzner:dowiz` results path) · `BLUEPRINT-P54-…` (sibling, in flight) · P56 (pending) ·
`docs/design/ARCHITECTURE.md` M6 · `docs/design/mesh-real/` (AC-6/F50 canon) ·
`docs/regressions/REGRESSION-LEDGER.md`. Memory: `test-integrity-rules-2026-06-27` ·
`verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` ·
`rust-native-bare-metal-decision-2026-07-14` (DECART duty) · `performance-priority-over-
minimal-change-2026-07-17` (span work is perf-scoped) · `cross-branch-todo-map-2026-07-10`
(bebop files → `/root/bebop-repo`, push `openbebop`). External: principlesofchaos.org ·
Claessen & Hughes ICFP 2000 · proptest-rs/proptest · DeMillo-Lipton-Sayward IEEE Computer 1978 ·
mutants.rs / sourcefrog/cargo-mutants · sqlite.org/testing.html · InfoQ Jepsen article ·
tc-netem(8).

## 9. Standard-compliance map (20 points, checkable)
1 ground truth → §0 (all cites fresh; 2 corrections: `compute_order_total` lives in
`domain.rs` not `money.rs`; P54 file is mid-write) · 2 DoD → §6 · 3 spec/TDD → §2 precedes §3;
every §6 row RED-first · 4 types/consts → §2 · 5 adversarial-incl-intentionally-failing →
§6 teeth column (every mechanism has a MUST-FAIL run) · 6 hazard-math → §4.1 · 7 links → §8 ·
8 scaling → §4.2 · 9 Linux verdicts → §4.4 · 10 bench → §7 · 11 isolation → §4.1/§4.3 ·
12 mesh awareness → §4.3 (SCOPE RULE banner inherited) · 13 rollback vocabulary → §4.3 ·
14 error-propagation gates → §3.1 is the item · 15 living memory → §4.3 · 16 tensor/eqc →
honest N/A: no new math; PB suites protect existing laws; eqc proof jobs already gated in both
CIs (cited §0.2) · 17 ledger → §6 · 18 agent instructions → §10 · 19 reuse-first → §0.2 +
both DECARTs' reject rows · 20 Hermetic → P2 Correspondence (one manifest = one truth about
what is tested), P6 Cause-and-Effect (deterministic (seed,plan) chaos; resolved-metadata
fences), P7 paired-witness (mutation testing IS the independent witness against
self-certifying test suites — the P36 §8 pattern applied to tests themselves).

## 10. Instructions for agentic workers (item 18 — zero session context)

dowiz edits in `/root/dowiz`; bebop edits in `/root/bebop-repo` (push `openbebop`, NEVER
`origin` — archived). Do NOT edit `CORE-ROADMAP-INDEX.md`/SOVEREIGN roadmap (lead's pass,
§4.5). Order: T1→T2 first (they make everything else enforceable), then T3..T7 in any order.

1. **T1 (M-2):** create `scripts/feature-matrix.txt` (§2 verbatim, re-verify each lane
   compiles-or-is-a-known-RED first), `scripts/ci-feature-matrix.sh` (runs lanes; per-push
   subset = kernel default + no-default + bebop workspace; full set under a new
   `schedule:`-triggered workflow, weekly, both repos), `scripts/ci-feature-coverage.sh` +
   exemptions file. Run §6 M-2 teeth 1+2, record.
2. **T2 (M-1, M-3, M-4):** `ci-gate-liveness.sh` + `ci-secure-defaults.sh` (+allowlist) +
   heartbeat append/staleness check. Run M-3's red-proof BEFORE P36's flip if it hasn't
   landed (§4.5). PREPARE both repos' branch-protection commands; present to operator; do
   not execute.
3. **T3 (PB):** dowiz-first: PB-1/PB-2 in `kernel/src/order_machine.rs`/`domain.rs`/
   `money.rs` test mods, copying `ports/payment.rs:644`'s proptest shape. Then bebop:
   `cargo add proptest@1.11 --dev --offline` in proto-cap (§5.1 probe — if offline
   resolution fails, record and make PB-3/4 CI-only); PB-3/PB-4; PB-5 inside the
   kernel-rlib-gated tests (AFTER P34 W-1 greens the lane — if still RED, PB-5 waits,
   noted). Run §6 PB teeth, record shrunk counterexamples.
4. **T4 (MT-1):** add the weekly cargo-mutants workflow job per repo (runner-installed
   binary, `--file` scope per §3.2, advisory exit-0 + JSONL row). Plant §6's survivable
   mutant once, confirm it surfaces, record.
5. **T5 (NET-1):** extend `ChaosSite` (2 variants, closed-set review note in the enum doc),
   implement `ChaosTransport` mirroring `ChaosStore`, write the four cells with the AC-6
   oracle for ABSENT. Run the inverted-oracle teeth.
6. **T6 (NET-2):** `scripts/netem-lane.sh` (netns-only, trap-teardown, host-qdisc
   assertion), wire into the weekly workflow (Linux runner), record first-run baselines
   (§7). Heavy local outputs → `rclone move` to `hetzner:dowiz/ci-results/`.
7. **T7 (SPAN-1):** register §3.4's sites in P24's site table (adapter-caller placement for
   bebop-side sites — the kernel fence is inviolable), re-run the overhead bench.

**Stop-and-flag conditions:** (i) any lane in T1 fails for a reason NOT in §0's known-RED set
(new finding — file it, don't absorb it); (ii) any impulse to edit P27/P24/chaos.rs internals
beyond the named extension points; (iii) executing branch protection or touching
`CORE-ROADMAP-INDEX.md` (operator/lead acts); (iv) proto-cap offline dep resolution failing
(record + CI-only fallback, never vendor by hand); (v) NET-2 detecting host-interface qdisc
mutation (harness bug — abort the lane); (vi) P34 W-1 / P36 R-1/R-2 still RED at T3/T1 time —
their rows stay RED-with-owner; fixing them here is out of scope.
