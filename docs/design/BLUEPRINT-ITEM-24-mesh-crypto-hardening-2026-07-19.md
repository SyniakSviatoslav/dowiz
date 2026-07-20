# BLUEPRINT — Item 24: Mesh/gossip crypto surfaces swept under the §4 hardening checklist

- **Date:** 2026-07-19 · **Tier:** parallel lane / §F mesh-gossip (roadmap §F) · **Status:** BLUEPRINT
  (planning artifact, no code) — depends on item 6's re-executing CI machinery (DONE) + item 14's
  toolchain-bump trigger (DONE) + item 23 (the surfaces to sweep must exist).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §F (line 414:
  "item 24 — crypto surfaces under §4 — depends on item 6's re-executing CI machinery and item 14's
  trigger"); `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §17(c) (line 304, the crypto
  discipline), §9 addendum item 24 (line 314); `docs/audits/hardening/CHECKLIST.md` (the four items),
  `docs/audits/hardening/HOT-PATHS.tsv`; live source `kernel/src/mesh.rs`, `kernel/src/pq/dsa.rs`,
  `kernel/src/ct_gate.rs`, `AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`.
- **Relationship to item 23:** item 23 builds the gossip-admission path (extending `import_unit`);
  item 24 applies the **full §4 hardening checklist** to the crypto surfaces that path exposes. They
  are sequenced together; item 24 is the hardening pass, not the feature.

---

## 1. Scope / goal (one paragraph)

Bring the mesh/gossip crypto surfaces under the **identical** §4 hardening discipline the NTT work
already proved out (synthesis §1.6) — "**not** a lighter 'protocol' variant" (synthesis §17(c)). The
moment gossip touches a signature, a capability token, or any secret-dependent comparison, it **is** a
crypto surface and inherits the full checklist: an **oracle** (a simple, obviously-correct reference
implementation of gossip-message verification and capability-token validation, retained forever as the
differential target); a **dudect-style gate** wherever verification touches a secret-dependent compare
(signature/MAC comparison is the classic constant-time-required site — named the way the FO
implicit-rejection tag-compare already was, planted-leak self-test included); a **debug-mode
differential cross-check** compiled out of release; and the **binary/assembly spot-check** for any
branch-free path, keyed to item 14's toolchain-pin trigger so it fires *structurally*, not on memory.
The designated-hot-path list gains gossip-message verification, capability-token validation, and the
consensus path — each registered in `HOT-PATHS.tsv`, so the item-6 gate fails a mesh/gossip diff that
lacks the artifacts.

---

## 2. Verified current state — grounded

- **The §4 checklist machinery exists and re-executes (item 6 DONE).** `docs/audits/hardening/CHECKLIST.md`
  (the four items + the §10/P7 re-execute-never-presence-check correction); `HOT-PATHS.tsv` (the
  machine-read manifest, `min_tests` floors, `mode=kani` rows); `scripts/hardening-gate.sh` re-executes
  oracles with parsed live counts; the dudect harness `kernel/src/ct_gate.rs` (a zero-dep Welch-t
  harness + reusable `ct_eq` primitive + planted-leak self-test) runs in release in the gate step. So
  item 24 does **not** build gate machinery — it registers rows and lands the crypto-specific artifacts.
- **The item-14 toolchain-bump trigger exists (DONE).** `rust-toolchain.toml` pins the channel;
  `toolchain-bump-gate` fires on a channel change and requires `docs/audits/toolchain/spot-check-<ver>.md`.
  Item 24's assembly spot-check for the mesh branch-free paths keys into this same trigger — "so it
  fires structurally, not on memory" (synthesis §17(c)).
- **The crypto primitives the surfaces reuse are KAT-gated.** `mesh.rs:28` uses `pq::dsa::{keygen,
  sign, verify}` (ML-DSA-65, byte-exact vs NIST ACVP); `mesh.rs:27` uses `event_log::sha3_256`. Item
  22 confirmed no invented crypto (audit §5). So item 24 hardens the *use* of these primitives on the
  gossip path, not the primitives themselves.
- **A known constant-time gap is already ledgered — the precedent for the dudect target.** The
  hardening checklist's own gap column (`CHECKLIST.md` "Honest gaps"): `kem.rs`/`hybrid.rs`
  variable-time tag compares are `KNOWN-RED(P91.2)` — "a dudect gate over them would honestly go RED."
  The mesh signature-compare on the gossip path is the same class of site; item 24 either uses the
  constant-time `ct_eq` (`ct_gate.rs`) from the start or ledgers the gap the same honest way.
- **The surfaces to sweep are named and their reuse-status is classified (item 22, DONE).** Audit §3:
  `SignedEntry::verify_sig` (`mesh.rs:132`), `MeshLog::verify_chain` (`mesh.rs:225`) — the signature/
  chain verification surface; capability-token validation and the consensus path are near-scratch
  above the log (audit §5), so their surfaces come into existence *with* item 23 — item 24 hardens
  them as they land.

---

## 3. Implementation plan — per surface, the four artifacts

For each of the three named surfaces (synthesis §9 item 24), land the four checklist artifacts and one
`HOT-PATHS.tsv` row:

1. **Gossip-message (signature) verification** — the `SignedEntry::verify_sig` path (`mesh.rs:132`)
   as exercised on the item-23 gossip-admission entry:
   - **Oracle:** a simple reference verifier (obvious-correct: verify signature, compare digest) as a
     test-only crate-internal module, differential-checked against `verify_sig` over a corpus of
     valid + adversarially-mutated signed entries (the 5 existing mesh red/green tests, `mesh.rs:280–386`,
     are the seed; extend to a corpus).
   - **dudect:** the signature/MAC **comparison** is the named constant-time site — route it through
     `ct_eq` (`ct_gate.rs`) with a planted-leak self-test proving the gate rejects a variable-time
     comparator. This is the FO-tag-compare precedent applied to the mesh path.
   - **debug-differential:** `debug_assert_eq!` the verifier result against the reference oracle per
     call, compiled out of release.
   - **asm spot-check:** the branch-free compare path gets a `docs/audits/toolchain/` entry keyed to
     item 14's channel-bump trigger.
2. **Capability-token validation** — the token-validation path (comes into existence with item 23's
   capability handling; `ports/agent/cap.rs`-adjacent):
   - **Oracle:** a reference token-validator retained as the differential target.
   - **dudect:** any secret-dependent token compare → `ct_eq`.
   - **debug-differential + asm spot-check** as above.
   - **Red-line note:** capability = auth-adjacent (`Resource::Auth` is red-line, `scope.rs:99`); the
     validation *mechanism* is item 24, the *authorization policy* is operator-gated (item 23 §7).
3. **The consensus path** (`mesh_consensus.rs`-analog, near-scratch above the log per item 22 §5):
   - Full four artifacts as it lands; if it has no secret-dependent compare, dudect records
     `N/A(no-secret-compare)` with reason (do not fake a dudect row).

**`HOT-PATHS.tsv`:** three new rows (`@ZONE` for gossip-verify, cap-validate, consensus) with the
per-item verdicts; the item-6 gate then **fails a mesh/gossip diff lacking the artifacts** (synthesis
§9 item 24 proof — "shown once with a deliberately artifact-less test diff"). All mesh rows carry
`--features pq` (mesh is `pq`-gated; the item-6 correction already runs pq KATs unconditionally, so
the pattern exists).

Zero new dependency (`ct_gate.rs` is zero-dep; reference oracles are test-only crate-internal modules).

---

## 4. Tests / proofs — 5-point hardening applicability (this item IS the checklist sweep)

Item 24's entire content is applying the checklist, so all four items apply per surface (§3). The
mapping to the 5-point standard:

- **Item 1 (oracle):** **YES, per surface** — the simple reference verifier/validator retained forever
  as the differential target (the strong form, since a live reference is buildable here — not the
  corpus-only weak form).
- **Item 2 (dudect):** **YES, at the signature/MAC/token compares** — `ct_eq` + planted-leak self-test.
  This is the item's headline: signature comparison is the classic CT site (synthesis §17(c)). Where a
  surface has no secret compare (e.g. a pure consensus tally), record `N/A(no-secret-compare)` honestly.
- **Item 3 (debug-differential):** **YES, per surface** — `debug_assert_eq!` against the oracle,
  release-compiled-out.
- **Item 4 (asm spot-check):** **YES, for branch-free paths** — keyed to item 14's channel-bump
  trigger (the structural-fire requirement, synthesis §17(c)). This is the *only* group in this
  blueprint set where item 4 is a live YES (the crypto surfaces have branch-free constant-time paths).
- **Item 5 (Kani/formal):** **optional / cross-item** — the mesh crypto arithmetic (if any new NTT/
  ring math is introduced) would inherit item 7's Kani discipline; but item 24 reuses the *existing*
  KAT-gated `pq::dsa`/`sha3_256` (item 22: no invented crypto), so no *new* Kani harness is owed unless
  a new primitive lands. Record `reuses-item-7-covered-primitives`.

---

## 5. Acceptance criteria (falsifiable) — synthesis §9 item 24

1. **The §4 CI job demonstrably fails a mesh/gossip diff lacking the artifacts** — shown once with a
   deliberately artifact-less test diff (exit 1), and green on a clean touch (exit 0). This is the
   item-6 gate's proven-RED-path discipline applied to the new mesh rows.
2. **Each gate's self-test catches its planted fault** — the dudect gate rejects a variable-time
   signature comparator (planted-leak self-test, the `ct_gate` precedent: leaky comparator detected at
   |t|≈300+, `ct_eq` |t|<1.3).
3. **Three `HOT-PATHS.tsv` rows registered** (gossip-verify, cap-validate, consensus) with honest
   per-item verdicts (dudect YES at compares, `N/A(no-secret-compare)` where none).
4. **Reference oracles retained** as test-only crate-internal modules, differential-checked.
5. **The asm spot-check keys into item 14's trigger** (a mesh branch-free path change on a compiler
   bump requires the spot-check artifact).
6. Zero new dependency; `cargo tree` unchanged.

---

## 6. Dependency gates

- **Depends on item 6** (the re-executing CI hardening machinery — **DONE**; `CHECKLIST.md`/
  `HOT-PATHS.tsv`/`hardening-gate.sh`/`ct_gate.rs`).
- **Depends on item 14** (the toolchain-pin structural trigger for the asm spot-check — **DONE**).
- **Depends on item 23** (the gossip-admission path + capability/consensus surfaces must **exist** to
  be swept; item 22 named them, item 23 builds them). Item 24 hardens what item 23 lands — sequenced
  after/with item 23.
- **Reuses (already hardened):** `pq::dsa` (ML-DSA-65, ACVP-KAT-gated), `event_log::sha3_256`, `ct_eq`
  — no new crypto primitive, so no new item-7-class Kani harness owed.

---

## 7. Open questions (operator ruling)

1. **Whether to fix the mesh signature compare to constant-time up front, or ledger it `KNOWN-RED`
   like `kem.rs`/`hybrid.rs` (P91.2).** The `CHECKLIST.md` gap precedent is explicit: item 6 did **not**
   silently fix crypto — the CT fix was "the gate's first customer, passing through the checklist it
   triggered." Item 24 should adopt `ct_eq` on the mesh compare *as its own first customer* (the clean
   path) — but if the mesh signature compare turns out to have a subtle FO-implicit-rejection shape
   like the KEM tag-compare, whether to fix-now-vs-ledger is the same call the operator faces on
   P91.2. Recommendation: fix via `ct_eq` (item 24 is a hardening pass, fixing is its job); flagged
   only because a discovered deep FO-shape might warrant the ledger-and-defer posture instead. Not a
   blocking operator gate; noted for consistency with the P91.2 handling.
2. **Capability-issuance authorization policy** (shared with item 23 §7) — what a gossip peer is
   authorized to assert is an operator ruling; item 24 hardens the *validation mechanism* regardless
   of the policy. Cross-referenced, not re-opened.
