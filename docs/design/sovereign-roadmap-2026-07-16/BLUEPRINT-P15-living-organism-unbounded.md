# BLUEPRINT — Phase 15: LIVING ORGANISM UNBOUNDED (general self-mod, sub-hubs, model autonomy)

> **Anchors:** M11, E17, E18, F3, F4, F7, F9, F10, F27.
> **Depends on:** **Phase 10 (HARD)** — the single mandated bound, the operator kill-switch, and the
> `HubPolicy`-as-data runtime entity must both exist before "unbounded" is safe to build; **Phase 5**
> (routing organism — the `deliberate()`/jury tier and per-agent spend budget this phase gates on);
> **Phase 9** (mesh — the wire-format frame kinds, ML-KEM session encryption, and signed-announcement
> path this phase rides). **Additionally:** E13's *execution* (not its design) is gated on an external
> GPU-unlock trigger (O18: `cargo add wgpu` / network), which is **not** in this phase's control —
> this phase ships the E13 blueprint; Phase 17 owns the trigger-gated activation.
> **Parallel-safe with:** Phase 14, Phase 16.
> Canon: `ARCHITECTURE.md` §0 (M5/M9/M10/M11 + SCOPE RULE), §6 (F1–F10, F27), §8.
> Primary evidence: `R1-B-hub-autonomy-agent-infra-gap-analysis.md` §M5/M11/F1–F10/E17/E18 (file:line,
> code-verified this session against `bebop2/core/src/self_mod.rs`, `self_mod_loop.rs`, `proto-cap/src/scope.rs`).
>
> **Planning artifact only. No code is written or edited by this document.** It designs the phase; the
> build lands under the Phase-1 CI floor, the Phase-6 verifier, and the Phase-15 done-tests in §10.

---

## 1. M11 is DEPENDENT, not independent (the ordering argument — the point of this blueprint)

The most important claim in this blueprint is a sequencing constraint, and it is drawn from canon's own
text. M11 reads: *"Living-organism experiment = UNBOUNDED: no caps, only kill-switch + configurable
access"* (`ARCHITECTURE.md:20`). Read precisely, **"unbounded" is a conditional, not an absolute.** The
sentence names exactly one bound — the kill-switch — and makes the whole guarantee ride on it. F50
restates the same shape: *"hubs freely evolve, only kill-switch bounds"* (`ARCHITECTURE.md:119`). M9
makes it a hard law: *"Kill-switch + flexible access ONLY … No other global control exists"*
(`ARCHITECTURE.md:18`).

The kill-switch **does not exist yet.** R1-B §M9 verified it directly: grep for
`kill-switch|kill_switch|killswitch` across bebop2 returns zero hits; the only way to stop a runaway hub
today is OS-level `systemctl kill`; the legacy `guard.rs` KillSwitch is a ≥2/3 *consensus* registry —
the **opposite** of M9's unilateral operator kill. Building the operator kill verb (anchor-signed frame →
COLD-backup-then-halt, restartable) is **Phase 10's** job, not this phase's.

Therefore the ordering is not a preference; it is entailed by the architecture's own wording:

> **You cannot build the thing that is "bounded only by the kill-switch" before the kill-switch exists.**
> Building any self-generalizing organism machinery while its sole permitted bound is missing produces an
> organism that is bounded by *nothing* — which is not what M11 says, it is the failure mode M11's own
> clause was written to exclude.

This inverts the naive reading of M11 as the roadmap's "freedom capstone." M11 is a *dependent* anchor:
it is satisfied exactly when (a) Phase 10's kill-switch is live, (b) Phase 10's `HubPolicy`-as-data entity
exists so there is something concrete to be "unbounded about," and (c) each F-line inside F1–F10/F27 has
its own **LOCK-qualifier** built. Canon never says "guardless" — every F-line carries a qualifier
(`+ sha3-verify-or-deny`, `+ max-depth-cap`, `+ eqc-gate-or-deny`, `+ proto-cap-sign`). Those qualifiers
**are** the build list for this phase. "Unbounded EXCEPT red-lines and the kill-switch" is the honest,
narrower claim the architecture actually supports; this blueprint builds that claim, not a looser one.

**Hard gate, stated once and enforced everywhere below:** no item in §3–§7 may be built, even partially,
before Phase 10 lands (kill-switch + `HubPolicy` entity), Phase 5 lands (the `deliberate()`/jury routing
tier + per-agent budget), and Phase 9 lands (wire-format frame kinds + signed announcement + ML-KEM). The
dependency graph (`R2:105`) records this as `P15 ← P10(hard), P5, P9 [+GPU-unlock for E13-exec]`.

---

## 2. Current-state evidence, gap-by-gap (code-verified)

Each item is the *narrowness* of what exists today versus the M5/M11 target. All file:lines re-read this
session unless marked (R1-B), which are code-verified there.

- **Self-mod is one scalar wide.** `bebop2/core/src/self_mod.rs` is the whole self-modification effector.
  It mutates exactly one parameter: a Kalman filter's `q_scaler` via `filter.set_q_scaler(...)`
  (`self_mod.rs:190`; the docstring at `:15-18` says it "mutates an *in-memory* `KalmanFilter` parameter
  … fully reversible"). This is nowhere near M5/F1's "hub changes its OWN rules." The floor discipline is
  already real and reusable: noether Σx² Lyapunov bound + snapshot-root non-regression + test-count
  non-drop (`:11-14`), immutable content-addressed audit log with `EventLog::verify` tamper detection
  (`:21-22, 198-203`), and a live driver `self_mod_loop.rs` that routes every candidate through the
  adversarial `deliberate()` mirror and **fail-closes if the mirror does not converge** (`observe` at
  `self_mod_loop.rs:58`, apply-only-if-agreed at `:70-81`).

- **No sub-hub concept anywhere.** R1-B §F10 verified: no sub-hub spawn primitive, no depth field, no
  depth cap — grep-negative including `mcp.rs`. Hermes spawns Python sub-agents with no hub semantics.
  Consequence for F10: its "max recursion depth cap" **has no value to enforce because there is no
  recursion to cap yet.** The numeric cap value is *itself* an open operator decision — **O8**
  (`R2:152`, Phase 2 ruling). This phase builds the *mechanism* that will refuse at `depth > cap`; the
  *cap* is O8's ruling.

- **No sha3-verify-or-deny on model ingestion.** F3/F27 both LOCK `+ sha3-verify-or-deny`
  (`ARCHITECTURE.md:64, 92`). R1-B §F3 verified: the `sha3_256` primitive and content-addressing pattern
  exist (`bebop2_core::hash`, KAT-green per `revocation.rs:18-19`), but **no model-artifact fetch/verify
  path exists** — a hub could today load an arbitrary, unverified model blob with no hash check.

- **MCP is a mechanism; its calls are not capability-bound.** R1-B §E17/F4 verified: a real MCP server
  exists — `crates/bebop/src/mcp.rs:1-40`, stdio JSON-RPC 2.0 (handshake + tools/list + tools/call),
  DoS-hardened (`MAX_TOOL_ARG_BYTES` 1 MiB `:37`, `MAX_ARG_STR_BYTES` 64 KiB `:41`), `field_gate` +
  red-line vetoes preserved (`:30-31`). But F4's qualifier — *"protocol-clean IF behind a capability"*
  (`ARCHITECTURE.md:65`) — **is not enforced:** MCP tool calls are not bound to minted capability tokens,
  and there is no signed mesh announcement of a hub's MCP surface.

- **The mature proto-cap system STOPS AT THE WIRE.** This is the deepest gap. R1-B §E18 verified against
  code: the capability system is fully built for *mesh frames* — `capability.rs:48-52,82-93`
  (`{subject_key, scope, nonce, expiry}` + optional `subject_key_pq` ML-DSA-65 hybrid identity), closed
  `Resource`/`Action` enums (`scope.rs:12-90`), anchor-rooted delegation chains with **real narrow-only
  attenuation** (`Scope::is_subset_of` set-subset at `scope.rs:139`; `Delegation::sign` + `verify_chain`
  reject out-of-subtree as `ScopeViolation`, test at `scope.rs:351`), red-line deny (`redline.rs:1-45`,
  default DENY), fail-closed throughout. **Yet agents, tools, and subagents carry NO minted capabilities
  at all.** `self_mod.rs:27-38` even defines a *local stand-in enum* `SelfModCapability` — with the
  explicit comment *"Defined LOCALLY (not via `bebop_proto_cap`) because core cannot depend on proto-cap"*
  — naming `Resource::Corpus / Action::Append` as the real upstream verb it would map to at the wire. The
  proto-cap crown jewel is signing frames the agent layer never touches.

- **No eqc-gated self-update.** F9 LOCKs `+ eqc-gate-or-deny` (`ARCHITECTURE.md:70`). R1-B §F9 verified:
  today `self_mod.rs` hard-refuses dep-install/push as `HumanGated` (`self_mod.rs:59-61, 144`); eqc exists
  and runs (`dowiz/tools/eqc/eqc.py`, CI job `eqc-proofs` at `.github/workflows/ci.yml:30`). A hub
  auto-updating its own kernel from git (F9's situation) has **no proof-gate before applying** the update.

- **F7 is vacuous.** F7 ("hub switches to no-consensus / pure anarchy", `ARCHITECTURE.md:68`) is
  currently unbuildable *by design*: R1-B §F7 verified there is **no runtime consensus engine to defect
  from** — `proto-cap/tests/mesh_consensus.rs` is a spectral *parity simulation* (`:10,413`), not a
  runtime rule. The M-series mandates no consensus for liveness (M7: "no leader election required").
  Correct treatment: **record F7 as "satisfied by absence,"** not build a consensus engine just so a hub
  can opt out of it.

---

## 3. Generalized `HubPolicy` self-mod (with the red-line-floor carve-out explicit)

**Target (M5/M11/F1):** generalize `SelfModEffector` from its one-scalar scope (`set_q_scaler`) to full
**`HubPolicy` revisions**, reusing Phase 10's `HubPolicy`-as-data structure (ports, bridges, model
endpoints, access-lists, `RedLinePolicy`, `HybridPolicy` — the operator-editable, hot-reload entity
Phase 10 defines). Self-modification becomes **unbounded EXCEPT for red-lines** — which is itself the
honest, narrower claim the architecture supports (the M11 clause plus M12's red-line deny).

**Design — reuse the discipline, widen the target:**

1. **Same three-stage gate, generalized payload.** Keep the `self_mod_loop.rs` flow verbatim in shape:
   *propose → `deliberate()` mirror dialogue → apply iff converged, else fail-closed.* Replace the scalar
   candidate (`propose_q_scaler`) with a **`HubPolicyRevision`** — a typed diff over the Phase-10
   `HubPolicy` fields (add/remove a port, swap a model endpoint, edit an access-list entry, adjust a lane
   width). The candidate is serialized deterministically (canonical-TLV style, reusing the
   `to_tlv_bytes` pattern already at `scope.rs:125`) so it can be hashed, mirrored, signed, and audited
   with the existing machinery.

2. **Floor-gate, generalized.** The current floor gate (noether Σx² / snapshot-root / test-count,
   `self_mod.rs:11-14`) was written for a numeric filter parameter. For a `HubPolicyRevision` the floor
   becomes: (a) the revision must **validate** against the Phase-10 `HubPolicy` schema (fail-closed on
   malformed); (b) it must not **regress the snapshot root** or **drop the test-count** (unchanged); (c)
   it must pass a **red-line floor-gate** (next point). A revision failing any leg is recorded `REJECTED`
   in the audit log exactly as `self_mod.rs:181-186` records a noether-drift rejection today.

3. **RED-LINE floor — the non-negotiable carve-out.** The `HubPolicy` fields that correspond to the four
   red-line categories — **money / auth / RLS / migrations-equivalent policy fields** — stay
   `HumanGated` no matter what the mirror agrees. This reuses `EffectorReject::HumanGated`
   (`self_mod.rs:59-61`) and the existing hard-refusal list (`human_gated_op` at `:144` already refuses
   push-to-main, RLS, migrations, dep-install, `.claude/` edits). Phase 3's **full red-line category
   mapping** (Money **and** Auth **and** Secrets **and** Migrations — `R2:79` done-test #4, today
   `redline.rs` maps only Money) is a hard input: this phase's floor must deny all four scoped categories,
   not just money. A self-revision touching a red-line field is **refused and logged REJECTED** — it
   never reaches the mirror, never applies.

4. **Audit + second-opinion.** Every proposal/approval/reject/apply stays an immutable event
   (`self_mod.rs:198-203`, tamper-evident via `EventLog::verify`). The "mirror" is the existing
   `deliberate()` author↔mirror dialogue; where Phase 6's V1 verifier is available, a floor-clean
   revision additionally requires a **key_V-signed second opinion** (Phase 6's independent-context
   verifier) before `Applied` — this is what the §10 done-test means by "a mirror/second-opinion process
   agrees with."

**Net:** self-mod widens from one scalar to the entire `HubPolicy` surface, minus a hard red-line floor
that no amount of autonomy can cross. That is M11 realized as canon actually phrases it.

---

## 4. Sub-hub spawn + capability-CARRIED depth counter (F10)

**Target (F10, `ARCHITECTURE.md:71`):** *"Hub delegates to a sub-agent that opens its own sub-hub …
LOCK + max-depth-cap."* Build the spawn mechanism such that the depth budget is **carried inside the
capability token**, not tracked in ambient global state.

**Design:**

1. **Depth as an attenuated capability field.** A hub may spawn a sub-hub only by presenting a capability
   token that itself **encodes the remaining depth budget**. Spawning consumes one unit: the child's
   token is minted with `depth_remaining = parent.depth_remaining - 1`. This rides the existing
   **narrow-only attenuation** machinery — `Scope::is_subset_of` (`scope.rs:139`) and
   `Delegation::sign`/`verify_chain` already guarantee a child capability is a strict subset of its
   parent (test `r4_attenuated_capability_outside_subtree_is_rejected`, `scope.rs:351`). Depth is one
   more monotonically-narrowing field: a child can never mint itself *more* depth than its parent carried,
   because the delegation chain verify would reject a widening.

2. **Refuse at the cap.** A spawn request whose token carries `depth_remaining == 0` (equivalently,
   whose depth would exceed the ruled cap) is **refused fail-closed** — the same `Unauthorized`/reject
   shape as `self_mod.rs:165-168`. The concrete cap value is **not this phase's to choose**: it is
   **O8** (`R2:152`), a Phase-2 operator ruling. This phase builds the mechanism and wires it to read
   O8's constant; it does not invent a number.

3. **Kill-switch reaches the subtree.** Because depth is capability-carried and each sub-hub is
   anchor-rooted through its delegation chain, Phase 10's operator kill-switch can address a **subtree**
   (M9's "hub/subtree" language, previously undefinable per ambiguity #3 / O13 because no hierarchy
   existed). Once sub-hubs exist, subtree-kill becomes definable: kill the parent capability → the whole
   attenuated chain beneath it is revoked via the existing monotonic `RevocationSet`
   (`revocation.rs:35-55`: "revoking a subject key kills every capability ever minted to it"). This is
   the resolution of O13 that F10 unblocks.

---

## 5. Model manifest `{url, sha3}` — verify-at-load, refuse-on-mismatch (F3/F27)

**Target (F3 `+ sha3-verify-or-deny`, F27 `+ sha3-gate`; `ARCHITECTURE.md:64, 92`):** a hub that pulls a
model from HuggingFace (or anywhere) at runtime verifies it before it runs; an unverified blob is
**refused, not warned.**

**Design:**

1. **Manifest schema.** A model is ingested only via a signed manifest `{ url, sha3, size, scope }`. The
   `sha3` is the expected content hash of the weight blob; `scope` is the `Resource`/`Action` capability
   the loaded model may run under.

2. **Verify-or-deny at load.** Fetch → compute `sha3_256` over the received bytes (reuse
   `bebop2_core::hash`, KAT-green per `revocation.rs:18-19`) → **compare to the manifest hash.** On
   mismatch, **refuse at load time** and record a `REJECTED` audit event — *not* "accept with a warning."
   This is the falsifiable behavior in §10: a blob with a wrong sha3 is refused, silently-accepted-with-a-
   warning is a FAIL.

3. **Capability-scoped run.** A verified model runs only inside the capability scope its manifest
   declares (F3 "model runs in capability scope"). This composes with §6's per-agent minting: the model
   is just another capability-bound tool.

This is content-addressing (the repo's existing pattern, `ARCHITECTURE.md:48`) applied to model supply
chain. It closes the F27 "runs unaudited model" risk without contradicting M5 autonomy: the hub may pull
*any* model — it just cannot run one whose bytes do not match what it committed to.

---

## 6. Per-agent capability MINTING → the MCP surface (E17/E18/F4) — where Phase 3/6 crypto FINALLY reaches the agent layer

**This section is the structural payoff of the whole roadmap's crypto work.** Say it plainly: the
proto-cap capability system was **hardened in Phase 3** (hybrid delegation links + anchors, C4b closed,
full red-line category mapping) and **used for identity in Phase 6** (key_K/key_V split-identity, the V1
verifier). Until now it has **stopped at the wire** — signing mesh frames, minting nothing for agents,
tools, or subagents. **Phase 15 is where that same mature machinery finally reaches the agent/tool
layer.** The `self_mod.rs:27-38` local stand-in enum is retired; agents carry real minted capability
tokens.

**Design:**

1. **Map agent surfaces to `Resource`/`Action` scopes.** MCP tools (`mcp.rs`) and Hermes tool classes
   each map to a closed `(Resource, Action)` pair from the existing enums (`scope.rs:12-90`) — e.g. a
   corpus-append tool ↔ `(Corpus, Append)`, an order-read tool ↔ `(Order, ReadProjection)`. No new
   attenuation scheme is invented; the closed enums and `NO-COURIER-SCORING` guard (`scope.rs:6`) hold.

2. **Mint per-agent tokens; attenuate on spawn.** Each agent/tool call must present a **minted capability
   token**; an MCP tool call attempted **without** one is **refused** (§10 done-test). A spawned
   sub-agent's capabilities are minted as a **strict subset** of its parent's — never equal, never
   broader — using `Scope::is_subset_of` + `Delegation::sign`/`verify_chain` (`scope.rs:139, 351`). This
   is the same narrow-only attenuation that already guards mesh delegation; it now guards agent spawn.
   Depth (§4) is one attenuated field; scope is the rest.

3. **Bind capabilities into the MCP surface + sign the announcement.** MCP tool dispatch checks the
   caller's minted token against the tool's declared scope before executing (behind the existing
   `field_gate` + red-line vetoes at `mcp.rs:30-31`, not replacing them). The MCP server's **own
   capability surface** — which tools it exposes, under which scopes — is **signed (hybrid, per Phase 3)
   and announced over the mesh** (F4's "proto-cap-sign" qualifier + E17 completion). A peer learns a
   hub's tool surface from a signed frame, not an unauthenticated handshake.

4. **Hybrid signing dependency.** Because minting and announcement sign with the Phase-3 hybrid identity
   (Ed25519 ⊕ ML-DSA-65), this section inherits Phase 3's C4b closure (variable-time `mod_l`
   side-channel) exactly as Phase 6 does — do not begin minting on the leaky signing leg.

**Net:** E18's "per-agent capability tokens" becomes true; F4's qualifier is enforced; E17 is completed.
The crown jewel now signs what agents do, not just what the wire carries.

---

## 7. eqc-gated self-update (F9)

**Target (F9 `+ eqc-gate-or-deny`; `ARCHITECTURE.md:70`):** a hub auto-updating its own kernel from git
must pass an eqc proof **before applying**, not apply-then-roll-back.

**Design — fetch → build → PROVE → apply-or-deny, inside the effector discipline:**

1. **Pull the candidate update** (git ref) into an isolated build (fresh worktree pattern, mirroring
   Phase 6's verifier-runner isolation).
2. **Run the floor before applying:** build the candidate, run the **eqc proofs** (`dowiz/tools/eqc/`,
   the same suite CI runs as `eqc-proofs`, `ci.yml:30`) **and** the test-count / snapshot-root
   non-regression floor (the existing `self_mod.rs` floor discipline, generalized in §3).
3. **Deny BEFORE application on any failing proof.** If the eqc proof fails, the update is **denied and
   never applied** — recorded `REJECTED` in the audit log. This is stronger than rollback: the §10
   done-test requires *denied before application*, so an apply-then-revert design is a FAIL.
4. **Red-lines still HumanGated.** A self-update touching dep-install, migrations, RLS, `.claude/`, or
   push-to-main remains hard-refused (`self_mod.rs:144`), regardless of eqc result. F9 self-update is for
   kernel logic that passes eqc; it is not a bypass of the §3 red-line floor.

This reuses two things the repo already has (the eqc prover and the floor-preserving effector) rather
than building a new proof system.

---

## 8. F7 — formal "vacuous, satisfied by absence" note (for canon)

F7 is **recorded, not built.** The formal note this phase contributes to canon (as a §8 honest-gap entry,
for the operator's merge — this document does not edit `ARCHITECTURE.md`):

> **F7 (hub switches to no-consensus / pure anarchy) — VACUOUS, SATISFIED BY ABSENCE.** There is no
> runtime consensus engine in the mesh to defect *from* (verified: `proto-cap/tests/mesh_consensus.rs` is
> a spectral parity *simulation*, not a runtime rule; the M-series requires no consensus for liveness —
> M7 "no leader election required"). A hub is therefore already, trivially, in the F7 state: it runs its
> own rules with no consensus obligation. Nothing is built for F7. **Non-vacuity trigger:** F7 becomes a
> real, buildable situation only if and when a runtime consensus engine ever ships (none is planned); at
> that point F7 would mean "a hub opts out of that specific engine," and this note must be revisited. Until
> then, forcing a consensus feature into existence *just so a hub can reject it* would violate ponytail
> (YAGNI) and M11 (no governing layer above hubs).

This is the correct, honest treatment: the anchor is satisfied by the absence of the thing it would
defect from, and the future condition that would make it non-vacuous is named.

---

## 9. E13 self-host provider — `LlmBackend` port design; E13-cpu unlockable NOW, E13-gpu stays O18-gated

**CORRECTED 2026-07-16 (triple-confirmed: this session's SELF-CRITIQUE §3, an independent LLM-infra
research pass, and live host probes all reached the same verdict — settled fact, not re-litigated).**
The original framing gated ALL self-host execution on GPU-unlock. That conflated two unrelated
triggers: llama.cpp is CPU-first by design (its whole reason for existing) and needs no GPU, while
only the vLLM/Modal tier genuinely benefits from one. Live evidence on this host (2026-07-16,
8-vCPU EPYC Milan, 32GB RAM): `/usr/local/lib/ollama/llama-server` already installed, no `/dev/nvidia*`,
`huggingface.co` → 200, `crates.io`/`cargo add wgpu` → 403. The two capabilities have *opposite*
current availability — proof they were never one trigger. See `HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md`
§2 H3 for the full design; this section is re-scoped to match it.

**Split gating, both still ship *design only* in this phase — deployment is a separate, later action:**
- **E13-cpu (llama.cpp, Tier 1):** gate = a dated DECART report + operator go. **No GPU condition.**
  Unlockable now — this phase still only designs it; the operator's go and the actual deploy are a
  following action, not automatically triggered by this blueprint landing.
- **E13-gpu (vLLM/Modal, Tier 2):** unchanged — stays gated on the real external GPU-unlock trigger
  (O18: network `cargo add wgpu` succeeding, operator/environment), with trigger-gated *activation*
  in **Phase 17**.

**Blueprint (the `LlmBackend` Trait-as-Port, per HARNESS-IMPROVEMENT-SYNTHESIS-PLAN §2 H3):**

1. **`ports/llm.rs` — one trait, one transport, three adapters.** `trait LlmBackend { id, caps, chat,
   embed, rerank, health }`; kernel sees only `&dyn LlmBackend`, adapters live in a separate crate the
   kernel never imports (compile firewall). One `OpenAiCompatTransport` with per-backend `Quirks` (the
   Hermes-proven pattern, `plugins/model-providers/custom/`): `ManagedApiAdapter` (Tier 0, default,
   live now), `LlamaCppAdapter` (Tier 1, `127.0.0.1:8080`, static binary + systemd unit, S1-native
   zero-OCI — operator-unlockable now), `VllmAdapter` (Tier 2, local GPU or Modal H100 burst,
   $0.001097/s scale-to-zero — stays O18-gated).
2. **Hub choice = config, not kernel change.** Selected per-hub via `HubPolicy.llm_backend` /
   `LLM_BACKEND=` + `LLM_BASE_URL=` in EnvFile — this is M5 ("every hub may use any models/API at its
   discretion") made real for the first time; no dev-time gate may block a runtime hub from switching
   (SCOPE RULE).
3. **sha3-verified weights.** Model weights are ingested through §5's manifest `{url, sha3}` verify-or-
   deny path — no unverified GGUF blob runs, for either tier.
4. **Budget ceiling.** A `TokenBucket` / `Budget` spend ceiling (reusing the Phase-5 per-agent budget and
   the existing `transport_policy.rs` `TokenBucket`) bounds inference cost for both tiers, degrade-closed
   when over budget.
5. **Honest absence.** With the backend down/unconfigured, `health()`/`embed()` return a typed `Err`
   (the E21 boundary pattern) — never a mock, never a silent fallback to a different tier.
6. **First Tier-1 consumers named (value, not capability theater):** the VERIFIABLE-COGNITION §3.3
   semantic-leakage gate (deferred solely for lack of an embeddings bridge — `llama-server
   /v1/embeddings` is that bridge, at $0/call) and a sovereign advisory judge (`eval-layer/
   openrouter_judge.py` already honors `OPENAI_BASE_URL` — advisory-only, never the gate).
7. **EV loop closure.** Local-backend calls feed Phase 1's harvested `track_record.jsonl` (per
   `HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md` H1), so `gov_route` prices local-vs-managed on measured
   data, not vibes.

**Do not** schedule any E13-gpu *deployment* work inside Phase 15 — that stays Phase 17 + O18. E13-cpu's
design ships here; its deployment (DECART report → operator go → actual rollout) is a distinct,
separately-scheduled action this phase does not itself trigger.

---

## 10. Acceptance criteria (numbered checklist)

The phase is done when **all** of the following are demonstrated (the falsifiable done-tests; each REJECT
must be logged, not silently swallowed):

1. **Ordering gate honored.** No §3–§7 mechanism was built before Phase 10 (kill-switch + `HubPolicy`
   entity), Phase 5 (routing/`deliberate()` tier + budget), and Phase 9 (wire frames + signed announce +
   ML-KEM) were green. (Traceable: the `HubPolicy` type, the kill-switch, and the mint-signing path are
   all *consumed*, not defined, here.)
2. **Red-line floor refuses.** A `HubPolicy` self-revision that would touch a red-line floor field
   (money/auth/RLS/migrations-equivalent — e.g. an attempt to edit the money-equivalent policy field) is
   **REFUSED and logged REJECTED**, never reaching the mirror.
3. **Floor-clean revision applies.** A floor-clean `HubPolicy` self-revision that the mirror /
   Phase-6 second-opinion process **agrees with** is **APPLIED** and audit-logged `Applied`. (Both #2 and
   #3 demonstrated — the pair is the test.)
4. **Depth cap refuses.** A sub-hub spawn request at depth greater than the O8-ruled cap (token carries
   `depth_remaining == 0`) is **refused** fail-closed.
5. **Attenuation holds on spawn.** A spawned sub-agent's minted capability is a **strict subset** of its
   parent's; an attempt to mint an equal-or-broader child is rejected as `ScopeViolation`.
6. **Model sha3 mismatch refused at load.** A model blob presented with an **incorrect sha3** is
   **refused at load time** (not accepted-with-a-warning).
7. **MCP call without capability refused.** An MCP tool call attempted **without a minted capability
   token** is **refused**; a hub's MCP surface is announced over the mesh as a **signed** (hybrid) frame.
8. **eqc self-update denied before application.** A self-initiated kernel update from git whose **eqc
   proof fails** is **DENIED before application** (not applied-then-rolled-back).
9. **F7 recorded vacuous.** F7 is recorded in canon text as **vacuous, satisfied by absence**, with the
   named non-vacuity trigger (a real consensus engine existing to defect from) — §8 note above, staged
   for the operator's canon merge.
10. **E13 is blueprint-only, split gating.** The `LlmBackend` port + three adapters exist as a **design**
    (sha3 weights + budget ceiling); **no deployment** work was done. E13-cpu (llama.cpp) is
    **unlockable now** via DECART report + operator go, **no GPU condition**; E13-gpu (vLLM/Modal)
    remains gated on O18 / Phase 17. Default `cargo build`/`test` dependency graph is byte-identical to
    today (no HTTP/adapter crate added by this phase).

**Anchor coverage check:** M11 (§1, the whole dependent-not-independent framing + red-line-bounded
realization), E17 (§6 MCP completion), E18 (§6 per-agent minting), F3 (§5), F4 (§6 sign qualifier),
F7 (§8 vacuous), F9 (§7), F10 (§4 depth cap, value = O8), F27 (§5 sha3-gate). All nine Phase-15 anchors
(`R2:187-189`) accounted for; zero built ahead of the Phase-10 hard gate.
