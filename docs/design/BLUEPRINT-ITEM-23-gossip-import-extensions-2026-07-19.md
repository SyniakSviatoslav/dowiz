# BLUEPRINT — Item 23: Gossip extensions as `import_unit()` extensions (one importer, second source)

- **Date:** 2026-07-19 · **Tier:** parallel lane / §F mesh-gossip (roadmap §F) · **Status:** BLUEPRINT
  (planning artifact, no code) — **sequenced strictly after item 22** (roadmap §F, synthesis §17(b)
  addendum item 23; one of the two verbatim-preserved dependencies "item 23 after item 22").
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §F (mesh/gossip
  lane, lines 411–413), §0 mesh ruling (line 19, REIMPLEMENT ZERO-DEP);
  `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §17(b) (lines 302, the three gossip
  extensions), §9 addendum item 23 (line 313); **`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`
  (read in full — the item-22 scoping handoff)**; live source `kernel/src/decision/import.rs`,
  `kernel/src/decision/mod.rs`, `kernel/src/mesh.rs`.
- **Relationship to item 24:** item 24 sweeps the crypto surfaces this item touches (gossip-message
  verification, capability-token validation) under the §4 checklist; they are sequenced together but
  item 24 is a distinct hardening pass.

---

## 1. Scope / goal (one paragraph)

Build the three planned gossip extensions to the Decision Compiler — **epoch max-merge**,
**key_V-shaped import replay**, and **lineage-in-one-log** — as **extensions of the existing
`import_unit()` six-check pipeline**, admitting gossip-sourced units through the *same* call path,
**never a parallel importer** (synthesis §17(b), the P2 Correspondence verdict: "one admission
mechanism, extended to a second source of input (gossip peers), never a second importer built
alongside the first"). This is explicitly **not new design work** — each extension maps one-to-one
onto a check `import_unit()` already performs (synthesis §17(b)): epoch max-merge composes with the
existing epoch-no-downgrade check; key_V-shaped import replay *is* the independent-replay check
already coded; lineage-in-one-log *is* the existing lineage-parent-resolves check. The gossip
extension inherits the pipeline's ordering, short-circuit, and typed-rejection semantics rather than
reinventing them. Item 22 established the scoping input: `mesh.rs` is "mostly stub above the log
layer" — the signed-log primitive is reusable as-is; sync/consensus/capability/gossip start
near-scratch, and gossip admission extends `decision/import_unit()` (item 22 audit §5).

---

## 2. Verified current state — grounded

- **The six-check pipeline the extensions ride is coded and named.** `import.rs:8–16` enumerates:
  (1) size, (2) integrity `sha3_256(artifact)==content_id`, (3) instance-set pin, (4) **independent
  replay** (harvested set replayed + compared to local oracle, "ANY disagreement ⇒ reject" — the P06
  `key_V` shape, `import.rs:11–13`), (5) **epoch check** (never downgrade a Live unit, `import.rs:14`,
  reject variant `EpochNotNewer` `:55–57`), (6) **lineage parent** (prev must resolve in the one log,
  `import.rs:15`, reject variant `LineageParentMissing` `:58–59`). Entry point `import_unit()`
  (`import.rs:81`); degrade-closed reject "nothing persisted" (`import.rs:78`).
- **Epoch max-merge already exists as a proven join-semilattice.** `decision/mod.rs:385–395`
  `merge_meta` (higher epoch wins; equal-epoch tie rule at `:395`), with the property test
  `epoch_merge_is_semilattice` (`decision/mod.rs:550`) asserting commutative/associative/idempotent.
  Synthesis §17(b): "epoch max-merge must compose with — never violate — the existing
  epoch-no-downgrade check." So the extension is: gossip-received units' epochs merge via the
  *existing* `merge_meta`, and the merge result feeds the *existing* epoch check (5) — no new merge
  law.
- **`import_unit()` has no gossip source today — verified.** Grep for `gossip`/`epoch.*merge`/
  `max.merge` inside `kernel/src/decision/` returned zero hits (synthesis §17(b)) — the extensions are
  genuinely unbuilt, but the mechanism they extend is fully present.
- **The mesh log primitive to reuse is classified (item 22, DONE).** Audit §5: reuse as-is (keep,
  don't rewrite) `SignedEntry`, `MeshLog`, `MlDsaSigner`, the `Signer` seam (`mesh.rs:36,159,84,72`) —
  a real, adversarially-tested ML-DSA-65-signed append-only hash chain wired to the KAT-gated
  `pq::dsa` + `event_log::sha3_256`. `HubTransport` (`mesh.rs:260`) is the transport-firewall seam to
  build the reimplementation *against*. **But**: zero production callers (audit §3, all bench-only or
  stub), `pq`-gated (not default). So item 23 turns `mesh.rs`'s log primitive from bench-only into a
  real consumer via the import path.
- **The §0 ruling is REIMPLEMENT ZERO-DEP** (roadmap line 19): bebop's mesh serves as parity oracle
  only, never a linked dependency. Item 23's gossip admission is dowiz-native, extending `import_unit`.

---

## 3. Implementation plan — exact edits (extend, do not fork)

The load-bearing constraint: **grep must show a single admission entry point** after this lands
(synthesis §9 item 23 proof). No `import_gossip_unit()` parallel function.

1. **`kernel/src/decision/import.rs`** — extend `import_unit()` (or add a thin `source: ImportSource`
   parameter distinguishing local-import from gossip-peer) so a gossip-sourced unit flows through the
   **same six checks**. The three extensions map onto existing checks:
   - **Epoch max-merge (extension of check 5):** before the epoch check, merge the incoming meta's
     epoch against the local Live epoch via the existing `merge_meta` (`decision/mod.rs:389`); the
     merged result feeds check 5 — an epoch-downgrade attempt via max-merge is rejected by the
     *existing* `EpochNotNewer`, not a new guard.
   - **key_V-shaped import replay (already IS check 4):** gossip-sourced units get the *same*
     independent-replay-against-local-oracle treatment as locally-imported units (`import.rs:11–13`).
     "The author-hub's own GREEN is never the certificate" applies identically to a gossip peer's
     claim. **No new replay path** — the same one.
   - **Lineage-in-one-log (already IS check 6):** a gossip unit's `prev_content_id` must resolve in
     the *one* `EventLog`, same as a local import (`import.rs:15`). A lineage-orphan gossip unit is
     rejected by the existing `LineageParentMissing`.
2. **`kernel/src/mesh.rs` → the gossip transport seam.** Wire `MeshLog`/`SignedEntry` (reused as-is,
   item 22 §5) as the *source* of gossip-received signed units that feed `import_unit`. The signature
   verification (`SignedEntry::verify_sig`, `mesh.rs:132`, via `pq::dsa::verify`) happens **before**
   the unit enters the import pipeline — a bad-signature gossip entry never reaches `import_unit` (the
   transport-firewall discipline `HubTransport` `mesh.rs:250–265`). This turns the bench-only mesh log
   into a real production consumer (closing item 22's zero-caller finding).
3. **No parallel importer, no new merge law, no new reject taxonomy** beyond what `ImportReject`
   (`import.rs:45–62`) already carries — the extensions reuse `EpochNotNewer`/`ReplayDisagreement`/
   `LineageParentMissing`. If a genuinely gossip-specific reject is needed (e.g. `SignatureInvalid`
   pre-pipeline), it lives on the *transport* seam, not as a second importer.

Pure `std` + `pq` feature (mesh is `pq`-gated); the import path is default-build. Zero new dependency
(bebop is parity-oracle-only per §0).

---

## 4. Tests / proofs — 5-point hardening applicability

The import pipeline is already a designated concern; gossip admission is an extension of it, and the
crypto surfaces it touches are **item 24's** dedicated §4 sweep (cross-referenced, not duplicated
here). For item 23 itself:

- **Item 1 (oracle):** **YES.** The adversarial test suite (synthesis §9 item 23): an
  epoch-downgrade attempt via max-merge → rejected by the existing no-downgrade check; a
  replay-disagreeing gossip unit → rejected with **nothing persisted**; a lineage-orphan → rejected
  against the one EventLog; **all six existing import tests still green** (the extension must not
  regress the local-import path). These reuse the existing A1–A6 adversarial-case structure
  (`import.rs:53–59`) applied to a gossip source.
- **Item 2 (dudect):** the **signature-verify comparator** is secret-adjacent (a MAC/signature
  compare is the classic constant-time site) — but this is `pq::dsa::verify`'s existing property and
  **item 24's** named work (synthesis §17(c), §9 item 24). For item 23, record `deferred-to-item-24`
  in the manifest gap column (do not silently claim it).
- **Item 3 (debug-differential):** the epoch-merge result cross-checked against `merge_meta`'s
  semilattice property in debug builds (the per-call reference exists).
- **Item 5 (formal):** the epoch max-merge composing with the no-downgrade check is a candidate for
  the **item-10 TLA+ `DecisionImport` spec extension** (the `EpochNoDowngrade` invariant must hold
  when the source is a gossip peer, not only a local import) — noted as a cross-item tie, not a new
  Kani harness. Native adversarial tests (item 1) carry the primary guarantee.
- **Item 4 (asm):** **N/A** for the import logic (no branch-free path); the crypto surfaces' asm
  spot-check is item 24 / item-14-triggered.

---

## 5. Acceptance criteria (falsifiable) — synthesis §9 item 23

1. **Adversarial tests green:** epoch-downgrade-via-max-merge rejected by the existing no-downgrade
   check; replay-disagreeing gossip unit rejected with nothing persisted; lineage-orphan rejected
   against the one EventLog.
2. **All six existing import tests still green** (no regression to the local-import path).
3. **Grep shows a single admission entry point** — no `import_gossip_unit` / parallel importer; the
   gossip source flows through `import_unit` (the P2 Correspondence proof).
4. **A bad-signature gossip entry never reaches `import_unit`** (transport-firewall: signature verify
   before the pipeline; demonstrated).
5. **`mesh.rs`'s signed-log primitive gains a real production caller** (closing item 22's zero-caller
   finding) — reused as-is, not rewritten.
6. Zero new dependency (§0 REIMPLEMENT ZERO-DEP; bebop parity-oracle-only).

---

## 6. Dependency gates

- **Sequenced strictly after item 22** (roadmap §F, synthesis §17(b) addendum — verbatim-preserved,
  never resequenced). Item 22 is **DONE** (audit filed) — this gate is satisfied; item 22's §5
  scoping handoff is the direct input.
- **Reuses (already exist):** `import_unit()` + the six checks, `merge_meta`, the `mesh.rs` signed-log
  primitive (item 22-classified reusable), `pq::dsa::verify`.
- **Feeds item 24:** the crypto surfaces item 23 exposes (gossip-message verification, capability-token
  validation, the consensus path) are item 24's §4-checklist sweep — sequenced together; item 23 must
  not claim the CT/dudect coverage item 24 owns.
- **Depends additionally on:** the reimplementation scoping (roadmap §F: "reimplementation work per the
  §0 ruling → item 23") — sync/consensus/capability admission above the log start near-scratch (item
  22 §5); item 23's *gossip-admission* half is the piece that extends `import_unit`, the rest of the
  protocol layer is separate reimplementation work.

---

## 7. Open questions (operator ruling)

1. **The sibling-dependency-vs-reimplement fork is already ruled (§0: REIMPLEMENT ZERO-DEP)** — so the
   §17(d) `NEEDS-OPERATOR-DECISION` that item 22 would have surfaced is **closed**; no open operator
   gate remains on *approach*. Recorded for completeness: item 23 proceeds dowiz-native.
2. **Capability issuance semantics.** Item 22 §5 lists "capability issuance" as near-scratch above the
   log. Whether gossip-admitted units may *carry* capability tokens (and how those tokens are
   validated in the import path) touches the red-line auth surface — the *validation mechanism* is
   item 24's crypto sweep, but the *policy* (what a gossip peer is authorized to assert) is adjacent to
   the operator-gated capability model (`ports/agent/cap.rs`, `RedLinePolicy`). Flagged: if item 23's
   gossip units can assert capabilities, the authorization policy is an operator ruling, not an
   importer-mechanics decision. Default: gossip units carry no capability-issuance authority until
   ruled.
