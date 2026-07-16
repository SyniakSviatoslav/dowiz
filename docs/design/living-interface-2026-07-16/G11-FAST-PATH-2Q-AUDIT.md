# G11 FAST-PATH — 2-Question Doubt-Check Audit (technology/architecture decisions)

> **What this is.** The AGENTS.md "Session/plan closing ritual — the 2-question doubt check" applied to the
> **engineering decisions** made across the six living-interface blueprints on the confirmed G11 fast-path
> (`Phase 0 → 1 → 2 → 3 → 4 → 5 → 6 → 9a`). Not a review of writing or file organization — a review of the
> load-bearing technology/architecture choices, each **investigated against the live repo HEAD (2026-07-16)**
> and cross-checked between documents. Standalone audit; **edits none of the six source blueprints or the
> roadmap** (a separate consolidation pass applies any confirmed fixes).
>
> **Standard the decisions must serve** (roadmap §8): *the interface is a continuation of the backend — it
> renders, it never independently decides.*
>
> **Verdict legend:** **CLEARED** (checked, holds up) · **CONFIRMED** (a real issue, root cause described) ·
> **CLEARED\*** (architecture holds, but a named residual risk/seam is unowned).
>
> **Sources audited in full:** P00, P01, P02, P06, P08 (§2 on-path + skim), P09A, roadmap §8. **Live checks:**
> `engine/Cargo.toml`, `engine/src/bridge.rs`, `kernel/src/{order_machine,event_log,wasm}.rs`,
> `.claude/skills/reliability-gate/SKILL.md`, `/root/bebop-repo/bebop2/core/src/{field,dmd}.rs` +
> `tests/eig_parity.rs`, whole-tree `Cargo.toml` inventory + `[workspace]` grep + `crates/` grep.

---

## §1 — Question 1: "What are you least confident about?" (7 load-bearing decisions, investigated)

### Q1-a — The dual-context device model (P02 §3.1): native Rust `wgpu` vs JS-owned WebGPU, `GpuUploadSink` seam — **CLEARED\***

**Claim under test.** Native Rust `wgpu` (CI/local visual loop, behind `feature="gpu"`) and JS-owned WebGPU
(production browser) share **one seam**, `GpuUploadSink`; is this one architecture or two divergent code paths?

**Investigated.** `engine/src/bridge.rs` — the seam is real and already exists: `pub trait GpuUploadSink`
(`:35`), `HeadlessSink` impl (`:50/:57`), the generic `upload_to<S: GpuUploadSink>` (`:174`), the honest
`new_gpu` stub (`:220`, returns `Err`), `HeadlessGpu` (`:251`). No `wgsl`/`include_str`/`WgpuSink`/
`create_shader` exists yet (all forward-looking). The **data path** (zero-copy `Float32Array`/`bytemuck` view →
one `writeBuffer`) is genuinely shared as a *contract*; on the browser side JS re-implements that contract (it
cannot `impl` a Rust trait) — the blueprint is honest about this and it is sound.

**The real divergence risk is not the upload seam — it is the shader compiler.** P02 §5.2 ships **one** WGSL
source via `include_str!`, handed as *text* to JS `createShaderModule` in production, and to Rust
`wgpu::create_shader_module` in CI. Same text, **two independent WGSL front-ends**: **naga** (native/Lavapipe,
what P00's `gpu_smoke.rs` compile-gate exercises) vs the **browser's Tint** (production). naga and Tint diverge
on accepted feature subset, uniformity/error strictness, and codegen. P00's `create_shader_module` gate runs
**naga only**; nothing in CI compiles the WGSL under a browser engine. A construct naga accepts but Tint rejects
(or vice-versa) passes CI and breaks in prod. This is a **distinct axis** from J5 (which is about rasterizer
f32/`fwidth` differences); the only thing covering it is R-DEV's *manual* "one real-device pass per shader
change" (P00 §4) — a human step, not a gate.

**Verdict: CLEARED\*.** Shipping a single WGSL source via `include_str!` is the correct call and prevents the
worst drift (two hand-authored shaders). Residual, currently-unowned risk: **naga (CI) ↔ Tint (prod) front-end
divergence is caught by no automated gate** — only the manual real-device pass. The fast path treats "same WGSL
text ⇒ same compile" as guaranteed; it is guaranteed to be the *same text*, not the *same compiler outcome*.

---

### Q1-b — "No workspace relocation; add a root `[workspace]` over existing paths" (P02 §2.1) — **CONFIRMED** (the deviation is safe; the members list is not)

**Claim under test.** P02 deviates from RW-01's literal "promote `kernel/` into `crates/`" and instead adds a
root `[workspace]` listing crates *in place*. Safe deviation, or does it quietly break path-dependent tooling?

**Investigated.** (1) `grep -rn 'crates/'` across all `*.sh/*.yml/*.mjs/*.toml/*.json` (minus node_modules/
target/attic) = **0 hits** — **no tooling assumes a `crates/` path.** So the *no-relocation* half of the
deviation is genuinely safe; RW-01's `crates/` promotion was directory aesthetics, and nothing globs it.

(2) But the *act of introducing a root `[workspace]` at all* has real blast radius that P02 under-specifies.
Live inventory: **no `[workspace]` exists anywhere today**, and the sibling crates are `engine/`, `kernel/`,
`agent-governance-wasm/`, `wasm/`, and four under `tools/` (`deep-clean`, `async-spool`, `native-spa-server`,
`telemetry/rust-spool`). P02 §2.1's members list is `["kernel","engine","wasm","field-math","audio", …]`. That
list **omits `agent-governance-wasm` and all four `tools/*` crates.** Under Cargo semantics, once a root
`[workspace]` exists, every `Cargo.toml` beneath it must be a `members` entry or in `exclude`, or building that
crate fails with *"current package believes it's in a workspace when it's not."* The two most damaging orphans
are **`tools/native-spa-server`** (the production SPA/CSP binary P00 itself edits) and **`tools/deep-clean`**
(has two live Hermes cronjobs per MEMORY.md). Phase 2 is **on the G11 fast path**, so this breaks the build/
deploy/cron surface the moment it lands.

**Verdict: CONFIRMED.** The RW-01 deviation itself (keep paths, no `crates/`) is safe — verified, zero
`crates/` assumptions. The *load-bearing defect* is that P02 §2.1's `members` list is incomplete: as written it
orphans `agent-governance-wasm` + the four `tools/*` crates (incl. deploy-critical `native-spa-server` and
cronjob-critical `deep-clean`). Fix is cheap (enumerate all siblings, or `exclude` the tools) but must be stated
before Phase 2. (This interlocks with the Q2 finding below — the workspace also orphans P01's `brand-resolve`.)

---

### Q1-c — `resolve()`'s "five consumers including the CI golden" (P01 §7/J8) vs P00's actual CI — **CLEARED\*** (correctly deferred; the wiring seam is unowned)

**Claim under test.** P01 §7 asserts `resolve()` feeds **five** consumers incl. "the CI golden," which must pin
to `token_hash`. Does P00's Lavapipe CI actually consume `token_hash` anywhere concrete, or is this an
aspiration P00 never implements?

**Investigated (cross-checked P01 ↔ P00).** P00's `wgpu-smoke.yml` + `engine/tests/gpu_smoke.rs` (§3) do
`create_shader_module` over `**/*.wgsl` + one offscreen frame + pixel-hash/SSIM — and reference **neither
`token_hash` nor brand** at all. But this is **not** a contradiction: P00's Phase-0 golden is explicitly a
*trivial clear/triangle* with no brand color; P01 §7 itself states the CI golden becomes a brand consumer only
"at Phase 3+" and that "Phase 1 must land before any Lavapipe golden that includes brand color is locked." So
P00 *correctly* does not implement it — brand-in-golden is Phase-3 work, and P00/P02's Phase-0/2 goldens are
brand-free. The two docs are temporally consistent.

**The residual issue:** across all six blueprints, the concrete mechanism *"the golden records/pins the
`token_hash` it was rendered against"* is specified by **no one**. P00 builds `gpu_smoke.rs` with **no
`token_hash` hook**; P01 §7 states the *requirement* but wires it to no file; P02's Phase-2 golden is the
brand-free prove-the-pipe shader. There is no Phase-3 blueprint in this set that adds the hook.

**Verdict: CLEARED\*.** No P00↔P01 contradiction — the CI-golden-as-consumer is a Phase-3 materialization,
correctly absent from Phase 0. But the `gpu_smoke.rs` golden ↔ `token_hash` pin is an **unowned seam**: it is a
firm requirement in P01 §7 with no owning file/deliverable. Assign it explicitly to `gpu_smoke.rs` at Phase 3,
or golden rot (P01 §7's own failure mode) reappears silently.

---

### Q1-d — reliability-gate re-point: "L0–L11 semantics + five threads are architecture-agnostic, preserved unchanged; only the file list changes" (P09A §7) — **CONFIRMED**

**Claim under test.** The G11 acceptance authority is `/reliability-gate`. P09A says re-pointing it at the new
kernel-wasm/event-log interface changes only the file-target list; the stage semantics and threads are unchanged.

**Investigated (read `.claude/skills/reliability-gate/SKILL.md` in full).** The **five threads** (exactly-once,
recoverable, cross-surface, proof-by-artifact, timely) *are* architecture-agnostic — true. But the **per-stage
PASS criteria are concretely Postgres/Express/pg-boss assertions**, not abstract ones:
- L2: *"`POST /orders` is one **BEGIN/COMMIT** with **`idempotency_keys`** + orders + items"* and *"`idempotency_keys`
  PK is composite `(location_id, key)` after **migration 029**"*.
- L5: *"`WHERE status=$currentStatus RETURNING id`; **rowCount=0 → 409**"* (SQL row-level anti-race).
- L7: *"`delivery_trace` INSERT uses **ON CONFLICT (order_id) DO NOTHING**"*, plus RLS **FORCE** + policies.
- L9: *"ratings UPSERT with **ON CONFLICT(order_id)**"*.
- L11/N=2: *"`PgMessageBus.publish` uses **NOTIFY** via pool → all Postgres LISTEN clients."*

These do **not** map cleanly onto the architecture P09A §7 re-points them to. It re-points the *order path* to
**`place_order_js` + `apply_event_js` + `order_machine.rs` + the local-first event-log (DZ-06)** — a pure kernel
function plus an `event_log` substrate. Live check of `kernel/src/wasm.rs`: `place_order_js` (`:276`) is a
**stateless** function that still trusts client `unit_price` (`:56/:64/:122/:131`) and has **zero** references
to idempotency, `AppendOutcome`, or `event_log` (grep = 0). So:
- L2's *"one BEGIN/COMMIT with `idempotency_keys`"* is **structurally unsatisfiable** by `place_order_js` (no
  transaction, no idempotency). P09A §7 item 3 even promises the artifact *"place_order_js … + idempotency
  guard"* — **that guard does not exist and is scoped as built by no blueprint.**
- L5's *"`WHERE … RETURNING id` rowCount=0 → 409"* maps to the kernel's `assert_transition` `Err(Illegal)`
  (`order_machine.rs:123`) — a **different artifact**, not the SQL guard the criterion names.
- Replay maps to `fold_transitions` (`:140`); dedup maps to `AppendOutcome::Duplicate` (`event_log.rs:222`) —
  again, different artifacts than the migration-029 composite PK.

P09A partially papers over this by having items 4/10/11/12 quietly say those surfaces are *"served by the
mesh/attic backend, not the rebuilt owner UI"* for a 9a GO — i.e. the **old Postgres backend is still the
audited order backend**, which *contradicts* §7's re-point of the order path to `place_order_js`. One stage's
criteria end up split across two architectures without saying so.

**Verdict: CONFIRMED.** The five threads are agnostic; the **per-stage PASS criteria are not** — they are
Postgres/Express/pg-boss-shaped, and several (L2 idempotency, L5 anti-race) cannot be verified against the
kernel artifacts P09A re-points to. "Only the file-target list changes / preserved unchanged" is inaccurate:
the re-point requires a real old→new **criteria mapping** (idempotency: `idempotency_keys` PK → `place_order_js`
guard wired to `AppendOutcome::Duplicate`; anti-race: SQL `RETURNING` → `assert_transition`; replay →
`fold_transitions`) **or** an explicit ruling that the mesh/attic Postgres backend remains the L2/L7 order
backend in 9a (in which case `place_order_js` is *not* the L2 target). This is the acceptance authority for the
**entire G11 phase**, so the ambiguity is load-bearing.

---

### Q1-e — event_log/order_machine two-layer ordering authority "serves order + audio + viz" (P06 §5) — **CLEARED** on the over-build/`epoch` question; **CONFIRMED-nuance** on the exactly-once claim

**Claim under test.** P06 says Layer A (`event_log` dedup) is a common substrate for order/audio/viz, and
reserving the `epoch` field is free. Is Phase 6 over-building for P07/P08 consumers that may never ship?

**Investigated.** (1) **`epoch` is genuinely near-free.** Live `kernel/src/event_log.rs`: `MeshEvent` (`:134`)
carries `{prev, actor_pubkey, actor_seq, payload}` — **there is no `epoch` field in the kernel** (grep
confirms). So `epoch` is one `u64` added to the *Phase-6-authored* `FieldSource` wire envelope (and P08's
`LayoutKeyframe`/`ActivityDelta`), a struct Phase 6 writes anyway. Marginal cost ≈ 8 bytes + a field decl. The
"reserving it is free" claim holds. And P06 §5.6 is disciplined: only Layer B (order_machine validate+fold) is
declared G11-required; Layer A + `epoch` are landed "because nearly free." Over-build risk is minimal.

(2) **But P06 overstates Layer A's role on the order path.** P06 §7: *"The reliability-gate's exactly-once …
property is **precisely** the `event_id` idempotency (Layer A) + `fold_transitions` (Layer B)."* Live check:
the reliability-gate's exactly-once (per SKILL.md L2/L7/L9 and P09A) is **Postgres `ON CONFLICT`** +
`idempotency_keys` composite PK + `ratings UPSERT` at the mesh/attic backend — **not** `event_log`'s
`AppendOutcome::Duplicate`. And `place_order_js` is **not wired to `event_log` at all** today (grep = 0). So
`event_log`'s content-id dedup is a *different, additional* mechanism that the gate does not currently verify;
in the target DZ-06 local-first design it would become the client-side half, but that wiring is unbuilt and
Phase-6-pending. Claiming Layer A **is** the gate's exactly-once conflates the client-local dedup with the
server-side DB idempotency the gate actually checks.

**Verdict: CLEARED** (reserving `epoch`/the common envelope is cheap and honest; not an over-build). **Nuance
CONFIRMED:** P06 §7's "Layer A *is* the reliability-gate's exactly-once" is overstated — the gate checks
backend `ON CONFLICT`, and `place_order_js`↔`event_log` wiring does not yet exist. (Same root as Q1-d.)

---

### Q1-f — "5 T1 tokens, no AI theming" enforced by return type alone (P01 §2/§4) — **CONFIRMED** (necessary, not sufficient)

**Claim under test.** P01: `POST /owner/brand/generate` returns `T1Inputs`, so it is "structurally incapable of
emitting a per-component theme." Is the return type sufficient, or can a handler bypass `resolve()`?

**Investigated (design-level; the crate does not exist yet).** The return type on `generate()` does prevent
*that endpoint* from emitting a full theme — real and correct. But "structurally incapable" over-claims, on two
axes P01 leaves open:
- **`ResolvedTokens` construction is not sealed.** `to_css()` / `to_gpu_table()` are methods on `ResolvedTokens`
  (P01 §2, `src/resolved.rs`/`css.rs`/`gpu.rs`). P01 never states that `ResolvedTokens`'s fields are private or
  that `resolve(&T1Inputs)` is its *only* constructor. If a handler can build a `ResolvedTokens` directly (or
  the struct exposes public fields for `canonical_bytes()`), it can hand-construct arbitrary CSS/GPU bytes and
  call `to_gpu_table()`, bypassing the 5-input constraint entirely. The "only implementation of the transform"
  guarantee requires a **sealed** `ResolvedTokens` (private fields + no public ctor) — unspecified.
- **The output paths are not gated to resolve()-origin.** Nothing structurally stops a future route from writing
  raw CSS to the CDN path (`/cdn/themes/{id}/{hash}.css`) outside the bake job. The real enforcement is P01's
  **`token_hash` tripwire** (§2: re-run `resolve(stored_T1)`, assert it equals the served CSS comment hash **and**
  the GPU-table header, else the bake fails) **+ the bake job being the sole CDN/GPU writer** — not the return
  type. P01 has the tripwire but only inside the bake job; a write outside it is ungated.

**Verdict: CONFIRMED (partial).** The return type is a necessary guard on `generate()`, but the "structural"
guarantee against per-component theming actually rests on (1) sealing `ResolvedTokens` to `resolve()`-only
construction and (2) the bake job as sole output writer + the `token_hash` tripwire. P01 supplies (2)'s tripwire
but does **not** specify (1), and frames the return type alone as the guarantee. Cheap to fix (private fields,
`pub(crate)` ctor); currently aspirational, not structural.

---

### Q1-g — Phase-5 depends on the vendored `field-math` returning eigen*vectors* (P08 §2.3) — **CLEARED** (cross-repo "done" claim verified against actual bebop2 source)

**Claim under test.** The entire Phase-5 spectral-coords primitive (and thus Phase-8) rests on the RW-01-vendored
bebop2 `field.rs` providing an **eigenvector** source, because P08 §1.4 correctly proves the *kernel's own*
`spectral.rs`/`householder` path returns eigen*values only*. P08 cites `physics-ui-capture §2` for "field.rs
already returns eigenmode vectors" — a claim taken from a doc, about an un-vendored cross-repo file.

**Investigated (read `/root/bebop-repo/bebop2/core/`).** Verified against actual source, not the doc:
`field::jacobi_eigen(&c, n)` returns **`(eigvals, eigvecs)`** (used in `dmd.rs:76`, columns consumed at
`:95-97`), and `tests/eig_parity.rs:113` (`w2_1_field_eigenvectors_align_with_authority_order`) asserts each
returned eigenvector satisfies `A·v = λ·v` (residual check). `kalman.rs:30` documents the same
`V[i*n+j] = component i of eigenvector j` Jacobi output. So bebop2 `field.rs` **does** expose a tested
eigenvector source — P08's load-bearing assumption is **true**.

**Verdict: CLEARED.** The cross-repo "done" was real, not stale — confirmed by running down the actual bebop2
function + its parity test. Two secondary flags (non-blocking): (i) the vendor-completeness of `field::jacobi_eigen`'s
transitive deps (e.g. a `Complex` type) into the `field-math` copy should be checked at vendor time, since RW-01's
copy-list is `field.rs/chebyshev.rs/fft.rs/algebra.rs`; (ii) **new deps lack an inline DECART report** — P02 §3.1
introduces `bytemuck` as "the one new native dep" (confirmed genuinely new: `engine/src/zerocopy.rs:8` states the
engine uses no `bytemuck`), and neither `bytemuck` nor the `brand-resolve` `wasm-bindgen` feature carries the
DECART table the Detailed-Planning-Protocol step 3 / Integration-Decart-Rule require (`wgpu` is covered by the
prior "wgpu-sole-graphics-dep" ruling; `bytemuck` is not). Process gap, not a correctness bug.

---

## §2 — Question 2: the biggest thing missing (cross-document inconsistency in a technology choice)

**The finding (the tokio/ureq-class one): P01 and P02 disagree on whether a Cargo `[workspace]` exists, and
P02's workspace silently orphans P01's own `brand-resolve` crate.**

Analogous to the harness arc's tokio-vs-ureq drift, one blueprint made a structural decision the other's design
silently assumes did **not** happen:

- **P01 (Phase 1, Wave 0)** creates its `resolve()` crate at `/root/dowiz/brand-resolve/` and states as
  ground-truth: *"a sibling directory of `engine/`, `kernel/` … (**the repo has no root workspace; each crate is
  a peer directory**)."* Everything in P01 — the crate layout, the native+wasm build, the bake-job caller — is
  written against **peer directories, no workspace.** (Live check confirms: **no `[workspace]` exists today**, so
  P01's statement is true *at Phase 1 time*.)
- **P02 (Phase 2)** then does the opposite: *"Add a **root `Cargo.toml` with `[workspace]`**, `resolver = "2"`,
  `members = ["kernel","engine","wasm","field-math","audio", …]`."* This **creates** the workspace P01 said did
  not exist — and its `members` list **omits `brand-resolve`** (as well as `agent-governance-wasm` and the four
  `tools/*` crates, per Q1-b).

**Why this is a real technology inconsistency, not wording.** Under Cargo semantics, the instant a root
`[workspace]` exists, every `Cargo.toml` beneath the root must be a `members` entry or `exclude`d — otherwise
`cargo build -p <crate>` fails with *"current package believes it's in a workspace when it's not."* So when
P02's Phase-2 workspace lands, **P01's `brand-resolve` (built one phase earlier) becomes an orphan and stops
building** — and the Phase-1 bake job + the `token_hash` CI cross-check both depend on `brand-resolve`
compiling. The two blueprints, written by two passes in the same session, disagree on the single most basic
structural fact of the repo (workspace or not), and the disagreement is **on the G11 fast path** (Phase 2 sits
squarely in `0→1→2→…→9a`). P01 never anticipates being pulled into a workspace; P02 never accounts for the
crate P01 created (nor the pre-existing `agent-governance-wasm`/`tools/*`).

**A fresh reader would spot in thirty seconds** what neither pass could from inside its own document: P01's very
first design sentence ("the repo has no root workspace") is falsified by P02's very first build step, and P02's
`members` list is missing P01's deliverable. Neither doc's self-critique caught it because each is internally
consistent — the contradiction only exists *between* them.

**This is the same class of miss as Q1-b** (incomplete `members`), but sharper: Q1-b is "P02 forgot the existing
sibling crates"; Q2 is "P02 forgot the crate **a peer blueprint on the same critical path just created**, and the
two docs assert opposite ground-truth about the workspace." Fix: (a) reconcile the workspace decision across P01
and P02 (pick one — a root workspace is the better call for one lockfile/target dir, so P01's "no workspace"
framing should be updated to *"Phase 2 introduces the workspace; `brand-resolve` is a member"*), and (b)
enumerate **all** siblings in `members` (`kernel, engine, wasm, field-math, audio, brand-resolve,
agent-governance-wasm, tools/deep-clean, tools/async-spool, tools/native-spa-server, tools/telemetry/rust-spool`).

---

## §3 — If anything here is a big deal, here's what to fix before this ships

Two findings clear the "big deal" bar (a decision that, left as-is, breaks something load-bearing on the G11
fast path). Three more are cheap fixes worth folding into the consolidation pass.

**BIG DEAL — fix before Phase 2 lands:**

1. **Q2 + Q1-b — the workspace `members` list is incomplete and self-contradictory (P01 ↔ P02).** As written,
   P02's Phase-2 root `[workspace]` orphans `brand-resolve` (P01's Phase-1 crate), `agent-governance-wasm`, and
   the four `tools/*` crates — including **`native-spa-server`** (the prod SPA/CSP binary P00 edits) and
   **`deep-clean`** (live cronjobs). Every one stops building with "believes it's in a workspace." **Fix:**
   reconcile P01's "no workspace" statement with P02's workspace creation, and enumerate *all* sibling crates in
   `members` (or `exclude` the `tools/*`). One paragraph in the consolidated doc; but if missed, it breaks the
   build/deploy/cron surface the moment Phase 2 executes.

2. **Q1-d + Q1-e — the G11 acceptance authority assumes a criteria-mapping that doesn't hold.** `/reliability-gate`'s
   per-stage PASS criteria are Postgres/Express/pg-boss-specific (BEGIN/COMMIT + `idempotency_keys` PK,
   `ON CONFLICT (order_id) DO NOTHING`, `WHERE … RETURNING` anti-race, `NOTIFY`, RLS FORCE). P09A §7 re-points
   the L2 order path to `place_order_js` — which is a stateless function with **no idempotency guard and no
   `event_log` wiring** (verified) — while calling the criteria "preserved unchanged." **Fix (pick one,
   explicitly):** either (a) write the old→new criteria map into the re-point (idempotency → `place_order_js`
   guard backed by `AppendOutcome::Duplicate`; anti-race → `assert_transition`; replay → `fold_transitions`) **and
   scope "wire `place_order_js` idempotency" as a named 9a deliverable**, or (b) rule that the mesh/attic Postgres
   backend remains the audited L2/L7 order backend in 9a (then the Postgres criteria stand as-is and
   `place_order_js` is not the L2 target). Today P09A does both implicitly and reconciles neither — and this is
   the *done-test for the entire G11 phase.*

**Cheap, fold into consolidation:**

3. **Q1-f — seal `ResolvedTokens`.** Make its fields private and `resolve(&T1Inputs)` its only public constructor,
   and state "the bake job is the sole CDN/GPU-table writer" — otherwise "no AI theming" is convention + the
   `token_hash` tripwire, not the "structural" guarantee P01 claims.

4. **Q1-c — assign the `token_hash`-pinned golden to a file.** Name `engine/tests/gpu_smoke.rs` as the Phase-3
   owner of "the golden records its `token_hash`," or P01 §7's golden-rot failure mode returns unowned.

5. **Q1-a — decide the naga↔Tint ceiling explicitly.** Either add a browser-engine (Tint) WGSL compile gate to
   CI, or mark the "manual real-device pass" as the *accepted* ceiling for compiler-front-end divergence with an
   `innovate:` upgrade trigger — don't leave "same WGSL text ⇒ same compile" as an unstated assumption.

**Cleared, no action:** Q1-g (bebop2 `field::jacobi_eigen` genuinely returns tested eigenvectors — Phase 5's
eigenvector source is real); the `epoch`-reservation-is-free half of Q1-e; and the temporal P00↔P01 consistency
in Q1-c. Current-state facts the blueprints assert were spot-checked and hold: **402 kernel tests, 51 engine
tests, `order_machine`/`event_log` line citations, the `place_order_js` client-price gap, the empty `gpu`
feature, and "no eigenvector→coords helper in-kernel" all verify exactly.**

---

*End audit. Planning/review only — no product code, CI, canon, or the six source blueprints edited. Every verdict
above was checked against live HEAD (2026-07-16) or the actual bebop2 source, not taken from the blueprints'
self-description. Confirmed fixes are for the in-progress consolidation pass to apply.*
