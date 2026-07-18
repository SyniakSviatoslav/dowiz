# BLUEPRINT — Pattern C′: the Contained Self-Certifying Bridge (CSC-LAW) (2026-07-17, round 2)

> **Status: design blueprint, no code, no commits.** Round-2 follow-up to
> `../R1-layout-versioning-bridges-grounding.md` (Patterns A′/B′) and
> `../BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md` (rows 1–3, §3.2 UT-LAW),
> reconciling the operator's ruling this turn with the session's RC-2 self-certification
> red-line. Every containment primitive cited below was live-read this pass (Provenance, §9).
>
> **The operator's ruling (verbatim):** *"yes validation happens at tge bridge - it is logical,
> yes adapter does self-certfies it is own work - because in terms of failure the adapter bridge
> is failing & not poisoning the other parts, fail-operational can be used but not for red line
> items like money, agree with everything else."*

---

## 0. The ruling, parsed precisely — what it does and does not reopen

Three clauses, read in the operator's own causal order:

1. **"validation happens at the bridge — it is logical."** Accepted. This was never the
   contested part: R1 §4.4 already adopts adapter self-validation as *good* (degrade-closed,
   one fewer bad tile emitted). The contested part was whether it is *sufficient*.
2. **"adapter does self-certify its own work — because in terms of failure the adapter bridge
   is failing & not poisoning the other parts."** The justification is **containment**, not
   trust. The operator is NOT saying "self-certification is trustworthy" (which would reopen
   RC-2 — the same gap P06's `key_V` split-identity verifier exists to close, the same reason
   WorkReceipt requires counterparty verification, the same watchdog-vs-inline toy proof in
   doc 19 §2.3). The operator is saying: *a failing bridge fails alone*. That claim is
   conditional — it is true **iff a specific containment mechanism actually exists in code**.
   This document designs that mechanism and then honestly tests whether it is complete.
3. **"fail-operational can be used but not for red line items like money."** This clause is
   load-bearing for the whole design: it supplies the tier boundary that makes clause 2 sound
   (see §5). It re-ratifies synthesis row 8 (REJECT critical-tier fail-operational) and row 9
   (ADOPT-WITH-CONSTRAINT telemetry-tier).

**"Agree with everything else"** ratifies the synthesis ledger as it stands — including row 2
(trusted proxy A′ = REJECT) and row 3 (untrusted translator B′ = ADOPT). So clause 2 is not a
reversal of the A′ rejection; it is a **third pattern** between them, admissible only where its
containment precondition is made structurally true. That pattern is defined next.

---

## 1. Pattern C′ defined — distinct from both A′ and B′

| | A′ trusted proxy (REJECTED) | B′ untrusted translator (ADOPTED, UT-LAW) | **C′ contained self-certifier (this doc)** |
|---|---|---|---|
| Who validates | adapter only | adapter (optional) **+ kernel inline gate (authority)** | adapter only (self-cert) **+ kernel structural gate + hard containment** |
| Kernel's reason to accept | the adapter's word | the kernel's own gate-passage | **the output cannot reach anything the kernel treats as authoritative** |
| Wrong translation becomes | committed kernel state (silent fail-open) | rejected iff gate-visible; **in-lane if gate-invisible** (see §5.3) | in-lane wrongness only, red-line unreachable **by construction** |
| Adapter death | silent (kernel keeps trusting) | loud (empty buffer → disable) | loud (trap/fault → `Failed` → disable) |
| Admissible for | nothing | any tier (gate cost permitting) | **fail-operational lanes only, never red-line** |

**Pattern C′, one sentence:** a bridge whose self-certification the kernel never *relies on*,
because the bridge's entire output universe is structurally confined to a lane that (i) contains
no red-line resource, and (ii) is consumed only by fail-operational consumers that already
tolerate wrong/stale/absent data — so the strongest possible lie the bridge can tell lands
somewhere it was already survivable.

The essential inversion vs A′: in A′ the kernel **trusts** the adapter's claim about its output;
in C′ the kernel is **indifferent** to it. Self-certification is retained purely as a local
quality mechanism (fewer bad tiles emitted, faster self-termination) — it carries **zero
authority delta**, which is what keeps C′ on the right side of the doc-19 axis-3 test.

---

## 2. The containment mechanism — three layers, each with its real in-code guarantee

The operator's clause "failing & not poisoning the other parts" decomposes into three distinct
poisoning channels. Each needs its own mechanism; prose covers none of them.

### 2.1 Layer 1 — spatial containment (memory / authority / compute)

**What the codebase actually provides today (live-read, honest tiering):**

| Tier | Mechanism | Real guarantee | Status |
|---|---|---|---|
| **WASM component** (default, the only tier C′ certifies today) | wasmtime + deny-by-default `Scope` → import mapping: `bebop2/wasm-host/src/lib.rs:139-211` (`instantiate`), `:25-33` (`HostError::ScopeViolation`), `:111` (`allowed_imports_for_scope`). Rejection happens **at instantiation, before the component runs** (`:8`, `:163-168`) — the empty-import gate. | Separate linear memory — the component **cannot name kernel memory at all**; its only effects are (a) explicitly granted host imports and (b) its return bytes. Enforced by the wasmtime linker refusing to link ungranted imports — "no hand-rolled lint, no trust" (`:13`). Compute bounded by the fuel seam (`kernel/src/ports/agent/admission.rs:56-59`, `FUEL_PER_UNIT`/`TRANCHE_UNITS` — **honest caveat: the constant is an explicit placeholder pending the B4 bench**; the *seam* exists, the calibration does not). Rate/budget bounded by the per-agent `TokenBucket` minted at admission (admission.rs step 5). | **Built + tested** (`wasm-host` `notify_scope_allows_only_notify_import` `:274-293`; deny-by-default matrix `:244`) |
| **Native process** | microVM tier, fail-closed KVM probe: `kernel/src/isolation/microvm.rs:20-28` (`SandboxTier::NativeProcessRequiresKvm`), `:52` (`kvm_available`), `:76` (`register_adapter`) — a host without KVM **refuses** the adapter rather than running it unsandboxed (test `r4_cannot_accept_native_adapter_without_kvm` `:156`). | Hardware address-space isolation — *when the VMM lands*. | **Probe only** — VMM launch is an explicit follow-up (`microvm.rs:14-16`). Today native adapters are simply refused on this host, which is the correct fail-closed posture. C′ cannot certify this tier until the VMM exists. |
| **Plain in-process Rust module** (`#![forbid(unsafe_code)]`) | Type safety + `catch_unwind` | **NOT a containment boundary.** Shared address space, ambient authority: a compromised transitive dependency inside the module holds the kernel's full power; `forbid(unsafe_code)` does not reach dependencies. | C′ self-certification is **never granted to this tier**. In-process shims stay under B′ (R1 §3.1 — output types as `NormalizedTile`, the inline gate runs anyway, translation is cheap so re-verification costs ~nothing). |

**Layer-1 verdict:** the operator's containment claim is *real* for exactly one tier today —
WASM component under the wasm-host deny-by-default gate — and *pending* for microVM. Any C′
deployment outside those tiers is A′ wearing C′'s name.

### 2.2 Layer 2 — structural containment: `Failed` is loud, "partial" is unrepresentable

**The crux question the task poses:** can "wrong translation" and "no translation" be made the
ONLY two outcomes, eliminating "plausible-wrong translation" as a category?

**Precise answer: the category splits, and only half of it can be eliminated.**

- **Structurally-plausible-wrong** (garbage that half-parses; truncated output; non-canonical
  encoding; out-of-bounds values; NaN payloads; wrong version tag) — **eliminated, fully**, by
  the construction below.
- **Semantically-plausible-wrong** (a byte-perfect, canonical, in-bounds, drift-passing encoding
  of the *wrong content* — e.g. two rows swapped, both valid `f64`) — **cannot be eliminated by
  any type or structural mechanism**, for an information-theoretic reason made explicit in §3.
  This half is what Layer 3 confines instead of catching.

**The construction (the type mechanism):**

```rust
/// The ONLY type through which bridge output enters the kernel. Sealed:
/// `Translated` has a private constructor reachable ONLY from the kernel-side
/// ingest gate below — an adapter cannot mint one, and no third variant exists.
pub enum BridgeResult<T: CanonicalOutput> {
    /// Fully decoded, kernel-canonicalized, bounds-checked, provenance-wrapped.
    Translated(Provenanced<T>),
    /// The bridge failed LOUDLY. Carries the reason; carries ZERO data bytes.
    Failed(BridgeFault),
}

/// Exhaustive fault enumeration — every arm is observable and every arm
/// feeds the same disable path (empty lane → kernel disables the adapter,
/// the R1 §4.2 / source-dialogue :63-64 fail-operational degrade).
pub enum BridgeFault {
    Trapped,                    // wasmtime trap: OOB, unreachable, div-by-zero
    FuelExhausted,              // compute bound hit (admission.rs fuel seam)
    ScopeViolation,             // ungranted import — caught at instantiation
    DecodeReject { offset: u32 },   // output bytes fail strict TLV decode
    NonCanonical,               // decoded but != NormalizedTile::canonicalize form
    BoundsReject,               // value-bound law: !is_finite, out-of-domain
    SelfTerminated { code: u32 },   // the adapter's own self-cert said no
}
```

**Why this makes "no partial / no silent" a theorem, not a review item:**

1. The adapter never returns `BridgeResult` — it returns raw bytes through the sandbox output
   channel. The **kernel-side ingest gate** constructs the value:
   `raw bytes → strict TLV decode (framing.rs discipline: unknown version → hard reject,
   `:54-60`) → NormalizedTile::canonicalize equivalence check → value-bounds
   (`is_finite`, the §2.1-synthesis law) → Translated`. Any deviation at any step → `Failed`.
   The gate is the same *shape* as `RetainedBase::admit` (`spectral_cache.rs:259-272`) — the
   canonicalize→bounds pipeline already in the tree.
2. The output channel is read **only after a clean sandbox return**; a trap mid-write delivers
   zero bytes (wasmtime semantics — the call returns `Err`, the host never observes a partial
   buffer as success). Fault ⇒ `Failed`, never truncated-bytes-as-data.
3. `Translated`'s constructor is private to the ingest module (sealed). There is no
   `TranslatedButWarned`, no `Degraded(T)`, no "use with caution" third state — the dialogue's
   `DATA_DEGRADED`-flag idea is deliberately NOT given a data-carrying variant here, because a
   data-carrying warning variant is exactly how plausible-wrong bytes leak downstream with a
   fig leaf attached.

So "the bridge fails" concretely = one of seven enumerated loud arms, each appending an
observable adapter-fault event and each feeding the disable path. The outcome universe is
exactly `{Translated(canonical), Failed(loud)}`. What survives inside `Translated` is only the
semantic half — handled next.

### 2.3 Layer 3 — authority containment: the lane, and red-line unreachability by construction

This is the layer that makes the operator's sentence true for the half the types cannot catch.
Mechanism, all pieces from the existing corpus:

1. **Provenance is unstrippable.** `Provenanced<T>` wraps the payload with
   `BridgeOrigin { adapter: NodeId, epoch, content_address }`. No `into_inner` that discards
   origin; consumers pattern-match through it. Every admitted translation is attributable
   forever (and the admission record / `AgentAdmitted` event in `event_log` — admission.rs
   step 7 — makes the blast-radius query "everything from adapter A since epoch E" answerable,
   which is containment-in-time: wrongness discovered later is excisable because it was never
   anonymous).
2. **The lane is capability-scoped, never self-assigned.** Which resources a bridge's output
   may touch = the adapter's granted `Scope`, deny-by-default
   (`allowed_imports_for_scope`), and the tier flag is checked against capability scope per
   register #15 / synthesis §3.1 — an adapter cannot declare "I'm telemetry." **Even in C′,
   the self-certification never extends to lane assignment** — the grant does that.
3. **Red-line resources are un-nameable in any bridge scope.** A const enumeration
   `BRIDGE_GRANTABLE_RESOURCES` (the `B1_NEW_SCOPES` exhaustive-array pattern,
   admission.rs:67-73, whose test iterates the array so an unlisted addition fails) that
   **excludes** `Resource::Ledger`, `Resource::Auth`, `Resource::Secret`, `Resource::Migration`
   (discriminants live at `ports/agent/scope.rs:60-69`). A grant request naming an excluded
   resource is refused at admission — before instantiation, before any byte flows.
4. **The type system closes the last door.** A sealed marker trait `RedLineAdmissible`
   implemented for direct/gate-verified inputs and **deliberately not implemented for
   `Provenanced<T>`** — so `commit_after_decide` on the money/command path
   (`event_log.rs:419` drift-gate variant and its siblings) structurally cannot accept
   bridge-origin data. Mirrors the SH-3 `&`-vs-`&mut` structural guard (admission.rs header)
   and the synthesis §3.1 compile-fail discipline (`interpolate_stale()` absent on `Critical`).

With Layers 1+3 both real, the operator's justification holds **as stated**: a failing —
or lying — bridge damages only its own lane, and its own lane is precisely the thing that
would be empty or stale without the bridge anyway. That is what "failing, not poisoning" means
in code.

---

## 3. The crux, stated as the theorem it is: why semantic wrongness cannot be typed away

A translation is `f: LegacyBytes → Tile`. "Wrong" means `output ≠ g(input)` for the ideal
translation `g`. To *detect* wrongness, some party must evaluate a relation
`R(input, output) ⟺ output = g(input)`. The kernel, **by design, does not know the legacy
layout** — removing that knowledge from the kernel is the entire point of the side-car. So the
kernel cannot evaluate `R`: from its viewpoint the bridge is a *source of new information*, and
**information you cannot independently derive, you cannot verify — only bound**. Any complete
in-kernel checker of `R` is a re-implementation of `g` (collapsing C′ back into "no bridge
needed"). This is not an engineering shortfall; it is the same theorem underneath `key_V`:
self-certification is exactly as trustworthy as the certifier, and containment bounds the
certifier's **reach**, never its **honesty**.

**The one construction that genuinely closes the semantic gap — the round-trip witness
(DEFER-WITH-TRIGGER):** for *bijective* layout translations (the actual use case — re-layout,
rename, reorder of the same information), the kernel can check `R` without knowing legacy
semantics: run the **pinned legacy encoder** `B` (the *old version's own codec*, a frozen,
content-addressed blob whose hash is checkable against the legacy epoch's signed release — not
new code) in a second sandbox, and verify `B(f(x)) == x` **byte-exact — a memcmp, fully
kernel-computable**. If `B` is the true inverse, equality forces `f(x)` to be the unique
preimage, i.e. correct; a colluding wrong output would need `B∘f` to fix the wrong point, which
a frozen pre-existing binary cannot be retrofitted to do. Cost: 2× translation + memcmp.
Boolean, source-independent, degrade-closed on mismatch — NOT the rejected #33 scoring class.
Inapplicable to lossy translations (no inverse exists; only incomplete invariant-projection
checks remain there). **Trigger:** enable per-lane whenever a bijective bridge lane feeds any
consumer above pure telemetry. (N-version translation — two independent translators, byte-exact
compare, mismatch ⇒ `Failed` — is the lossy-case fallback: also boolean and admissible, but 2×
cost plus a second codebase; DEFER behind the same trigger.)

---

## 4. Honest tension check — does C′ close RC-2?

**Split the RC-2 concern into its two precise formulations; the answer differs.**

- **RC-2-narrow (the formulation this session's red-line actually guards):** *self-certification
  must never be the reason bad state becomes accepted KERNEL-AUTHORITATIVE state.*
  **CLOSED — fully**, by construction, not by review: bridge-origin data is never
  kernel-authoritative. It cannot reach red-line resources (Layer 3 items 3–4: un-nameable in
  scope + un-typeable at commit), cannot corrupt kernel memory (Layer 1), cannot arrive as
  silent-partial (Layer 2). The strongest lie a compromised adapter can tell is a canonical,
  bounds-passing tile *in its own lane* — state the kernel never treats as authoritative and
  fail-operational consumers already survive being wrong.
- **RC-2-broad (*no consumer ever acts on unverified wrong data*):** **NOT closed, and no
  containment mechanism can close it** (§3 theorem). A semantically-wrong, well-formed
  translation IS accepted into its lane. Containment converts "kernel poisoning" into "bounded
  in-lane wrongness" — but in-lane wrongness is still wrongness: a consumer reading that lane
  sees wrong telemetry until the witness (§3), staleness, or cross-checks surface it.
  **Residual gap, stated plainly:** C′ guarantees *"wrong data stays in its lane,"* never
  *"no wrong data."* The Layer-2 gate catches STRUCTURAL failure only; SEMANTIC correctness of
  anything money-adjacent — or of any lane whose consumers act irreversibly — still needs
  independent re-verification (B′ gate where gate-visible; round-trip witness where bijective;
  refusal where neither exists).

**Consequence for the operator's ruling:** it is **sound exactly as scoped**. Clause 3 ("not
for red-line items like money") is not a polite carve-out — it is the *load-bearing premise*
that makes clause 2 true. Delete clause 3 and clause 2 becomes false: self-certification with
containment but *without* the red-line exclusion is A′ with extra steps. The ruling is
internally consistent only as the package the operator stated, and this blueprint keeps it so
structurally (the exclusion is enforced by types and const enumerations, not by remembering).

**Relation to key_V / WorkReceipt doctrine — no reopening:** `key_V` and WorkReceipt protect
exactly the class C′ never touches (done-gates, settlements, kernel-authoritative facts —
red-line by definition, hence B′/counterparty-verified forever). C′ carves out the complementary
class: lanes where no counterparty *can* verify (the kernel lacks legacy semantics in
principle), and compensates with reach-limitation instead of verification. The doctrine line:
**verify where verification is possible; contain where it is impossible; refuse where neither
holds.**

---

## 5. Reconciliation with B′/UT-LAW — specialization by consumer tier, not replacement

### 5.1 The ruling

**C′ does not replace B′.** They share Layers 1–2 (sandbox + structural gate + loud faults +
provenance) and diverge only at the semantic layer. Selection is by **consumer tier**, and the
operator's clause 3 supplies the boundary:

| Lane / consumer | Pattern | Why |
|---|---|---|
| Red-line: money, auth, capability, revocation, the `event_log` command path | **B′ mandatory, C′ inadmissible** | Forced by clause 3 + money RED-LINE + register #4; also nearly free — the inline gate (`commit_after_decide_drift_gate`, `RetainedBase::admit`) already exists and already runs on all input |
| Gate-visible properties on any lane (drift bounds, canonicality, value bounds, capability chains) | **B′** (the gate runs regardless) | Re-verification the kernel can compute costs ~nothing; never waive what is free |
| Fail-operational lanes (telemetry-class, legacy-compat) where faithfulness is **uncheckable in principle** by the kernel | **C′** | Re-verification is not expensive here — it is *impossible* (§3); containment is the only available control, and the lane's consumers tolerate wrong/stale/absent by definition |
| Bijective bridge lane feeding anything above pure telemetry | **C′ + round-trip witness** | The one full-closure mechanism (§3), boolean and degrade-closed |

So: **C′ = B′ minus semantic-reverify plus lane-confinement**, admissible only below the
red-line boundary — a specialization for the tier where B′'s extra step has nothing to grab.

### 5.2 What actually gets built

One substrate, one flag: the sandbox tiers, `BridgeResult`/`Provenanced`, the fault enum, the
grantable-resource const, and the sealed `RedLineAdmissible` trait are **common** to both
patterns. B′ lanes additionally route `Translated` payloads through the existing inline gate
before commit (already-built machinery). C′ lanes stop at ingest. There is no separate C′
codepath to maintain — C′ is B′'s substrate with the (impossible-anyway) semantic step
honestly absent instead of pretended.

### 5.3 An honest note that sharpens R1, not softens it

R1 §4.3's B′ guarantee was precisely scoped: the side-car *"can never cause a tile the kernel
would reject to be accepted."* True — for **gate-visible** properties. A semantically-wrong
translation is a tile the kernel would *not* reject, so **B′ never claimed translation
faithfulness either**; the semantic residual of §4 is not a gap C′ opens relative to B′ — it is
a gap B′ silently carried and C′ makes explicit, bounded (Layer 3), and executable-pinned
(§7 T3). That is the honest net effect of this round: not a weakening of UT-LAW, but the first
statement of its exact semantic ceiling, plus the only two mechanisms that raise it (witness,
N-version) with triggers.

---

## 6. Name, law, predefined types

- **Name:** **Pattern C′ — the Contained Self-Certifying Bridge.**
- **Law:** **CSC-LAW (Contained-Self-Certification Law):** *self-certification is admissible
  exactly and only where structural containment makes the certifier's strongest possible lie
  land inside a lane that (i) contains no red-line resource by construction — un-nameable in
  scope, un-typeable at commit — and (ii) is consumed solely by fail-operational consumers;
  everywhere else UT-LAW (B′) governs, and the self-certification carries zero authority
  delta in all cases.*
- **Predefined types (contract §2.4, named before implementation):** `BridgeResult<T>`
  (§2.2 — sealed, two variants, no data-carrying third state), `BridgeFault` (seven loud arms),
  `Provenanced<T>` + `BridgeOrigin` (unstrippable), `BRIDGE_GRANTABLE_RESOURCES` (const
  exhaustive array excluding red-line discriminants), sealed trait `RedLineAdmissible`
  (never implemented for `Provenanced<T>`), `SandboxTier` (existing,
  `isolation/microvm.rs:20`).

---

## 7. DoD — the adversarial falsifiable tests

The designed adversary: **a deliberately malicious WASM adapter** that (a) requests an
ungranted import, (b) attempts unbounded compute, (c) traps mid-output, (d) returns a
**well-formed, canonical, in-bounds, drift-passing tile that is NOT the translation of its
input** (two rows swapped). Tests, RED-first where a hole is being proven:

| # | Test | Proves | Expected |
|---|---|---|---|
| T1 | `evil_adapter_ungranted_import_refused` | Layer 1: escape attempt dies **at instantiation** | extends the existing `notify_scope_allows_only_notify_import` (`wasm-host:274-293`); `ScopeViolation`, component never runs |
| T2 | `bridge_fault_is_loud_never_partial` | Layer 2: property/fuzz over trap points, fuel exhaustion, truncation — outcome ∈ {`Translated`, `Failed`} for every injection; zero bytes ever delivered on any fault arm | no third state representable; fuzz finds no partial |
| T3 | `red_line_unreachable_from_bridge_provenance` | Layer 3: trybuild **compile-fail** — `Provenanced<T>` into the money/command commit path does not compile (sealed trait); plus runtime: a grant request naming `Resource::Ledger` is refused at admission | RED form: temporarily add the impl → the wrong tile commits (proving the trait is load-bearing); GREEN: impl absent, compile-fail is permanent |
| **T4** | `residual_semantic_gap_pinned` | **The honest one.** The malicious well-formed wrong tile from (d) **IS accepted into its own lane** — the test *asserts acceptance*, permanently pinning §4's residual as executable documentation | PASSES by asserting the gap exists; if anyone later closes it, this test flips and forces this document to be revised — the residual can never silently rot into a forgotten assumption |
| T5 | `round_trip_witness_closes_bijective_case` | §3's closure mechanism | RED: witness off → T4's wrong tile accepted in-lane; GREEN: witness on (pinned, content-address-checked legacy encoder) → `Failed` on memcmp mismatch |
| T6 | `csc_never_granted_to_inprocess_tier` | §2.1 tiering | an in-process (non-sandboxed) translator registering for a C′ lane is refused; only `SandboxTier::WasmComponent` (and, post-VMM, KVM-backed native) qualify |

**The containment claim is falsified iff** T1–T3 can be made to fail by any adapter behavior —
i.e. iff any injection produces kernel-memory effect, silent-partial data, or red-line reach.
T4 is deliberately *not* a containment test: it is the executable admission that containment
was never a semantic-correctness mechanism.

---

## 8. Exists-today vs to-build ledger

| Piece | Status | Where |
|---|---|---|
| Deny-by-default WASM import gate (empty-import) | **Built + tested** | `bebop2/wasm-host/src/lib.rs:111,139-211,244-293` |
| Fail-closed sandbox tiering + KVM probe | **Built (probe); VMM launch = follow-up** | `kernel/src/isolation/microvm.rs:20-76,156` |
| Fuel/budget seam | **Seam built; constant is an explicit placeholder pending B4 bench** | `admission.rs:56-59` + per-agent `TokenBucket` |
| Inline gate shape (canonicalize→drift→bounds→address) | **Built** | `spectral_cache.rs:259-272`; `event_log.rs:419` |
| Exhaustive-scope-array pattern; SH-3 structural guard; never-self-assigned tier | **Built (patterns to mirror)** | `admission.rs:67-73`, header; register #15 |
| `BridgeResult` / `BridgeFault` / `Provenanced` / sealed `RedLineAdmissible` / `BRIDGE_GRANTABLE_RESOURCES` | **NEW — this blueprint** | Layer B (gate substrate) + Layer D (scope law), per synthesis §6 routing |
| Round-trip witness (pinned legacy encoder, memcmp) | **NEW — DEFER-WITH-TRIGGER** (§3) | trigger: bijective bridge lane above pure telemetry |
| T1–T6 | **NEW** | T3/T4 are the two that must never be skipped |

---

## 9. Provenance

Live reads this pass: `bebop2/wasm-host/src/lib.rs` (`:1-59,98-115,130-211,244-293`),
`dowiz/kernel/src/isolation/microvm.rs` (`:1-76,154-158`) + `isolation/mod.rs`,
`dowiz/kernel/src/ports/agent/admission.rs` (`:1-90,865`), `ports/agent/scope.rs` (`:16-83`),
`dowiz/kernel/src/spectral_cache.rs` (`:20-21,100-172,208-281`), `dowiz/kernel/src/event_log.rs`
(`:405-434`), `dowiz/kernel/src/wasm.rs` (`:1-40`, confirmed wasm-bindgen glue, not a sandbox).
Docs read in full: `../R1-layout-versioning-bridges-grounding.md`,
`../BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md`, `../00-SOURCE-DIALOGUE.md`.
The operator ruling quoted verbatim from this turn. No code written; no commits.
