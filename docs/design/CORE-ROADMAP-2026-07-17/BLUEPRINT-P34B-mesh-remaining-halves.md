# BLUEPRINT P34B — Finish the planned mesh halves: per-node storage, CRDT fence, iroh, ML-KEM KAT (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). This phase IS `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §10.5.2's **P34B** — the four mesh-real units (MESH-06/08/09-iroh-half/13) that §10.5.2 recorded
> as "genuinely 0-30%". Sibling of `BLUEPRINT-P34-mesh-kernel-wiring.md` (same folder, same
> discipline); P34's scope (kernel wiring) is NOT re-litigated here — P34B is explicitly the
> remaining halves P34 did not cover. Source blueprint reused, not re-derived:
> `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` (design source ONLY — its status column is
> stale, per P34 §0 row 20; status below comes from this pass's live verification).
>
> **Headline ground-truth finding of this pass (leads everything below):** §10.5.2's "genuinely
> 0-30%" is stale in THREE of four units. Live: MESH-06's per-node store is **substantially
> built** in dowiz-kernel (`kernel/src/event_log.rs` names itself "MESH-06" in line 1; durable
> `FileEventStore` exists in `hydra.rs:920`); MESH-08's compile-fence is **built and CI-wired**
> (`ci/crdt-fence/` + `ci.yml:88-89`); MESH-13's KAT + zeroize contract is **built** in
> `proto-crypto/src/pq_kem.rs` (the "zeroize absent from the entire workspace" claim is stale);
> and MESH-09's QUIC carrier is a **real quinn transport**, not a stub (`iroh_transport.rs:1`).
> What genuinely remains is smaller and different than §10.5.2 believed: **written decisions,
> stale-premise corrections, one dowiz-side fence extension, a dual-ML-KEM single-authority
> consolidation, and an epoch-snapshot unit** — this blueprint is those, precisely.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `f9b2eb9bb` and `bebop-repo` `main` @
`e56ba6a35258ced76752510625511f37a6367a77` (= the `openbebop` remote's `main`, ls-remote
confirmed). All bebop paths relative to `/root/bebop-repo/`, dowiz paths relative to
`/root/dowiz/`.

| # | Claim | Fresh `file:line` (this pass) | Inherited claim | Status |
|---|---|---|---|---|
| 1 | MESH-06 content-addressed per-node event log EXISTS in dowiz-kernel | `kernel/src/event_log.rs` (936 lines): module doc `:1` literally "MESH-06 — per-node pgrust content-addressed event-log (local-first + sync)"; `MeshEvent` `:134`; `event_id() = SHA3-256(prev‖actor_pubkey‖actor_seq‖payload)` `:148` (exactly the MESH-06 idempotency spec); `trait EventStore` `:182` with typed durability barrier (`insert -> Result<(), StoreError>`, "a lost write is now typed, never a silent success"); `MemEventStore` `:209`; `EventLog<S: EventStore>` `:282`, decide-before-network (`:279-281`: "append/commit_after_decide runs **before** any network IO … the network layer never re-runs decide — it only verifies signatures"); `FaultyStore` failure-injection double `:540`; 13 `#[test]`s | §10.5.2: MESH-06 "genuinely 0-30%", "a written decision + blueprint exists" is the DoD | **STALE-LOW** — the mechanism is built; the missing halves are the durable-store *selection decision*, the stale-premise correction (row 3), and epoch compaction (row 5) |
| 2 | Durable per-node store EXISTS | `kernel/src/hydra.rs:904` "G4 — std-only durable append-only event store"; `pub struct FileEventStore` `:920` (append one JSON line + `fsync` per insert `:1037`, crash-safe, replay-on-open, forward-tolerant corrupt-line skip); `impl EventStore for FileEventStore` `:1029`. Referenced from `event_log.rs:180` | not in any inherited cite | **NEW** — the durable half of MESH-06 already exists; it is wired for the hydra loop, not yet named as THE per-node mesh store |
| 3 | 🔴 Stale pgrust-per-node premise, live in two files | `kernel/src/event_log.rs:9-10`: "# Store abstraction (pgrust stand-in) — The production node persists this log in **pgrust** (the WAVE0 DK-05 Postgres client…)"; `deploy/README.md:4`: pgrust described as "the per-node source-of-truth" | §10.5.2 P34B DoD-4: "conflating the two is the failure mode this DoD item exists to prevent" | **CONFIRMED live** — the conflation §10.5.2 warned about is not hypothetical; it is committed text in both repos' docs. §2.1 retires it |
| 4 | Zero pgrust in bebop-repo | `grep -rln "pgrust" /root/bebop-repo --include="*.rs" --include="*.toml"` → 0 hits | §10.5.2 DoD-4: "zero pgrust references anywhere in bebop-repo" | **MATCH** |
| 5 | No snapshot/compaction on the mesh log | `grep -n "snapshot\|compact\|epoch" kernel/src/event_log.rs` → 0 hits; P34 §4.2 names the ~10⁶-leaf break point and hands it "to P34B's per-node storage scope by design" | P34 §4.2 handoff | **CONFIRMED open** — the one genuinely unbuilt MESH-06 mechanism |
| 6 | pgrust's real position, both repos | dowiz `kernel/Cargo.toml:35` `pgrust = ["dep:sqlx", "dep:tokio"]` — the OPTIONAL living-memory SQL adapter (`:30` comment), default-off; hub deployment config `deploy/pgrust.{service,toml,env}` (DK-05, systemd, hub host); hub tenant-schema rebuild = `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (SCOPE/PROPOSAL, council-gated red-line, "must NOT be applied while the server tier is absent") | brief: "pgrust is positioned as hub-tier" | **MATCH** — every live pgrust artifact is hub-facing; none is per-node |
| 7 | The sqlless-main-path ruling already exists | `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md:108-113` (operator correction, 2026-07-16): "the spectral/sqlless approach (content-addressed BlockStore + JSONL FileEventStore) is the MAIN storage/retrieval path … with **pgrust as the uniform SQL-fallback/backup target, not SQLite**" | — | verified — §2.1's decision is the mesh-storage instantiation of this standing ruling, not a new doctrine |
| 8 | MESH-08 fence BUILT + CI-wired | `ci/crdt-fence/src/lib.rs`: `GUARDED_CRATES = ["bebop2-core","bebop","bebop-delivery-domain","bebop-mesh-node"]` `:15-20`; `CRDT_PATTERN = r"(?i)crdt\|automerge\|cr-sqlite\|merge-crdt"` `:24`; red-proof unit tests over injected metadata `:221-247` (`red_injected_automerge_dep_is_offense`, `red_injected_cr_sqlite_dep_is_offense`); `--metadata FILE.json` mode in `main.rs:4-5`; CI step `.github/workflows/ci.yml:88-89` (`sovereign-guards` job) | §10.5.2 DoD-1: "a build-level mechanism … red-proof committed" reads as unbuilt | **STALE-LOW** — built, wired, red-proven bebop-side. Open: the DOWIZ side (row 9) |
| 9 | The fence does not cover dowiz | `GUARDED_CRATES` contains no dowiz crate; dowiz `.github/workflows/ci.yml` has no crdt-fence invocation; P34's `dowiz-mesh-adapter` (kernel-owned state host) will join neither set unless added | — | **CONFIRMED open** — the real remaining MESH-08 work (§3.2) |
| 10 | MESH-09 QUIC carrier is REAL | `bebop2/proto-wire/src/iroh_transport.rs` (725 lines): `:1` "QUIC transport — real node-to-node carrier (pure-Rust quinn/rustls)"; `:7-10` "Why quinn and not iroh": `iroh` crate conflicts with the `ed25519-dalek` pin + offline-build requirement; `:23` "iroh DHT hole-punching is OUT of scope here"; real `quinn::{Endpoint, ClientConfig, ServerConfig}` `:33-34` | §10.5.2 done-inventory omits it; EXCELLENCE review: "iroh = working quinn+rustls QUIC (not a stub)" | **MATCH (EXCELLENCE)** — the carrier exists. The *iroh-crate adoption* is what was deferred |
| 11 | 🔴 The stale `iroh` feature + comment persist | `proto-wire/Cargo.toml:51` `iroh = []` (empty); `:41-48` comment still claims "`IrohTransport` is a compile-clean stub today" — false vs row 10 | §10.5.2 DoD-3: "an empty stub feature persisting is a fail" | **CONFIRMED live** — the DoD-3 fail state exists in both the feature list and the (now-wrong) comment |
| 12 | insecure-tls interplay (P36's, cited not owned) | `proto-wire/Cargo.toml:50` `default = ["insecure-tls"]`; `iroh_transport.rs:156` `InsecureAcceptAny`, `:216-228` verifier selection by feature, `:205` "Remaining follow-up: a prod operator-cert" | §10.5.2: P34B DoD-3 "hard-depends on P36 DoD-2" | **CONFIRMED** — §3.3 sequences behind P36 DoD-2, exactly as chartered |
| 13 | MESH-13 KAT + CT + zeroize BUILT (proto-crypto) | `bebop2/proto-crypto/src/pq_kem.rs` (735 lines): module doc `:11-27` states the "MESH-13 contract" verbatim — FIPS 203 NTT-domain, Algorithms 16/17/18; `ml_kem_external_ACVP_KAT_bit_exact` `:620` (pk/sk/ct/ss byte-exact vs external reference vectors from the dowiz-pq kernel ML-KEM-768, two vector sets `:592-640`); implicit-rejection constant-time decaps + dudect-style `ml_kem_constant_time`; `MlKemSecretKey` zeroizing-on-`Drop` wrapper `:474` + compile-time zeroize proof `:722-732`. Exercised by `cargo test --workspace` (`ci.yml:24`) | §10.5.2 DoD-2: "passes official FIPS-203 KAT vectors, or is replaced by one that does; zeroize applied (currently absent from the entire workspace)" | **STALE** — the KAT'd, zeroizing implementation exists. Open: rows 14-15 |
| 14 | 🔴 DUAL ML-KEM — two from-scratch impls in one workspace | `bebop2/core/src/pq_kem.rs`: schoolbook coefficient-domain production mult (`:289-291`, self-described "FIPS-203-compliant alternative to the NTT (FIPS 203 §6 permits any algorithm)"), test-only schoolbook reference + NTT cross-assertions `:33-40`, NO external KEM KAT (only the Keccak `fips202_kat` `:772`) — vs proto-crypto's KAT'd NTT impl (row 13). No parity test binds them; no production consumer exists for either (`grep -rln pq_kem bebop2/*/src/*.rs` → core-internal + proto-crypto only; the hybrid identity uses ML-DSA, not KEM) | not in any inherited cite | **NEW finding** — same dual-authority class as P36's dual field-math and P34 §0 row 18's dual legality tables. §2.3 resolves which one is canonical |
| 15 | zeroize coverage boundary | zeroize exists ONLY in proto-crypto (`grep -rn zeroize bebop2/` → 4 hits, all `proto-crypto`); core secret material (`pq_dsa` sk, `sign.rs` Ed25519 sk, `x25519`, `rng` DRBG seeds, `at_rest` keys) has none | §10.5.2's "absent from the entire workspace" | **DRIFT** — partially closed; the open half is core's named secret types (§3.4) |
| 16 | BPv7 half of MESH-09 done; discovery exists | `proto-wire/src/bpv7.rs` (611 lines, per P34 §0-style done inventory in §10.5.2); `proto-wire/src/discovery.rs` present | §10.5.2 P34 done-inventory | **MATCH** — only the "iroh half" is P34B's |
| 17 | Regression-ledger substrate | dowiz `docs/regressions/REGRESSION-LEDGER.md` exists; bebop-repo has NO `docs/regressions/` | P34 §5 uses the dowiz ledger | verified — bebop-side rows need a home (§5; created by P36, reused here) |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — what P34B owns and what it deliberately does NOT own

**P34B's single sentence:** close the four planned-but-unfinished mesh halves by (i) writing the
per-node storage decision that retires pgrust-per-node and names the existing event-log stack as
the store (+ the one missing mechanism, epoch snapshots), (ii) extending the already-built CRDT
fence to the dowiz side, (iii) formally retiring the empty `iroh` feature against the real quinn
carrier, and (iv) collapsing the dual ML-KEM to a single KAT-gated authority with zeroize on
named secret types.

**P34B owns (build items §3):**

| Item | §10.5.2 DoD | Content |
|---|---|---|
| V-1 | DoD-4 (MESH-06) | The per-node storage DECISION (§2.1) + stale-premise corrections (§0 row 3) + epoch-snapshot unit (§0 row 5) |
| V-2 | DoD-1 (MESH-08) | Dowiz-side fence coverage: `GUARDED_CRATES` += dowiz crates; fence invoked from dowiz CI via `--metadata`; red-proof re-run |
| V-3 | DoD-3 (MESH-09 iroh half) | Retire `iroh = []` + correct the stale stub comment with a dated decision note; name hole-punch/relay as an explicit deferred unit with trigger |
| V-4 | DoD-2 (MESH-13) | Single-authority ML-KEM ruling + parity proof + zeroize extension to core's named secret types (or per-type dated deferral) |

**P34B does NOT own (anti-scope, binding — each with its owner):**

- **P34's kernel wiring** — the adapter crate, CI gate, vocabulary/claim/matcher/solo-island
  proofs are P34's W-1..W-6; P34B does not restate or amend them. V-2 *consumes* the
  `dowiz-mesh-adapter` crate P34 creates; it does not define it.
- **P36's regressions** — the insecure-tls default flip and the `no_std` wasm32 fix are P36
  DoD-1/DoD-2. V-3's hardened posture *depends on* P36 DoD-2 (§10.5.2's stated edge) and must
  not fix it here.
- **The hub pgrust tenant rebuild** — `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` is a
  SEPARATE, council-gated, red-line track (RLS tenant boundary, NOBYPASSRLS inversion, needs a
  server tier). §2.1 cites and *distinguishes* it; nothing here duplicates, advances, or gates
  on it. Treating it as satisfying MESH-06 is the failure mode §10.5.2 DoD-4 names.
- **No new crypto primitives** beyond FIPS-203 compliance of the existing ML-KEM path
  (§10.5.2 anti-scope, restated verbatim). V-4 consolidates existing impls; it introduces none.
- **MESH-07 Sync·Pull / MESH-10 WSS / MESH-11 revocation / MESH-12 genesis** — done or
  operator-gated elsewhere (P34 done-inventory; MESH-12 remains the operator's HUMAN gate).
- **P37's node binary** — MESH-06's durable-store *selection at process boot* happens in the
  node binary that P37/DELIVERY owns; V-1 delivers the seam, the impl, and the documented
  selection rule, and names the binary wiring as a P37 handoff (not silently deferred).

---

## 2. Predefined types, constants & the three written decisions (standard item 4)

### 2.1 THE MESH-06 DECISION — per-node storage is event-log-first and pgrust-free

**Decision (the §10.5.2 DoD-4 "written decision", stated falsifiably):**

> The per-node local-first store IS the already-built dowiz-kernel stack:
> `EventLog<S: EventStore>` (content-addressed `event_id = SHA3-256(prev‖actor_pubkey‖
> actor_seq‖payload)`, decide-before-network, idempotent no-TTL dedup — `event_log.rs:134-302`)
> over `hydra::FileEventStore` (append-only JSONL + per-insert `fsync`, replay-on-open —
> `hydra.rs:920-1042`). **No pgrust process runs on a node.** pgrust is hub-tier only, in
> exactly two roles: the kernel's optional living-memory SQL adapter (`kernel/Cargo.toml:35`)
> and the council-gated hub tenant-schema rebuild
> (`BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` — CANONICAL-repo hub storage, a
> related-but-distinct concern this decision explicitly does NOT touch).

**Why this is structurally forced, not stylistic (hazard-safety-as-math, item 6):**

1. **The per-node write path has no SQL-shaped need.** Every per-node read is either (a) the
   fold of the log (projections = deterministic replay of `decide`-admitted events — the same
   purity that gives P34 §4.3 its Snapshot-Re-entry property), or (b) a point lookup by
   content-id (`EventStore::get`, `event_log.rs:190`). Neither requires a query planner, SQL
   surface, or cross-table joins. A per-node pgrust would add a process, a socket, an auth
   surface, and an SQL-injection class to a device whose entire query language is "replay and
   fold" — surface with zero capability gain (minimal-sufficient-isolation, the same doctrine
   as DK-05's own README).
2. **Multi-tenant RLS — pgrust's actual value — is structurally absent per node.** MESH-06's
   own blueprint states the invariant: "тільки-own-operator-data,
   no-cross-node-query-surface-by-construction" (`BLUEPRINTS-MESH-REAL.md:69`). A store that
   holds exactly one operator's data has no tenant boundary to enforce; RLS enforcement is the
   HUB's problem, which is precisely the pgrust tenant-rebuild blueprint's scope.
3. **The standing ruling already decided the storage class.** The 2026-07-16 operator
   correction (§0 row 7) fixed the MAIN path as content-addressed/append-only sqlless with
   pgrust as the *uniform SQL-fallback/backup target*. A node is the main path; a hub archive
   is the fallback tier. This decision instantiates that ruling for mesh storage — it does not
   invent a new doctrine.
4. **Phone-class nodes.** Courier devices are the node floor (§10.5.2 P35: "KVM ≠ телефон").
   `FileEventStore` is `std::fs` + SHA3 — no daemon, no listener, no migration runner. The
   pgrust deploy unit (`deploy/pgrust.service`) is a systemd server artifact by construction.

**DECART table (one line each, per the standing rule):**

| candidate | verdict | reason |
|---|---|---|
| pgrust-per-node (MESH-06's 2026-07-13 phrasing) | **REJECTED** | SQL/process/auth surface with zero per-node query need; RLS value is null per node; contradicts the 2026-07-16 sqlless-main-path ruling |
| rusqlite/SQLite | REJECTED | standing rejection, already corrected once in this exact blueprint (`BLUEPRINTS-MESH-REAL.md:108-113`) |
| bebop2-core `EventLog`+`AtRestStore` as the dowiz node store | REJECTED-for-now | would add a `bebop2-core` dep to `dowiz-kernel`, violating P34's kernel-manifest-untouched isolation (P34 §4.3); at-rest encryption is a named follow-up (§3.1c), interim posture = OS-level device encryption |
| existing `EventLog<S>` + `FileEventStore` (+ epoch snapshots, §3.1b) | **CHOSEN** | already built, already tested, already fail-closed typed-durability; the only genuinely missing mechanism is compaction |

**Consequence:** MESH-06 is ~85% "confirm and correct", ~15% "build" (the epoch-snapshot unit).
That is the honest answer to §10.5.2's DoD-4 — the decision existed nowhere in writing, the
stale opposite claim existed in two places (§0 row 3), and one mechanism is real work.

### 2.2 New named types/constants (everything new, up front — no magic numbers)

```rust
// ── kernel/src/event_log.rs — the ONLY new production types P34B introduces ──

/// Epoch-snapshot break point (§0 row 5; P34 §4.2's measured handoff: ~10⁶
/// leaves ≈ 3 months at 1 000 orders/day × ~10-11 frames/order). Crossing it
/// triggers snapshot-and-archive, never deletion (demote-never-delete).
pub const EPOCH_SNAPSHOT_LEAF_THRESHOLD: usize = 1_000_000;

/// A sealed epoch: the content-id of the chain tip at seal time plus the
/// number of events folded into it. The NEXT segment's first event uses
/// `prev = tip` — the chain is continuous ACROSS segments by construction
/// (same hash rule as event_id itself; no new hash domain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochSeal {
    pub tip: [u8; 32],
    pub events: u64,
}

/// Archive policy for sealed segments. Only one variant today — the archive
/// file stays on-device; hub upload (pgrust SQL-fallback tier, §2.1) is the
/// named FUTURE variant, added only when the hub archive lands (trigger, not
/// scaffold).
pub enum SealedSegment {
    /// `<log-path>.epoch-<n>` — same JSONL format, immutable, fsynced once.
    LocalArchive(std::path::PathBuf),
}

impl<S: EventStore> EventLog<S> {
    /// Seal the current segment: fsync + rename to the archive name, start a
    /// fresh segment whose genesis prev = seal.tip. Returns the seal. Pure
    /// bookkeeping — NO event is dropped, mutated, or re-hashed.
    pub fn seal_epoch(&mut self) -> Result<EpochSeal, StoreError>;
}
```

```rust
// ── ci/crdt-fence/src/lib.rs (bebop-repo) — the V-2 additive diff, verbatim ──
pub const GUARDED_CRATES: &[&str] = &[
    "bebop2-core",
    "bebop",
    "bebop-delivery-domain",
    "bebop-mesh-node",
    "dowiz-kernel",        // NEW (V-2): kernel-owned state, fence-guarded from the dowiz CI side
    "dowiz-mesh-adapter",  // NEW (V-2): P34's host crate — kernel state transits it
];
```

### 2.3 The MESH-13 single-authority ruling (V-4's spec)

**Decision: `bebop2/proto-crypto/src/pq_kem.rs` is THE ML-KEM-768 authority.** Grounds: it is
the only impl with (a) external bit-exact KAT (`:620`, two vector sets vs the independent
dowiz-pq reference — P7-grade external certification, not self-tested), (b) FIPS 203
Algorithm 16/17/18 NTT-domain arithmetic (wire-interoperable with any conforming impl, `:4-9`),
(c) dudect-style CT gate, and (d) the zeroizing secret-key type. `bebop2-core/src/pq_kem.rs`'s
schoolbook impl relies on the FIPS 203 §6 "any algorithm" clause — permitted, but never
externally KAT'd (§0 row 14) and coefficient-domain (its encapsulation keys are NOT
wire-compatible with NTT-domain implementations' serialized forms unless converted — the exact
interop trap MESH-13 was written to close).

Disposition of the core impl (one of two, decided in V-4, not left open): **(i) demote to
test-reference** — move under `#[cfg(test)]` next to its own schoolbook test harness, keeping
the NTT-vs-schoolbook cross-assertions (`:33-40`) as a property test of the authority; or
**(ii) delete** if the cross-assertions are re-anchored onto proto-crypto fixtures. Option (i)
is the default (reuse-first; the cross-check has independent verification value). Forbidden
third state: both impls staying reachable in production namespaces.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 V-1 — MESH-06: the decision, the corrections, the epoch unit (DoD-4)

**(a) Stale-premise corrections (append-only dated notes, aebbbe199 style).** Two edits:
`kernel/src/event_log.rs:9-10` — replace the "pgrust stand-in / production node persists this
log in pgrust" paragraph with the §2.1 decision (three sentences + a pointer to this blueprint);
`deploy/README.md:4` — "the per-node source-of-truth" → "the HUB-tier SQL-fallback/archive
store; the per-node source-of-truth is the kernel event-log (P34B §2.1)". RED: grep finds the
stale claims today (§0 row 3). GREEN falsifier:
`grep -rn "per-node source-of-truth" deploy/README.md` and
`grep -n "production node persists this log in" kernel/src/event_log.rs` both empty.

**(b) Epoch-snapshot unit.** Spec = §2.2's types. Tests in `event_log.rs`'s test mod:

1. `epoch_seal_preserves_chain_continuity` — append N events, `seal_epoch()`, append M more;
   assert the first post-seal event's `prev == seal.tip` and that replaying
   archive-then-live reproduces the identical tip as an unsealed N+M log (Snapshot-Re-entry
   as an equality of hashes, not a prose claim).
2. `epoch_seal_loses_nothing` — every content-id inserted before the seal remains `get`-able
   (from the archive segment index) after it; `len()` across segments is conserved.
3. `epoch_threshold_is_advisory_not_automatic` — crossing `EPOCH_SNAPSHOT_LEAF_THRESHOLD`
   does NOT auto-seal (sealing is an explicit call by the node's maintenance path — no hidden
   background mutation of money-adjacent state; same explicitness discipline as the pgrust
   blueprint's "migrate() EXPLICIT, never called by default").

**Adversarial (designed to break V-1):** (i) kill-mid-seal — `FaultyStore`-style injection
(reuse `event_log.rs:540`) fails the rename step; assert the log recovers on reopen to the
PRE-seal state with zero loss (the archive rename is the commit point; a torn seal is a no-op,
never a half-archive); (ii) tampered archive — flip one byte in a sealed segment file; replay
must refuse at the exact seq (content-id mismatch), never silently skip (distinguished from the
forward-tolerant *short-line* skip, which only applies to a torn TAIL write).

### 3.2 V-2 — MESH-08: extend the fence to the dowiz side (DoD-1)

Spec: §2.2's `GUARDED_CRATES` diff (bebop-side edit — repo-routing rule: committed in
`/root/bebop-repo`, pushed to `openbebop`). Consumption: a new step in dowiz's `mesh-adapter`
CI job (the job P34 §3.1c creates — additive to it, after P34 lands):

```yaml
      # V-2 (P34B): CRDT fence over the dowiz closure, reusing bebop's fence binary
      - run: cargo metadata --format-version 1 --manifest-path dowiz/mesh-adapter/Cargo.toml > /tmp/mesh-meta.json
      - run: cargo run --manifest-path bebop-repo/Cargo.toml -p ci-crdt-fence -- --metadata /tmp/mesh-meta.json
```

RED→GREEN: the fence's own injected-metadata unit tests (`ci/crdt-fence/src/lib.rs:221-247`)
extend with `red_injected_crdt_into_dowiz_adapter_is_offense` — a synthetic metadata blob where
`dowiz-mesh-adapter` reaches `automerge`; RED before the `GUARDED_CRATES` diff (the crate name
is not guarded, offense list empty), GREEN after. **Adversarial:** the clean-graph test
re-runs against the REAL post-P34 `mesh-adapter` metadata and must pass — proving the guard
detects injection without false-positives on the live graph (teeth in both directions, same
discipline as P34 §3.2's table-sabotage check).

**Honest limitation, named not papered over (item 14):** the fence is a *dependency-name* gate
(`CRDT_PATTERN`, `:24`) — a hand-rolled in-tree merge function would not match it. The deeper
gate is structural: kernel state is reachable only through `decide`/fold (P34's firewall +
purity proofs), so a hand-rolled merge must bypass the facade — the class P34's W-6 red-proof
guards. The two gates compose; neither is claimed to be the other.

### 3.3 V-3 — MESH-09 iroh half: formal retirement with a dated decision note (DoD-3)

§10.5.2 DoD-3 offers two exits: implement or formally retire. Live truth (§0 rows 10-11) shows
the third, unlisted state: *the capability was implemented under a different name while the
feature flag and comment rotted*. Resolution — retire the FLAG, keep the CARRIER:

1. Delete `iroh = []` from `proto-wire/Cargo.toml:51`. Replace the stale `:41-48` comment block
   with a dated note: the QUIC carrier is REAL (`iroh_transport.rs`, quinn/rustls, since the
   clean-slate publish); the `iroh` *crate* adoption is RETIRED (dalek-pin conflict + the
   offline-build floor — grounds already recorded in `iroh_transport.rs:7-10`, now promoted to
   the manifest where the stale claim lived); NAT-traversal/hole-punch/relay (iroh's genuine
   value-add, explicitly out of scope at `:23`) is a NAMED deferred unit — trigger: first
   real-world two-node deployment behind distinct NATs fails to connect directly.
2. Optional-but-recommended honesty rename considered and REJECTED (DECART one-liner): renaming
   `iroh_transport.rs` → `quic_transport.rs` touches every import for a cosmetic gain and
   breaks blame history; the module's own header already states the truth. A doc-comment
   pointer suffices.
3. Hardened posture: unchanged here — the carrier's verifier selection (`:216-228`) flips when
   P36 DoD-2 lands. V-3 adds ONE test tying the retirement down:
   `retired_iroh_feature_absent` — `cargo metadata` asserts `bebop-proto-wire` exposes no
   `iroh` feature. RED today (feature present), GREEN after.

**Adversarial:** re-introduce `iroh = []` in a scratch manifest copy → the test must fail —
proving the gate pins the retirement rather than merely documenting it (an empty stub feature
re-entering is §10.5.2's exact fail condition, now mechanically impossible to reintroduce
silently).

### 3.4 V-4 — MESH-13: single ML-KEM authority + zeroize extension (DoD-2)

Spec: §2.3's ruling. Steps, RED-first:

1. **Parity pin before any demotion** (verify-before-delete, standing directive): a test
   generating K=16 deterministic seeds, running keygen/encaps/decaps through BOTH impls, and
   asserting shared-secret agreement where representations meet (core's coefficient-domain
   values converted through its own NTT round-trip helpers `:33-40`). If parity FAILS, stop —
   that is a finding, not a cleanup (the impls were never proven equivalent; §10.5.2's
   "replaced by one that does [pass KAT]" branch activates instead).
2. **Demote** `bebop2-core::pq_kem`'s public keygen/encaps/decaps to `#[cfg(test)]`
   (option §2.3-i). Grep-falsifier: exactly one non-test `pub fn encaps` in the workspace.
3. **Zeroize extension, bounded list** (no open-ended "everything"): `pq_dsa` secret key,
   `sign.rs` Ed25519 secret scalar + nonce buffers, `x25519` secret, `rng` DRBG seed buffer,
   `at_rest` per-hub key — each either wrapped in a `Drop`-zeroing type (the `MlKemSecretKey`
   pattern, `proto-crypto/src/pq_kem.rs:474` — hand-rolled, zero new deps) or given a dated
   per-type deferral note with reason. RED: a compile-time zeroize-proof test per wrapped type
   (the `:722-732` pattern); the list itself is the checklist — no type may be silently absent.
4. **CI**: no new job — `cargo test --workspace` (ci.yml:24) already runs proto-crypto's KAT;
   the falsifier is the KAT test's continued existence by name
   (`grep -n ml_kem_external_ACVP_KAT_bit_exact` non-empty), enforced by the live-test claim
   lint already in the sovereign-guards job (`ci.yml:92`).

**Adversarial:** (i) KAT-sabotage teeth — flip one byte in a REF vector in a scratch copy; the
KAT must fail (proving it compares, not just runs); (ii) zeroize teeth — a test reads the
secret buffer's memory after `drop` in a controlled harness and asserts zeros (already the
`:722-732` pattern — extended to each newly wrapped type); (iii) 🔴 red-line discipline: every
V-4 code change is crypto-adjacent → operator-gated commit + independent review per the
standing crypto-safe-first-pass precedent (3-model review; correctness over speed). V-4 is the
one P34B item that MUST NOT be merged on agent authority alone.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 4.1 Hazard-safety as math (item 6) — unsafe states and why each is unreachable

- **Split-brain money via CRDT merge:** unreachable at two levels after V-2 — the dependency
  graph cannot reach a CRDT-merge crate from any kernel-owned-state crate (fence, both repos),
  and a hand-rolled merge cannot reach kernel state except through `decide` (P34 firewall).
  Reaching the unsafe state requires defeating a CI dep-graph gate AND the compilation
  firewall simultaneously.
- **Divergent-store double-apply:** `event_id = H(prev‖actor_pubkey‖actor_seq‖payload)` makes
  a replayed event a structural no-op (`contains` before `insert`, `event_log.rs:183-188`);
  the same 64→256-bit distinction as P34 §4.1's birthday analysis does not arise — the full
  256-bit id is the key, collision horizon ≫ any node lifetime.
- **Torn-write state loss:** `insert` returns `Result` over the fsync barrier ("a lost write
  is now typed, never a silent success", `:184-186`); a crash between fsync and network emit
  re-offers the event on replay (at-least-once locally, exactly-once by content-id) — the
  failure direction is duplicate-offer-refused, never silent-loss.
- **Epoch-seal loss:** the archive rename is the single commit point (§3.1 adversarial-i); a
  torn seal leaves the pre-seal file intact — no state in which some events exist only in a
  half-written archive is representable.
- **ML-KEM interop fork:** after V-4 exactly one impl is production-reachable and it is
  externally KAT-pinned; a silent semantic fork requires changing the impl AND the two
  hard-coded external vector sets in the same diff — a named CI RED, not a runtime surprise.
- **Trust-model note:** nothing in P34B touches capability issuance, revocation, or the
  MESH-12 genesis gate; trust remains signed capability, never reputation (standing rejection,
  double-locked by the untouched `ci-no-courier-scoring.sh`).

### 4.2 Schemas designed for scaling (item 8)

- **Per-node log:** linear in admitted events; P34 §4.2's traffic shape (~10-11 frames/order,
  1 000 orders/day ≈ 10⁴ leaves/day) hits `EPOCH_SNAPSHOT_LEAF_THRESHOLD` (10⁶) in ~3 months —
  the seal unit (§3.1b) IS the break-point mechanism, stated with its constant. JSONL record
  overhead (~2× payload hex) is accepted at node scale; the named upgrade trigger for a binary
  record format is a measured node where log I/O exceeds 1% of device write budget.
- **Fence:** O(crates × edges) metadata walk, trivially within CI budget; scaling axis is
  workspace size, break point none foreseeable.
- **ML-KEM:** fixed FIPS-203 parameter set (n=256, q=3329, k=3); no scaling axis — sizes are
  constants of the standard.

### 4.3 Isolation (11), mesh awareness (12), rollback vocabulary (13), living memory (15)

- **Isolation:** V-1 touches only `kernel/src/event_log.rs` (+ its own tests) — no engine, no
  wasm, no adapter surface; a store defect's blast radius is the node's own log, never a
  peer's (sync re-verifies signatures + content-ids on the receiving side, `:279-281`). V-2's
  fence runs in CI only. V-4 is inside two crypto crates with no transport coupling.
- **Mesh awareness:** the per-node store is **node-local by definition**; sync rides MESH-07's
  Sync·Pull + MerkleLog (built, P34 §0 row 10) — P34B changes no wire format, no payload
  budget. V-3 is manifest-and-docs only; the carrier's ~5-6 KB/frame budget (P34 §4.2) is
  unchanged.
- **Rollback (item-13 vocabulary, used precisely):** V-1 claims **Snapshot-Re-entry**
  (epoch seal = cheap regenerative recovery point; replay-equality proven by test §3.1b-1) and
  **Self-Termination/unrepresentable-state** (typed durability barrier; torn-seal no-op).
  Self-Healing (redundancy math) is NOT claimed — single-device storage has no redundancy;
  the k-of-n and hub-archive redundancy stories belong to PoD (P37) and the hub tier.
  Mechanical rollback of P34B whole: revert the two doc corrections, the fence-const diff, the
  manifest diff, the demotion commit — every item is a plain revert; the epoch unit is
  additive API (nothing calls it until the node maintenance path exists).
- **Living memory (item 15):** the per-node log is the living-memory pattern *at node scale* —
  content-addressed, append-only, demote-never-delete (sealed segments are archives, not
  deletions), exactly the attic/tier discipline of
  `internal-retrieval-living-memory-arc-2026-07-14`; the hub pgrust adapter
  (`kernel/Cargo.toml:35`) is where its SQL-tier recall applies. One principle, two tiers,
  now explicitly two documents.

### 4.4 Linux-discipline verdict framework (item 9)

`BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md` categories: V-1's decision =
**ALREADY-EQUIVALENT** ("one implementation of one concept" — the log IS the store; refusing a
parallel SQL store per node is the principle, applied); V-2 = **REINFORCES** (extends a proven
gate to a new consumer without new machinery); V-3 = **ALREADY-EQUIVALENT** (dead-flag removal;
the kernel's config-cruft discipline); V-4 = **REINFORCES** (single-authority consolidation,
same verdict class as EXCELLENCE B2's eigensolver ruling — and note the repo has done this
exact move before: `linalg` at `bebop2/core/src/lib.rs:343` is "the single authoritative
eigensolver"). Nothing here is EXTENDS or GAP — P34B builds almost no new machinery, by
design.

### 4.5 Non-contradiction constraints (sequencing, hard)

- V-2 depends on P34 (the adapter crate must exist to be guarded); do not dispatch V-2's CI
  step before P34's T2. The `GUARDED_CRATES` const diff itself is safe to land first
  (guarding a nonexistent crate is a no-op in `find_offenses`).
- V-3's hardened-default behavior arrives only with P36 DoD-2 — V-3 itself is P36-independent
  (manifest + docs + one metadata test) and must NOT wait for it (§10.5.2: nothing in P34B
  blocks the critical path; equally nothing here serializes behind P36 except the already-named
  edge).
- V-4 is operator-gated (red-line crypto); V-1/V-2/V-3 are not — do not batch V-4 into a
  combined commit with the others (gate-topology hygiene: reversible items must not ride a
  gated item's approval, nor vice versa).
- If the V-4 parity pin (§3.4-1) fails, STOP V-4 and file the finding — do not "fix" either
  impl to force parity (test-integrity rule: a failing proof is information).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Sharpens §10.5.2's P34B DoD-1..4 (kept 1:1, reordered to build items, none weakened):

| Item | §10.5.2 | RED (fails before) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|---|
| V-1a | DoD-4 (decision) | no written per-node-storage decision exists; two files claim pgrust-per-node (§0 row 3) | §2.1 exists (this file); both stale claims corrected with dated notes | `grep -n "production node persists this log in" kernel/src/event_log.rs` empty; `grep -n "per-node source-of-truth" deploy/README.md` empty; this blueprint cites and distinguishes the hub pgrust blueprint by path (DoD-4's literal requirement) |
| V-1b | DoD-4 (mechanism) | no snapshot/compaction exists (§0 row 5) | `seal_epoch` + 3 named tests + 2 adversarials green | `cargo test -p dowiz-kernel epoch_` green; falsified by any test deleting or re-hashing a sealed event |
| V-2 | DoD-1 | fence guards zero dowiz crates (§0 row 9) | `GUARDED_CRATES` includes both dowiz crates; dowiz CI runs the fence via `--metadata`; injected-offense test green | falsified by `grep -n dowiz ci/crdt-fence/src/lib.rs` empty, or the dowiz CI job lacking the fence step |
| V-3 | DoD-3 | `iroh = []` + stale stub comment live (§0 row 11) | feature deleted; dated decision note in the manifest; `retired_iroh_feature_absent` green; hole-punch deferral named with trigger | falsified by `grep -n '^iroh' bebop2/proto-wire/Cargo.toml` non-empty — §10.5.2's own fail condition, now a test |
| V-4a | DoD-2 (KAT) | two production-reachable ML-KEM impls, one un-KAT'd (§0 row 14) | exactly one production impl, externally KAT'd; parity pinned before demotion | `grep -rn "pub fn encaps" bebop2/*/src/ \| grep -v test` → 1 hit; KAT test present by name |
| V-4b | DoD-2 (zeroize) | zeroize only in proto-crypto (§0 row 15) | each named core secret type wrapped-or-deferred-with-note; per-type zeroize proof tests green | the §3.4-3 list checked item-by-item; falsified by a listed type with neither wrapper nor note |

Permanent regression rows (item 17): dowiz ledger (`docs/regressions/REGRESSION-LEDGER.md`):
(1) "P34B epoch-seal chain continuity — guardrail: `epoch_seal_preserves_chain_continuity`";
(2) "P34B dowiz CRDT fence — guardrail: dowiz CI fence step + injected-offense test".
Bebop ledger (created by P36 R-0, §0 row 17): (3) "P34B iroh-feature retirement — guardrail:
`retired_iroh_feature_absent`"; (4) "P34B single ML-KEM authority — guardrail: KAT-by-name
lint + one-`encaps` grep". Ledger ratchet rule applies verbatim.

---

## 6. Benchmark plan (item 10) — measure the store, build no new harness

**The actual perf question, named:** per-insert durability cost (JSONL + fsync per event) and
seal cost at threshold scale. Not ML-KEM (no production consumer yet — a bench would measure
nothing load-bearing; deferred with that stated trigger) and not the fence (CI-only).

1. `event_log/insert_durable_one_event` (criterion, dowiz kernel benches — the existing
   `kernel/benches/criterion.rs` harness, extended not duplicated): `FileEventStore::insert`
   including fsync, vs `MemEventStore` as the floor. Expectation to falsify: fsync dominates
   (ms-class on spinning/emulated media, sub-ms on NVMe); at 0.12 frames/sec sustained (P34
   §4.2) even 10 ms/insert leaves ≥3 orders of magnitude headroom — the bench pins the trend.
2. `event_log/seal_epoch_1e5` — seal cost at 10⁵ events (10⁶ is CI-hostile; the scaling is
   linear and stated, measured at 1/10 scale with the extrapolation recorded honestly).
3. Baselines recorded in `BENCH_HISTORY.md` alongside the existing rows; telemetry hook =
   P-H's bench-regression gating, same dependency note as P34 §6.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.3 (invariants 2/5) / §10.5.2 (the
P34B charter; P34/P35/P36 boundaries) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (sibling spec:
§0 row 12's regression lineage, §4.2's break-point handoff consumed by §3.1b, §3.1c's CI job
extended by V-2) · `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` (MESH-06/08/09/13 design
source; the `:108-113` sqlless correction is §2.1's ground) ·
`docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (**cited to DISTINGUISH, not to
absorb**: hub tenant-schema, council-gated, RLS inversion — per-node storage shares no table,
no role, no consistency model with it; §10.5.2 DoD-4 satisfied by §2.1's explicit contrast) ·
`docs/design/GROUND-TRUTH-2026-07-17.md` (live repo state) ·
`bebop-repo/docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` (B2 single-authority
precedent; iroh premise correction) · `docs/regressions/REGRESSION-LEDGER.md` (item-17
mechanism). Memory: `mesh-real-arc-2026-07-13` ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4.3) · `ops-reliability-arc-2026-07-13`
(pgrust-now = hub ops, resurrect-from-attic) · `rust-native-bare-metal-decision-2026-07-14`
(DECART discipline in §2.1/§3.3) · `crypto-safe-first-pass-2026-07-14` (V-4's review gate) ·
`cross-branch-todo-map-2026-07-10` (repo routing: fence + manifest diffs → `/root/bebop-repo`,
pushed to `openbebop`) · `worktree-remote-push-collision-avoidance-2026-07-18`.
Supersedes: the *status* of MESH-06/08/09/13 rows everywhere they appear (this pass's §0 is
newer than §10.5.2's); the *design* source remains BLUEPRINTS-MESH-REAL.md.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): one per-node store (the event log — a
  second store per node is now a *documented rejection*, not an absence); one ML-KEM authority
  (V-4); one fence pattern covering both repos' kernel-state crates.
- **P6 CAUSE-AND-EFFECT** (determinism as law): projections are deterministic folds; epoch
  replay-equality is asserted as a hash equality (§3.1b-1) — recovery is re-derivation, never
  restoration-by-copy of derived state.
- **P7 GENDER** (paired creation, no self-certification): the ML-KEM authority is certified by
  EXTERNAL vectors from an independent implementation (dowiz-pq), not by its own round-trip;
  the parity pin (§3.4-1) is a second witness before any demotion; V-4 itself requires an
  independent reviewer (operator gate).

(P1/P3/P4/P5 are not load-bearing for these four units and are not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 17 rows, live-verified; 4 stale-status corrections + 2 new findings (dual ML-KEM, dowiz-fence gap) surfaced |
| 2 DoD | §5 — RED→GREEN, commands + falsifiers, §10.5.2 DoD-1..4 kept 1:1 and sharpened |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED tests precede code; §3.1b asserts event/hash sequences, not end-state prose |
| 4 predefined types/consts | §2.2 — `EPOCH_SNAPSHOT_LEAF_THRESHOLD`, `EpochSeal`, `SealedSegment`, `seal_epoch`, the fence-const diff verbatim |
| 5 adversarial/breaking tests | §3.1 torn-seal + tampered-archive, §3.2 both-direction teeth, §3.3 flag-reintroduction, §3.4 KAT-sabotage + zeroize teeth |
| 6 hazard-safety as math | §4.1 — five unsafe states with structural unreachability arguments |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — leaf threshold with constant + measured basis; JSONL upgrade trigger named |
| 9 Linux discipline | §4.4 — four verdicts, incl. the in-repo `linalg` precedent for V-4's move |
| 10 benchmarks+telemetry | §6 — durable-insert + seal benches, honest 1/10-scale note, ML-KEM bench deferred with reason |
| 11 isolation/bulkhead | §4.3 — blast radii per item; store defects node-local by sync-side re-verification |
| 12 mesh awareness | §4.3 — node-local store; zero wire-format changes; carrier budget untouched |
| 13 rollback/self-heal vocabulary | §4.3 — Snapshot-Re-entry + Self-Termination claimed with mechanisms; Self-Healing explicitly NOT claimed |
| 14 error-propagation gates | §3.2 (fence, both repos) + §3.3 (metadata test) + §3.4-4 (KAT-by-name lint); §3.2's honest name-gate limitation stated |
| 15 living memory | §4.3 — node tier vs hub SQL tier, one principle two documents |
| 16 tensor/spectral + eqc reuse | **Explicit N/A, not decorative**: storage plumbing, dep-graph lint, manifest hygiene, and KEM consolidation contain no closed-form math organ; the spectral machinery is untouched. Reasoned exemption per "where applicable" |
| 17 regression ledger | §5 — four named permanent rows across the two ledgers |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | the whole §2.1 decision IS reuse-first (store already exists); §3.2 reuses the fence binary; §3.4 reuses the zeroize pattern; DECART rejections recorded |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Two sibling repos: `/root/dowiz` and `/root/bebop-repo` (push remote `openbebop`; `origin` is
ARCHIVED read-only — never push there). Bebop files are edited in `/root/bebop-repo`, dowiz
files in `/root/dowiz`. Execute in order; U1/U3/U4 are mutually independent (may fan out),
U2 waits for P34's T2.

1. **U1 (V-1; dowiz).** (a) Apply the two stale-premise corrections (§3.1a) with dated
   append-only notes citing this blueprint. (b) Implement §2.2's epoch types + `seal_epoch` in
   `kernel/src/event_log.rs`; write the 3 named tests + 2 adversarials (§3.1b) RED-first
   (tests committed against a `todo!()` seal fail, then go green). Acceptance: the two DoD
   greps empty; `cargo test -p dowiz-kernel epoch_` green; no event ever deleted or re-hashed.
2. **U2 (V-2; both repos — AFTER P34 T2 exists).** In `/root/bebop-repo`: add the two dowiz
   crate names to `GUARDED_CRATES` (§2.2 verbatim) + the
   `red_injected_crdt_into_dowiz_adapter_is_offense` test; push to `openbebop`. In
   `/root/dowiz`: add the two fence steps (§3.2 YAML) to the `mesh-adapter` CI job.
   Acceptance: fence unit tests green; the live-graph clean run passes; CI job green.
3. **U3 (V-3; bebop-repo).** Delete `iroh = []` from `bebop2/proto-wire/Cargo.toml`; replace
   the `:41-48` stale comment with the dated retirement note (§3.3-1's three clauses: carrier
   real, iroh-crate retired with grounds, hole-punch deferred with trigger). Add
   `retired_iroh_feature_absent`. Do NOT touch `iroh_transport.rs` code or the insecure-tls
   feature (P36's). Acceptance: `grep -n '^iroh' bebop2/proto-wire/Cargo.toml` empty; test
   green; push to `openbebop`.
4. **U4 (V-4; bebop-repo — 🔴 OPERATOR-GATED, do not merge on agent authority).** (a) Write
   the 16-seed parity pin FIRST; if it fails, STOP and file the finding. (b) Demote core's
   ML-KEM to `#[cfg(test)]` per §2.3-i. (c) Wrap the §3.4-3 secret-type list (or write the
   per-type deferral note), each with a zeroize-proof test. (d) Run the KAT-sabotage teeth
   once, record in the PR. Acceptance: §5 V-4a/V-4b rows green; independent review recorded;
   operator merge.
5. **U5 (close-out).** Append the four REGRESSION-LEDGER rows (§5; bebop rows go in the ledger
   P36 R-0 creates — if it does not exist yet, create `bebop-repo/docs/regressions/
   REGRESSION-LEDGER.md` with the dowiz format and note it in the P36 PR). Run the §6 benches,
   record numbers. Push both repos; fetch before every push, never force.

**Stop-and-flag conditions (do not improvise past these):** (i) the V-4 parity pin failing
(§4.5 — finding, not cleanup); (ii) any impulse to add a per-node SQL store, a new KEM, an
event variant, or transport code (owned elsewhere or rejected — §1/§2.1); (iii) `seal_epoch`
needing to mutate or re-hash any existing event (design error — stop); (iv) any V-4 commit
without the operator gate; (v) the two stale-premise greps still matching after U1 (the
correction was incomplete).
