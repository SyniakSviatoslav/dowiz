# Research / Grounding Pass — Kleene Three-Valued Logic, "Truthfulness" Terminology, Open-Source Hardening (Miri / Kani / Sentinel / Shadow-Mode / Contribution-Gates)

**Date:** 2026-07-19 · **Role:** grounding/investigation ONLY (no synthesis, no blueprint, no
recommendations) · **Consumer:** a later Fable synthesis pass · **Sources:**
`RAW-PROMPT-6-truthfulness-logical-deduction-kleene-unknown-2026-07-19.md` +
`RAW-PROMPT-7-opensource-hardening-miri-kani-sentinel-shadow-mode-2026-07-19.md` (treated as one
combined dialogue), `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` (items 33–44),
`CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md` (items 45–49),
`SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md`.

**Epistemic tag on every finding:** **GROUNDED** (cited against live source / verified in-tree) or
**NOT FOUND / DOES NOT EXIST** (exhaustive search returned nothing). No claim below is asserted
from memory.

**One load-bearing repository fact established up front (governs several answers):** much of the
machinery the raw prompts reason about — the FDR module (`kernel/src/fdr/`), the `Reading<T>` /
`Absence` types, `ct_gate.rs` (dudect), `scripts/hardening-gate.sh`, `HOT-PATHS.tsv`,
`ZERO-DEP-ALLOWLIST.txt`, the item-6 hardening-gate CI job — **does NOT exist on `main`** (main HEAD
= `bfd633c50` at investigation time; `git ls-files kernel/src/fdr*` → empty). It lives on the
**unmerged** branch `exec/space-grade-tier0-2026-07-19` (worktree `/root/dowiz-wt-space-grade-exec`,
HEAD `ae2da4a9d`). Where a finding rests on that branch rather than `main`, it is stated explicitly.

---

## CRITICAL FIRST QUESTION — the "Truthfulness" terminology collision

**Answer: primarily #2 (two genuinely DIFFERENT, orthogonal, but compatible properties an output
can hold independently), and *therefore* also #3 (a real terminological CONFLICT — the same word
now names two different formal criteria in one roadmap, so one use needs disambiguation going
forward). It is definitively NOT #1 (they are not the same property from two angles; byte-
reproducibility is not the mechanism that makes logical-validity checkable).**

### The decisive evidence: one definition is content-FREE, the other is content-BASED

**Swarm-safety "Truthfulness" (byte-reproducibility) is explicitly content-free.**
`SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` §1 (lines 33–41):

> "truthfulness, defined operationally: *given byte-for-byte identical input and conditions, the
> model must produce byte-for-byte identical output, checked at different points in time. Any
> divergence is a signal.* This is cache semantics… **no interpretation of the value's *content* is
> needed to know it.** The check is **content-free**, deterministic, and adversary-hostile…"

**RAW-PROMPT-6 "Truthfulness" (logical-deductive validity) is explicitly content-based.**
RAW-PROMPT-6 line 56:

> "Truthfulness (Правдивість як логічна властивість): … Якщо твоя система приймає аксіому A і має
> правило виводу A -> B, то B — це істина. Якщо модель видає C, це… **порушення логічного
> інваріанту**."

and lines 74–75 (the mechanism):

> "**Proof-Checkable Output:** Якщо модель видає відповідь, вона має надати… 'шлях виводу'
> (reasoning path). Твій кернел **перевіряє цей шлях на відповідність правилам логіки**."

A check that requires **no interpretation of content** and a check that is **an inspection of the
reasoning path's content against logic rules** cannot be the same property. This alone rules out #1.

### Orthogonality shown by counterexample (why #2, not #1)

- **Byte-reproducible but logically invalid:** a deterministic function can reliably emit the same
  unsupported/wrong answer on every run. Determinism ≠ derivational validity. The swarm-safety
  check *cannot even detect this* — it is, by its own text, "content-free," so a stable-but-invalid
  output passes it. (Swarm §1 makes no correctness claim about the content; it only claims a
  *cache-corruption* detector.)
- **Logically valid but not byte-reproducible:** the same valid conclusion reached via two different
  valid derivations, or via batch-nondeterministic logits — exactly the ~92%-divergence world
  swarm-safety §2 documents as the default on ordinary serving infra ("**~92% of identical-input
  runs already diverge with zero poisoning or hallucination present**"). Each run is logically
  valid; the bytes differ.

### Why #1's specific claim is false

#1 proposes "byte-reproducibility as the MECHANISM that makes logical-validity checkable/auditable."
It is not: validity is checked by a proof-checker over the reasoning path (RAW-6 "Proof-Checkable
Output"), which neither needs nor is enabled by the model's output being byte-identical across runs.
You can check one output's validity without re-running it; and perfect byte-reproducibility of an
invalid output does nothing toward making it valid. There **is** a weaker, genuine relationship
worth flagging (but it is NOT #1's claim): the determinism of *the kernel's checker/Gatekeeper* — a
different object than *the model's output* — is what makes the pass/fail *verdict* itself
reproducible and auditable. Both raw prompts want a deterministic Gatekeeper; that shared substrate
is real, but it is the checker being deterministic, not the model output being byte-reproducible.

### Consequently #3 (the naming conflict is real, not merely cosmetic)

The two uses originate in different tracks — swarm-safety (`SWARM-SAFETY-SYNTHESIS-2`) vs the
space-grade kernel track (RAW-PROMPT-6/7). But the roadmap now spans both, and both define
"Faithfulness → Truthfulness" as a headline replacement of an inferior alignment concept, so a
reader meeting "Truthfulness" in the roadmap cannot tell which formal criterion is meant without the
originating document. That is a genuine collision requiring one term to be renamed/qualified. (There
is already an in-repo precedent for handling exactly this: `CRASH-CONSISTENCY-…-SYNTHESIS` §4
maintains a "Terminology Collisions and Non-Mapping Analogies" table — seL4-capability,
OTP-supervisor, TMR, `no_std` — precisely so future docs cite the disambiguation instead of
re-deriving it.) **GROUNDED — resolution of *which* term renames is a synthesis decision, deliberately
left to Fable; the grounding conclusion is only that #2+#3 hold and #1 is refuted.**

---

## 1. Existing 3-valued / optional-uncertainty (Kleene) logic patterns

**NOT FOUND** — no genuine three-valued Kleene (True/False/Unknown) logic type exists anywhere
(`main`, the `exec/space-grade-tier0` branch, or `engine/`). Searched for `TruthState`, `Kleene`,
`Ternary`/`Trilean`/`ThreeVal`, `#[repr(u8)]` truth enums, and any enum literally carrying `True` +
`False` + `Unknown` together — zero hits.

What exists and was checked, none of it Kleene:
- **`kernel/src/budget.rs:44-51` `enum JobStatus { Pending, Running, Done, Failed, Unknown }`** — a
  5-variant job-poll status, not a truth value. `Unknown` = "polling a handle the offline port could
  never have issued" (line 49). Not epistemic-uncertainty-over-a-proposition. **GROUNDED (not Kleene).**
- **FDR `Reading<T>` (exec branch, `kernel/src/fdr/schema.rs:62-65`): `enum Reading<T> { Value(T),
  Unavailable(Absence) }`** — a **two-state** named-absence type (`Option<T>` with a typed, reason-
  bearing `None`; `Absence` at `schema.rs:25` = `{ NonLinuxHost, NoRaplInterface, PermissionDenied,
  ReadError, SamplingDisabled, … }`). The task's own framing is confirmed: this is two-state
  named-absence, **not** a three-valued truth logic. **GROUNDED (two-state, not three-valued).**
- Numerous `LlmError::Unavailable` / `ToolError::Unavailable` / `LoopOutcome::AssistantUnavailable`
  variants (`ports/llm.rs:172`, `ports/tool.rs:113`, `agent/loop.rs:79`) — binary error variants, not
  tri-state.

No `Some/None/Error` tri-state used as an epistemic uncertainty carrier, no custom trilean. **The
Kleene `TruthState { False, True, Unknown }` of RAW-PROMPT-6 is greenfield in this codebase.**

## 2. proptest / quickcheck — precise inventory

**GROUNDED.** `proptest` is a **dev-dependency only** and its use is narrow.

- **Dependency status:** `kernel/Cargo.toml` under `[dev-dependencies]`: `proptest = "1.11"`, with an
  in-file comment: *"Property-test harness for the P47 reconciliation invariant (B3)… Cached offline
  (1.11.0), **dev-only, never linked into a production build path.**"* (alongside `criterion = "0.5"`
  and `paste = "1.0"`, also dev-only). `quickcheck` — zero hits anywhere.
- **Scope / properties:** three files use it (one more than the prior pass's "two payment files"):
  - `kernel/src/ports/payment.rs:657-658` — `proptest! { #![proptest_config(ProptestConfig::with_cases(400))] … }`; the property is the **settlement-reconciliation invariant** (`payment.rs:703`:
    `folded == derived`; `:709`: `a == o.placed_total`) — arbitrary order sequences folded vs.
    settlement totals, exact-i64 equality.
  - `kernel/src/ports/payment_provider.rs:1122-1123` — same `with_cases(400)` config.
  - `kernel/tests/firewall_p47.rs` — imports proptest (P47 firewall integration test).
  So: **~400 cases/run in each of the two payment modules**, matching the CRASH-CONSISTENCY synthesis
  §1.4 characterization ("two payment files, 400 cases each"), plus one integration-test usage.
- **Zero-dep-gate compatibility:** the exec-branch `ZERO-DEP-ALLOWLIST.txt` states the default no-dev
  tree is "the `dowiz-kernel` root ONLY — ZERO external crates," and the gate is `cargo tree -e
  no-dev`-scoped (per the DETERMINISTIC-AI synthesis §1.4 and roadmap item 44's proof: "`cargo tree
  -e no-dev` still resolves to the kernel root alone"). Because `proptest` is a **dev**-dependency,
  `-e no-dev` **excludes** it — it does **not** count against the zero-external-crate claim.
  **GROUNDED conclusion:** adding NEW proptest usage is compatible with the zero-dep posture **iff it
  stays strictly in `[dev-dependencies]` / `#[cfg(test)]`**; any move of proptest into a non-dev build
  path would appear in the `-e no-dev` tree and break the gate.

## 3. Miri — CI / documentation presence + unsafe inventory

**DOES NOT EXIST as an enforced check.** Miri is run nowhere.

- **CI:** grep of all four workflows (`.github/workflows/ci.yml`, `heartbeat-monitor.yml`,
  `safety-floor.yml`, `skill-security.yml`) → **zero** `miri` occurrences. No `Makefile` exists; no
  script under `scripts/` invokes miri.
- **Documentation:** `miri` appears **only in design/blueprint prose as an aspirational/planned
  check**, never as a wired gate — e.g. `kernel/src/arena.rs:32` (doc-comment: a future done-check
  "runs this module's tests under Miri"), `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md`
  (W5 done-check), `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md:713` ("only where
  `unsafe` exists"), and `ROADMAP-LIVE-STATUS-2026-07-18.md:24` which states outright: **"Miri gate
  not run (component absent this toolchain)."**
- **`unsafe` inventory (natural first Miri targets), `kernel/src/`:** 21 total `unsafe` tokens.
  Density ranking: `simd.rs` (6), `arena.rs` (6), `messenger.rs` (3), `householder.rs` (3),
  `slot_arena.rs` (1), `chaos.rs` (1), `bounded_drainer.rs` (1). **Correction to the raw prompt's
  guess:** it predicted "the PQ crypto modules" as high-`unsafe` density — **`kernel/src/pq/` has
  ZERO `unsafe` blocks** (safe Rust throughout). The real highest-density sites are the SIMD/arena
  surfaces (`simd.rs`, `arena.rs`, `householder.rs`, `messenger.rs`). **GROUNDED.**

## 4. Kani — cross-reference with item 6 / item 7

**DOES NOT EXIST (entirely unbuilt); item 6 did not touch it.**

- **Kani usage:** no `.kani` files, no `kani` in any workflow, script, or kernel source. The **only**
  `kani` token in the tree is `cargo:rustc-check-cfg=cfg(kani)` inside the **`zerocopy` transitive
  dependency's** build output (`kernel/target/.../zerocopy-*/output`) — a dependency declaring the
  `kani` cfg name, **not** actual Kani verification. CRASH-CONSISTENCY synthesis §1.4 corroborates:
  "Kani and TLA+ are planned-only, zero usage."
- **What item 6 actually shipped (commit `ae4964e61`, exec branch — verified via `git show --stat`):**
  `feat(ci): hardening checklist CI gate + HOT-PATHS manifest + minimal dudect harness (roadmap item
  6)`. Files: `.github/workflows/ci.yml` (+40, the `hardening-gate` job), `docs/audits/hardening/
  CHECKLIST.md` (+69), `docs/audits/hardening/HOT-PATHS.tsv` (+41 rows), `kernel/src/ct_gate.rs` (+277,
  a zero-dep **dudect** Welch-t harness + `ct_eq` + a **planted-leak self-test**),
  `kernel/src/householder.rs` (+15, `debug_assert` Vieta trace/det differential),
  `kernel/src/order_machine.rs` (+13, `FSM_ADJ` dual-representation `debug_assert`), `kernel/Cargo.toml`
  (+8), `scripts/hardening-gate.sh` (+136). **No Kani anywhere in the commit.**
- **Item 7's open scope (unbuilt):** Kani wiring over Keccak, FSM graph algorithms, NTT arithmetic,
  GCRA transition (roadmap line 221; and the RAW-PROMPT-7 ask to prove `TruthState`/Guardian can't
  reach an unhandled state). **GROUNDED: Kani is 100% item-7 scope, still entirely open.**

## 5. Integrity-hash / "Sentinel" pattern precedent

**NOT FOUND for the exact pattern** (an `integrity_hash` field on a **live in-memory struct**,
recomputed and compared **on every read**, forcing a Safe State / `Unknown` on mismatch). No
`integrity_hash`, no `canary`, no `magic_number`/`MAGIC` struct-corruption sentinel; no
`#[derive(Hash)]`-and-compare-on-read of a live struct (the `#[derive(…Hash)]` sites — `order_machine.rs:7`,
`ports/tool.rs`, `slot_arena.rs:72`, `csr.rs:777`, etc. — derive `Hash` for map-key use, not integrity
self-checking).

Close-but-different precedents (all confirming the task's framing that the exact pattern is absent):
- **`event_log.rs` hash-chaining + `verify_chain` (F2):** `event_log.rs:212` — the chain lets
  `EventLog::verify_chain` "detect corruption at rest"; `:466`/`:536` note the torn-write /
  corruption-between-hash-and-persist window. This is **append-time chaining + a chain-walk
  verification at rest**, NOT per-read verification of a live struct's own integrity field. (Matches
  the task's stated distinction.)
- **`backup.rs` content-addressed store:** `:122` ("corruption yields fail-closed `None`, never
  unverified bytes"), `:252` ("stored bytes and compare; a mismatch (corruption/bit-rot)"), `:689`
  ("a 1-bit on-disk corruption makes `get_owned` [fail]"). This is **on-disk/at-rest content-address
  verification on GET-from-store**, the closest existing analog — but it guards a disk-backed record,
  not a live in-memory struct read hot-loop.
- **FDR ring CRC32 (exec branch):** one CRC32 per on-disk line — on-disk record integrity (as the task
  notes), not in-memory struct integrity.
- **`spectral_cache.rs:125/488` content-addressing** (NaN→`CANONICAL_QUIET_NAN`, deterministic
  "corruption id") and **`chaos.rs`** (models "corruption between hash and persist" for *testing*) —
  content-identity / test scaffolding, not a read-time struct sentinel.

**GROUNDED: the in-memory read-time struct `integrity_hash`/Sentinel is genuinely absent; the nearest
real mechanism is content-addressed on-disk verification (`backup.rs`) + the `event_log` chain-walk.**

## 6. Differential / "shadow mode" testing precedent

**Genuinely NEW pattern for this codebase.** Every differential/disagreement mechanism found treats
**disagreement as a test failure or a hard rejection**, never as a logged-but-non-failing field signal:

- **`kernel/src/decision/import.rs`** — `import_unit` replays an imported unit against a local oracle;
  ANY disagreement ⇒ **reject** (`ImportReject::ReplayDisagreement`, `:53-54`, `:124`); it *does* emit
  `telemetry(false, Some("ReplayDisagreement"))` (`:125`) but the disagreement still **fails/rejects**,
  it is not a shadow log-and-continue.
- **`kernel/src/pq/dsa.rs:1086`** G10 differential probe vs the pq-crystals ML-DSA reference — a **test**.
- **`kernel/src/spool.rs:287`** (P77 B1 differential), **`kernel/src/retrieval/spine.rs:673`** (P77 B2
  differential/adversarial), **`kernel/src/stats.rs:305`** (inverted primitive "must disagree"),
  **`kernel/src/spectral_laplacian.rs:256`** (mode eigenvalue must not disagree) — all **oracle tests
  where disagreement ⇒ failure.**

The only "advisory, does NOT gate" precedents are narrower and different in kind: `kernel/src/metrics.rs:85`
& `:382` — a claim-latency anomaly flag that is *"advisory only — does NOT gate a merge"* (a single
**deterministic** anomaly flag in the CI/merge plane, not a probabilistic-vs-deterministic runtime
shadow comparison), and `kernel/src/leak_gate.rs` — an advisory cosine companion that **downgrades to
exact-only on backend error** (fail-closed advisory, not a disagreement logger). The
`SWARM-SAFETY-SYNTHESIS-2` §1 "replay probes / divergence is a signal" concept is **design-only,
unbuilt.**

**GROUNDED: running a probabilistic component alongside a deterministic one and LOGGING disagreement
as a field signal WITHOUT failing the build has no existing implementation here. Closest kin =
`metrics.rs` advisory-flag (deterministic, merge-plane) + `decision/import` telemetry-on-disagreement
(but that rejects).**

## 7. CI contribution-gate posture (PR enforcement)

**GROUNDED.** `.github/workflows/ci.yml` runs on `pull_request: branches:[main]` (line 10-11). The
PR-enforced jobs are:

`telemetry-selftest`, `eqc-proofs` (`cargo test --release` in `tools/eqc-rs`), `claim-latency-ledger`,
`v5c-reexec` (independent re-execution of the diff range; runs the full re-exec only when a red-line
path — `money.rs`/`order_machine.rs`/`event_log.rs`/auth — is touched), **`cargo-test`** (kernel +
engine, offline, unconditional — the P01 CI-Truth Floor), `bench-regression` (kernel hot paths,
baseline-gated, `--threshold 10`), `gitleaks`, **`dco-check`** (requires a `Signed-off-by:` trailer on
**every** commit in `origin/main..HEAD` — a real contribution gate, `:210-226`), `supply-chain`
(`cargo audit` + `cargo deny` + zero-OCI), **`decart-dep-lint`** (a new dependency without a DECART
report ⇒ RED), `no-courier-scoring`, `no-pub-raw-matrix-hash`, `fence-check`, `regression-digest`,
`firewall-agent-loop`, `mesh-adapter`.

Against RAW-PROMPT-7's specific asks:
- **`cargo clippy --deny warnings`: NOT FOUND** (zero `clippy` occurrences in any workflow).
- **`cargo fmt --check`: NOT FOUND** (zero `fmt`/`rustfmt` occurrences).
- **`cargo miri test`: NOT FOUND** (see §3).
- **"PR must include a test": NOT FOUND as a direct gate.** `cargo-test` re-executes the suite but does
  not require a *new* test per PR. No PR template exists (`.github/` contains only `workflows/`).
- **"PR must justify non-determinism": NOT FOUND directly**; the adjacent-but-narrower controls are
  `decart-dep-lint` (justify new **deps**) and `v5c-reexec` (re-execute red-line diffs).
- **Item 6's hardening-gate as a partial "Formal Proof of PR":** on the **exec branch** (not merged to
  `main`'s ci.yml), item 6 added a `hardening-gate` job that imposes a **diff-triggered obligation** —
  a PR touching a registered hot ZONE without a `HOT-PATHS.tsv` manifest row is RED, and each row's
  named test filter is **re-executed** with an asserted `N passed >= min_tests` (a filter matching
  **zero** tests is RED — the anti-forgery core). This **partially** prefigures the raw prompt's
  "critical kernel regions carry a proof" idea for hot paths, but (a) it is **unmerged** and (b) it does
  **not** cover the clippy/fmt/miri triad. **GROUNDED.**

## 8. Open-source readiness cross-reference

**GROUNDED — a real, tracked plan exists, but the public flip is operator-gated and NOT authorized;
the raw prompt's "imminent" framing overstates urgency.**

- **The plan:** `docs/adr/0020-oss-license-tm-dco.md` (ADR-0020, Status **Accepted**; license text
  landed `ac1caba40`, 2026-07-14). `LICENSE` at repo root = full AGPLv3; `TRADEMARK.md`, `CONTRIBUTING.md`,
  `DCO`, `NOTICE`, `MANIFESTO.md` all in tree. Decision = **AGPLv3 + trademark leash + DCO 1.1**.
- **Gating status (from the ADR's own Consequences):** two operator gates are **deliberately NOT
  resolved and explicitly not authorized**:
  - **EUTM brand + filing** — an operator action with real cost, "to be decided before any filing
    occurs, not implied by this ADR."
  - **Public-flip go** — "remains fully operator-gated, one-way, and is **not authorized, triggered,
    or implied** by anything in this ADR."
  - Plus a recommended (not gating) **all-origin-refs `gitleaks` sweep** before any public-flip discussion.
- **One nuance the ADR predates:** ADR-0020 point 3 flagged the DCO CI enforcement as an OPEN gap
  ("no `dco-check` job exists… re-verified 2026-07-16, zero grep hits"). That gap has since **closed** —
  a `dco-check` job **is** present in `main`'s `ci.yml:210-226`. So of the ADR's three open
  implementation gates, the DCO-CI one has landed; the two **operator** gates (EUTM, public-flip)
  remain open and unauthorized.
- **Memory cross-ref:** matches `open-source-goal-adr020-2026-07-03` (AGPLv3+TM+DCO, gated on
  secrets/EUTM) and the secrets-exposure incident record.

**GROUNDED conclusion:** open-sourcing is real and licensed but the go-live is operator-gated and
un-triggered — the contribution-gate hardening the raw prompt frames as urgent is **not** blocking any
imminent public flip.

---

## Repository-state caveats (so the synthesis pass doesn't mis-cite)

1. **`main` vs `exec/space-grade-tier0-2026-07-19`:** the FDR module + `Reading<T>`/`Absence` + `ct_gate.rs`
   + `hardening-gate.sh` + `HOT-PATHS.tsv` + `ZERO-DEP-ALLOWLIST.txt` + item-6 CI job are on the **unmerged
   exec branch**, not `main`. The zero-dep "empty allowlist" claim and item-6 "DONE" both refer to that
   branch (only docs from item 6 were pushed to `origin/main`).
2. **Concurrent writers active today** (git worktree list shows ~40 live worktrees). All findings above are
   read-only snapshots at investigation time (main HEAD `bfd633c50`, exec HEAD `ae2da4a9d`).
3. **This document is grounding only** — no roadmap, no recommendations, no adoption decisions for
   Miri/Kani/Sentinel/shadow-mode/contribution-gates. Those are the Fable pass's scope.
