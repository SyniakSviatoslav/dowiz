# BLUEPRINT — Item 65: Typed In-Process AI/Agent Capability Boundary

- **Date:** 2026-07-19 · **Tier:** proportionate seL4 slice (roadmap §K) · **Status:** BLUEPRINT
  (planning artifact, no code) — **GATED**: builds only after items 64 + 45 (see §6).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 65
  (lines 1121–1137) + item 63 (the item-45 disposition table it extends, lines 1081–1100) + the §K
  dependency line (1250 `65 after {64 + 45}`); `docs/audits/hardening/CHECKLIST.md`. Ground truth for
  code citations: this worktree at HEAD `6701bbb6f`.
- **Upstream:** item 64 (composition root — SOLE token minter); item 45 (the AI-optional
  compile-firewall gate, spec-level, no blueprint yet); the compile firewall it hardens
  (`agent-facade`/`agent-loop`/`agent-adapters` path-dep structure); red-line policy
  (`kernel/src/ports/agent/scope.rs:257`); attenuation machinery
  (`kernel/src/capability_cert.rs:227`).
- **Downstream:** item 73 (§L Gate-Root Invariant — reuses "no capability type granting write access
  to the root EXISTS in the type system," the meta-level of this item's construction).

---

## 0. What this closes that item 45 does not — read this first

The compile firewall (root CLAUDE.md; verified live below) already makes cross-references
*illegal to name*: `agent-loop` imports only `agent-facade`, which does not re-export kernel mutation
symbols, so `agent-loop` **structurally cannot name** `decide`/`fold`/stores. Item 45 formalizes that
as a CI gate. That is a **compile-time** guarantee about what code *references*.

Item 65 adds the **runtime-authority** guarantee that a compiled-in-but-untrusted path — one that
*did* end up linked against a kernel port, e.g. through `agent-facade` itself (the one agent crate
that legitimately imports `dowiz-kernel`, `agent-facade/Cargo.toml:12`) — still cannot *exercise*
core-write authority unless it was handed an unforgeable token. Roadmap line 1128: "45 stops
cross-references at compile time; this also stops runtime authority a compiled-in-but-untrusted path
might exercise." The two are complementary; neither subsumes the other.

The proportion is honest (line 1123): item 45 + the Wasmtime-fuel pattern already scoped ~70% of the
seL4 slice; this is the remaining ~30% — a capability *token* at the call site, not a new
capability *system*.

---

## 1. Scope / goal

A **zero-sized, unforgeable capability type** — constructible only by item 64's composition root —
that the AI/agent subsystem must present **by signature** to call a kernel port that touches the
deterministic core:

```
// illustrative signature shape (NOT production code — planning only)
fn append_ledger(cap: &CoreWriteCapability, entry: LedgerEntry) -> Result<...>
```

`cap: &CoreWriteCapability` makes "authority to touch the deterministic core" **illegal-state-
unrepresentable at the call site**: code never handed the token cannot construct one, so the call
does not type-check. Strictly additive over item 45.

Plus the **OTP-slice companion**: a *uniform* per-port fail-closed containment property test —
one failing/panicking adapter cannot escalate past its own port boundary — asserted across **every**
`ports/` seam rather than left to per-port convention. It composes with item 9's circuit breaker
(roadmap §D) as the containment receiver.

**Non-goals.** No new crypto, no new dependency, no invented memory-capability system (reuse
`capability_cert`'s attenuation/scoping internally — line 1129). Not a sandbox/isolation mechanism
(that is `isolation/microvm.rs` + the Wasmtime-fuel pattern). Not a replacement for item 45's gate.

## 2. Verified current state (grounded)

| Fact | Citation (live HEAD) |
|---|---|
| `agent-facade` is the ONLY agent crate importing the kernel | `agent-facade/Cargo.toml:6,12` (`dowiz-kernel = { path = "../kernel" }`; doc: "the ONLY crate in the agent lane that imports dowiz-kernel … mutation symbols are not re-exported") |
| `agent-loop` cannot name the kernel directly (firewall) | `agent-loop/Cargo.toml:11-16` (depends on `agent-facade` + `llm-adapters` only; "must NEVER list dowiz-kernel directly") |
| Closed resource/action namespace + red-line classification | `kernel/src/ports/agent/scope.rs:29` (`Resource`), `:107` (`Action`), `:96`/`:176` (`is_red_line`), `:247` (`touches_red_line`) |
| Deny-by-default red-line policy already armed | `kernel/src/ports/agent/scope.rs:257` (`RedLinePolicy::DenyByDefault`), `:267` (`check`) |
| Attenuation / delegation machinery to reuse internally | `kernel/src/capability_cert.rs:227` (`mint`), `:368` (`CertDelegation`, subset-scope), `:382` (`may_delegate`) |
| The core-write red lines the token guards | `scope.rs:99` (`Ledger`/`Auth`/`Secret`/`Migration` resources), `:179` (settlement/authenticate/deploy-secret/run-migration actions) |

## 3. Implementation plan (numbered)

Lives in a NEW kernel module (proposed `kernel/src/ports/agent/capability.rs`), always-compiled but
inert without a minted token.

1. **Define the zero-sized token type.** `pub struct CoreWriteCapability(());` — a private-field
   unit struct: no `pub` constructor, `#[non_exhaustive]`-equivalent by the private field, zero
   runtime footprint. Its ONLY constructor is a `pub(crate)` `fn` reachable only from `compose/` (item
   64 §3 step 6). Nothing outside the composition root can `CoreWriteCapability { .. }` because the
   field is private to the defining module and the mint fn is `pub(crate)`-scoped there.
2. **Thread the token into the red-line port methods by signature.** Every kernel port method whose
   `(Resource, Action)` is red-line per `scope.rs` (`Ledger`/`Auth`/`Secret`/`Migration` × their
   verbs) takes `cap: &CoreWriteCapability` as a leading parameter. The type is the authority; the
   existing `RedLinePolicy::check` (`scope.rs:267`) stays as the *value*-level deny-by-default check —
   the token adds a *type*-level uncallable-without-authority guarantee on top (defense in depth: one
   fails closed at runtime, the other fails closed at compile time).
3. **Delegate attenuated tokens, don't clone authority.** Where the agent subsystem legitimately
   needs a *narrowed* authority (e.g. a menu-read-only sub-agent), the root hands a token whose scope
   is attenuated using the existing `CertDelegation` subset-scope model (`capability_cert.rs:368`,
   `may_delegate=false` for leaf sub-agents) — no new attenuation logic, no crypto invented.
4. **Build the OTP-slice per-port containment property test.** A single generic property test that,
   for each `ports/` seam, drives a deliberately failing/panicking adapter and asserts the failure is
   contained at that port's boundary — it returns a typed error / trips the item-9 breaker, and does
   NOT escalate to a neighboring port or the core. Asserted across every port uniformly (a table of
   port seams, one property, not per-port bespoke tests).
5. **Wire the containment receiver to item 9's breaker.** A contained port failure feeds the breaker
   as its open-circuit signal; the property test asserts the breaker receives it. (Item 9 breaker is
   roadmap §D — if not yet landed when this builds, the receiver is a trait seam and the assertion is
   deferred with a ledgered `gap:` row, never silently dropped.)

## 4. Required tests / proofs (CHECKLIST.md 5-point mapping)

Capability tokens are zero-sized type-system constructs with no runtime timing surface — the honest
mapping:

- **Item 1 (oracle).** The per-port containment property test (§3 step 4) IS the oracle: uniform,
  exhaustive over the `ports/` seam table, differentially checking "failure contained ⟺ no cross-port
  escalation." Plus a **compile-fail test** (the load-bearing proof): a red-line port method is
  **uncallable** from code never handed the token — `trybuild`/`compile_fail` doctest that must fail
  to compile. This is the "illegal-state-unrepresentable" guarantee, mechanically demonstrated.
- **Item 5 (formal/type proof).** The compile-fail test above is the type-level proof; additionally a
  **grep + visibility proof** that the token's only constructor site is the composition root (the
  `pub(crate)` mint fn appears in exactly one module — grep-checkable, and the private field makes any
  other construction a compile error).
- **Item 2 (dudect):** **N/A(not-a-timing-path)** — a zero-sized token carries no secret and no
  branch; nothing to time. Recorded, not faked.
- **Item 3 (debug cross-check):** **N/A(no-per-call-numeric-reference)** — there is no numeric oracle
  to `debug_assert_eq!` against; the guarantee is structural (type + visibility), not arithmetic.
  Manifest records `N/A(structural-guarantee)` rather than faking a cross-check.
- **Item 4 (asm spot-check):** **N/A(no-branch-free-crypto)**.

Zero-dep invariant: `cd kernel && cargo tree -e no-dev` byte-unchanged (line 1136). `trybuild` (if
used for the compile-fail test) is a **dev-dependency** and must stay out of the `no-dev` graph —
verify with the same `cargo tree -e no-dev | grep -c trybuild` = 0 discipline as the serde check.

## 5. Falsifiable acceptance criteria

1. A red-line port method is uncallable without a `&CoreWriteCapability` — the compile-fail test fails
   to compile as designed. **Falsifier:** the call type-checks without a token → FAIL.
2. `CoreWriteCapability` has exactly one constructor site, inside `compose/`; grep + the private-field
   visibility confirm no other construction is possible. **Falsifier:** a second constructor, or a
   `pub` field, exists → FAIL.
3. The per-port containment property test is green across **every** `ports/` seam (a new port added
   without a containment row goes RED — the anti-forgery floor, item-6 idiom). **Falsifier:** a port
   escalates a failure past its boundary, or a seam has no containment assertion → FAIL.
4. `cargo tree -e no-dev` byte-unchanged; `trybuild`/test-only deps stay out of the runtime graph.
   **Falsifier:** any runtime dependency added → FAIL.

## 6. Dependency gates (honest)

- **Double-gated: builds only after {item 64 + item 45}.** Item 64 is the SOLE minter — without its
  composition root there is no legal construction site for the token, so this item is inert until 64
  lands. **Item 45 has no blueprint yet** (spec-level in the roadmap, extended by item 63's disposition
  table); the roadmap sequences item 65 explicitly "after {64 + 45}" (line 1250). This item therefore
  **cannot be dispatched now** — it waits on both.
- **Interlocks with item 9** (breaker as containment receiver, roadmap §D). If item 9 has not landed,
  the receiver is a trait seam and the breaker assertion is a ledgered `gap:` row (not a blocker for
  the token itself, only for the §3 step 5 wiring).
- **Feeds item 73** (§L): the meta-level "no capability granting write access to the governance root
  EXISTS in the type system" is exactly this item's private-field-unit-struct construction applied one
  level up.

## 7. Operator-decision points (flagged)

- **Blast radius of the signature change.** Threading `cap: &CoreWriteCapability` into every red-line
  port method is a mechanical but wide signature edit. Whether to do it in one sweep or incrementally
  per port (each behind a ledgered row) is an operator sequencing call, not an engineering default.
  Recommend: land the type + minter + one exemplar port first (ledger), prove the compile-fail gate,
  then fan out.
- **`trybuild` vs hand-rolled compile-fail check.** A `compile_fail` doctest needs no dep;
  `trybuild` is ergonomic but adds a dev-dependency. Recommend the doctest form to keep even the dev
  graph lean, but flag for the operator/executor to rule.
