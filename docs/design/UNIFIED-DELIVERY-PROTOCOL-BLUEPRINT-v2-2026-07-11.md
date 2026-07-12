# Unified Delivery Protocol — Blueprint v2 (independent synthesis)

> Authored 2026-07-11 as an INDEPENDENT unification, not a re-statement of the prior
> `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-2026-07-11.md` (which was an index). This document
> is synthesized directly from PRIMARY sources across both repos + both memory stores, and
> resolves tensions the index left implicit.
>
> Provenance of every load-bearing claim is cited inline as `source:line`.
>
> PRIMARY SOURCES READ FOR THIS SYNTHESIS
> - dowiz vision/foundation: `sovereign-core-mvp/MANIFESTO.md`, `DECISIONS.md`, `GRAND-PLAN.md`
> - dowiz hub recon (957 lines, on-target): `docs/research/2026-07-11-hub-architecture-review.md`
> - dowiz gap sequencing: `docs/design/gap-blueprints-2026-07-11/MASTER-EXECUTION-PLAN.md`
> - bebop PQ crypto status (verified from source): `bebop2/core/src/{pq_dsa,kdf,pq_kem,sign}.rs`
> - bebop protocol design: `fable-protocol-2026-07-11/{F1,F2,F3,F4}.md`,
>   `delivery-protocol/PROTOCOL-CENTRALIZATION-MAP.md`
> - audits: `plan-audit-dowiz-2026-07-11.md`, `plan-audit-bebop-2026-07-11.md`,
>   `plan-audit-memory-2026-07-11.md` + dowiz recon delegate (deleg_7875385e)
> - memory: `~/.claude/projects/-root-dowiz/memory/MEMORY.md` (+ active arcs),
>   `~/.hermes/SOUL.md` (no project memory yet — only bebop sessions can't Write memory, G08)
>
> The plan audit file (`docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`) and the
> 13 gap blueprints (G01–G13) feed the sequencing in §6.

---

## 0. The one sentence (carried, sharpened)

Build ONE **open-source, self-hosted owner hub** — local-first, Universal-Rust/WASM, pure
deterministic event-sourced core, integer money, **no AI in runtime** — that funnels a food
vendor's multi-channel / multi-device order entrypoints into a single **0%-commission direct
checkout** and dispatches **their own couriers**, with the event log's **canonical-bytes +
signature-slot seams baked in now** so that **post-quantum signatures, mesh/P2P sync, and an
open competitive matcher market** switch on later without a rewrite.

The MVP is a shippable **Trojan horse**; decentralization + PQ + open-matcher is the **declared
destination**, operator-Red-Teamed and hard-gated behind the MVP (MANIFESTO §8; DECISIONS D6).

---

## 1. Governing invariants (non-negotiable — inherited, not re-litigated)

1. **Determinism > AI.** No probabilistic decisions in runtime/protocol business logic. AI is a
   build-/back-office tool only (MANIFESTO §1; ethics charter in AGENTS.md).
2. **Pure core.** No clock/RNG/env/float/network in `dowiz-core` / `bebop2-core`. Enforced by the
   `clippy.toml` wasm32 disallowed-methods gate (DECISIONS D2; already caught a real uuid-v4
   entropy dep, MEMORY `sovereign-core-phase-zero`).
3. **Event-sourced law.** `Intent → decide → Event`; `state = fold(events)`; forbidden transitions
   are compile/runtime errors. `kernel::decide` is the ONLY business-mutation door
   (GRAND-PLAN §1.1; MANIFESTO §3).
4. **Integer money.** `Lek(i64)`, no `From<f64>`, single money surface end-to-end (MANIFESTO §3;
   DECISIONS D1 — aggregator intake banned because it breaks this invariant).
5. **Local-first / no central chokepoint** as a design invariant reachable "for free" from a signed,
   replayable event log (MANIFESTO §6; DECISIONS D2). The hosted hub is a *thin, replaceable access
   layer*, never the chokepoint.
6. **Verified-by-Math.** Every change ships a falsifiable proof with a RED case (DECISIONS D5;
   HERMES "Verified-by-Math" rule). No false-green.
7. **Decentralize the matcher, not just the ledger.** A single dispatch server = "DoorDash with
   extra steps." The matcher must be a pure replicable function any node can run
   (hub-architecture-review §1.1; platform-vs-protocol-logistics brief).
8. **Ethics charter.** No military/warfare use; AI-as-commons; peace-for-everyone; overrides all
   (AGENTS.md / HERMES.md ethics section).

---

## 2. The reconciliation the index elided: TWO topologies, ONE boundary

The existing index stated "local MVP + network protocol" but did not pin the boundary. The
primary sources force a sharp answer:

- **Local (MVP-shaped):** ONE food-vendor owner runs their hub, owns their data, dispatches their
  OWN couriers. Money on one checkout surface. Ships first (DECISIONS D1).
- **Network (protocol-shaped):** permissionless matchers, per-actor PQ identity, mesh sync, open
  settlement. The owner hub becomes ONE node among many (MANIFESTO §6; F3 centralization map).
- **THE BOUNDARY (load-bearing):** the owner hub is ALWAYS the vendor's sovereign control point.
  The *network* only ever adds (a) courier liquidity and (b) cross-node trust — it NEVER takes
  custody of the vendor's money or data. The matcher decentralizes; the vendor's ownership does
  NOT dilute. This is what makes "decentralized" compatible with "one owner hub" — the hub is the
  anchor, the mesh is an optional courier-pool extension (hub-architecture-review §1; F3 L3/L4).

This resolves the conflict the memory audit flagged (memory-audit §5.2: "owner-managed couriers"
vs "open matcher market"): they are orthogonal axes — ownership is per-vendor, dispatch liquidity
is pooled. The hub owns; the matcher lends couriers.

---

## 3. Layered architecture (unified) — with REAL status, not roadmap

```
 L0  PURE CORE          dowiz-core (order machine, money, pricing, idempotency, codec)
                        bebop2-core (PQ crypto: ML-KEM-768, ML-DSA-65, Ed25519,
                        XChaCha20-Poly1305, Argon2id, SHA-2/3, in-tree CSPRNG)
                        — deterministic, RNG-free hot path, WASM-pure, no phone-home
 ─────────────────────────────────────────────────────────────────────────────────────
 L1  IDENTITY / PoD     self-certifying id = H(pq_pub ‖ classical_pub); hybrid
                        ML-DSA-65 ⊕ Ed25519 signatures; Proof-of-Delivery = signed claim
                        `order:<id>|courier:<vault_id>|at:<ts>|loc:<x,y>`  [bebop pod.rs]
 L2  EVENT LOG / SYNC   append-only signed event log; content-hash + signature slot per
                        event; transport-agnostic SyncPort (append / read_since).
                        Postgres today = one transport; libp2p/gossip = another later.
 L3  MATCHER / DISPATCH pure replicable matcher fn (same input → same fingerprint on any
   (DANGER #1)          node); owner-dispatch for MVP; open competitive matcher market +
                        force-inclusion fallback for the protocol.  [bebop matcher.rs]
 L4  SETTLEMENT         PoD-gated payout; non-custodial (funds → merchant wallet,
                        stablecoin); device-sig THRESHOLD verifier, never a single oracle
                        (or it re-centralizes at DANGER #3). Hot path never touches DLT;
                        only final settlement does.
 L5  ARBITRATION        fail-closed dispute state machine
                        OPEN→EVIDENCE→AUTO_ARBITRATE→ESCALATE→JURY→SETTLE; any
                        timeout/ambiguity → escrow HOLD + default refund to claimant.
 ─────────────────────────────────────────────────────────────────────────────────────
 ACCESS / SDK           thin owner-hub client + reference alt-client (DANGER #2: "open
   (DANGER #2)          protocol, closed access" — must ship a second client to prove
                        the access layer is not a chokepoint).
 GUARD                  consensus kill-switch: ≥2/3 supermajority suspends a peer; no
                        central off-button.  [bebop guard.rs]
```

### 3.1 Status of each layer (ground truth, verified)

- **L0 dowiz-core:** DONE + pushed (0b-1 money, 0b-2 Envelope, 0b-3 `decide` composition, 0b-5
  keystone RED-proof). BUT the *Rust* checkout currently **bypasses `kernel::decide`** on staging
  — no `Command::PlaceOrder` exists in the api crate (hub-architecture-review §3; G06). This is a
  coherence violation that must close before any prod Rust flip.
- **L0 bebop2-core:** GREEN. ML-KEM-768, Ed25519, XChaCha20-Poly1305, SHA-2/3, Argon2id KAT-green.
  **ML-DSA-65 roundtrip GREEN** (commit `fb4e651`, 2026-07-11 — FIPS 204 Decompose centered-mod +
  q−1 special case; MakeHint uses r+z). NOT yet NIST-bit-exact (no oracle in sandbox). Ed25519
  tests hang on perf (sign.rs:61 `mod_p_be` bit-by-bit heap division) — blocks full-suite CI, not
  correctness (pq_dsa.rs / sign.rs source-verified 2026-07-11).
- **L1 identity/PoD:** DESIGN + partial CODE in bebop (`pod.rs`, `vault.rs`); signature slot in
  dowiz-core is NULL for the whole MVP (DECISIONS D2 defers signing to Phase 3). The hinge gap:
  promote signature slot NULL→real per-actor PQ identity as the FIRST decentralization step.
- **L2 sync:** event log real; SyncPort = Phase 1.3 (OPEN, 0 artifacts); only Postgres impl today.
- **L3 matcher:** CODE, test-proven (bebop `matcher.rs`, `matcher_is_replicable_no_hidden_server`).
  Owner-dispatch already exists in legacy TS (attemptHonestDispatch). Open-matcher market = design.
- **L4 settlement:** POETRY (0 lines). Must be built as device-sig threshold verifier, not oracle.
- **L5 arbitration:** DESIGN-ONLY (F2 fail-closed state machine). No code.
- **ACCESS/SDK:** DANGER #2 — SDK/bootstrap layer is the genuine re-centralization risk; thin
  client + reference alt-client specified, not coded (F3 DANGER #2).
- **GUARD:** CODE (bebop `guard.rs`, ≥2/3 kill-switch).

---

## 4. The crypto reality (bebop2) — independent assessment

bebop2 is a **NON-AI, deterministic, from-scratch, zero-dependency PQ crypto core** — exactly the
trustless substrate the protocol needs (RNG-free hot path, no phone-home, KAT-anchored). It is the
right backing for: PoD signatures (ML-DSA-65 ⊕ Ed25519 hybrid), key exchange (ML-KEM-768), at-rest
vault (XChaCha20-Poly1305 + Argon2id), self-cert identity.

**Honest gaps (must not be papered over):**
1. **ML-DSA-65 / ML-KEM-768 are NOT FIPS-interoperable by construction** (coefficient-domain keys,
   CBD-sampled matrix A, 32-byte vs 48-byte challenge). "Bespoke schemes wearing FIPS names" —
   cannot validate against ACVP/official vectors until re-derived (G09; bebop audit item 4).
2. **Live timing leaks (KyberSlash class):** ML-KEM `compress`/`decompress` divide-by-q; ML-DSA
   schoolbook secret-branch; Ed25519 `scalar_mul` secret-scalar branch (G09 §3; bebop audit).
3. **Two crypto cores exist:** `bebop2/core` (new PQ) vs legacy `crates/bebop/src` `vault.rs`
   (XChaCha20 + scrypt). Must re-point vault/pod at bebop2; retire scrypt for Argon2id.
4. **wasm32 empty-import gate FAILS (~94 errors)** — the ONLY honest bare-metal/sovereign-node
   proof. Blocker for any "runs on the vendor's device with no reachable clock/RNG/socket" claim.
5. **Ed25519 perf hang** (sign.rs) — blocks green CI; fast fix available (2²⁵⁵−19 reduction).

**Policy (carried from G09):** adopt bebop2 PQ ONLY via the 4-tier hybrid policy
(ML-DSA-65 ⊕ Ed25519). No primitive guards value at Tier 3 (sole-guard) without external audit +
ACVP interop + constant-time proof. Re-derive to FIPS interop before any Tier-3 use.

---

## 5. The unification gaps — resolved explicitly

| # | Gap / conflict | Resolution (this blueprint) |
|---|---|---|
| 1 | Central-money invariant vs decentralization | Single money surface *per owner node*; network adds liquidity + trust, never custody. SyncPort + per-source reconcile is the seam. |
| 2 | PQ quarantined / non-FIPS / timing-leaky | Hybrid ML-DSA-65 ⊕ Ed25519 only; re-derive to FIPS/ACVP + constant-time before Tier-3; until then Tier ≤2. |
| 3 | "Runs locally / OSS" vs central Fly+Supabase+BYPASSRLS | Local-first owner binary is the target; secret-history scrub (open blocker) + NOBYPASSRLS flip + ADR-020 (AGPLv3, never committed) must clear for OSS release. |
| 4 | Signature slot NULL for whole MVP | Promote NULL→per-actor PQ identity as FIRST decentralization step — the hinge for PoD/courier/vendor trust. |
| 5 | Exit gate false-green (verification debt) | Fix before new features: `hub_checkout` gates nothing, `replay-parity` placeholder, `cause_hash`="placeholder", vacuous Playwright (G06). |
| 6 | Rust checkout bypasses `kernel::decide` | Wire `Command::PlaceOrder` through the kernel so the hub obeys its own law (hub-architecture-review §3). |
| 7 | Two crypto cores | Re-point vault/pod at bebop2; retire scrypt for Argon2id. One crypto core. |
| 8 | Couriers omitted from hub docs | Merge the deliver-v2/ADR-0013 courier lineage INTO the hub definition. Define courier-payout authority (split S5-write/S7-read today). |
| 9 | PoD signature ≠ human received box | Treat PoD as *contestable*; route to L5 arbitration. Multi-signal attestation, never signature-as-ground-truth. |
| 10 | Non-AI goal vs AI-heavy harness | The AI harness (VSA/codebase-memory/model-routing/THE EYE) is BUILD-TIME only; must not leak into shipped protocol. Purity law enforces this. |
| 11 | wasm32 empty-import gate fails (~94 errors) | ~1 day mechanical (no_std + alloc + libm). Blocker for bare-metal claim. |
| 12 | Fee framing "0% atomic bomb" | Poetry. 0% is a subsidy, not a moat. Real moat = earned local reputation graph + credible neutrality. Viable fee 1–3% + value-added sinks (F1). |
| 13 | "Many order sources" transport-false | Attribution captured but ZERO readers of `metadata->>'channel'` in prod; messengers/bots are dark scaffold. Build QR-kit reader + out-of-app courier beep FIRST (hub-architecture-review §0 verdict). |
| 14 | Hub front gate broken | `/claim` 404; web checkout 400-fails 3 of 6 contact options + every "deliver to someone else" (G03); prod worker stopped since 07-03 (G11). |

---

## 6. Sequenced execution plan (from gap-blueprints MASTER + hub-review verdict)

**Wave 0 — Protect & stop bleeding (same day):**
- Pause the ~6-hourly cloud push loop re-dirtying origin (G02); mirror-bundle the sole pre-scrub
  history copy; install gitleaks CI hard-fail; land the 3 uncommitted gate diffs (G13).
- Protect bebop crypto WIP: bundle → apply the 2 one-line ML-DSA fixes (already done, `fb4e651`) →
  commit+push. (bebop sessions can't Write memory — G08 D4 carve-out needed.)

**Wave 1 — The prod vehicle (days 1–2), THE critical path:**
- Single curated PR to origin/main: GDPR trio + AnonymizerService storage-DI fix (G01) + G03
  (6-kind enum + `receiver{}` schema) + G11 (`/claim` SPA route + demo storefront provisioning).
- Discharges the only live legal exposure AND unblocks the revenue funnel in one merge.
- Success metric (the one that matters, G07/G11): **a real order row from a non-operator customer
  on a claimed venue.** RED = 0 claims after 10 contacts across 5 venues → stop/pivot.

**Wave 2 — Validation week (days 3–7, operator-personal):**
- QR sheets (already built) + attribution reader (one card) + out-of-app courier beep (closes gap 13).
- Concierge outreach ArtePasta → Dubin & Sushi → Apollonia. Sign the G07 arbiter doc (ranking:
  validate-first; Sovereign MVP > rebuild > bebop > OSS).

**Wave 3 — The scrub window (scheduled, after Wave 1):**
- Freeze → mirror-verified → force-push + branch deletes → fresh-clone gitleaks. Unblocks ADR-020
  gate 1 (open-source). Consider fresh-repo-swap (0 forks, cheap).

**Wave 4 — Program hygiene & gated tracks (parallel, post-Wave-1):**
- **Sovereign Core (ranked #1):** close verification debt (gap 5) → build 2.1 channel/QR entrypoints
  + 1.4 signed envelope → wire Rust checkout through `decide` (gap 6) → approve+port S7 courier
  plane, merge couriers into hub (gap 8).
- **bebop2 assurance:** fix Ed25519 perf (gap 11/§4.5) → wasm32 gate (gap 11) → constant-time
  ML-DSA/ML-KEM → re-derive to FIPS/ACVP interop; keep hybrid. ML-DSA-65 roundtrip already GREEN.
- **Turn on the network:** libp2p/gossip SyncPort; open matcher market + force-inclusion (L3);
  PoD-gated non-custodial settlement via device-sig threshold (L4); fail-closed arbitration (L5);
  ship reference alt-client (DANGER #2); NOBYPASSRLS + ADR-020 AGPLv3 OSS cutover.

---

## 7. Repo boundary (standing rule)

- **`/root/dowiz`** = product + Sovereign Core (owner hub, orders, couriers, checkout, MVP).
- **`/root/bebop-repo`** = PQ crypto core (`bebop2/core`) + protocol primitives (matcher, pod,
  reputation, guard) + protocol design (fable F1–F4, centralization map, delivery-protocol/).
- Files referencing bebop / `feat/wire-native-core` belong in `/root/bebop-repo`, not `/root/dowiz`.
- The unified protocol **consumes** bebop2 crypto + bebop protocol primitives from the dowiz hub;
  it does not fork them.

---

## 8. Load-bearing design rules (carry into every downstream decision)

1. The matcher is a pure replicable function — prove two nodes produce identical fingerprints.
2. Settlement is a threshold of device signatures, never a single oracle.
3. PoD is contestable; arbitration is fail-closed (default refund to claimant on ambiguity).
4. The owner hub is a thin replaceable access layer; ship a second client to prove it.
5. No primitive guards value at Tier 3 without external audit + ACVP interop + constant-time proof.
6. Every event carries canonical bytes + a signature slot from day one, even while NULL.
7. The AI harness never enters the shipped protocol. The runtime is deterministic Rust/WASM.
8. Every claim of "done" ships a falsifiable RED+GREEN proof (Verified-by-Math).

---

## 9. What this v2 adds over the prior index blueprint

- **Explicit topology boundary** (§2): ownership is per-vendor, dispatch liquidity is pooled — this
  is what reconciles "one owner hub" with "decentralized matcher" (the prior index left it vague).
- **Real per-layer status** (§3.1) verified from source + hub-review, including the `kernel::decide`
  bypass (gap 6) and the prod front-gate breakage (gap 14) the index under-weighted.
- **Crypto honesty** (§4): non-FIPS-interop + KyberSlash-class timing leaks + wasm32 gate are stated
  as blockers, not footnotes.
- **Sequencing anchored to the gap-blueprints MASTER** (§6) + hub-review "fastest path" verdict, with
  the one success metric (real order from a non-operator customer) made explicit.

*End of blueprint v2. Primary sources cited inline; underlying corpora remain authoritative.*
