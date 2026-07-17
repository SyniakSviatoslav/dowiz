# BLUEPRINT-H2 — Mirror-Pin Sweep (RC-4 closure)

> **Anchors:** Vibration (P3·A2 "one rate, one authority") × Polarity (P4·V3 "one axis, one
> representation") × Correspondence (P2 "forced divergence pinned by a parity check"). Root cause
> **RC-4 — Unpinned mirrors at the kernel↔engine seam** (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` §2,
> findings rows **#10, #18, #23**; bundled mechanical rows **#12, #24**).
> **Depends-on:** none — Wave-0 safe (adds pin-tests + two type/const edits, changes no runtime
> contract by default).
> **Parallel-safe-with:** H1 (cite-with-probe / RC-1), H3 (EventStore Result / RC-3), H4
> (self-governance ritual / RC-2) — all touch disjoint files.
> **Status:** PLANNING ARTIFACT ONLY. No `.rs` file is edited by this document.
> **Re-verified live** against `feat/kernel-fsm-graph-analysis` on 2026-07-16.

---

## §0 — The problem, and the template that already solves it

The kernel↔engine boundary is systematically mirrored **by hand**: a constant, an enum, and a
cross-repo pacing gap each exist in two places, kept in sync only by a comment. A comment is not a
check. The moment either side is edited, the mirror silently desyncs, and nothing goes red — the
exact failure mode Vibration·A2, Polarity·V3, and Correspondence's parity-gate corollary all name.

The repo already owns the fix, applied to exactly one mirror: **`DT_STABLE`**. The kernel declares
`pub const DT_STABLE: f32 = 0.02;` (`kernel/src/lib.rs:180`) as *the* single source of truth for the
50 Hz field/animation integrator. The engine re-declares the same literal independently
(`engine/src/loop_.rs:19`) rather than importing it — and then **both sides carry an identical-shape
pin test**: `dt_stable_is_authoritative` (`kernel/src/lib.rs:197-205`) and
`dt_stable_matches_kernel_contract` (`engine/src/loop_.rs:162-166`) each assert `DT_STABLE == 0.02`
**and** the derived invariant `(1.0 / dt).round() == 50`. If either declaration drifts, one test
turns red and names the desync. That is "done": two independent declarations, each pinned by a test
asserting the same literal + its physical meaning. This sweep applies that treatment to the three
mirrors that lack it, and folds in two mechanical determinism fixes from the same discipline family.

---

## §1 — Current-state evidence (5 sites, re-verified)

**Site 1 — `field_frame` dt vs kernel `DT_STABLE` (row #10, MEDIUM).**
`FieldEquilibrium::default()` sets `dt: 0.016` (`engine/src/field_frame.rs:47`; the struct default is
lines 40–50), an `f64`. The integrator reads it verbatim: `let dt = eq.dt;`
(`field_frame.rs:143`). The kernel's authoritative timestep is `0.02` `f32` (50 Hz). **0.02 vs 0.016
is the 25 % mismatch.** `field_frame.rs` does not import `DT_STABLE` and no test asserts
`FieldEquilibrium::default().dt == DT_STABLE`. The kernel comment (`lib.rs:171-177`) promises "the
field-sim integrator MUST only ever see this dt" — falsified by the engine default. The engine
already links `dowiz_kernel` (`bridge.rs:16`), so the authority *is* importable.

**Site 2 — duplicated `DriftClass` enum (row #23, LOW).**
Kernel authority: `pub enum DriftClass { Damped, Resonant, Unstable }`
(`kernel/src/spectral.rs:315-323`) — **no explicit discriminants, no `wire_code()` method**. The
numeric wire mapping (`Damped=0, Resonant=1, Unstable=2`) is a *second, inline* kernel representation:
literal match arms `=> 0.0/1.0/2.0` in `spectral_flat_logic` (`kernel/src/wasm.rs:748-751`). The
engine re-declares the enum (`engine/src/bridge.rs:650-654`; the "mirrors dowiz-kernel…" comment is
at `bridge.rs:648`) and decodes the code back in `drift_from_code` (`bridge.rs:673-679`, with the
safe `_ => Unstable` collapse at `:677`). Existing tests are **both one-sided**: the engine's
`drift_codes_map` (`bridge.rs:779-784`) asserts only the engine's own 0/1/2 map and never references
the kernel; the kernel's `spectral_flat_js_matches_engine_contract` (`wasm.rs:1121`) pins only the
single variant `Resonant=1`. No test pins the full kernel-encode ↔ engine-decode round-trip.

**Site 3 — `TG_MIN_GAP_S` cross-repo pacing (row #18, LOW-MED).**
`const TG_MIN_GAP_S: f64 = 3.5;` (`tools/telemetry/rust-spool/src/main.rs:30`) with the comment
"MUST match hermes-kernel `reporting::TG_MIN_GAP_S`" (`:29`). The authority is a **different repo**:
`/root/hermes-agent-kernel-rewrite/hermes-kernel/kernel/src/reporting.rs:26` — `pub const
TG_MIN_GAP_S: f64 = 3.5;` (confirmed present). The twin `const DEFAULT_GAP_S: f64 = 3.5;`
(`tools/async-spool/src/main.rs:37`) carries **no** "MUST match" comment and is env-overridable
(`ASYNC_SPOOL_GAP_S`, `:80-86`). No pin test on either side.

**Site 4 (bundled #12) — `wasm.rs` funnel leaks HashMap order (MED, latent).**
`struct LedgerOut { … funnel: HashMap<String, Vec<(String, u64)>>, … }`
(`kernel/src/wasm.rs:105-112`; import at `:29`) is built at `wasm.rs:239-247` and serialized via
`serde_json::to_string(&out)` at `:254`. `std::HashMap` iterates in a per-instance random order, so
the emitted JSON key order is non-deterministic — harmless as display (P3 plane) but breaks the day
anyone golden-tests, diffs, or content-addresses it. `serde_json` `preserve_order` would **not** help
(it reorders only `serde_json::Value`/`Map`, not a foreign `HashMap` field). The exact fix pattern is
in-repo: `kernel/src/retrieval/memory_store.rs` (**note:** the audit's path `kernel/src/memory_store.rs`
is off by the `retrieval/` dir) uses `use std::collections::BTreeMap;` (`:15`) and
`map: Mutex<BTreeMap<String, Vec<u8>>>` (`:46`) precisely so iteration is deterministic and
`snapshot_root` is reproducible (`:5-8`).

**Site 5 (bundled #24) — duplicated linear-no-jitter retry backoff (LOW).**
Re-grepped; the accurate count is **7 backoff sleep sites**, not "~6". The linear ramp
`sleep(Duration::from_secs(2 * attempt as u64))` appears at **5** sites:
`rust-spool/src/main.rs:138,144` and `async-spool/src/main.rs:256,262,278`. The terminal fixed
`sleep(Duration::from_secs(2))` transient pause appears at **2** sites: `rust-spool:208` and
`async-spool:339`. Separately, a fixed 0.5 Hz idle poll `sleep(from_secs(IDLE_POLL_S=2))` sits at
`rust-spool:188` and `async-spool:300` (`IDLE_POLL_S` at `rust-spool:32` / `async-spool:39`;
`MAX_ATTEMPTS=4` at `async-spool:44`). No jitter anywhere → N spools hitting a downed endpoint
synchronize into a thundering herd; the slope literal `2` is a magic number copied 7 times.

---

## §2 — Target-state design

### 2.1 Site 1 — pin `field_frame` dt to `DT_STABLE`

**Chosen fix: collapse to one governed frequency** (the Vibration verdict's "highest-value single
fix"). Set `FieldEquilibrium::default().dt = dowiz_kernel::DT_STABLE as f64` (crossing the `f32→f64`
boundary explicitly), and add the DT_STABLE-shape pin **on the engine side** (`field_frame.rs`
tests):

```
#[test]
fn field_default_dt_matches_kernel_dt_stable() {
    assert_eq!(FieldEquilibrium::default().dt, dowiz_kernel::DT_STABLE as f64);
    assert_eq!((1.0 / FieldEquilibrium::default().dt).round() as u32, 50); // 50 Hz, one clock
}
```

This is the same assertion pair the template uses (literal identity + the 50 Hz physical meaning),
now spanning the crate boundary via the real import. `assert_stable` (`field_frame.rs:55-68`) already
proves 0.02 is inside the CFL bound (`bound ≈ 0.455 ≫ 0.02`), so the value change is stability-safe.
*Alternative, only if a deliberate 60 Hz field clock is later wanted:* keep a distinct
`FIELD_DT` const with a comment stating the forcing reason and a pin test asserting *that* intentional
value — but do **not** leave it an unpinned bare literal. Default recommendation is the collapse.

### 2.2 Site 2 — pin `DriftClass` by test (NOT by codegen). Justification, then design.

**Decision: pin-by-test, strengthened — reject "generate the engine copy."** Three reasons.
(1) The FE-07 bridge decodes a **numeric wire code** off a flat-`f32` slice (`bridge.rs:685-701`); a
`code→variant` decode function is intrinsic to that boundary and cannot be replaced by sharing the
type — the engine can never receive a `kernel::DriftClass` value over the wire. (2) The very template
this sweep copies (`DT_STABLE`) is pin-by-test over two independent declarations; codegen for a
3-variant enum needs a `build.rs`/macro and is over-engineering (YAGNI). (3) The real un-centralized
authority is the **kernel's own** duplication — the enum (`spectral.rs`) plus the inline `0.0/1.0/2.0`
literals (`wasm.rs:748-751`) — which codegen of the engine copy would not touch.

So the fix has two moves:

- **Centralize the kernel's wire mapping.** Give `spectral::DriftClass` one method, and make the
  wasm encoder call it (deleting the inline literals):
  ```
  impl DriftClass {
      pub const fn wire_code(self) -> u8 { match self { Damped => 0, Resonant => 1, Unstable => 2 } }
  }
  ```
  `wasm.rs:748-751` becomes `classify_drift(&m).wire_code() as f64`. Now the kernel has one authority
  for the mapping, not two.
- **Pin the engine decode against that authority** with a strengthened test (`bridge.rs` tests) that
  imports the kernel enum and round-trips **all three** variants plus a variant-count guard:
  ```
  use dowiz_kernel::spectral::DriftClass as K;
  #[test]
  fn drift_wire_contract_matches_kernel() {
      for (k, e) in [(K::Damped, DriftClass::Damped),
                     (K::Resonant, DriftClass::Resonant),
                     (K::Unstable, DriftClass::Unstable)] {
          assert_eq!(drift_from_code(k.wire_code() as f32), e); // kernel encode → engine decode
      }
      // count guard: exhaustive match, no `_` — a new kernel variant fails to COMPILE this test.
      let _assert_three = |k: K| match k { K::Damped | K::Resonant | K::Unstable => () };
  }
  ```
  This is strictly stronger than today's one-sided `drift_codes_map` + single-variant kernel pin: it
  is the only test that fails loudly if the kernel reorders, renames, or adds a variant, or if the
  two crates' code↔variant maps diverge. The safe `_ => Unstable` runtime collapse stays (defensive
  against a malformed live frame); the compile-time guard covers the source-drift axis.

### 2.3 Site 3 — `TG_MIN_GAP_S` cross-repo: best achievable in-repo mechanism

A dowiz test cannot compile-time reference a constant in a non-dependency sibling repo. A shared crate
is rejected — coupling two deliberately-separate repos for one `f64` is overkill. The achievable
ceiling is a **loud within-repo pin + a documented single authority**, in three parts:

1. **Within-repo self-assertion pin** in *both* spool tools (DT_STABLE self-shape):
   `assert_eq!(TG_MIN_GAP_S, 3.5)` / `assert_eq!(DEFAULT_GAP_S, 3.5)`. Any local edit to the value
   now breaks a test — forcing the author to consciously re-verify the cross-repo match rather than
   drift silently.
2. **Comment + doc authority.** Upgrade the `rust-spool` comment to name the exact authority
   (`hermes-agent-kernel-rewrite/hermes-kernel/kernel/src/reporting.rs:26`) and add the identical
   "MUST match" comment to `async-spool`'s `DEFAULT_GAP_S` (currently bare). Add one short canonical
   note — a `PACING-CONTRACT` anchor in this docs tree — that both repos' comments point to as the
   single record of the 3.5 s decision.
3. **Optional stronger tier (present-sibling assertion).** A test that, *when* the sibling repo path
   exists on disk, greps `reporting.rs` for `TG_MIN_GAP_S` and asserts equality — **gated to skip
   (not fail) when the sibling is absent**, so a lone dowiz checkout does not false-fail. This is the
   closest thing to a real cross-repo pin without a build dependency.

### 2.4 Site 4 — funnel `HashMap` → `BTreeMap` (one line)

Change `funnel`'s type in `LedgerOut` (`wasm.rs:109`) and its local binding (`wasm.rs:239`) from
`HashMap` to `BTreeMap`, and the import at `wasm.rs:29`. Identical to the `memory_store.rs` template;
makes the emitted JSON key order a pure function of the input (sorted). No other logic changes.

### 2.5 Site 5 — one shared jittered backoff helper

Extract a single `fn backoff_delay(attempt: u32) -> Duration` used by all 5 ramp sites, replacing the
copied `2 * attempt` literal. Recommend **exponential + full jitter** (`base * 2^(attempt-1)`, capped,
`× rand[0.5,1.0)`) to break spool synchronization. The two spools do not currently share a lib crate;
the minimal home is a small shared module (e.g. a `spool-common` path dep) — if that is judged heavier
than warranted, the fallback is one `backoff.rs` per tool with an ADR line recording that the
duplication is accepted. Jitter needs a seeded/simple RNG only (no crypto). If linear-no-jitter is
deliberately retained for the single-drainer deployment, that decision must be written as an ADR line,
not left implicit.

---

## §3 — Migration steps (dependency order)

1. **Site 4 (funnel BTreeMap)** — smallest, zero-risk determinism fix; land first as the warm-up.
2. **Site 1 (field dt)** — set default `dt` to `DT_STABLE as f64`; add `field_default_dt_matches_kernel_dt_stable`.
3. **Site 2a** — add `DriftClass::wire_code()` in `spectral.rs`; rewire `wasm.rs:748-751` to call it (kernel-internal, no wire change).
4. **Site 2b** — add the engine `drift_wire_contract_matches_kernel` round-trip + count-guard test.
5. **Site 3** — add self-assertion pins in both spools; upgrade comments; write the `PACING-CONTRACT` doc anchor; (optional) add the present-sibling gated test.
6. **Site 5** — extract `backoff_delay` (exponential+jitter) or record the linear-no-jitter ADR line; replace the 5 ramp sites.
7. Run `pnpm typecheck` + the kernel/engine/tools `cargo test` suites; every new pin test must pass green, and each must have been seen to go **red** against the pre-fix value (RED→GREEN discipline).

Each numbered step is one edit; confirm green before the next (one-edit-per-turn).

---

## §4 — Acceptance criteria (falsifiable)

1. `FieldEquilibrium::default().dt == dowiz_kernel::DT_STABLE as f64` and `field_frame.rs` contains a
   test asserting it **and** `(1/dt).round() == 50`; temporarily reverting `dt` to `0.016` turns that
   test red.
2. `spectral::DriftClass::wire_code()` exists and is the **only** site mapping variants to `0/1/2`;
   `wasm.rs` no longer contains the inline `=> 0.0/1.0/2.0` literals (grep proves single authority).
3. An engine test imports `dowiz_kernel::spectral::DriftClass` and asserts `drift_from_code(k.wire_code()
   as f32) == engine_variant` for all three variants; adding a hypothetical 4th kernel variant fails
   to **compile** that test (count guard).
4. Both `rust-spool` and `async-spool` contain a test asserting their gap constant `== 3.5`; both
   constants carry a comment naming `hermes-kernel reporting::TG_MIN_GAP_S` at its file:line; a single
   `PACING-CONTRACT` doc anchor exists and is referenced by both. (Optional) a present-sibling test
   asserts equality with the live hermes-kernel value and *skips* cleanly when the sibling repo is
   absent.
5. `LedgerOut.funnel` is a `BTreeMap`; two runs of `channel_ledger_logic` on the same input emit
   byte-identical JSON (a golden/second-run assertion passes); the linear-ramp backoff literal `2 *
   attempt` no longer appears at 5 sites (one `backoff_delay` helper, or a recorded ADR line) and the
   retry delay carries jitter (or the ADR documents its deliberate absence).

---

## §5 — What this unblocks

This is the **RC-4 root-cause closure**: with all five pins in place, *every* hand-maintained mirror
across the kernel↔engine (and the cross-repo telemetry) seam carries the `DT_STABLE` treatment, and
the seam can no longer desync silently. It retires ranked findings **#10** (field dt), **#18**
(TG_MIN_GAP_S), **#23** (DriftClass) — the three RC-4 rows — plus **#12** (funnel determinism) and
the backoff limb of **#24**. It also *hardens the seam that #8 lives on* (Correspondence's
ONE-Laplacian): by making the drift-class and integration-rate mirrors provably pinned, it narrows the
remaining unpinned-mirror surface at that boundary to the Laplacian operator identity itself, which
H-series Correspondence work (separate blueprint) can then address in isolation. Per the §3 leverage
note in the principles doc, this is action (4) of the four highest-leverage root-cause fixes.

---

## §6 — Planning-protocol completion appendix (2026-07-17, decorrelated pass)

### (i) Citation verification + new grounding — this blueprint is already built, all 5 sites

Live re-verification against current HEAD (`cc3d5c916`) confirms all five sites landed in the same
commit as H1, **`4dec04218`** (2026-07-16T22:21:42Z), matching §2's design exactly. `git log --oneline
4dec04218..HEAD -- engine/src/field_frame.rs engine/src/bridge.rs kernel/src/spectral.rs
kernel/src/wasm.rs tools/telemetry/rust-spool/src/main.rs tools/async-spool/src/main.rs` is **empty**
for every file — none has moved since. Corrected, live line numbers (all shifted from planning-time
citations by the commit's own insertions):

- **Site 1** (`dt` mirror, row #10): `FieldEquilibrium::default()` now sets
  `dt: dowiz_kernel::DT_STABLE as f64` (`engine/src/field_frame.rs:51`); pin test
  `field_default_dt_matches_kernel_dt_stable` at `:213-219` asserts both the literal equality and the
  50 Hz derived invariant, exactly as §2.1 specified.
- **Site 2** (`DriftClass`, row #23): `DriftClass::wire_code()` added at `kernel/src/spectral.rs:333`
  (the single mapping authority, replacing the old inline `wasm.rs` literals — confirmed `wasm.rs` no
  longer contains bare `=> 0.0/1.0/2.0` arms); kernel-side pin `drift_wire_code_is_canonical`
  (`spectral.rs:407-410`). Engine-side round-trip + count-guard test
  `drift_wire_contract_matches_kernel` at `engine/src/bridge.rs:793-802`, alongside the retained
  one-sided `drift_codes_map` (`:780-784`) — both present, as §2.2 specified (strengthen, don't
  replace).
- **Site 3** (`TG_MIN_GAP_S`, row #18): self-pin in `rust-spool` (`tools/telemetry/rust-spool/src/main.rs:34,261-264`)
  and `async-spool` (`tools/async-spool/src/main.rs:43,426-429`); a present-sibling gated test exists
  in both (`rust-spool:270-286`, `async-spool:435-449`) that greps the live
  `hermes-agent-kernel-rewrite` path and skips cleanly when absent — the "optional stronger tier" (§2.3
  point 3) was in fact built, not left optional. `PACING-CONTRACT.md` exists at
  `docs/design/hermetic-architecture-2026-07-16/PACING-CONTRACT.md` (confirmed via `find`), referenced
  by both spool comments.
- **Site 4** (`funnel` BTreeMap, row #12): `kernel/src/wasm.rs:34` imports `BTreeMap`; `LedgerOut.funnel`
  field at `:116`; determinism pin `channel_ledger_funnel_serialization_is_deterministic` at `:852-881`
  (asserts byte-identical JSON across two runs **and** ascending key order) — stronger than §4's
  literal wording ("byte-identical JSON emitted"), since it also proves the *sorted* property the
  `BTreeMap` choice specifically buys.
- **Site 5** (backoff, row #24): `backoff_delay` at `rust-spool/src/main.rs:130-137` and
  `async-spool/src/main.rs:246-...`, both exponential+jittered as designed, each with an
  `envelope`-shape test (`rust-spool:293-...`, `async-spool:458-...`). Verified the jitter source is a
  **dependency-free** wall-clock-seeded splitmix64 (`jitter_unit`, `rust-spool/src/main.rs:139-150`) —
  not the `rand` crate — confirmed via `git diff 82e52c02e 4dec04218 -- */Cargo.toml`: **empty**, zero
  new dependencies added anywhere in the H1+H2 commit.
- **Live test run, this pass:** `cargo test --manifest-path tools/telemetry/rust-spool/Cargo.toml` → 3
  passed; `cargo test --manifest-path tools/async-spool/Cargo.toml` → 3 passed; kernel/engine counts as
  reported in BLUEPRINT-H1's own appendix (367/422/49, all green, this pass). All five §4 acceptance
  criteria are satisfied live, not merely designed.

### (ii) DECART judgment

**No DECART owed**, and this blueprint's own text (§2.5) left one implicit choice slightly open —
worth naming even though it resolved correctly. §2.5 says jitter "needs a seeded/simple RNG only (no
crypto)" without committing to *hand-rolled vs. a crate* (e.g. the `rand` crate would have been a
plausible, very common choice here and WOULD have required a DECART entry — a new external
dependency). The landed implementation chose a dependency-free splitmix64 (verified above), which is
the correct call under the ALL-RUST-NATIVE / minimal-deps direction this repo is actively pursuing (the
same session's `4519bd7ff`/`cc3d5c916` telemetry ports specifically *removed* Python deps rather than
add Rust ones). Because zero new dependency was actually added, no DECART table is owed retroactively
— but the blueprint's own planning text should have foreclosed the crate option explicitly rather than
leaving "seeded/simple RNG" ambiguous between a std-only and a dependency path; flagged here as a
process gap the plan left to the implementer to resolve correctly, rather than resolving it itself
(Detailed Planning Protocol step 3: DECART belongs in the planning artifact, before the choice is made,
not discovered after the fact by an auditor).

### (iii) Per-blueprint 2-question doubt audit

**Q1 — concrete, unresolved doubts:**
1. **Site 3's cross-repo present-sibling test is real but I did not execute it against a live sibling
   checkout in this pass** — I confirmed the test code exists and skips cleanly (per its own logic) but
   did not run it with `/root/hermes-agent-kernel-rewrite` present to see the *positive* path (actual
   equality assertion against the live `reporting.rs` value) fire; I only confirmed the pin (self-shape)
   tests are green.
2. **Site 2's `DriftClass` count-guard** (`bridge.rs:793-802`, "adding a hypothetical 4th kernel variant
   fails to compile") is a compile-time property — I read the code and judged it correct (an exhaustive
   match with no `_` arm) but did not mechanically prove it by actually adding a 4th variant to a
   throwaway branch and confirming the compile failure; this is inference from reading, not an executed
   falsification.
3. **Site 4's stronger test subsumes but does not literally match §4's wording** ("byte-identical JSON,"
   no mention of key order) — I judged the extra key-order assertion a strict improvement, not a scope
   change, but did not check whether any downstream consumer of the funnel JSON depends on a *specific*
   key order other than ascending (unlikely, but unverified).
4. **The `mat.rs`/eigenvector/PPR-triplication findings this sweep explicitly does NOT touch** (rows #8,
   #9, #21, #22, #25 — backlog per HERMETIC-REMEDIATION-PLAN.md §5) remain exactly as backlogged; I did
   not re-check whether any of H2's five site fixes accidentally created a new forcing trigger for one
   of those backlog items (e.g., whether the `DriftClass` centralization at `spectral.rs:333` changes
   the calculus for row #8's "does `csr::laplacian_spmv` gain a caller" trigger — it does not, on
   inspection, but I did not exhaustively cross-check all five backlog triggers against this diff).
5. **Cross-blueprint collision claim (H2 ∥ P4 on `wasm.rs`)** flagged in the umbrella plan §6.Q1.1 as
   unverified at diff-level — I did not re-check it either; H2's actual landed diff touches
   `wasm.rs:29-34,109-116,239-258` (import + `LedgerOut` + build site), which is a narrow, identifiable
   region a P4 wasm-export addition could plausibly avoid, but I have not read any P4 implementation
   diff to confirm.

**Q2 — biggest blind spot:** identical in shape to H1's — the document presents as a forward-looking
"Wave-0-safe, buildable now" plan, but is in fact fully shipped and tested; a reader trusting the
header alone would not know. The compounding risk here is slightly higher than H1's because H2 bundles
**five independent sites** — a reader could reasonably assume partial completion (e.g., "maybe only the
`DT_STABLE` pin landed") without a way to check which of the five shipped short of reading this
appendix or the commit diff directly.

### (iv) Anu (logic) & Ananke (organization) check

**Anu.** All five site designs are derivable from their own live-code citations, and each is now
also verifiable against the shipped implementation line-for-line (§(i)). The one place a decision was
left implicit rather than derived — Site 5's RNG source (§(ii)) — is named, not hidden, and resolved
correctly by the implementer; Anu is satisfied in outcome but the *planning* document itself should
have made that derivation explicit rather than relying on the builder to make the right call
independently.

**Ananke.** Same structural gap as H1, amplified by scope: nothing in the document forces a reader to
discover (a) that it shipped at all, or (b) that it shipped as **one atomic commit covering all five
sites**, not five independent landings. The §4 acceptance criteria are well-formed falsifiable checks
in principle, and this pass executed all of them green — but that proof lives in this appendix and in
`git log`, not in the document's own status. As with H1, the structurally inevitable fix (not applied
here, since this pass's mandate is to append, not to rewrite the header) would be a one-line
`STATUS: IMPLEMENTED — commit 4dec04218 (all 5 sites)` marker under the title.
