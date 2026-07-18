# BLUEPRINT P36 — Bebop remediation: kill the two live regressions, close remaining P1/P2 (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). This phase IS `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §10.5.2's **P36** and §10.3 invariant 6's named obligation. Sibling of
> `BLUEPRINT-P34-mesh-kernel-wiring.md` / `-P34B-` / `-P35-` (same folder, same discipline).
> Source review reused, not re-derived:
> `bebop-repo/docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` — amended per DoD,
> never rewritten. All work here is **bebop's own remediation**; nothing dowiz-side is
> redesigned (anti-scope, §1).
>
> **This file reads incident-first by design.** Two items are not "planned work" but actively
> broken/dangerous right now, and §0 re-verified BOTH live this pass: the `no_std` wasm32
> build of `bebop2-core` fails TODAY (E0425, `at_rest.rs:74`), and `proto-wire` ships
> `default = ["insecure-tls"]` TODAY. §3.1/§3.2 are their incident write-ups — root cause,
> minimal fix, and (the part §10.5.2 demands beyond "fix it") the **recurrence-proof**: the
> adversarial question "how would this survive a future refactor?" answered with named,
> falsifiable gates, not intentions.
>
> **The pattern that names this phase (one sentence, three instances):** *ungated (or
> unenforced) build paths rot silently* — (1) the `no_std` target HAD a CI gate and rotted
> anyway because the gate is advisory (§3.1's actual root cause); (2) the `kernel-rlib`
> cross-repo build had NO gate and rotted (the E0004 delivery-domain regression — owned by
> P34 as DoD-0/W-1, cited here as the sibling instance:
> `BLUEPRINT-P34-mesh-kernel-wiring.md` §0 row 12); (3) the insecure-TLS default was
> deliberate-but-unfenced and its "temporary" posture calcified into the shipped default.
> This is why every P36 DoD item below is CI-gate-shaped, not fix-shaped.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `bebop-repo` `main` @ `e56ba6a35258ced76752510625511f37a6367a77`
(= `openbebop/main`, ls-remote confirmed; working tree clean). Paths relative to
`/root/bebop-repo/` unless noted.

| # | Claim | Fresh `file:line` (this pass) | Inherited claim | Status |
|---|---|---|---|---|
| 1 | 🔴 **no_std wasm32 RED — re-run live this pass** | `cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` → `error[E0425]: cannot find type String in this scope --> bebop2/core/src/at_rest.rs:74:8` (`Io(String)`); compiler's own suggestion: `use alloc::string::String;`. ONE error, 5 warnings | §10.5.2 DoD-1: RED with E0425 at `at_rest.rs:74` (dispatch brief said "E0004" — that is the *delivery-domain* sibling regression's code; this one is E0425) | **CONFIRMED STILL RED** — verbatim reproduction |
| 2 | The introducing commit | `git log -- bebop2/core/src/at_rest.rs` → `d23e7aa` "feat: P09 §6 at-rest XChaCha20-Poly1305 encryption for per-hub store (F30, zero-dep, round-trip tested)" (2026-07-17) — added `at_rest.rs` (345 lines) + `event_log.rs` +31 + `lib.rs:371` `pub mod at_rest;` (unconditional include) | §10.5.2: "regression introduced by commit d23e7aa (2026-07-17), AFTER the remediation doc claimed GREEN" | **MATCH** |
| 3 | Mechanism of the break | `at_rest.rs:74` `Io(String)` inside `AtRestError` — the enum is NOT `std`-gated (only the disk-I/O impl block is, `:181` `#[cfg(feature = "std")]` using `std::fs`/`std::io` `:189-220`); under `--no-default-features` the crate is `no_std` (`lib.rs:18` `#![cfg_attr(not(feature = "std"), no_std)]`) where `String` is not in the prelude. `alloc` IS available (`lib.rs:23` `extern crate alloc`); the crate's own precedent imports it (`event_log.rs:22` `use alloc::vec::Vec`) | — | verified — the fix is one `use` line, spec §3.1(b) |
| 4 | 🔴 **The gate EXISTED and fired uselessly — the actual root cause** | `.github/workflows/ci.yml:48-49`: `bebop2-core — pure no_std build (empty-import wasm32, --no-default-features)` → `cargo build -p bebop2-core --no-default-features --target wasm32-unknown-unknown`, in job `rust-hardened` (`:36`); present since the clean-slate root `f38f2c5` (2026-07-14, `git log -S wasm32-unknown-unknown -- .github/workflows/ci.yml`); triggers on push to `main` (`:8-12`). `d23e7aa` is ON main and pushed (`openbebop/main` contains it) ⇒ the workflow ran RED **after** the commit was already on main — CI is advisory (direct pushes, no required checks) | §10.5.2 DoD-1(a): "this target build added to CI so it cannot silently regress again" — **the build is ALREADY in CI** | **DRIFT — the inherited remedy is aimed at the wrong gap.** The missing piece is ENFORCEMENT (pre-push + required-checks), not the CI step. §3.1(c) re-aims DoD-1(a) accordingly |
| 5 | The stale-GREEN doc claim | `docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md:31-36`: "Bare-metal --no-default-features build: GREEN … 0 errors … Verified 2026-07-14 via bebop2/core/scripts/check-wasm32.sh" — true when written, false since `d23e7aa` | §10.5.2: "the doc is currently lying about build state" | **CONFIRMED** — correction is DoD-1(b); the cited `check-wasm32.sh` EXISTS (`bebop2/core/scripts/check-wasm32.sh`) and is the reusable local gate for §3.1(c) |
| 6 | 🔴 **insecure-tls default-on — re-checked live** | `bebop2/proto-wire/Cargo.toml:50` `default = ["insecure-tls"]`; `:52-54` comment: "the iroh/QUIC client accepts ANY TLS cert … Default-ON preserves current behavior"; verifier fork `iroh_transport.rs:216-228` (`InsecureAcceptAny` `:156` under the feature; webpki-roots Mozilla bundle without it, `:225-228`); wss client accept-any under the same default (`wss_transport.rs:210`) | §10.5.2 DoD-2: "currently ships default=["insecure-tls"] — a live security footgun" | **CONFIRMED STILL DEFAULT-ON** |
| 7 | The flip's test surface is ALREADY feature-gated | accept-any-dependent tests carry `#[cfg(feature = "insecure-tls")]` / `#[cfg(all(test, feature = "insecure-tls"))]` (`wss_transport.rs:337,776,894`); the hardened-path negative test carries `#[cfg(not(feature = "insecure-tls"))]` (`:836` — "webpki rejects self-signed"); CI already runs BOTH lanes (`ci.yml:46-47` default lane, `:50-51` `--no-default-features` lane, green per §10.2) | — | verified — the flip inverts which lane is "default"; no test rewrite needed, §3.2(b) |
| 8 | The default's blast radius includes the mesh spine | `bebop2/mesh-node/Cargo.toml:10` `bebop-proto-wire = { path = ../proto-wire }` with DEFAULT features ⇒ `delivery-domain[kernel-rlib]` → `mesh-node` → `proto-wire[insecure-tls]` — P34's `dowiz-mesh-adapter` closure inherits accept-any TLS **today** | — | **NEW finding** — the flip protects the P34 wiring too; named in §3.2, owned here |
| 9 | P1 dual field-math — live | `rust-core/src/lib.rs` (1346 lines, package `bebop-core` per `rust-core/Cargo.toml:2`): Chebyshev spectral propagator with global `Mutex` singletons `STATE`/`ACCUM`/`ENERGY`/`CALLS` (`:37,:48,:152,:158`); duplicate confirmed by the OTHER side's own doc — `bebop2/core/src/chebyshev.rs:1-9`: "Matches old `rust-core::spectral_propagate` EXACTLY — same fexp/fcos shims, same trapezoid, same three-term recurrence … only structural change is CSR passed BY REFERENCE instead of a global Mutex"; plus `bebop2/core/src/field.rs` (spectral kernel) and `lib.rs:343` `linalg` ("the single authoritative eigensolver") | §10.5.2 DoD-3: "duplicate Chebyshev/VSA singleton in rust-core/src/lib.rs:25-159"; EXCELLENCE B2 claimed the duplicate "is not in the tree" | **CONFIRMED live — EXCELLENCE B2's claim is WRONG as written** (it held for the eigensolver, not the propagator). §10.5.2's cite is right; `:25-159` under-states the span (the singletons + propagator run through the file). §3.3 |
| 10 | rust-core is CI-load-bearing | `ci.yml:25-31`: `cargo test -p bebop-core --test eqc_proofs_test` + 4 eqc examples run against `bebop-core` | — | verified — consolidation must keep the eqc proofs green (constraint, §3.3) |
| 11 | P2 Node/Docker cruft — live inventory | root `package.json` (`:56-64` scripts = thin cargo wrappers + `verify` chaining `verify-doc-claims.mjs` + `guardrail-falsifiable-proof.mjs`; `:65-67` engines node>=20); `docker-compose.sovereign.yml` + `Dockerfile.sovereign` at root; **10** `.mjs` files in `scripts/` (the 4 named gates `verify-doc-claims`, `guardrail-falsifiable-proof`, `logic-gate`, `law-hooks` + 6 others); ZERO `.mjs` references in any workflow (`grep -rn "\.mjs" .github/workflows/` → 0); BUT `verify-doc-claims.mjs` IS live-wired in the untracked local `.git/hooks/pre-commit` (blocks commits; `core.hooksPath` unset), and `scripts/build-unikernel.sh` references `docker-compose.sovereign.yml` (P35 §0 row 11's handoff) | §10.5.2 DoD-4: "port or delete — after verifying none are CI-wired or codegen inputs" | verified — two live wires found (local hook + unikernel script); the verify-before-delete step has real teeth here, §3.4 |
| 12 | P2 QRNG — live | `proto-cap/src/entropy.rs`: `SeedPool` `:197` ("fail-closed seed pool mixing the OS floor with optional advisory sources", SHA3-512 mix + monotonic counter `:16,:200-203`), `AnuQrng` `:86` (advisory, disabled by default, `enable()` `:113`), `OsEntropy` `:64` over `bebop2_core::rng::entropy_provider`; exported at `proto-cap/src/lib.rs:53`. **Zero consumers**: no call site outside `entropy.rs` (grep); production keying draws `entropy_provider()` directly (`core/src/rng.rs:500,:511` — ChaCha20 DRBG seed/reseed) | §10.5.2 DoD-5: "AnuQrng is off-by-default today; real keygen uses a separate OS-only entropy_provider()" | **MATCH** — SeedPool is built-but-stranded; keygen bypasses the mix (the EXCELLENCE M-7 finding, still true). §3.5 |
| 13 | Done inventory (spot-checked, not re-litigated) | C1 CT decap + C2 rng nomem + C3 gated keygen + C4/C4b CT scalar/mod_l + C6 derive_pq_seed + C7 bounded/TLV — per §10.5.2's done list; sovereign-guards job runs the standing fences (`ci.yml:60-94`: no-scoring, G1-G5, C3, B3, B4, crdt-fence, kernel fence, claim lint, empty-import); **P4 publish DONE** (OpenBebop public) | §10.5.2 done inventory | accepted as done — anti-scope: C4b not re-reported, P4 not reopened, no batch-verify work |
| 14 | No regression ledger exists bebop-side | no `docs/regressions/` in bebop-repo (dowiz has `docs/regressions/REGRESSION-LEDGER.md`) | — | **gap owned here as R-0** — P34B §5 and P35 §5 both need bebop rows; this phase creates the file |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — what P36 owns and what it deliberately does NOT own

**P36's single sentence:** restore the two shipped-broken postures (no_std build, TLS default)
with minimal fixes PLUS recurrence-proof gates aimed at the *actual* root cause (enforcement,
not just detection), then close the remaining P1 (single field-math authority) and P2
(cruft-with-verified-wires, QRNG-through-the-mix) items — all inside bebop-repo.

**P36 owns (build items §3):**

| Item | §10.5.2 DoD | Content |
|---|---|---|
| R-0 | (new, substrate) | `bebop-repo/docs/regressions/REGRESSION-LEDGER.md` created (dowiz format) — the home P34B/P35 rows already reference |
| R-1 | DoD-1 | no_std regression: 1-line fix + enforcement ratchet (pre-push gate + required-checks) + doc correction + recurrence proof |
| R-2 | DoD-2 | insecure-tls: default flip + CI fence + lane inversion + honest runtime-consequence note |
| R-3 | DoD-3 | dual field-math: parity-pin → consolidate to bebop2-core → eqc proofs stay green |
| R-4 | DoD-4 | Node/Docker cruft: port-or-delete with the two live wires (§0 row 11) handled first |
| R-5 | DoD-5 | QRNG: keygen-through-SeedPool or dated OS-only acceptance (operator decision framed, both outcomes falsifiable) |

**P36 does NOT own (anti-scope, binding):**

- **Nothing dowiz-side is redesigned.** P36's only dowiz-visible effect is that P34's adapter
  closure stops inheriting accept-any TLS (§0 row 8) — a consequence, not a work item.
- **The delivery-domain E0004 regression is P34's** (DoD-0/W-1 — `BLUEPRINT-P34` §0 row 12,
  §3.1). Cited as the sibling instance of the rot pattern; not fixed here, not double-fixed.
- **No new crypto primitives; C4b is NOT re-reported as new work; P4 (publish) is NOT
  reopened; no batch-verify performance work** — §10.5.2 anti-scope verbatim (the batch-accept
  honesty question is settled: every accept re-verifies singly; correctness over speed).
- **The `iroh = []` feature retirement is P34B V-3's** — R-2 touches the same manifest's
  `[features]` block; §4.5 sequences the two edits to avoid a collision.
- **`AnuQrngRemote` completion (TLS client, timeouts)** — EXCELLENCE P2b's larger half stays
  future work with its trigger (an operator actually wanting live QRNG advisory); R-5 is only
  the keygen-path decision.
- **P2c model-agnostic Proposer seam** — real item, ZERO overlap with the regression/crypto
  surface; deferred to its own lane with the EXCELLENCE cite (this blueprint lists it so it is
  not lost, but it is not P36 DoD — §10.5.2 scoped P36's P2 row to cruft+QRNG only).

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── bebop2/core/src/at_rest.rs — R-1's ENTIRE production diff (one line) ──
use alloc::string::String;   // no_std: String must be imported from alloc
                             // (precedent: event_log.rs:22 `use alloc::vec::Vec`)

// REJECTED alternative (DECART, one line): #[cfg(feature = "std")] on the
// `Io(String)` variant — makes the enum's SHAPE feature-dependent, so every
// exhaustive `match AtRestError` downstream needs its own cfg fork: the exact
// exhaustiveness-drift class P34 §0 row 12 just demonstrated. One import wins.
```

```toml
# ── bebop2/proto-wire/Cargo.toml — R-2's production diff ──
[features]
default = []                 # was: ["insecure-tls"]  (C5 closure — hardened IS the default)
insecure-tls = []            # opt-in only, dev/local-first
insecure-test = []           # unchanged (already off-default)
```

```bash
# ── scripts/ci-no-insecure-default.sh (NEW — R-2's fence, sovereign-guards job) ──
# Exits 1 if bebop-proto-wire's resolved DEFAULT feature set contains any
# feature matching (?i)insecure. Reads `cargo metadata` (not a text grep of the
# manifest) so a default re-entering via feature-indirection is also caught.
```

```bash
# ── .githooks/pre-push (NEW — R-1's local half of the enforcement ratchet) ──
# Runs bebop2/core/scripts/check-wasm32.sh (REUSED, §0 row 5) before any push.
# Repo-tracked; activated by `git config core.hooksPath .githooks` (documented
# in CONTRIBUTING). Honest limit stated in-file: local hooks are opt-in — the
# authoritative gate is the required-checks setting (§3.1c-3).
```

Named constants for R-3: none new — the consolidation target types already exist
(`bebop2/core/src/chebyshev.rs` / `field.rs`); R-3 deletes/re-points, it does not mint.

---

## 3. Build items — incident write-ups first, then P1/P2 (items 3, 5)

### 3.1 R-1 — 🔴 INCIDENT: the no_std wasm32 regression (DoD-1)

**Timeline (from §0 rows 2-5, all git-verified):**
2026-07-14 — clean-slate root `f38f2c5` ships WITH the wasm32 no_std CI step and the
`check-wasm32.sh` proof script; EXCELLENCE doc records GREEN (true).
2026-07-17 — `d23e7aa` adds `at_rest.rs` with an un-imported `String` in a non-cfg-gated enum;
pushed directly to `main`. The `rust-hardened` job runs on that push and goes RED — after the
fact, blocking nothing.
2026-07-18 (this pass) — the exact command re-run: still RED, E0425, verbatim.

**Root cause, stated precisely:** not a missing gate (the gate exists, §0 row 4) and not an
exotic bug (a one-line prelude difference between std and no_std). The root cause is
**enforcement topology**: nothing runs the no_std build *before* code reaches `main` — no
local pre-push check, no required-status-check on the branch. Any future contributor (human or
agent) editing `bebop2-core` under a std-feature dev loop will reproduce this class with
probability ~1 over enough commits. The fix must therefore change WHERE the gate runs, not
merely re-green the build.

**(a) Reproduce RED first** (already done this pass — the incident IS the red proof; T1 in
§10 re-confirms at execution time per the P34 T1 discipline).

**(b) The minimal fix:** §2's one `use` line. Nothing else in `at_rest.rs` changes — the
`std`-gating of the disk-I/O block (`:181`) is correct as-is (the `Io` variant carrying an OS
error string is legitimately part of the portable error type; only its *producer* is
std-only). GREEN = the §0-row-1 command exits 0 AND `check-wasm32.sh` passes (debug+release,
empty-import section re-verified — the script checks more than compilation; use it, not the
bare build, as the acceptance command).

**(c) The recurrence-proof (the DoD's adversarial core — "how does this survive a future
refactor?"):** three independent layers, each with its own falsifier:

1. **Compile-visible, zero-infrastructure:** a `no_std`-shaped unit test inside `bebop2-core`
   — `#[cfg(not(feature = "std"))] mod no_std_surface_pins { … }` that constructs
   `AtRestError::Io(String::from("x"))` and one value of every other public error/enum type
   under `--no-default-features`. This makes the *type surface* itself part of the tested
   contract on the no_std path, so the next un-imported-prelude-type fails `cargo test`, not
   just the separately-invoked target build. Falsifier: delete the module → nothing pins the
   surface.
2. **Local gate, before push:** `.githooks/pre-push` running `check-wasm32.sh` (§2). Honest
   limit stated: opt-in (a hook can be skipped) — which is exactly why layer 3 exists.
3. **Authoritative gate, before merge:** branch protection on `openbebop` `main` requiring
   the `rust-hardened` job (🔴 **operator-gated act** — a GitHub settings change, one command:
   `gh api -X PUT repos/SyniakSviatoslav/OpenBebop/branches/main/protection` with
   `required_status_checks.contexts=["Rust hardened (--no-default-features)"]`; per the
   standing never-bypass-human-gates rule the agent PREPARES this and the operator executes/
   approves it). Falsifier: `gh api repos/…/branches/main/protection` shows no required
   checks → the ratchet is not engaged, DoD-1 is NOT done regardless of the build being green.
   **This converts the workflow from advisory to blocking — the actual root-cause fix.**

**(d) Doc correction (DoD-1's "(b)"):** append a dated correction note to
`EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md:31-36`'s claim block (append-only,
aebbbe199 style): GREEN 2026-07-14 → regressed by `d23e7aa` 2026-07-17 → restored + enforced
by R-1 (commit hash filled at land time). The doc's status table is amended, the review is not
rewritten (§10.5.2's own instruction).

**Adversarial cases (teeth, run once, recorded in the PR):** (i) revert-dry-run — stash the
fix, run `check-wasm32.sh`: MUST fail (proves the gate detects exactly this incident);
(ii) hook-bypass drill — `git push --no-verify` in a scratch clone against a protected test
branch: the push lands but the PR/merge is blocked by the required check (proves layer 3
catches what layer 2 misses); (iii) surface-pin teeth — remove the `use` line with the pin
module present: `cargo test --no-default-features` (native target) fails too, proving layer 1
fires without needing the wasm toolchain installed.

### 3.2 R-2 — 🔴 INCIDENT: insecure-TLS is the shipped default (DoD-2)

**Anatomy:** C5's original finding (EXCELLENCE `:52`) was "gate `InsecureAcceptAny` behind a
feature" — done. But the feature landed **default-on** ("Default-ON preserves current
behavior", `Cargo.toml:52-54`), so every default build of every consumer — including
`mesh-node` and, transitively, P34's future `dowiz-mesh-adapter` closure (§0 row 8) — accepts
ANY TLS certificate on both carriers (quinn client `iroh_transport.rs:216-222`, wss client
`wss_transport.rs:210`). The signed-frame hybrid gate remains the real *authentication* layer
(the comment's mitigation is true), but channel privacy/MITM-resistance silently depends on a
feature nobody opted into. A "temporary dev default" with no fence is how postures calcify —
instance 3 of the header pattern.

**(a) The flip:** §2's diff — `default = []`. Consumers needing accept-any (local dev loops)
opt in explicitly: `--features insecure-tls`. No source change in `iroh_transport.rs`/
`wss_transport.rs` — the cfg forks already exist and the hardened path is already
implemented-and-tested (webpki-roots verify + reject-self-signed test, §0 rows 6-7). This is
deliberately NOT "refuse to compile until wired" (the Cargo.toml comment's old promise) — the
hardened verifier EXISTS now, so the flip is behavior-selection, not a build wall.

**(b) Lane inversion, no test rewrite:** today CI's default lane exercises accept-any tests
and the `--no-default-features` lane exercises hardened tests (§0 row 7). After the flip the
SAME two lanes cover the SAME two sets with the labels swapped — plus one addition so the
accept-any tests don't silently stop running:

```yaml
      # ci.yml rust-hardened job — R-2 lane addition (the OLD default, now opt-in):
      - name: proto-wire — dev-loop tests (--features insecure-tls, opt-in lane)
        run: cargo test -p bebop-proto-wire --features insecure-tls
```

Falsifier for the silent-skip hazard: the run log of the default lane must show the gated
tests as *not run* while the opt-in lane shows them green — both lanes named in the DoD.

**(c) The fence (the recurrence-proof):** `scripts/ci-no-insecure-default.sh` (§2) added to
the `sovereign-guards` job next to its ten siblings (`ci.yml:60-94` — this repo's established
fence idiom; reuse, not invention). It reads `cargo metadata`'s resolved default features for
`bebop-proto-wire` and exits 1 on any `(?i)insecure` match — so the default cannot re-acquire
the feature via rename or indirection without a named CI RED. Protected by the same
required-checks ratchet as §3.1(c)-3 (one operator act covers both incidents — `rust-hardened`
and `sovereign-guards` both become required).

**(d) Honest runtime-consequence note (not buried):** after the flip, a default-built node
REFUSES peers presenting self-signed certs (webpki-roots verification). Local-first mesh
deployments without CA-signed certs must either build with `--features insecure-tls`
(explicit, visible in their build command — the point of the flip) or wait for the named
follow-up already recorded in the code: "a prod operator-cert" story
(`iroh_transport.rs:205`) — operator-issued certificate pinning as the sovereign alternative
to the Mozilla bundle. That follow-up is real design work and is NOT smuggled into P36; it is
named with its trigger (first hardened two-node deployment on operator infrastructure).

**Adversarial cases:** (i) fence red-proof — run `ci-no-insecure-default.sh` against the
CURRENT (pre-flip) manifest: MUST exit 1 (recorded in the PR — the fence detects the live
incident, not a hypothetical); (ii) MITM teeth — the existing reject-self-signed test
(`wss_transport.rs:836` lane) now runs in the DEFAULT lane: a default-built client refusing an
untrusted cert is asserted by CI on every push, which is the incident's inverse stated as a
permanent test; (iii) closure teeth — `cargo tree -p bebop-mesh-node -e features | grep
insecure` empty after the flip (the §0-row-8 inheritance is severed and stays checkable).

### 3.3 R-3 — P1: one spectral-propagator authority (DoD-3)

Spec: §0 rows 9-10. `bebop2/core/src/chebyshev.rs` already claims exact parity with
`rust-core::spectral_propagate` BY DESIGN (its header, §0 row 9) — R-3's job is to make that
claim load-bearing and then remove the duplicate:

1. **Parity pin first** (verify-before-delete): a test feeding K fixed CSR graphs + impulse
   vectors through BOTH propagators, asserting element-wise agreement within documented f64
   tolerance (the eig-parity test family in `bebop2/core/tests/eig_parity.rs` is the in-repo
   template). If parity fails → STOP, finding-not-cleanup (same rule as P34B §3.4-1).
2. **Re-point or re-export:** `rust-core`'s public surface delegates to
   `bebop2_core::{chebyshev, field}` (thin re-export shims keeping `bebop-core`'s API), OR the
   eqc proofs/examples (`ci.yml:25-31`, §0 row 10) move their target to `bebop2-core` and the
   rust-core propagator+singletons are deleted. Choice made at implementation by whichever
   keeps the eqc suite green with the smaller diff (both end states satisfy the falsifier).
3. **Falsifier (grep, §10.5.2's own):** exactly one three-term-Chebyshev-recurrence
   implementation outside tests — `grep -rn "three-term" rust-core/src bebop2/core/src`
   resolves to one production site; the four `Mutex` singletons (`:37,:48,:152,:158`) are gone
   (the by-reference design was the stated point of the port).

**Adversarial:** the parity pin run once with a deliberately perturbed coefficient (1 ULP-class
tampering in a test-local copy) must FAIL — proving the tolerance actually binds (a sweep with
a vacuous epsilon would rubber-stamp divergence).

### 3.4 R-4 — P2: Node/Docker cruft, with the wires verified FIRST (DoD-4)

§0 row 11's live inventory found exactly two live wires — both must be handled before any
deletion (standing verify-before-delete directive, now with named targets):

1. **`verify-doc-claims.mjs` ← local pre-commit hook (live, blocking).** Port the *checks that
   still check anything* to the repo's native idiom — a shell/Rust gate in `scripts/`
   (`ci-doc-claims.sh`) run in `sovereign-guards` alongside the existing claim lint
   (`ci.yml:92` — which already covers the CLOSED-claims-cite-a-test half; the port may be a
   deduplication into it). Then repoint `.githooks/` (created in §3.1) and delete the `.mjs`.
2. **`docker-compose.sovereign.yml` ← `scripts/build-unikernel.sh` (live reference — P35's
   handoff).** Decide the script's own fate first: if the unikernel path is still roadmap
   (sovereign Phase-3 per the DK-06 blueprint), rewrite the script's compose dependency out;
   if not, retire script+compose together with a dated note. No deletion while the reference
   stands.
3. **The rest — delete after a recorded zero-wire grep:** `package.json` (scripts are cargo
   one-liners, `:56-64`; the `verify` chain dies with the ports), `Dockerfile.sovereign`, and
   the remaining 8 `.mjs` (the other 2 named gates `logic-gate`/`law-hooks` first re-checked
   for hook wiring — §0 found them wired nowhere, but the grep is re-run and pasted into the
   PR at execution time, not trusted from this pass).

Falsifiers: `ls package.json *.mjs docker-compose.sovereign.yml Dockerfile.sovereign` in
bebop-root → only sanctioned survivors; every deletion commit body contains its zero-wire grep
output. **Adversarial:** after the ports, `git stash` the ported `ci-doc-claims.sh` and
introduce a doc claim violation the old `.mjs` would have caught → `sovereign-guards` must go
RED (the port lost no teeth — run once, recorded).

### 3.5 R-5 — P2: QRNG through the SeedPool, or the dated OS-only decision (DoD-5)

Live state (§0 row 12): `SeedPool` is built, fail-closed, and stranded; keygen seeds draw
`entropy_provider()` directly. Per the standing stance (QRNG **seeds/mixes, never replaces**
OS entropy) both §10.5.2 outcomes remain legitimate; this blueprint frames the choice and its
costs so the operator decides once, falsifiably:

- **Option A (recommended): route key-seed acquisition through `SeedPool`.** With zero
  advisory sources enabled (the default — `AnuQrng` stays off), `SeedPool` output =
  SHA3-512(OS floor ‖ counter): OS-equivalent entropy, plus the mix/counter hygiene (R8
  never-repeat) and the single place a future advisory source plugs in. Cost: a crypto-path
  change → 🔴 operator-gated + independent review (crypto-safe-first-pass discipline), plus a
  KAT-style determinism check that test-only deterministic keygen paths (`C3`'s gates) are
  untouched.
- **Option B: dated OS-only acceptance note** (in the EXCELLENCE doc's amendment + this
  file's close-out): production keygen uses `entropy_provider()` directly by decision;
  `SeedPool` remains the advisory-mixing seam for non-key uses. Cost: none now; the M-7
  "bypasses the mix" note becomes policy instead of debt.

Falsifier either way: Option A → `grep -rn "entropy_provider()" bebop2/core/src/rng.rs`'s
keygen call sites route via the pool (one named seam), with the review recorded; Option B →
the dated note exists and M-7 is closed-as-decided in the amended status table. No third
state (open-ended "later").

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 4.1 Hazard-safety as math (item 6)

- **Silent no_std rot:** after R-1, reintroduction requires simultaneously (a) not tripping
  the in-crate surface-pin test, (b) skipping the pre-push hook, and (c) merging past a
  required RED check — three independent gates; the conjunction is the recurrence-proof.
- **Accept-any TLS re-entering the default:** requires defeating the metadata fence
  (rename-proof, regex on resolved features) inside a required check — a named CI RED, not a
  review-attention item.
- **MITM on default builds:** post-flip, unverified-cert acceptance is *unrepresentable in the
  default cfg* (the `InsecureAcceptAny` type is not compiled in); the hybrid signed-frame gate
  remains the auth layer regardless — defense in depth in both directions, neither claimed as
  the other.
- **Dual-authority numeric drift (R-3):** two propagators can silently diverge (the P34 §0
  row 18 class, numeric flavor); after R-3 there is one implementation, and until deletion the
  parity pin makes divergence a named RED. The `Mutex`-singleton hazard (cross-test
  serialization + hidden global state) is removed by construction, not by care.
- **Deleting a live wire (R-4):** unreachable by process — both known wires are enumerated
  with their handlers, and every deletion commit must embed its zero-wire grep (a deletion
  without one is DoD-falsified, reviewable after the fact).
- **Keygen entropy downgrade (R-5):** both options preserve the OS floor by construction
  (`SeedPool` is floor-mandatory, fail-closed; Option B changes nothing) — no path weakens
  entropy; the decision only concerns hygiene layering.

### 4.2 Schemas designed for scaling (item 8)

No data schemas change in P36. The scaling statement owed: the CI lane count grows by 2 steps
(opt-in TLS lane, fence) on jobs measured in minutes — negligible against the wall-clock
budget; the enforcement ratchet's cost is one required-check round-trip per merge, the
explicit price of non-advisory CI (accepted, that is the point). Stated so the tradeoff is a
decision, not an accident.

### 4.3 Isolation (11), mesh awareness (12), rollback vocabulary (13), living memory (15)

- **Isolation:** R-1's diff is one import in one file; R-2's is manifest+CI only; R-3 is
  confined to the two field-math surfaces with the eqc suite as the canary; R-4 touches no
  `bebop2/` crate code at all. No item crosses into proto-cap/delivery-domain source — the
  P34 seam files are untouched (§4.5's non-collision guarantee).
- **Mesh awareness:** R-2 changes the TRANSPORT trust posture, not wire formats or budgets;
  peers on mixed builds interop at the frame layer regardless (signed frames verify
  identically; only channel establishment tightens). Named migration reality: a hardened node
  and an accept-any node still connect iff the *server* presents a cert the hardened side
  trusts — operator-cert follow-up named in §3.2(d).
- **Rollback (item-13 vocabulary, used precisely):** P36 claims **Self-Termination /
  unrepresentable-state** (post-flip default cfg cannot accept an unverified cert; post-R-3
  the second propagator does not exist) and adds **deterministic re-entry for process state**
  only in the trivial sense (every item is a plain `git revert`; R-4 deletions are recoverable
  from history — move-not-delete honored by git itself plus the dated notes). Self-Healing is
  NOT claimed anywhere in P36.
- **Living memory (item 15):** reasoned exemption — no stored data with temporal access
  changes; the regression LEDGER (R-0) is itself the living-memory instrument for this class
  (append-only lesson store keyed by named guardrails).

### 4.4 Linux-discipline verdict framework (item 9)

R-1's required-checks ratchet = **EXTENDS** (enforcement topology is new for this repo,
justified by a demonstrated advisory-CI failure); R-1's surface-pin test = **REINFORCES**
(the repo's compile-time-gate culture, e.g. exhaustive matches); R-2 = **ALREADY-EQUIVALENT**
("secure by default" — the kernel's own deny-by-default doctrine applied to its transport
default) + **REINFORCES** via the fence idiom reuse; R-3 = **ALREADY-EQUIVALENT** ("one
implementation of one concept" — completing the port `chebyshev.rs` itself documents);
R-4 = **ALREADY-EQUIVALENT** (dead-code removal with reference-checking, the kernel janitorial
norm); R-5 = **REINFORCES** (single entropy seam). Nothing here is GAP or DOES-NOT-TRANSFER.

### 4.5 Non-contradiction constraints (sequencing, hard)

- **P34 independence both ways** (§10.5.2: "do NOT serialize the wiring behind remediation"):
  no P36 item touches `delivery-domain`, `proto-cap`, or any P34 seam file. R-2's flip lands
  whenever ready; P34's adapter simply inherits the safer default at its next build.
- **Manifest collision with P34B V-3:** both edit `proto-wire/Cargo.toml`'s `[features]`
  block (R-2 flips `default`, V-3 deletes `iroh`). Sequence: whichever lands second rebases
  trivially (disjoint lines) — but do NOT batch them into one commit (V-3 is unGated, R-2 is
  an incident fix, R-5-style operator gates differ; separable commits keep separable reverts).
- **R-5 and R-3 are gated differently:** R-5 Option A is red-line crypto (operator + review);
  R-3 is numeric consolidation (normal review). Do not let R-3's merge carry R-5's change.
- **DoD-1 blocks any no_std/wasm32 consumer of bebop2-core; DoD-2 blocks P34B DoD-3's
  hardened-iroh posture** (§10.5.2's stated edges — P36 is upstream of both, another reason
  not to let it drift).

---

## 5. DoD — the incident-remediation checklist (item 2; falsifiable, RED→GREEN)

Sharpens §10.5.2's DoD-1..5, re-aiming DoD-1(a) at enforcement per §0 row 4 (a strengthening —
the inherited wording is satisfied AND exceeded; nothing weakened):

| Item | §10.5.2 | RED (fails before — verified live) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|---|
| R-0 | (new) | no bebop ledger exists (§0 row 14) | `docs/regressions/REGRESSION-LEDGER.md` exists, dowiz format, seeded with the R-1/R-2 rows + the P34B/P35 handoff rows | `ls docs/regressions/REGRESSION-LEDGER.md` |
| R-1a | DoD-1 fix | §0-row-1 command RED (E0425) — reproduced this pass | `bash bebop2/core/scripts/check-wasm32.sh` green (build + empty-import, debug+release) | re-run both; falsified by any at_rest cfg-fork of the enum (the rejected alternative) |
| R-1b | DoD-1(a) enforce | gate advisory: `d23e7aa` reached main through a RED workflow (§0 row 4) | surface-pin test in-crate + tracked `.githooks/pre-push` + **required checks on `openbebop` main incl. `rust-hardened` (operator-executed)** | `gh api repos/SyniakSviatoslav/OpenBebop/branches/main/protection` lists the contexts; falsified by empty required checks — **DoD-1 is NOT done on a green build alone** |
| R-1c | DoD-1(b) doc | EXCELLENCE `:31-36` claims GREEN, false since 07-17 | dated correction note appended (append-only) | `grep -n "d23e7aa" docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` non-empty |
| R-2a | DoD-2 flip | `Cargo.toml:50` `default = ["insecure-tls"]` — reproduced this pass | `default = []`; both CI lanes green incl. the NEW opt-in lane; reject-self-signed test now in the default lane | `grep -n 'default = \[\]' bebop2/proto-wire/Cargo.toml`; `cargo tree -p bebop-mesh-node -e features \| grep -i insecure` empty |
| R-2b | DoD-2 fence | no fence; the "temporary" default calcified for 4+ days | `ci-no-insecure-default.sh` in sovereign-guards, red-proof against the pre-flip manifest recorded, job required on main | falsified by the fence absent from `ci.yml` or its red-proof unrecorded |
| R-3 | DoD-3 | two propagators live; parity claimed in prose only (§0 row 9) | parity pin green (+tamper teeth) → single production implementation; eqc suite green; Mutex singletons gone | the §3.3-3 grep resolves to one site; `grep -n "static STATE" rust-core/src/lib.rs` empty |
| R-4 | DoD-4 | 10 .mjs + package.json + compose/Dockerfile live; two verified wires (§0 row 11) | both wires ported/retired with teeth-preservation proof; remainder deleted with embedded zero-wire greps | the §3.4 `ls` falsifier; every deletion commit body carries its grep output |
| R-5 | DoD-5 | SeedPool stranded; keygen bypasses the mix (§0 row 12) | Option A routed+reviewed OR Option B dated note — one of the two, recorded | the §3.5 per-option falsifiers; a third state (silence) fails the row |

**Permanent regression rows** (item 17, seeded into R-0's ledger): (1) "P36 no_std surface —
guardrails: `no_std_surface_pins` + `check-wasm32.sh` pre-push + required `rust-hardened`";
(2) "P36 secure-default TLS — guardrails: `ci-no-insecure-default.sh` + default-lane
reject-self-signed test"; (3) "P36 single spectral propagator — guardrail: parity pin until
deletion, then the one-site grep lint". Ledger ratchet rule verbatim: red→green proof before
any "done", no weakened/ignored/inverted test may stand.

---

## 6. Benchmark plan (item 10) — flatness proofs, not feature benches

P36 changes no hot path by intent; the benches exist to PROVE that:

1. **Crypto flatness:** the existing criterion crypto benches run before/after R-1/R-2
   (a one-line import and a feature default cannot move them; movement = process error or
   environmental noise → investigate, do not hand-wave).
2. **R-3 propagator parity bench:** `spectral/propagate_fixed_graph` measured on BOTH
   implementations before consolidation (the by-reference port SHOULD be ≥ the Mutex version —
   record the actual number; if the retained implementation is measurably slower, that is a
   finding to record, and per the standing performance-priority directive it is fixed, not
   accepted silently). After consolidation the surviving bench is the permanent row.
3. **CI wall-clock:** the added lanes' minutes recorded once in the PR (the §4.2 cost made a
   number). Baselines in `BENCH_HISTORY.md`; regression gating = P-H's deliverable (same
   dependency note as the siblings).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.3 invariant 6 + §10.5.2 P36
(charter; anti-scope restated §1) ·
`bebop-repo/docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` (the review this
phase completes; amended R-1c, never rewritten) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (§0
row 12 = the sibling ungated-rot instance, owned there as DoD-0/W-1 — cross-cited per the
dispatch instruction, fixed exactly once, by P34) · `BLUEPRINT-P34B-mesh-remaining-halves.md`
(V-3 manifest-collision sequencing §4.5; DoD-2→iroh-hardening edge) ·
`BLUEPRINT-P35-docker-swap-wasm-runtime.md` (§0 row 11 handoff consumed by R-4-2) ·
`docs/regressions/REGRESSION-LEDGER.md` (dowiz — the format R-0 mirrors). Memory:
`crypto-safe-first-pass-2026-07-14` (R-5-A's review gate; the correctness-over-speed
precedent behind the batch-verify anti-scope) · `verified-by-math-2026-07-07` +
`test-integrity-rules-2026-06-27` (teeth discipline; no fake-green) ·
`never-bypass-human-gates-2026-06-29` (R-1b's required-checks act is PREPARED by the agent,
EXECUTED by the operator; R-5 operator-decided) · `anu-ananke-strict-discipline-feedback-
2026-07-17` (R-4's actually-delete-confirmed-dead-code directive — with the verify-first
teeth) · `environment-and-ops-facts-2026-07-16` + `secrets-exposure-incident-2026-07-03`
(context for why enforcement topology, not diligence, is the fix) ·
`cross-branch-todo-map-2026-07-10` (ALL P36 files → `/root/bebop-repo`, push `openbebop`,
never `origin` — archived read-only) · `worktree-remote-push-collision-avoidance-2026-07-18`.
Supersedes: EXCELLENCE B2's "duplicate … is not in the tree" claim (§0 row 9 corrects it with
cites) and its `:31-36` GREEN status (R-1c amends it in place).

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): one spectral propagator (R-3), one
  entropy seam (R-5), one build-posture default (R-2 — the hardened path is THE path, not a
  parallel truth), one runtime story (R-4 completes the cargo-only world the clean-slate
  publish promised).
- **P6 CAUSE-AND-EFFECT** (determinism as law): every remediation carries a reproducible
  cause chain (git-verified timeline §3.1; metadata-resolved fence §3.2c) and a deterministic
  detector — no gate here depends on reviewer attention.
- **P7 GENDER** (paired creation, no self-certification): the incident pattern IS a
  self-certification failure — the repo certified its own posture in prose (EXCELLENCE GREEN,
  "Default-ON preserves current behavior") while the enforcing witness was absent or advisory.
  Every R-item installs an independent witness: required checks that outrank the author,
  fences that read resolved metadata rather than intentions, parity pins that outrank a
  header's "EXACTLY", and an amended doc that cites its corrector.

(P1/P3/P4/P5 are not load-bearing for this remediation and are not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 14 rows; both regressions re-reproduced live this pass; 3 corrections of inherited claims (E0004→E0425 label, "gate missing"→"gate advisory", EXCELLENCE B2) + 2 new findings (mesh-spine TLS inheritance, two live cruft wires) |
| 2 DoD | §5 — incident-checklist form; RED verified live, GREEN falsifiable, enforcement explicitly part of done (R-1b's "not done on a green build alone") |
| 3 spec/event-driven TDD | §2 diffs-as-spec precede §3; RED exists by construction (the incidents); teeth runs are ordered before fixes land (§3.1/§3.2 adversarials) |
| 4 predefined types/consts | §2 — the one-line fix + rejected alternative, the manifest diff, both new gate scripts named with contracts |
| 5 adversarial/breaking tests | §3.1 revert-drill + hook-bypass + pin teeth; §3.2 fence red-proof + MITM teeth + closure teeth; §3.3 tamper teeth; §3.4 teeth-preservation drill |
| 6 hazard-safety as math | §4.1 — three-gate conjunction for R-1; unrepresentable-cfg for R-2; process-embedded greps for R-4 |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — honest N/A for data; the CI-cost tradeoff stated as a number to record |
| 9 Linux discipline | §4.4 — six verdicts, incl. the one genuine EXTENDS (enforcement topology) with its justification |
| 10 benchmarks+telemetry | §6 — flatness proofs + the R-3 parity bench with the performance-priority rule applied |
| 11 isolation/bulkhead | §4.3 — per-item blast radii; P34 seam files provably untouched |
| 12 mesh awareness | §4.3 — trust-posture vs wire-format distinction; mixed-build interop reality + operator-cert follow-up named |
| 13 rollback/self-heal vocabulary | §4.3 — Self-Termination claimed with mechanisms; Self-Healing explicitly NOT claimed |
| 14 error-propagation gates | the phase IS this item: §3.1(c) three layers, §3.2(c) fence, §3.3-3 grep lint, §3.4 embedded greps |
| 15 living memory | §4.3 — exemption + the ledger (R-0) as the class's memory instrument |
| 16 tensor/spectral + eqc reuse | R-3 is the item: it PROTECTS the single spectral core and keeps the eqc proof suite green (§0 row 10 constraint); no new math introduced |
| 17 regression ledger | R-0 creates the bebop ledger; §5 seeds three rows + hosts the P34B/P35 handoff rows |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | check-wasm32.sh reused (§3.1c-2), fence idiom reused (§3.2c), eig-parity template reused (§3.3-1), claim-lint dedup considered (§3.4-1); DECART rejection recorded (§2) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

ALL edits in `/root/bebop-repo` (push remote `openbebop` → `github.com/SyniakSviatoslav/
OpenBebop`; the `origin` remote is ARCHIVED read-only — never push there). Dowiz repo is
read-only context for this phase. Execute R-0/R-1/R-2 first (incidents), then R-3/R-4/R-5 in
any order. R-1b's protection change and R-5's option choice are OPERATOR acts — prepare,
present, wait.

1. **T0 (R-0).** Create `docs/regressions/REGRESSION-LEDGER.md` mirroring the dowiz file's
   format. Seed with §5's three P36 rows (marked pending until their items land) + placeholder
   rows for P34B V-3/V-4 and P35 WR-01 (cross-referenced by those blueprints).
2. **T1 (R-1a).** Reproduce RED first:
   `cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` must
   fail E0425 at `at_rest.rs:74` (if green, someone fixed it — re-verify §0 before
   proceeding). Add `use alloc::string::String;` to `bebop2/core/src/at_rest.rs` (top-of-file
   imports; NO cfg-gating of the enum — §2's rejected alternative). Acceptance:
   `bash bebop2/core/scripts/check-wasm32.sh` green.
3. **T2 (R-1b).** Add the `no_std_surface_pins` test module (§3.1c-1) — run the pin-teeth
   drill (remove the import, confirm the new test fails, restore; paste output in the PR).
   Create tracked `.githooks/pre-push` (§2) + a CONTRIBUTING note on
   `git config core.hooksPath .githooks`. PREPARE the branch-protection change (exact
   `gh api` invocation with `rust-hardened` + `sovereign-guards` contexts) and present it to
   the operator — do NOT execute it yourself. R-1 is done only when §5 R-1b's API falsifier
   passes.
4. **T3 (R-1c).** Append the dated correction note to
   `docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` (append-only; cite `d23e7aa`,
   the restore commit, and this blueprint).
5. **T4 (R-2).** FIRST run the fence red-proof: write `scripts/ci-no-insecure-default.sh`,
   run it against the CURRENT manifest, confirm exit 1, record. THEN flip
   `bebop2/proto-wire/Cargo.toml` `default = []`, add the opt-in CI lane (§3.2b YAML), wire
   the fence into `sovereign-guards`. Run the closure-teeth check
   (`cargo tree -p bebop-mesh-node -e features | grep -i insecure` → empty). Add §3.2(d)'s
   operator-cert follow-up as a one-line note in the manifest comment. Mind the §4.5 collision:
   if P34B U3 already removed `iroh = []`, rebase — do not reintroduce it.
6. **T5 (R-3).** Parity pin FIRST (§3.3-1, eig-parity template; STOP on failure — file a
   finding). Then consolidate per §3.3-2, keeping `cargo test -p bebop-core --test
   eqc_proofs_test` + the 4 eqc examples green throughout. Run the tamper-teeth once, record.
   Acceptance: §5 R-3 falsifiers.
7. **T6 (R-4).** Handle the two live wires first (§3.4-1/2: port the doc-claims gate with the
   teeth-preservation drill; resolve build-unikernel.sh's compose reference). Then delete the
   remainder, each commit embedding its zero-wire grep output. Acceptance: the §3.4 `ls`
   falsifier.
8. **T7 (R-5 — 🔴 OPERATOR-DECIDED).** Present §3.5's A/B framing with costs; on Option A,
   implement behind the crypto review gate (independent review recorded, C3 test-keygen gates
   untouched); on Option B, write the dated note into the EXCELLENCE amendment. Either way,
   close the ledger row.
9. **T8 (close-out).** Update R-0's pending rows to live; run every §5 command once more in
   sequence and paste the transcript into the closing PR. Push to `openbebop`; fetch before
   every push, never force.

**Stop-and-flag conditions (do not improvise past these):** (i) T1's RED not reproducing
(stale ground truth — re-verify §0); (ii) any second production diff in `bebop2/core/src/`
beyond the one import + the pin module (scope alarm); (iii) executing the branch-protection
change or R-5 Option A without the operator (human-gate rule); (iv) the R-3 parity pin
failing (finding, not cleanup — do not "fix" either implementation to force agreement);
(v) deleting any file whose zero-wire grep was not run THIS session; (vi) any impulse to
touch delivery-domain/proto-cap (P34's seam) or to re-report C4b / reopen P4 / add
batch-verify work (settled).
