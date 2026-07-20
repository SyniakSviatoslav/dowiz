# BLUEPRINT — Items 73–78: Governed Self-Evolution Arc (AI-Proposed Change Governance under an untouchable human gate)

- **Date:** 2026-07-19 · **Roadmap:** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §L (lines
  1257–1435) · **Tier:** governance/self-evolution (post-§K) · **Status:** BLUEPRINT (planning
  artifact, **zero code**).
- **Author:** System Architect (planning). **Shape:** one combined doc for the six tightly-coupled,
  strictly-sequential items (73←74←75←{76,77,78}) — the `BLUEPRINT-ITEMS-01-13` /
  `BLUEPRINT-ITEMS-15-16-17-19` precedent for a cluster that shares one governing invariant.
- **Sources read this session (all citations live-verified against the worktree at `main`, merge
  `6701bbb6f` = `origin/exec/space-grade-tier0-2026-07-19` merged):** roadmap §L; `DECISIONS.md`
  D0; `kernel/src/ports/agent/scope.rs`; `kernel/src/money.rs`; `kernel/src/order_machine.rs`;
  `kernel/src/decision/import.rs`; `kernel/src/event_log.rs`; `kernel/src/fdr/*`;
  `kernel/src/capability_cert.rs`; `kernel/src/spectral_cache.rs`; `kernel/src/markov.rs`;
  `tools/eqc-rs/src/lib.rs`; `docs/audits/hardening/{CHECKLIST.md,HOT-PATHS.tsv}`;
  `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md`; `BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md`.

---

## 0. Read this first — what this arc is, and the one gate it all sits behind

### 0.1 Two governance planes — do not conflate them (load-bearing)

There are two completely separate "governance gate" concepts in this repo, and this arc is about the
**second, not the first**:

1. **Dev-session governance (SUSPENDED).** `.claude/CLAUDE.md` records the operator's 2026-07-15
   suspension of mandatory-proof / ship-discipline / self-improvement gates *for this agent's own
   self-management of the repo*. That suspension governs **how blueprints/code get written and
   landed during development**. It is orthogonal to §L.
2. **Product governance (THIS ARC — being *built*, never suspended).** §L is a **PRODUCT capability
   of the shipped dowiz kernel/OS**: the machinery by which the *product's own AI* may PROPOSE code
   changes to the running OS, gated by a human "apply" token. The roadmap says this verbatim (§L,
   line 1266): *"This is a PRODUCT capability of the dowiz kernel/OS itself, not a statement about
   this development session."*

Confusing the two would be catastrophic: the dev-session suspension does **not** license building an
autonomous-apply path into the product. This arc's entire reason to exist is the opposite — to make
the product's AI-proposal channel **structurally incapable** of applying anything without a human
token, and to make *that property itself* unmodifiable by the pipeline it governs.

### 0.2 D0 grounding — the arc is subordinate to the six invariants and the human-gate rule

`DECISIONS.md` D0 (lines 6–8): **decentralized · local-first · post-quantum · crypto · mesh ·
reliability-over-latency** — the lens for every change; a change that breaks any is rejected. §L
touches D0 in three ways that must stay honest:

- **`local-first` + `mesh` + `decentralized`:** "the human operator" is not a global singleton. On a
  decentralized mesh each sovereign node has *its own* operator. Whose token gates a given node's OS
  change is an **open governance question** (§9, Q1) — it ties directly to the recorded D10
  `RootDelegationPolicy = OperatorSigned` ruling and must not be silently resolved to "one global
  admin."
- **`post-quantum` + `crypto`:** if the human "apply" token is a cryptographic artifact (the likely
  shape — see item 75), it must be PQ (ML-DSA-65, per D4), never a classical signature. Whether the
  token is a signed commit, a `capability_cert`, or an out-of-band ceremony is an **operator ruling**
  (§9, Q2), but "if crypto, then PQ" is not negotiable.
- **`reliability-over-latency`:** the pending-approval state is *permanent until a human acts*
  (§L line 1353: "silence is never consent"). Expiry-to-safe-pole is a reliability property, not a
  convenience; the TTL is a named operator constant (§9, Q3).

The standing memory rule **"Never bypass human-gated decisions — blanket permission ≠ per-change
approval"** is the behavioral twin of item 73's structural invariant. §L makes that rule a property
of the type system rather than a discipline someone follows.

### 0.3 The one governing invariant (everything below is subordinate to item 73)

The human-approval gate, the red-line registry, and the verification machinery are **themselves
CORE** — permanently outside the reach of the pipeline they govern. No sequence of individually
plausible approved proposals may ever reach them. This is the **corrigibility / tamper-proof-oversight
problem** from the AI-safety literature, and §L addresses it **structurally, not by policy**.

### 0.4 THE central, testable acceptance property (stated once, binding on 73/74/75/76)

> **Unconstructible-without-human-token.** There exists **no code path** — in-tree, AI-generated, or
> composed from individually-approved changes — that applies a `ChangeProposal`, mutates the
> red-line registry, or edits the gate/verification code, without a human-minted approval token; and
> **attempting to write one is a compile error**, not a runtime rejection.

Every item that touches the proposal-apply seam (73, 74, 75, 76) carries, in addition to the
`CHECKLIST.md` 5-point standard, an explicit **"prove the human-gate cannot be bypassed" falsifier**
(the `§X.4-bypass` blocks below). This is the acceptance criterion of the whole arc — not a footnote.

### 0.5 Standing laws (inherited, non-negotiable for every §L item)

- **Zero new external crates** (`cargo tree -e no-dev` byte-unchanged; the `kernel/ZERO-DEP-ALLOWLIST.txt`
  gate, items 1+13). Compile-fail proofs may use a **dev-only** harness (see §2.4) — dev-deps are
  outside `-e no-dev`, but prefer the native form.
- **P7 re-execution, never presence-check** (`CHECKLIST.md` §"§10/P7"): every verdict is a live
  process exit + parsed live counts; a filter matching zero tests is RED.
- **P3 rate/authority discipline:** any threshold/TTL/cadence is a **named constant with one
  authority site**; no lineage/cost/health value ever feeds a hash/gate/replay/decision surface.
- **item-25 dependency-replacement procedure** for any dependency question.
- **Planning only — no §L item starts before the operator dispatches it** (§L, line 1272). §9 flags
  which questions must be operator-ruled *before* code, not during.

---

## 1. Verified current-state grounding (shared across all six items)

### 1.1 What already exists on `main` to EXTEND, never fork

| Existing mechanism | File:line (verified) | What §L reuses it for |
|---|---|---|
| `RedLinePolicy::DenyByDefault` + `AllowList` | `kernel/src/ports/agent/scope.rs:257–283` | item 74's classifier posture; item 73's deny-by-default meta-level |
| `Resource::is_red_line` / `Action::is_red_line` / `Scope::touches_red_line` | `scope.rs:96–101, 176–184, 247–251` | the *runtime-advice-plane* red-line set; item 74 is its **code-plane** twin |
| Closed-set **fail-closed** decode (`from_discriminant` → `None` on unknown byte) | `scope.rs:77–92, 157–173` | item 75's `ChangeProposal` decode must fail-closed identically |
| `capability_cert.rs` `Capability` / `root_scope` / attenuation (narrow-only) | `kernel/src/capability_cert.rs:205–231` | item 75's approval-token shape (item 65 reuse), no new crypto |
| `import_unit` **replay-before-persist** gate; `ReplayDisagreement` | `kernel/src/decision/import.rs:81, 54, 119–125` | item 75's "verify on the proposed state before it can be applied"; named precedent (§L line 1320) |
| `FSM_GOLDEN_SIGNATURE` + drift gate; `FSM_SPECTRAL_RADIUS` const-proof | `kernel/src/order_machine.rs:513, 549–563, 383` | item 74 registry row (2): the exemplar proof surface AI may never touch |
| `money.rs` integer-only checked arithmetic (`i64`/`i128`, no NaN) | `kernel/src/money.rs:8, 17, 22` | item 74 registry row (1): product red-line |
| `event_log.rs` SHA3 hash chain (`sha3_256`, `prev` content-id) | `kernel/src/event_log.rs:109, 135–160` | item 74 row (4): forensic truth surface (item 76 depends on it) |
| `fdr/` ring + schema + CRC32 (`ring.rs`, `schema.rs`, `json.rs`, `pmu.rs`) | `kernel/src/fdr/*` | items 75/76/77/78 FDR lineage records |
| `HOT-PATHS.tsv` `@ZONE` path-prefix → deterministic diff classification; `min_tests=0` placeholder idiom | `docs/audits/hardening/HOT-PATHS.tsv:5–25, 18–19` | item 74's registry **is a TSV in this exact idiom** |
| `markov::Verdict{Healthy,LimitCycle,StrangeAttractor}` (observational-only) | `kernel/src/markov.rs:42–48, 98, 110–115` | item 77 trigger input |
| `RetainedBase::admit` rejects `DriftClass::Unstable` (the one live fail-closed health consumer) | `kernel/src/spectral_cache.rs:267–269` | item 77 precedent: a classifier already gating admission |
| `eqc-rs` `emit_proof_program` (equation → generated Rust + self-assertion) | `tools/eqc-rs/src/lib.rs:442` | items 75/78 "arrive as a PROVEN artifact" |
| **Type-system red-lines already in use:** routing enums omit `Ord`/`PartialOrd` (a "quality router" is unrepresentable); `no-courier-scoring` CI job; `eqc_gen.rs` "GENERATED — do not hand-edit" parity pin | `kernel/src/decision/mod.rs`, `kernel/src/domain.rs`, root `CLAUDE.md` | item 73's method: **extend this same "make it a compile error" pattern to the meta level** |

**The house standard (§1.5 "illegal-state-unrepresentable"):** the repo already proves *negatives* by
type construction — `ValidatedProposal` constructible only via `admit` (item 47, roadmap line 689);
item 65's capability token constructible only by the composition root (line 1124); item 9's
`Result<Permit, Tripped>` with no "tripped-but-permitting" state (line 368). Item 73 is that same
pattern **applied to the governance mechanism itself**. §L invents no new safety primitive.

### 1.2 What does NOT yet exist (the real dependency gates — verified absent)

| Missing prerequisite | Verification | Blocks |
|---|---|---|
| **Composition root / capability minter** (items 64/65) | `grep -r 'CompositionRoot\|CoreWriteCapability' kernel/src` → **0 hits** | item 73's *structural* half (root placement, sole minter); item 75's approval token |
| **`admit`/`ValidatedProposal`/`RejectionClass`** (items 47/50) | roadmap: spec-after-35, wiring-after-42; not in tree | item 75's admission grammar |
| **FDR `span_id`/`parent_span_id`** (item 62) | `grep 'span_id\|parent_span_id\|parent\|trace' kernel/src/fdr/schema.rs` → **0 hits** (schema is flat) | item 76's lineage tree |
| **Cost-oracle classification** (items 67/68) | not landed (§K spec) | item 76's impact-class-at-approval-seam; item 78's cost delta |
| **`markov::Basis` epistemic-basis field** (item 56) | `markov.rs` has `Verdict` + `verdict_str` but **no `Basis`** | item 77's trigger-evidence law |
| **item 9 breaker / item 54 Sentinel** | spec-level (roadmap §B item 9:368, §J item 54:867) | item 77's additional trigger inputs (not gating) |

**Consequence:** items 73 and 74 are **buildable at spec level now** and 74's registry FILE + classifier
is buildable now (HOT-PATHS idiom exists); item 73's *structural teeth* and items 75–78 are **gated**
on the above. §8 makes every gate explicit. This is honest sequencing, not a blocker to blueprinting.

---

## 2. Item 73 — the Gate-Root Invariant (FIRST and foundational; everything in §L is subordinate)

### 2.1 Scope / goal

Make the governance mechanism — the human-approval gate, the red-line registry (item 74), and the
verification code — **structurally unmodifiable by the pipeline it governs**. Three structural
clauses, **none of them "a rule someone follows"** (roadmap lines 1283–1308):

- **(i) Root placement.** The registry + gate-enforcement code live at/behind item 64's composition
  root — the sole capability minter. **No capability type granting write access to the root, the
  registry, or the gate code EXISTS in the type system** — not "is never handed out": is
  *unconstructible*. By construction nothing downstream of the root (including any AI-generated
  proposal, however it entered) can hold authority over the root itself (seL4 "init task holds all
  capabilities" completed: the root delegates everything EXCEPT authority over itself).
- **(ii) Dependency-direction check (item-45 style).** A CI check asserts no module reachable from
  the proposal pipeline references any gate/registry mutation surface — *trivially checkable because
  no such mutation surface is `pub`*: the registry is static data + code with **no runtime mutation
  API at all**.
- **(iii) Out-of-band-only modification.** Changing the gate/registry/verification logic happens
  ONLY through the normal human engineering process (a human editing code, review, the full CI
  suite) — **never** through the proposal pipeline, even "with extra approvals": a proposal whose
  diff touches a red-line path is REFUSED at **classification time (step zero)** — a category error,
  not a reviewable request. **No "beneficial change to the gate" exception exists** — that exception
  *is* the corrigibility trap.

### 2.2 Current-state grounding (file:line)

- The pattern to extend already ships: `scope.rs` `RedLinePolicy::DenyByDefault` (`:272`) rejects any
  red-line scope at runtime, and closed-set decode fails closed (`:77–92`). Item 73 lifts "deny by
  default + unconstructible bypass" **one plane up** — from *authorizing an agent's runtime action*
  to *authorizing a mutation of the gate itself*.
- Illegal-state-unrepresentable is already the house standard (§1.1). The negative-by-construction
  proof (`grep` for the constructor site + a compile-fail case) is already used by item 65's spec
  (roadmap line 1135) and item 47's `ValidatedProposal` (line 689).
- **Gate absent:** no composition root exists (`§1.2`), so clause (i)'s *structural* landing must wait
  for items 64/65. Clauses (ii) and (iii) are buildable at spec + CI-grep level against item 74's
  registry the moment 74 lands.

### 2.3 Implementation plan (numbered)

1. **Spec the meta-capability negative.** In the item-74 registry module doc (and, when items 64/65
   land, in the composition-root module), record the binding law: *no `pub` type, fn, or macro
   anywhere in the crate constructs a value conferring write authority over the registry, the gate
   code, or the composition root.* This is the "GENERATED — do not hand-edit"/`no-courier-scoring`
   discipline generalized to the meta level.
2. **Registry has NO mutation surface (clause ii's enabler).** Item 74's registry is `static`/`const`
   data + pure classifier functions. It exposes **zero** `pub fn` taking `&mut self`, zero `pub`
   `static mut`, zero interior-mutability handle. Proof is then trivial: you cannot mint a capability
   to mutate a thing that has no mutation API.
3. **Dependency-direction CI check (clause ii), item-45 shape.** A new gate leg — extend
   `scripts/hardening-gate.sh`'s diff-scope logic or add `scripts/gate-root-invariant.sh` — asserts:
   (a) no module reachable from the proposal-pipeline path-prefix references the registry/gate
   mutation-surface identifiers (there are none to reference — the check's *green* state is that the
   grep set is empty); (b) the proposal-pipeline modules do not `use` the composition-root minter.
   Reuse item 45's "core never imports AI" grep precisely, pointed at pipeline→gate.
4. **Step-zero classification refusal (clause iii).** Item 75's `admit` calls item 74's classifier
   FIRST; a red-line-touching diff returns `RejectionClass::Refuted{ red_line: <named cause> }`
   **before any verification runs and before any human is asked**. Wire this as the first statement
   of `admit`, not a late branch — a category error surfaces earliest.
5. **Record the recursion.** Item 74's registry carries §L's *own* gate code + the registry itself as
   rows (roadmap line 1329, "(8)"). The out-of-band-only law is written into `CHECKLIST.md` and the
   registry module doc. The recursion is *recorded, not implied*.
6. **Structural landing (deferred to 64/65).** When the composition root exists: place the gate/registry
   construction behind it; make the minter the sole constructor of any write authority; the "no
   gate-mutation capability type exists" property becomes a compile-checked fact of the type graph.

### 2.4 Required proofs (`CHECKLIST.md` 5-point + the bypass falsifier)

| CHECKLIST point | Item 73 disposition |
|---|---|
| 1. Oracle | **Exhaustive planted-proposal oracle:** one planted proposal per red-line path *class* (money/auth/registry/gate/verification) → each refused at step zero with its typed named cause. Input space is the enumerable set of red-line classes → exhaustive, the FSM-oracle precedent. |
| 2. dudect | **N/A** — no secret-dependent timing (the registry is public data; a red-line classification is not a secret). Recorded `N/A(no-secret-timing)` per the CHECKLIST honesty rule. |
| 3. debug cross-check | Classifier dual-representation (path-prefix match vs an explicit per-path table) `debug_assert_eq!` — the `order_machine` FSM_ADJ cross-check pattern. |
| 4. asm spot-check | **N/A** — no branch-free crypto path. `N/A(no-crypto-path)`. |
| 5. formal/compile-fail | **The load-bearing one — see §2.4-bypass.** |

**§2.4-bypass — prove the human-gate cannot be bypassed (mandatory):**

- **(a) Compile-fail: no gate-mutation capability constructor exists.** A dev-only compile-fail case
  (`kernel/tests/compile_fail/gate_root_no_write_cap.rs`, `trybuild`-style dev-dep OR a scripted
  `cargo build` asserted to fail) that *attempts* to construct any value conferring write authority
  over the registry/gate/root → **does not compile** (no such type/constructor is nameable). RED→GREEN:
  the case must fail-to-compile *for the right reason* (unknown/private constructor), demonstrated by
  first showing a hand-added `pub fn mint_gate_write()` makes it compile (RED), then removing it (GREEN).
- **(b) Grep proof: registry has no mutation surface.** CI grep asserts zero `pub fn.*&mut` /
  `pub static mut` / interior-mutability handle in the registry module. Planted `pub fn` → RED.
- **(c) Dependency-direction RED path.** A planted pipeline→gate reference (a `use
  registry::internal_mutate`) turns the dependency-direction check RED (item-45 planted-reference
  precedent). Restore → GREEN.
- **(d) Step-zero refusal RED path.** A planted proposal whose diff touches a gate/registry/red-line
  path is refused at classification time with the typed cause + an FDR record, **before** verification
  runs — proven by asserting the verification function was *never entered* (a test spy / call counter
  at 0).

### 2.5 Falsifiable acceptance criteria

- No in-tree type/fn/macro constructs write authority over registry/gate/root (grep + compile-fail).
- The dependency-direction check is GREEN with an empty violation set and RED on a planted reference.
- Every red-line-class planted proposal is refused at step zero with a distinct typed cause + FDR record.
- The out-of-band-only law is recorded in `CHECKLIST.md` + registry module doc; item 74 carries §L's
  own gate as rows (self-reference verified by item 74's self-row test).

### 2.6 Dependency gates

- **Spec + clauses (ii)/(iii) at CI-grep level:** buildable now (needs item 74's registry to point at).
- **Clause (i) structural landing:** GATED on items 64 (composition root) + 65 (capability boundary),
  both **unbuilt** (§1.2). Do not fake a minter to land clause (i) early.

### 2.7 Operator-decision points (flagged, NOT resolved — see §9)

- **Q4:** the *boundary* of "the gate/registry/verification code" for the classifier — exactly which
  path-prefixes are meta-governance vs. ordinary-red-line — is a judgment the operator must ratify;
  an agent must not draw that line unilaterally (it is the line that decides what the AI can never
  touch).

---

## 3. Item 74 — the red-line/core registry (machine-readable enumeration of what AI may NEVER touch)

### 3.1 Scope / goal

A machine-read manifest in the `HOT-PATHS.tsv` idiom (path-prefix zones → deterministic diff
classification), **every row carrying WHY + the real source**. It is the code-plane twin of
`scope.rs`'s runtime red-line set. The safety-critical deliverable of this arc: **zero hand-waving,
every row cited.**

### 3.2 Current-state grounding (file:line)

- The idiom exists and is proven: `HOT-PATHS.tsv` (`:5–25`) already does path-prefix `@ZONE` →
  deterministic diff classification, parsed by `scripts/hardening-gate.sh`, with a `min_tests=0`
  placeholder idiom (`:18–19`) for "a future item's obligation that has no code yet" — **exactly** the
  shape item 74 needs for rows referencing unbuilt machinery (items 9/54/64/65).
- The runtime-plane red-line set already exists to mirror: `scope.rs` `is_red_line` (`:96–101,
  176–184`) enumerates money/auth/secrets/migrations. Item 74 is the **code-path** enumeration of the
  same red-lines plus the proof machinery.
- Every seed row's target verified present on `main` (§1.1 table). Rows (7) breaker/Sentinel and the
  64/65 composition-root rows target **unbuilt** machinery → they land as `min_tests=0`-style
  placeholder rows (diff-visible obligation, no floor yet).

### 3.3 Implementation plan (numbered)

1. **Create `docs/audits/governance/RED-LINE-REGISTRY.tsv`** in the HOT-PATHS idiom. Columns:
   `path_prefix  class  why  source  removal_authority`. `class` ∈ the enumerated red-line classes;
   `source` is a real `file:line` or ruling citation; `removal_authority` = `out-of-band-only` (all
   rows) + `operator-ruling-required` (all rows, per §3.4 monotonicity).
2. **Seed the rows, each cited (roadmap lines 1314–1330):**
   - **(1) Product red-lines:** `kernel/src/money.rs` (integer-only checked, `:8,17,22`);
     auth/capability surfaces (`ports/agent/scope.rs`, `capability_cert.rs`); pgrust migrations;
     bulk-op paths. Source: standing memory `test-integrity-rules-2026-06-27` + SELF-DEVELOPMENT
     charter "red-lines preserved even under full autonomy."
   - **(2) Proven-FSM core:** `kernel/src/order_machine.rs` + `FSM_GOLDEN_SIGNATURE` (`:513`) drift
     gate (`:549–563`) + `FSM_SPECTRAL_RADIUS` const-proof (`:383`).
   - **(3) Verification seam:** `kernel/src/decision/import.rs` `import_unit` (`:81`) replay-before-persist;
     item 47 `admit`/`Invariants` + item 50 K3 fold **when they land** (placeholder rows).
   - **(4) Forensic truth surface:** `kernel/src/event_log.rs` (`:109,135–160` SHA3 chain) +
     `kernel/src/fdr/*` (ring/schema/CRC). *Item 76 depends on this being un-modifiable-by-the-subject.*
   - **(5) Crypto:** `kernel/src/pq/` (KAT-gated), `kernel/src/ct_gate.rs`, `kernel/src/capability_cert.rs`.
   - **(6) The proof machinery itself:** `scripts/zero-dep-gate.sh` + every `ZERO-DEP-ALLOWLIST.txt`,
     `scripts/hardening-gate.sh` + `CHECKLIST.md`/`HOT-PATHS.tsv`, `rust-toolchain.toml` + bump gate,
     the miri/lint/ai-optional gates, `ci.yml` gate jobs.
   - **(7) Safety machinery as it lands (placeholder rows):** item 9 breaker, item 54 Sentinel, items
     64/65 composition root + capability types.
   - **(8) §L's own gate code + this registry** (item 73's recursion, recorded as rows).
3. **Classifier script** `scripts/red-line-classifier.sh` (or a leg of the gate-root check): maps a
   change-set (merge-base diff) → `touched-red-line: {class, path, source}` verdict deterministically,
   path-prefix zones exactly as `hardening-gate.sh` does. This is the concrete mechanism item 75's
   step-zero calls.
4. **Self-row test:** the registry file is itself in the registry (roadmap line 1336) — a test asserts
   `RED-LINE-REGISTRY.tsv`'s own path-prefix appears as a row.
5. **Monotonicity — row REMOVAL is the dangerous direction.** Adding a row is always safe (widens
   protection). Removing a row additionally requires an **explicit operator ruling recorded in the
   row's history** (roadmap line 1331 — the allowlist-monotonicity insight, *inverted*: shrink is
   dangerous here, where for the dependency allowlist shrink was safe). A CI check (the zero-dep-gate
   `comm -13` precedent, reversed) fails on a removed row unless the diff carries the operator-ruling
   marker.

### 3.4 Required proofs (5-point + bypass falsifier)

| CHECKLIST point | Disposition |
|---|---|
| 1. Oracle | **One planted diff per row CLASS** demonstrably refused (roadmap line 1335). Enumerable over the class set → exhaustive. |
| 2. dudect | N/A(no-secret-timing). |
| 3. debug cross-check | Classifier path-prefix match vs explicit per-row table `debug_assert_eq!`. |
| 4. asm spot-check | N/A(no-crypto-path). |
| 5. formal | Self-row test + monotonicity CI check (re-executed, P7). |

**§3.4-bypass — prove the gate cannot be bypassed via the registry:**

- **(a) No mutation surface** (shared with §2.4-bypass-b): the registry is a `.tsv` + `static`
  classifier; **there is no runtime API to edit it**. The only way to change it is a human editing the
  file through the normal CI suite. Grep proves zero runtime write path to the file from any pipeline
  module.
- **(b) Removal-guard RED path:** a diff removing a registry row without the operator-ruling marker →
  CI RED. Planted removal (RED) → add marker (GREEN). Proves the allowlist cannot silently *shrink*
  (the corrigibility-adjacent attack: quietly drop a protected path so a later proposal can touch it).
- **(c) Self-inclusion RED path:** delete the registry's self-row → self-row test RED.

### 3.5 Falsifiable acceptance criteria

- Every row resolves to a real `file:line` or recorded ruling (no un-cited rows — grep every source).
- The classifier maps a change-set → touched-red-line verdict deterministically; one planted diff per
  class refused.
- The registry is in the registry (self-row test green).
- Row removal without operator-ruling marker is RED; row addition is always green.

### 3.6 Dependency gates

- **Buildable now** for the on-`main` targets (classes 1–6, class 8). Classes 3 (items 47/50), 7 (items
  9/54/64/65) land as **placeholder rows** (`min_tests=0` idiom) with the obligation diff-visible until
  the referenced machinery exists. After item 73's spec (which this operationalizes).

### 3.7 Operator-decision points (§9)

- **Q4** (shared with 73): the exact path-prefix boundaries — especially class (6)/(8) meta-rows.
- **Q5:** the operator-ruling **marker mechanism** for row removal (a commit-trailer? a signed ruling
  doc? a `DECISIONS.md` D-entry?) — a governance-record format the operator must define.

---

## 4. Item 75 — the change-proposal pipeline (typed `ChangeProposal` → gate → human token → applied)

### 4.1 Scope / goal

An AI-generated change is **DATA**: a typed `ChangeProposal` carrying the diff/artifact + its trigger
+ its claimed intent. It flows: **step-zero classification (item 74) → kernel's OWN existing gates
re-executed on the proposed state → explicit human "apply" token → recorded application.** The hard
property (roadmap lines 1349–1358): **an explicit human "apply" action is required for every
application — no autonomous apply path exists** — structurally, the apply function requires a
human-approval token only the operator's out-of-band action mints. Absence of approval is a permanent
pending state that **expires**; silence is never consent.

### 4.2 Current-state grounding (file:line)

- **Admission grammar to reuse verbatim (never fork):** item 47's `admit(Proposal, &Invariants) ->
  Result<ValidatedProposal, Rejection>` (roadmap line 688) with `ValidatedProposal` constructible only
  through `admit`; item 50's `RejectionClass::{Refuted, Undecidable}` + the strong-Kleene fold, with
  Kleene-Unknown collapsing to the safe pole (line 786–789). Item 75 is *"same shape, different plane,
  no parallel admission concept"* (line 1339).
- **Verification = the kernel's OWN gates re-executed** on the proposed state — zero-dep gate,
  hardening-gate rows, full suite, miri, item-44 CI discipline — **never a parallel/lighter checklist**
  (line 1344). The precedent is exactly `import_unit`'s replay-before-persist (`decision/import.rs:81,
  119–125`): the candidate's own GREEN is never trusted; the local oracle re-executes.
- **Pre-verified arrival shape (preferred):** `eqc-rs` `emit_proof_program` (`tools/eqc-rs/src/lib.rs:442`)
  — a proposal can arrive as a PROVEN artifact rather than raw untrusted code. Reuse it; do not invent a
  second generator discipline (line 1346).
- **Approval token = item 65's capability shape at the approval seam** (line 1352), reusing
  `capability_cert.rs` attenuation (`:205–231`), **no new crypto**. Item 65's minter is the composition
  root — **unbuilt** (§1.2), so the token's *structural* landing is gated.

### 4.3 Implementation plan (numbered)

1. **`ChangeProposal` type (data, no authority).** Fields: `diff_or_artifact`, `trigger`
   (`Trigger::{HealthVerdict, OperatorPrompt, UpgradeCandidate}` — feeds items 77/78), `claimed_intent`,
   `content_id` (SHA3, for FDR linkage). Decode fail-closed exactly as `scope.rs` (`:77–92`).
2. **`admit(ChangeProposal, …) -> Result<VerifiedChangeProposal, Rejection>`** — same signature shape
   as items 47/50. `VerifiedChangeProposal` constructible **only** through `admit` (illegal-state-
   unrepresentable). Order of operations, non-negotiable:
   - **step zero:** item 74 classifier. Red-line touch → `Rejection{ class: Refuted, red_line: <cause> }`
     **before anything else** (item 73(iii)). No human is asked.
   - **verification:** re-execute the kernel's own gate suite on the proposed state. Any gate failure →
     `Rejection` with `RejectionClass::{Refuted | Undecidable}`; Kleene-Unknown → Undecidable → safe
     pole (not-applied), logged distinctly. **A proposal failing any gate NEVER reaches a human**
     (machines filter, humans decide).
3. **The apply seam — the load-bearing type.** `apply(VerifiedChangeProposal, token: &HumanApprovalToken)
   -> Applied`. `HumanApprovalToken` is:
   - constructible **only** by the operator's out-of-band mint (item 65's root minter when it lands;
     until then, **no `apply` function is built at all** — the pipeline STOPS at pending-approval, which
     is the correct behavior and the whole point);
   - required **by value/reference at the call site**, so `apply` is *uncallable* without one
     (`cap: &CoreWriteCapability` shape, roadmap line 1126).
4. **Pending state + expiry.** A `VerifiedChangeProposal` with no token is a `Pending{ expires_at }`
   state. On expiry → `Expired` (safe pole, not-applied) + FDR record. TTL is a **named constant, one
   authority site** (P3); its *value* is an operator ruling (§9, Q3).
5. **FDR records at every transition:** admitted-to-pending, approved, refused, expired — each an FDR
   event (feeds item 76's lineage).

### 4.4 Required proofs (5-point + bypass falsifier)

| CHECKLIST point | Disposition |
|---|---|
| 1. Oracle | Planted valid proposal passes all gates and **STOPS at pending-approval**; planted gate-failing proposal never surfaces; planted red-line proposal refused at step zero with typed cause. Exhaustive over the `RejectionClass` × trigger space. |
| 2. dudect | N/A(no-secret-timing). |
| 3. debug cross-check | K3 fold dual-check (item 50 truth-table); admit-order assertion (step zero ran first). |
| 4. asm | N/A. |
| 5. formal | The apply-seam compile-fail (§4.4-bypass-a) + the item-50 K3 exhaustive truth-table (all 9 cases/operator). |

**§4.4-bypass — prove no code path applies without the human token (THE arc's headline falsifier,
roadmap line 1358 verbatim: "unconstructible, compile-fail test"):**

- **(a) Compile-fail: `apply` uncallable without a token.** A dev-only compile-fail case attempting
  `apply(vp)` (no token) or `apply(vp, forged_token)` where `HumanApprovalToken` has no public
  constructor → **does not compile**. RED→GREEN: temporarily add `pub fn forge() -> HumanApprovalToken`
  → the case compiles (RED, proving the test is real); remove it → GREEN.
- **(b) No autonomous-apply path exists.** Grep proof: the *only* construction site of
  `HumanApprovalToken` is the operator's out-of-band mint (composition-root minter when it lands); zero
  other sites. Until the minter exists, grep proves **zero** `apply` call sites (the pipeline
  physically cannot apply).
- **(c) Pending permanence + expiry-to-safe-pole.** A verified proposal left un-approved stays
  `Pending` indefinitely, never auto-applies (test asserts no state transition to `Applied` without a
  token); at TTL it becomes `Expired` (not `Applied`) + FDR record. Silence → not-applied, proven.
- **(d) Verification-before-human RED path.** A gate-failing proposal never reaches the pending state
  (test spy asserts the approval-request FDR event was never emitted).

### 4.5 Falsifiable acceptance criteria

- `apply` is uncallable without `HumanApprovalToken` (compile-fail) and the token has exactly one
  (out-of-band) constructor (grep).
- A valid proposal halts at `Pending`; a gate-failing one never surfaces for approval; a red-line one is
  refused at step zero — all three with distinct FDR records.
- Kleene-Unknown collapses to not-applied (K3 truth-table exhaustive).
- Expiry produces `Expired`, never `Applied`; the whole flow re-executed in CI (P7), never presence-checked.

### 4.6 Dependency gates

- GATED on: **item 73 spec + structural halves** (compile-fail meta-property), **item 74** (step-zero
  classifier), **items 47/50** (`admit` grammar — unbuilt), **item 65** (token minter — unbuilt, needs
  item 64). Until item 65's minter exists, item 75 lands **without any `apply` function** — the pipeline
  ends at `Pending`, which is *itself* the correct and testable first deliverable.

### 4.7 Operator-decision points (§9)

- **Q2:** what physically constitutes the operator's out-of-band token-mint (signed commit? PQ
  `capability_cert`? hardware key? air-gapped ceremony?). **If cryptographic, must be ML-DSA-65 (D4).**
- **Q3:** the pending-approval TTL value.
- **Q1:** on a decentralized mesh, whose token gates whose node (ties to D10).

---

## 5. Item 76 — proposal lineage + cost-classified impact at the approval seam

### 5.1 Scope / goal

Make "the same amount of predictability/transparency/telemetry" **mechanical**. Every proposal carries
a full FDR-logged causal trail (item 62's `span_id`/`parent_span_id`): trigger → generation → per-gate
verification verdicts (item 55 class discipline: Proved/Refuted/Undecidable per gate) → human approval
or rejection (operator identity + timestamp, an FDR event) → application record — a **reconstructible
causal TREE** end to end. AND the approval screen is **never blind**: the proposal's predicted
cost/impact goes through item 67's classification (`ORACLE-EXACT / ORACLE-BOUNDED / MEASURED-ONLY` +
evidence pointer), presented to the human **before** approval; an unclassifiable impact is presented
**AS the forbidden/unclassified state** (precision-honest), never a fabricated estimate.

### 5.2 Current-state grounding (file:line)

- **FDR is flat/unlinked today** — verified: `grep 'span_id\|parent_span_id\|parent\|trace'
  kernel/src/fdr/schema.rs` → **0 hits**. Item 62 adds `span_id: u64` + `parent_span_id: Reading<u64>`
  with `Unavailable(NoParent)` at a root (roadmap line 1069). Item 76 **consumes** that — it is gated on
  item 62.
- **Cost classification** = item 67's `ORACLE-EXACT/BOUNDED/MEASURED-ONLY` buckets (roadmap line
  1152–1156) + item 68's tables/intervals + item 70's aggregate propagation via ρ(A). All **unbuilt**
  (§K spec) → item 76 gated on 67 (68/70 enrich).
- **Forensic surface must be trustworthy:** item 74 row (4) makes `event_log.rs`/`fdr/*` un-modifiable
  by the subject of the evidence — item 76 *depends on that* (roadmap line 1321). The hash chain
  (`event_log.rs:135–160`) + FDR CRC give the tamper-evidence.

### 5.3 Implementation plan (numbered)

1. **Lineage tree over item 62's links.** Each `ChangeProposal` gets a root `span_id`; generation,
   each gate verdict, approval/rejection, application each emit a child FDR record with
   `parent_span_id`. Reconstruction = walk parent links from a recovered ring (item 62's own proof
   shape).
2. **Per-gate verdict records (item 55 discipline):** each verification gate writes a
   `Proved/Refuted/Undecidable` FDR record — not a single pass/fail — so the human sees *which* gate
   said *what* and *why*.
3. **Approval record carries identity + timestamp + the impact class.** The FDR approval event records
   operator identity (per Q1's resolution), timestamp, and the item-67 impact class + resolving evidence
   pointer.
4. **Impact classification at the seam.** Before the human is asked, run the proposal's predicted cost
   through item 67's classifier. Present bucket + evidence pointer (item 68 table; item 70 aggregate
   where the change touches the call graph). **Unclassifiable → present the forbidden/unclassified
   state literally** (the `unavailable` reason idiom, item 69's precedent), never a number.
5. **P3 firewall:** no lineage value and no cost value feeds any hash/gate/replay/decision surface
   (grep proof, item 62 + item 68 precedent). Lineage/impact are *presented to a human*, never *acted
   on by a machine*.

### 5.4 Required proofs (5-point + bypass falsifier)

| CHECKLIST point | Disposition |
|---|---|
| 1. Oracle | End-to-end test recovers the complete lineage tree from a real recovered ring; the approval record carries impact class + evidence pointer; an unclassifiable-impact proposal shows the honest marker (red→green vs a fabricated number). |
| 2. dudect | N/A. |
| 3. debug cross-check | Parent-link walk vs an explicit expected tree `debug_assert_eq!`. |
| 4. asm | N/A. |
| 5. formal | P3 grep proof (no lineage/cost value feeds a decision surface) — re-executed, green. |

**§5.4-bypass — prove lineage/impact cannot become an apply-authority backdoor:**

- **(a) Presentation-only, never authority.** Grep proof (P3): no `span_id`/`parent_span_id`/impact-class
  value is read by `admit`, `apply`, any gate, any hash, or any replay path. A planted read of an impact
  value inside `apply` → P3 grep RED. This closes the subtle attack where "the impact looked cheap, so it
  auto-applied" — impact is **shown to the human**, it never gates.
- **(b) Tamper-evidence integrity.** Because item 74 row (4) forbids AI modification of `event_log.rs`/`fdr/*`
  (and item 73 refuses such a proposal at step zero), the lineage is trustworthy: a proposal to weaken
  the very forensic surface that would record its own misbehavior is refused *by construction* — proven by
  the item-73 planted-red-line-proposal test targeting `event_log.rs`/`fdr/`.

### 5.5 Falsifiable acceptance criteria

- The full lineage tree reconstructs from a recovered ring (trigger→…→application).
- The approval record carries the impact class + a resolving evidence pointer; unclassifiable shows the
  honest marker, not a number.
- P3 holds: no lineage/impact value feeds any decision/gate/hash/replay surface (grep green).

### 5.6 Dependency gates

- GATED on **item 75** (the pipeline), **item 62** (span linkage — **unbuilt**, FDR schema flat), **item
  67** (cost classification — unbuilt). Enriched by items 68/70 when they exist. Item 74 row (4) must be
  in place for lineage trustworthiness.

### 5.7 Operator-decision points (§9)

- **Q1:** "operator identity" recorded in the approval event presupposes a defined identity model
  (per-node? mesh-wide? ties to D10).

---

## 6. Item 77 — self-healing specialization (health-classifier-triggered fix proposals; SAME pipeline)

### 6.1 Scope / goal

Health-triggered **fix** proposals through the FULL item-75 pipeline — explicitly **NOT a lighter
gate** (roadmap line 1379). Two kinds of self-healing, kept apart:

- **(A) Process-level restart-recovery stays AUTOMATIC** — crash → PostMortem → restart from last-good
  state (items 45–49; the kill-9 test's 300/300 recovery, zero human intervention). Pure recovery,
  modifies no code/logic → **no approval gate applies, none is needed.**
- **(B) Code/logic-level fix is the NEW capability and takes the FULL item-75 pipeline:** a **recurring**
  adverse classification (e.g. `LimitCycle`/`StrangeAttractor` on the same subsystem across ≥N
  consecutive windows — N a named constant, P3) generates a `ChangeProposal` with the health evidence
  attached; then it is a proposal like any other — verification, human approval, FDR lineage, *never*
  autonomous. "It's just fixing a detected problem" is **not an exception** (operator's directive is
  explicit).

Prior art: the autonomic-computing **MAPE-K** loop (Kephart & Chess 2003) — Monitor → Analyze → Plan →
Execute over shared Knowledge — with **one deliberate, stated deviation: Execute is never autonomous
for code-level change; the human operator IS the Execute gate.** "Regenerative software" = this loop
under those constraints, not a new mechanism.

### 6.2 Current-state grounding (file:line)

- **Health tracking already exists; the consumer is what's missing** (roadmap line 1382). Verified:
  `markov::Verdict{Healthy,LimitCycle,StrangeAttractor}` is real and live (`markov.rs:42–48`) but
  **observational-only** — its only non-test consumer is the `markov_attractor` bin emitting CLI JSON.
- **One live fail-closed health consumer already exists** as the pattern to follow:
  `RetainedBase::admit` (`kernel/src/spectral_cache.rs:267–269`) rejects `DriftClass::Unstable` input.
  So a classifier already gating admission is *in-tree*.
- **Trigger-evidence law inherits item 56** (line 1399): only `Basis::Measured` verdicts count — an
  unevaluated-Healthy (or unevaluated-anything) window is **never** trigger evidence. Item 56's `Basis`
  field is **unbuilt** (`markov.rs` has `Verdict` + `verdict_str` at `:98`, **no `Basis`**) → item 77
  gated on item 56.
- Additional trigger inputs cited by the source: item 9 breaker `Tripped` and item 54 Sentinel — both
  spec-level; they enrich, they do not gate (item 77 gates on 75 + 56).

### 6.3 Implementation plan (numbered)

1. **The missing trigger-consumer (not a new classifier).** A recurrence tracker over `markov::Verdict`
   + `spectral::DriftClass` + (when they land) item 9 `Tripped` / item 54 alarms: counts consecutive
   adverse windows per subsystem. At ≥N consecutive → emit ONE `ChangeProposal` with
   `Trigger::HealthVerdict` + the health trail attached.
2. **N is a named constant, one authority site (P3).** A single adverse window does **not** trigger. N's
   *value* is an operator ruling (§9, Q6).
3. **Trigger-evidence law (item 56):** only `Basis::Measured` windows count. Unevaluated-basis windows
   (`WindowTooShort`/`AnalyzerError`) are **excluded from trigger evidence in either direction**. This is
   why item 56 is a hard gate.
4. **The proposal is then item-75-identical** — no lighter path, no "self-healing exception." One
   proposal per recurrence, halts at `Pending`.
5. **Keep (A) automatic and untouched.** No approval gate on process restart-recovery; the kill-9 test
   stays green through this item.

### 6.4 Required proofs (5-point + bypass falsifier)

| CHECKLIST point | Disposition |
|---|---|
| 1. Oracle | A synthetic recurring-adverse verdict stream yields **exactly ONE** proposal carrying the full health trail, which STOPS at pending-approval; a single adverse window does NOT trigger (threshold pinned); planted unevaluated-basis windows are provably excluded (red→green against byte-identical records). |
| 2. dudect | N/A. |
| 3. debug cross-check | Recurrence counter vs explicit expected-count `debug_assert_eq!`. |
| 4. asm | N/A. |
| 5. formal | Threshold-pinned + basis-exclusion re-executed (P7). |

**§6.4-bypass — prove self-healing is not an autonomous back door:**

- **(a) Reuse item 75's §4.4-bypass wholesale.** A health-triggered proposal STOPS at `Pending` exactly
  like any other — the item-75 red-proof re-run with `Trigger::HealthVerdict`. There is no
  self-healing-specific apply path (grep: `apply` has no health-trigger branch).
- **(b) Recovery class (A) stays automatic and green** (kill-9 test unchanged) — proving the AUTOMATIC
  path is *only* pure-recovery (modifies no code/logic), never code mutation.
- **(c) No trigger inflation.** Planted unevaluated-basis windows do not trigger — closing the attack
  where fabricated "unhealthy" windows manufacture proposals to spam the approval seam.

### 6.5 Falsifiable acceptance criteria

- A recurring-adverse stream yields exactly one pending proposal with the full health trail; a single
  window does not trigger.
- Unevaluated-basis windows are excluded from trigger evidence (both directions).
- Class (A) restart-recovery remains automatic; kill-9 300/300 green.

### 6.6 Dependency gates

- GATED on **item 75** (pipeline) + **item 56** (`Basis` retrofit — **unbuilt**). Enriched by items 9/54
  when they land (not gating).

### 6.7 Operator-decision points (§9)

- **Q6:** N (consecutive-adverse-window threshold).
- **Q7 (raised honestly):** should code-level self-healing be built **at all** in the first product
  iteration, or should only class (A) automatic-recovery ship until the human-gate machinery has proven
  itself in the lower-risk item-78 upgrade path? This is an operator sequencing ruling, not an agent's.

---

## 7. Item 78 — self-upgrading specialization (improvement proposals beyond fixes; SAME pipeline)

### 7.1 Scope / goal

Trigger = not a detected problem but a **proposed improvement**: cost-oracle-informed candidates (item
70's twin identifying bottlenecks/regressions), pre-proven rewrite candidates (item 71's eqc-rs
extraction arriving *with its proof program*), or operator-prompted upgrade requests — all routed
through the **same typed shape**. Upgrade proposals additionally carry a before/after predicted-cost
**DELTA** from the oracle on the approval screen. **The gate is IDENTICAL**; and item 73's law binds
**hardest** here (roadmap line 1421): an upgrade proposal touching a red-line path is refused
identically at step zero, with **no beneficial-change exception** — *the corrigibility trap is precisely
a sequence of individually-beneficial-looking upgrades reaching the gate.*

### 7.2 Current-state grounding (file:line)

- **Pre-proven arrival is real today:** `eqc-rs` `emit_proof_program` (`tools/eqc-rs/src/lib.rs:442`)
  already emits a self-contained Rust program proving generated code ≡ the `Expr::eval` reference — an
  upgrade can arrive as a PROVEN artifact. This is the item-75-preferred arrival shape reused, not a
  second discipline.
- **Cost delta** = items 67/68 (buckets + tables) aggregated via item 70's ρ(A) — all **unbuilt** (§K).
  Item 78 is *enriched by* 70/71 but **not gated on them** (roadmap line 1431): an operator-prompted or
  eqc-rs-pre-proven upgrade flows without them; the cost-delta screen is richer once they land.

### 7.3 Implementation plan (numbered)

1. **Reuse the item-75 pipeline unchanged.** `Trigger::UpgradeCandidate` (cost-oracle) /
   `Trigger::OperatorPrompt` / a pre-proven eqc-rs artifact all become the same `ChangeProposal`.
2. **Attach the cost delta** (when items 67/68/70 exist): before/after predicted-cost on the approval
   screen, precision-honest (unclassifiable → forbidden-state marker, item 76 §5.3).
3. **Item 73's law binds hardest — no beneficial exception.** The step-zero classifier refuses any
   upgrade touching a red-line/gate/registry path *identically*, with no "but it's an improvement"
   branch. This is the single most important test in the arc (§7.4-bypass-b).

### 7.4 Required proofs (5-point + bypass falsifier)

| CHECKLIST point | Disposition |
|---|---|
| 1. Oracle | An eqc-rs-generated, pre-proven rewrite flows end-to-end to pending-approval with its cost delta + proof-program result attached; refused + approved + expired upgrade proposals all leave complete FDR lineage. |
| 2. dudect | N/A. |
| 3. debug cross-check | Cost-delta presentation vs oracle value `debug_assert_eq!` (presentation only, P3). |
| 4. asm | N/A. |
| 5. formal | eqc-rs `emit_proof_program` green on the arriving artifact (compiled by real rustc, self-asserting). |

**§7.4-bypass — prove the corrigibility trap is closed:**

- **(a) Reuse item 75's §4.4-bypass** with `Trigger::UpgradeCandidate` — upgrades stop at `Pending`
  identically.
- **(b) The no-beneficial-exception RED path (THE corrigibility test).** A planted **"beneficial"**
  proposal touching gate/registry paths (e.g. "optimize the red-line classifier," "cache the approval
  check," "streamline the composition root") is refused at **step zero** — item 73's red-proof
  re-executed at the upgrade level. There must be **zero** code branch that treats a gate-touching
  upgrade differently from a gate-touching fix (grep: the classifier has no `is_improvement` bypass).
  This is the structural answer to "a system able to modify its own safety constraints can eventually
  remove them through individually-plausible steps."

### 7.5 Falsifiable acceptance criteria

- A pre-proven eqc-rs rewrite flows to `Pending` with cost delta + proof result; the proof program
  compiles and self-asserts.
- A "beneficial" gate/registry-touching upgrade is refused at step zero identically to a fix (no
  beneficial-change branch — grep + red→green).
- Refused/approved/expired upgrades all leave complete FDR lineage.

### 7.6 Dependency gates

- GATED on **item 75** only. Enriched by items 70/71 (cost delta / pre-proven extraction) — **not
  gating**; item 78 can ship with `Trigger::OperatorPrompt` + eqc-rs arrivals before the twin exists.

### 7.7 Operator-decision points (§9)

- **Q7 (shared):** whether item 78 (lower-risk, human-gated upgrades) should be the **first** self-
  evolution capability to ship, ahead of item 77's self-healing — an operator sequencing ruling.

---

## 8. Consolidated dependency graph + gate table

```
73 (spec) ── governs everything
   ├─ clauses (ii)/(iii) at CI-grep level:  buildable after 74
   └─ clause (i) STRUCTURAL:                 GATED on 64 + 65        [both UNBUILT]
74 (registry)                                 after 73 spec; classes 1–6,8 buildable NOW;
                                              classes 3 (47/50) + 7 (9/54/64/65) = placeholder rows
75 (pipeline)                                 after {73 + 74}; apply-seam GATED on 65 (⇐64), 47/50  [UNBUILT]
                                              → lands WITHOUT an apply fn until 65 exists (halts at Pending)
76 (lineage + cost)                           after {75 + 62 + 67}  [62, 67 UNBUILT]; enriched by 68/70
77 (self-healing)  ∥  78 (self-upgrading)     after 75
   77 also GATED on 56 (Basis) [UNBUILT]      78 enriched by 70/71 [not gating]
```

| Item | Buildable at spec now? | Hard code gates (unbuilt) | Enrichers (not gating) |
|---|---|---|---|
| 73 | Yes (spec + ii/iii grep) | 64, 65 (clause i) | 74 (points the checks) |
| 74 | Yes (file + classifier) | — (placeholder rows for 47/50/9/54/64/65) | — |
| 75 | Yes (type + admit + Pending) | 65 (⇐64), 47, 50 | eqc-rs (present) |
| 76 | No (needs 75+62+67) | 62, 67 | 68, 70 |
| 77 | No (needs 75+56) | 56 | 9, 54 |
| 78 | Partial (needs 75) | 65 (via 75) | 70, 71 |

§L consumes §K's machinery (56, 62, 64/65, 67/68, 70/71) and item 47/50's grammar; **it gates nothing
outside itself.** The AI that proposes remains behind item 45's `inference` gate and item 65's
capability boundary at all times — **§L grants a governed PROPOSAL channel, never authority.**

---

## 9. Operator-decision register — RESOLVED 2026-07-20, see DECISIONS.md D11

**All 7 questions below were ruled by the operator on 2026-07-20 — recorded verbatim in
`DECISIONS.md` D11 ("Governed Self-Evolution (items 73-78) — apply-token, boundary & sequencing
rulings"). Read D11 for the authoritative answer to each Q below; the original questions are kept
here unedited as the historical record of what was asked.** This ruling resolves the *content* of
items 73-78's design — it does NOT authorize dispatching them to code; per §10 below, that stays its
own explicit decision, and item 75's `apply` seam stays gated on items 64/65 regardless.

These are exactly the class the standing rule "**Never bypass human-gated decisions**" reserves for a
human. I flag them; I do **not** answer them. Each must be operator-ruled **before the corresponding
item moves from spec to code**, not during.

- **Q1 — Whose token, on a mesh?** D0's `decentralized`/`local-first`/`mesh` means no global admin.
  Whose human-approval token gates a given node's OS change — that node's own operator, a delegated
  authority, or a mesh-wide root? Ties directly to **D10** (`RootDelegationPolicy = OperatorSigned`,
  operator-may-override). Blocks items 75/76 (approval identity).
- **Q2 — What physically is the "apply" token?** Signed git commit? PQ `capability_cert`? Hardware key?
  Air-gapped ceremony? Constraint (not a ruling): **if cryptographic, ML-DSA-65 (D4), never classical.**
  Blocks item 75's apply seam.
- **Q3 — Pending-approval TTL value.** The named constant's number (P3 gives the *shape*; the operator
  gives the *value*). Blocks item 75's expiry.
- **Q4 — The meta-governance boundary.** Exactly which path-prefixes count as gate/registry/verification
  (item 74 classes 6/8) vs ordinary red-line. This is *the line that defines what the AI can never
  touch* — an agent must not draw it. Blocks items 73/74.
- **Q5 — Row-removal ruling marker.** The record format that authorizes shrinking the registry (commit
  trailer? signed ruling doc? a `DECISIONS.md` D-entry?). Blocks item 74's monotonicity guard.
- **Q6 — Self-heal threshold N.** Consecutive-adverse-window count before a fix proposal. Blocks item 77.
- **Q7 — Sequencing / go-no-go for self-modification at all.** See §10.

## 10. Honest scope verdict — should this arc proceed to code without an operator sign-off?

**Recommendation (mine, as System Architect — a flag, not a decision):**

- **Blueprinting (this doc) is appropriate and complete for the spec plane.** It is planning, it lands
  no code, and the roadmap explicitly authorizes spec-level work with "planning only — no item starts
  before the operator dispatches it."
- **But items 73–78 are the single most red-line-adjacent capability in the entire 78-item roadmap.**
  Building a channel by which the product's AI proposes changes to its own OS is the corrigibility
  problem itself. I recommend the operator treat **the transition from spec to code as a distinct,
  explicit go/no-go decision** — not folded into a routine "dispatch the next item" — and specifically:
  1. **Do not build item 75's `apply` function until items 64/65 exist** and Q1/Q2 are ruled. Until
     then item 75 legitimately ends at `Pending` — that is a *feature*, not an incomplete state.
  2. **Consider shipping item 78 (human-gated upgrades) before item 77 (self-healing)** so the human-gate
     machinery is battle-tested on operator-initiated, lower-urgency changes before anything is
     *triggered by the system's own health signal* (Q7). Self-healing's urgency pressure is exactly where
     a "just this once, automatically" shortcut is most tempting — and most dangerous.
     **OVERRIDDEN by D11 (2026-07-20): operator ruled item 77 before item 78, the roadmap's original
     numeric order.** This recommendation is kept here as the record of what was suggested and
     explicitly not taken — not a live instruction.
  3. **Q4 (the meta-governance boundary) must be operator-ratified before item 74's registry is
     authored**, because that boundary *is* the safety perimeter; an agent-drawn perimeter is a perimeter
     the agent could, in principle, have drawn to its own advantage.
- **What does NOT need a special gate:** item 77's class (A) automatic restart-recovery already ships and
  is proven (kill-9); it modifies no code/logic and needs no approval. Keep it automatic; do not
  over-govern pure recovery.

Nothing in this blueprint resolves Q1–Q7. They are recorded here so the operator sees the full decision
surface before dispatching any of items 73–78 to an executor.

---

## 11. Deliverables and what NOT to do

**Spec-plane deliverables (this doc IS the deliverable):** the six item designs above, each with a
5-point-standard proof mapping + a `§X.4-bypass` human-gate falsifier, dependency gates, and the Q1–Q7
register.

**When an executor is eventually dispatched (per item, per operator ruling), it MUST NOT:**

- Build any `apply` path before item 65's minter exists and Q1/Q2 are ruled (the pipeline ends at
  `Pending` until then — by design).
- Add any "beneficial change" / "self-healing" / "it's just a fix" exception to the step-zero red-line
  refusal (item 73(iii) — the corrigibility trap).
- Fork item 47/50's `admit` grammar or invent a second generator discipline (reuse `admit` +
  `emit_proof_program`).
- Introduce any new external crate (zero-dep gate) or new crypto/hash primitive (reuse CRC32/SHA3/
  `capability_cert`/`ct_eq`).
- Draw the meta-governance boundary (Q4) or the registry contents' safety perimeter without an operator
  ruling.
- Edit `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` or `CORE-ROADMAP-INDEX.md` (parallel agents
  own other ranges).

**Under-scoping guard:** the arc's value is the *unconstructible-without-human-token* property (§0.4). A
version that adds an approval *check* without making the bypass a *compile error* is item-75-in-name-only
and must not be accepted as landed — the `§X.4-bypass` compile-fail cases are the acceptance core.
