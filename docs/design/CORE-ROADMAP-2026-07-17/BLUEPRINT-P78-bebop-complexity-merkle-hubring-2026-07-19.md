# BLUEPRINT P78 — bebop complexity fixes: MerkleLog::add + hub_ring::ranked (2026-07-19)

> **Standalone PERF blueprint (bebop2 `proto-wire` + `delivery-domain`).** One coherent,
> independently-buildable unit against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Planning document — writes ZERO product code, touches no
> branches, pushes nothing. Research source: `docs/research/OPUS-PERF-BEBOP-AUDIT-2026-07-18.md`
> (R4) findings P1 (rank #1) and P2, reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md`
> §3.2 (B3/B4) + §5. Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
> Grounding tree read live this pass: `/root/bebop-repo/bebop2` at HEAD.
>
> **One sentence:** two behavior-preserving, small (~5-line + ~10-line) algorithmic fixes on the
> bebop mesh path — stop `MerkleLog::add` re-sorting the whole leaf vector on every insert, and stop
> `hub_ring::ranked` recomputing the HRW hash twice per sort comparison — each shipped with a
> red→green benchmark and an identity test proving the output is byte-for-byte unchanged.
>
> **Repo discipline (binding):** these files live in `/root/bebop-repo/bebop2`, NOT `/root/dowiz`.
> Per memory `cross-branch-todo-map-2026-07-10.md`: bebop/wire-native-core files → `/root/bebop-repo`,
> push to the `openbebop` remote (`git@github.com:SyniakSviatoslav/OpenBebop.git`). This blueprint
> file is a dowiz planning artifact; the code it plans is a bebop change.

---

## VERDICT (stated up front)

**GO — both fixes are strictly-better, behavior-preserving, and cheap.** Neither changes any wire
byte, any root hash, any ownership assignment, or any public signature; each is the removal of
redundant work the code already proves it doesn't need. The honest caveat is on *magnitude* (§0.3):

- **B3 (MerkleLog) is the real scaling win** — its cost is quadratic in the anti-entropy leaf count,
  which grows with mesh/event-log volume; R4 ranked it #1 in the bebop audit.
- **B4 (hub_ring) is a regression-from-blessed-code cleanup**, not a scaling cliff — the hub count is
  bounded-small, so the absolute per-call saving is modest; its value is (a) removing a needless
  2×-hash-per-comparison that diverges from the already-correct `matcher::assign` pattern, and (b)
  turning `owner_hub`'s full-sort-to-take-`[0]` into an O(n) scan. Stated plainly, not inflated.

Both ship only with a criterion before/after number (standing Performance Rule,
`.claude/CLAUDE.md:182-195`: no rewrite without a bench proving hotness/win).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

### 0.1 B3 — `MerkleLog::add` re-sorts the entire leaf vector on every insert

`bebop2/proto-wire/src/sync_pull.rs`:

| Element | Cite | Fact |
|---|---|---|
| struct `MerkleLog { leaves: Vec<[u8;32]>, seen: HashSet<[u8;32]> }` | `:421-425` | content-addressed anti-entropy digest; `seen` is the dedup set |
| `add(&mut self, id)` | `:449-453` | `if self.seen.insert(id) { self.leaves.push(id); self.leaves.sort_unstable(); }` — **a full `sort_unstable()` on every accepted insert** |
| `root(&self)` | `:457-481` | already does `let mut level: Vec<[u8;32]> = self.leaves.clone();` (`:461`) then folds a binary SHA3-256 tree bottom-up |

**Cost fact.** Dedup is *already* handled by `self.seen.insert(id)` (`:450`) — the per-insert
`sort_unstable()` exists only to keep `leaves` in sorted order so `root()` is stable. Building a
digest of `n` distinct leaves therefore costs `Σ_{k=1..n} O(k log k) = O(n² log n)` on the
anti-entropy fold path. **Naming correction to the synthesis/R4 shorthand:** the type is `MerkleLog`
and the method is `add` (R4/§3.2 wrote "`MerkleDigest::add`" — no `MerkleDigest` type exists;
`sync_pull.rs:449` is the real target). Ground truth over inherited claim (standard §2 item 1).

### 0.2 B4 — `hub_ring::ranked` recomputes `hrw_weight` twice per comparison; `owner_hub` full-sorts to take `[0]`

`bebop2/delivery-domain/src/hub_ring.rs`:

| Element | Cite | Fact |
|---|---|---|
| `ranked(order_id, hubs) -> Vec<Hub>` | `:52-60` | `ranked.sort_by(\|a,b\| hrw_weight(order_id,&b.pubkey).cmp(&hrw_weight(order_id,&a.pubkey)).then_with(\|\| b.pubkey.cmp(&a.pubkey)))` — **each comparison calls `hrw_weight` twice** (once per side), so the weight of every hub is recomputed `O(log n)` times |
| `hrw_weight` | `proto-cap/src/matcher.rs:41-54` | an FNV-1a over `order_id ‖ pubkey` (40 bytes) — cheap but not free, and needlessly repeated |
| `owner_hub(order_id, hubs) -> Hub` | `:78-80` | `assign(order_id, hubs, 0).owner` — runs a **full sort** of all hubs only to read `ranked[0]` |
| the blessed pattern | `proto-cap/src/matcher.rs:63-73` | `assign` here **precomputes** `(hrw_weight(...), pubkey)` once into a `Vec`, then `sort_by(\|a,b\| b.0.cmp(&a.0).then_with(\|\| a.1.cmp(&b.1)))` — the exact Schwartzian transform B4 should mirror |
| the hub_ring tests | `:94` | gated behind `#[cfg(all(feature = "kernel-rlib", test))]` — invisible under default `cargo test` (this is the R8/P76 hidden-tests finding; see §4 dependency) |

**Cost fact.** Hub count is bounded-small (a mesh has few hubs), but `ranked`/`owner_hub` are called
**per order** (ownership overlay, `hub_ring.rs:1-18`). `matcher::assign` already got this right —
`hub_ring` is a *divergence* from blessed in-repo code, not a novel problem.

### 0.3 Honest magnitude assessment (standard §2 item 6 — no inflation)

- B3: genuine `O(n² log n)`→`O(n log n)` improvement on a path whose `n` scales with real mesh
  event volume. **This is the one that will show a clear curve on a bench sweep.**
- B4: bounded `n`; the win is constant-factor (`2×` fewer hashes per comparison) plus
  `O(n log n)`→`O(n)` for `owner_hub`. On today's hub counts this is microseconds, not a cliff. It
  is worth doing because it is trivial, removes a foot-gun as hub counts grow, and re-aligns with the
  blessed pattern — **not** because it is a current bottleneck. If the bench shows no measurable move
  at realistic hub counts, B4 still lands (correctness/consistency), but the DoD does not *claim* a
  speedup it can't show (standard §2 item 10).

---

## 1. Prior-art / reuse map — adopt, don't invent (standard §2 item 19)

| Need | In-tree pattern to reuse | Cite | Why extension, not new code |
|---|---|---|---|
| rank-by-derived-key without recompute | Schwartzian precompute `(key, item)` then sort tuples | `proto-cap/src/matcher.rs:63-73` (`assign`) | B4 is literally "make `hub_ring` match the sibling overlay that already does it right" — zero new abstraction |
| stable Merkle root over an unordered multiset | sort once at read time, fold the clone | `sync_pull.rs:457-481` (`root` already clones `leaves`) | B3 moves the sort the code already needs into the read that already clones — no new field, no new type |
| benchmark harness | criterion (bebop `verify_lane.rs` / the new criterion sibling P82 owns) | R4 P3 / synthesis §3.3-C3 | B3/B4 add bench *groups*, they do not stand up a new harness (that is P82's scope) |

**No new dependency, no new primitive** (standard §2 item 19). Both fixes are subtractive.

---

## 2. Scope — what P78 owns vs deliberately does NOT (standard §2 items 11, 18)

### 2.1 P78 OWNS
1. **B3:** remove the per-insert `sort_unstable()` from `MerkleLog::add`; sort the existing clone in
   `MerkleLog::root` so the root is byte-identical; add `merkle_ingest` + `merkle_root` benches.
2. **B4:** rewrite `hub_ring::ranked` as a Schwartzian precompute mirroring `matcher::assign`;
   rewrite `owner_hub` as a single `max_by` scan; add `hub_ring_ranked` + `hub_ring_owner` benches.
3. **Identity/regression tests** proving the output (root hash; `(owner, replicas)` ordering) is
   unchanged for the same inputs (standard §2 item 17).

### 2.2 P78 does NOT own
- **The Merkle tree shape / root algorithm** (`root`'s SHA3 fold, odd-leaf-pairs-with-itself
  `:467-471`) — untouched; only *when* the sort happens moves.
- **`hrw_weight` itself** (`matcher.rs:41-54`) — the FNV-1a math is correct and shared; not retouched.
- **Un-gating the `hub_ring`/`delivery-domain` hidden tests** — that is **P76** (R8 G-T1). P78 depends
  on P76 so its B4 identity test actually executes (see §4).
- **The `HybridGate`/sign/KEM bench expansion** and the criterion-sibling harness stand-up — that is
  **P82** (synthesis §3.3-C3). P78 contributes 4 bench groups into whatever harness P82 lands (or the
  existing `verify_lane.rs` sibling), it does not design the harness.
- **`SyncNode::pull` per-actor seq index** (R4 P4 / Tier D-3) — deferred, bundled with anti-entropy
  hardening; P78 does not touch it.
- **`crates/bebop` (legacy TUI)** — out of scope (R4 §5 / E13); this is `bebop2` product crates only.

### 2.3 Dependencies (named by artifact — standard §2 item 7)
- **P76** (bebop hidden-tests un-gate) — soft, repo-sequencing: B4's identity test lives next to the
  `hub_ring` tests currently behind `kernel-rlib` (`:94`); P76 makes them run. Sequence **P76 → P78**
  in the bebop lane (synthesis §5, Wave W1→W2; ledger §3 Wave 2: `P78 → P82`). If P76 has not landed,
  B4's test must be added under plain `#[cfg(test)]` (not the feature gate) so it runs regardless.
- **P75** (CI bench-regression gate + `<group>/<n>` bench-id schema) — soft; P78's 4 benches are
  written into P75's schema, never redefine it.
- **Feeds P82** (bebop bench expansion) and, via P82's KEM lane, the D-9 NTT decision — indirectly.

---

## 3. Predefined types & constants (standard §2 item 4)

No new types. This is a subtractive fix. The only *named* additions are bench-group ids, pinned to
P75's `<group>/<n>` convention:

```
merkle_ingest/<n>      // add() n distinct leaves, sweep n ∈ {8, 64, 256, 1024}  — B3 before/after gate
merkle_root/<n>        // root() over an n-leaf log, same sweep                   — proves root cost unchanged
hub_ring_ranked/<h>    // ranked() over h hubs, sweep h ∈ {4, 16, 64}            — B4 comparator gate
hub_ring_owner/<h>     // owner_hub() over h hubs, same sweep                     — B4 max_by gate
```

Sweep sizes state the scaling axis (standard §2 item 8): `merkle_*` scales with **anti-entropy leaf
count** (real mesh event volume); `hub_ring_*` scales with **hub count** (bounded-small today — the
`64` upper end exists to keep the curve on record, mirroring the money/ppr growth-tripwire treatment,
not because 64 hubs is expected).

---

## 4. Build items — spec → RED test → code (standard §2 items 2, 3, 5)

### 4.1 B3 — MerkleLog: drop per-insert sort, sort the clone in `root()`

- **Spec:** `add` becomes `if self.seen.insert(id) { self.leaves.push(id); }` (no sort). `root`
  becomes `let mut level = self.leaves.clone(); level.sort_unstable();` before the fold. Root output
  is a pure function of the leaf **multiset**, so sorting at read time yields the identical value the
  read-after-sorted-inserts produced before.
- **RED `red_merkle_root_identical_after_fix`:** build two logs from the same ids inserted in
  **different orders**; assert `root()` is byte-equal between them AND equal to a golden root captured
  from the current (pre-fix) implementation. Goes RED if the fix ever changes the root; GREEN proves
  behavior preservation. (Standard §2 item 17 — permanent regression test.)
- **RED `red_merkle_ingest_is_linearithmic`:** the `merkle_ingest/1024` bench must complete with a
  wall-clock ratio to `merkle_ingest/256` consistent with `O(n log n)`, not `O(n² log n)` — i.e. the
  4×-size step must NOT be ≈16-20× slower. This is the criterion before/after gate (measured number,
  not estimate — standard §2 item 10). RED on the current code, GREEN after.
- **Adversarial `red_merkle_dedup_still_holds`:** inserting the same id twice must not grow `leaves`
  or change `root` — proves the `seen` set, not the sort, was doing dedup all along.

### 4.2 B4 — hub_ring: Schwartzian `ranked`, `max_by` `owner_hub`

- **Spec:** `ranked` precomputes `let mut weighted: Vec<(u64,Hub)> = hubs.iter().map(|h|
  (hrw_weight(order_id,&h.pubkey), *h)).collect();` then
  `weighted.sort_by(|a,b| b.0.cmp(&a.0).then_with(|| b.1.pubkey.cmp(&a.1.pubkey)));` returning
  `weighted.into_iter().map(|(_,h)| h).collect()` — **weight computed once per hub**, identical total
  order (weight DESC, pubkey DESC tie-break preserved exactly as `:57`). `owner_hub` becomes a single
  `hubs.iter().max_by(|a,b| hrw_weight(order_id,&a.pubkey).cmp(&hrw_weight(order_id,&b.pubkey))
  .then_with(|| a.pubkey.cmp(&b.pubkey))).copied()` — O(n), no sort. (Care: `max_by` returns the
  *last* maximum on ties, so the tie-break must match `ranked[0]`; the test below pins it.)
- **RED `red_hub_ring_ranking_identical_after_fix`:** over a fixed hub set + a sweep of order_ids,
  assert the new `ranked` returns the **exact same `Vec<Hub>`** as the current implementation
  (golden-captured), and `owner_hub == ranked[0]` for every order_id. RED if the total order or the
  owner ever differs; GREEN proves behavior preservation. **This test must run under a config where
  it executes** — plain `#[cfg(test)]`, or after P76 un-gates the module (§2.3).
- **RED `red_hub_ring_no_double_hash`:** `hub_ring_ranked/64` measured against a count-instrumented
  `hrw_weight` (or an iai-callgrind instruction count per P75's ns-scale lane) shows the weight is
  computed exactly `n` times, not `O(n log n)` times. This is the mechanism gate, made machine-checkable.
- **Adversarial `red_hub_ring_tie_break_stable`:** two hubs whose `hrw_weight` collides for some
  order_id must still order identically (by pubkey) in both `ranked` and `owner_hub` — proves the
  `max_by` last-max-on-tie hazard is handled.

---

## 5. Invariants to preserve (standard §2 items 6, 13)

Made unrepresentable / test-pinned, not asserted in prose:

1. **Root determinism (B3):** `root()` is a pure function of the leaf multiset. Preserved because the
   sort simply moves to the read that already clones; `red_merkle_root_identical_after_fix` +
   `red_merkle_dedup_still_holds` are the falsifiers. Anti-entropy convergence (a differing root
   triggers a pull, `sync_pull.rs:420`) is unaffected — same root ⇒ same convergence decisions.
2. **HRW rendezvous determinism / No-SPOF (B4):** every node computes the identical `(owner, replicas)`
   from `(order_id, hub_set)` with zero coordination (`hub_ring.rs:5-18`). Preserved because the total
   order is byte-identical; `red_hub_ring_ranking_identical_after_fix` is the falsifier. The existing
   AC-11 tests (`ac11_owner_is_rendezvous_deterministic` `:105`, `ac11_no_spof_owner_removal_promotes_replica`
   `:127`, `ac11_locality_owner_stable_unless_removed` `:179`) must stay green (standard §2 item 17).
3. **`pubkey` is identity, never a score (B4):** the HRW weight is over the pubkey, never a
   reputation (`hub_ring.rs:27` "it is never a score"). Untouched — no NO-COURIER-SCORING surface is
   introduced.
4. **Rollback as math (item 13):** both changes are self-terminating diffs — if the identity test
   fails, the change is simply not merged; there is no runtime state to roll back. Snapshot re-entry
   is N/A (no persisted state changes).

---

## 6. DoD — falsifiable, RED→GREEN (standard §2 item 2)

| # | Done when… | Falsifier |
|---|---|---|
| D1 | `MerkleLog::add` no longer sorts; `root()` sorts its clone; root byte-identical | `red_merkle_root_identical_after_fix`, `red_merkle_dedup_still_holds` |
| D2 | `merkle_ingest` scales `O(n log n)` not `O(n² log n)` on the sweep | `red_merkle_ingest_is_linearithmic` (criterion before/after) |
| D3 | `hub_ring::ranked` computes each hub's weight once; total order unchanged | `red_hub_ring_no_double_hash`, `red_hub_ring_ranking_identical_after_fix` |
| D4 | `owner_hub` is an O(n) `max_by`; owner == `ranked[0]` incl. on ties | `red_hub_ring_tie_break_stable`, `red_hub_ring_ranking_identical_after_fix` |
| D5 | all existing bebop tests (AC-11 hub_ring, sync_pull anti-entropy) stay green | `cargo test -p bebop-delivery-domain -p bebop-proto-wire` (+ `--features kernel-rlib` until P76) |
| D-BENCH | 4 bench groups exist under P75's `<group>/<n>` schema with committed baselines | criterion output in the gate |
| D-NOREG | no wire byte / public signature changed; no new dependency | `cargo build` diff review + `Cargo.lock` unchanged |

---

## 7. Benchmarks + telemetry + the measure-first honesty gate (standard §2 item 10)

Per the standing Performance Rule (`.claude/CLAUDE.md:182-195`): a rewrite requires a bench proving
hotness/win. The four groups in §3 are that proof.

- **B3 is expected to move visibly** on `merkle_ingest/{256,1024}` — the before/after ratio is the
  headline number. Record both curves (pre-fix quadratic, post-fix linearithmic) in
  `kernel/benches/BENCH_HISTORY.md`'s bebop analog / P75's committed trend store.
- **B4 may show no measurable wall-clock move at h ≤ 64** — that is an acceptable, honestly-reported
  outcome (§0.3). Its gate is the *mechanism* count (`red_hub_ring_no_double_hash`), not a speedup
  claim. Do **not** manufacture a speedup number; if wall-clock is flat, state "constant-factor
  hash-count halved; no measurable wall-clock change at realistic hub counts."
- Telemetry: no runtime telemetry hook needed (pure hot-function micro-fixes); the criterion baselines
  in CI (P75) are the regression detector (standard §2 item 14).

---

## 8. Rollout / sequencing (consistent with the master ledger)

Per `MASTER-STATUS-LEDGER-2026-07-19.md` §3 and `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §5:

- **Lane:** bebop (never touches the dowiz kernel — fully parallel to P77/P79/P80).
- **Order within the bebop lane:** **P76 → P78** (P78 sequenced after P76 to avoid CI-churn overlap
  and to inherit the un-gated `hub_ring` tests), then **P78 → P82** (Wave 2; P82's bebop bench
  expansion absorbs P78's 4 groups). Ledger §3 Wave 2 dowiz-vs-bebop table: `P78 → P82`.
- **Gate-0 caveat (from the ledger, do not skip):** the entire bebop lane is currently frozen behind
  the **C3 ungated-keygen HARD-law red state** + the unremediated `986646a` NTT base (ledger §3
  finding 2; OD-3). Until C3 is resolved (operator, OD-3) and **P85** closes, no hook-respecting
  commit lands bebop-side — **including P78**. P78 can be fully *written and reviewed* now; it cannot
  *commit through the hooks* until the bebop gate-0 clears. State this to the executing worker.
- **Push target:** `openbebop` remote after each milestone (memory
  `worktree-remote-push-collision-avoidance-2026-07-18.md`).

---

## 9. Open operator-decision points

P78 introduces **no new** operator decision. It only *inherits* two upstream gates it must wait on:

| # | Decision | Owner | Effect on P78 | Source |
|---|---|---|---|---|
| OD-3 | Resolve bebop C3 ungated-keygen red state (or explicit `--no-verify` ruling) | operator | Unfreezes the bebop commit lane — P78 cannot merge until then | ledger §5 OD-3 |
| OD-6 | P85 NTT remediation closure path | operator | Same freeze; P85 must close before bebop commits flow | ledger §5 OD-6 |

Engineering decisions P78 makes itself (operator need not): whether B4's identity test uses plain
`#[cfg(test)]` or waits for P76's un-gate (§2.3); exact bench sweep endpoints (§3, cite P75's schema).
D-2 (`reputation.rs` delete-or-event-source) is **P76's** flag, not P78's — B4 touches `hub_ring`,
which is pubkey-HRW (not reputation) and needs no ruling.

---

## 10. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the Merkle root *is* a pure function of the leaf set —
  after B3 the sort is where the read needs it, so the value is self-describing from the multiset, not
  an artifact of insertion order. B4: every node's owner assignment corresponds to the identical
  `(order_id, hub_set)` — the fix preserves that correspondence exactly.
- **Cause & Effect:** ownership follows a deterministic HRW *cause* (the pubkey weight), never a score
  or channel — B4 keeps trust = self-certifying identity (`hub_ring.rs:27`).
- **Polarity / no-middle:** a root either matches (no pull) or differs (pull) — B3 introduces no
  intermediate "maybe sorted" state; the log is unordered-with-a-dedup-set and the root sorts on read.

---

## 11. Standard-compliance map (all 20 points — standard §2)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (incl. the `MerkleDigest`→`MerkleLog` naming correction) |
| 2 | Falsifiable DoD | §6 |
| 3 | Spec→test→code, event-driven | §4 (spec-first per B3/B4; anti-entropy convergence is the event surface) |
| 4 | Predefined types & constants | §3 (bench-group ids; no new domain type — subtractive fix) |
| 5 | Adversarial/breaking tests | §4 (`red_merkle_dedup_still_holds`, `red_hub_ring_tie_break_stable`) |
| 6 | Hazard-safety from structure | §5 (root determinism / HRW determinism as test-pinned invariants) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §3 (merkle=event volume; hub_ring=hub count, bounded) |
| 9 | Linux engineering discipline | REINFORCES fail-closed/pure-function patterns; EXTENDS nothing; DOES-NOT-TRANSFER (no daemon) — §1 |
| 10 | Benchmarks + telemetry + measure-first | §7 (B3 headline curve; B4 honest "may be flat") |
| 11 | Isolation / bulkhead | §2 (disjoint from kernel; disjoint files within bebop; no shared mutable state) |
| 12 | Mesh awareness | §0.1 (Merkle = anti-entropy fold path), §0.2 (hub_ring = per-order ownership overlay) |
| 13 | Rollback/self-heal as math | §5.4 (subtractive diff; no runtime state; snapshot N/A) |
| 14 | Error-propagation / smart index | §7 (criterion baselines in P75's CI gate catch regressions) |
| 15 | Living-memory awareness | N/A honestly — anti-entropy digest is content-addressed, not living memory; stated |
| 16 | Tensor/spectral | N/A honestly — sort/hash micro-fixes, not linear algebra; stated (ponytail) |
| 17 | Regression tracking | §4/§6 (identity tests are permanent; REGRESSION-LEDGER entry for each) |
| 18 | Clear worker instructions | §13 |
| 19 | Reuse-first | §1 (B4 mirrors `matcher::assign`; B3 reuses `root`'s existing clone); no new dep |
| 20 | Hermetic principles | §10 |

---

## 12. Links to docs & memory (standard §2 item 7)

- `docs/research/OPUS-PERF-BEBOP-AUDIT-2026-07-18.md` (R4) P1 (MerkleDigest per-insert sort, rank #1),
  P2 (hub_ring double-hash), §2 (verified-clean list — do NOT "optimize" `matcher::assign`, the batch
  verify, the wire codec), §5 (`crates/bebop` legacy TUI out of scope).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 (B3/B4 rows), §5 (Wave W1/W2 sequencing), §6 (E13
  legacy-crate, E15 verified-clean bebop paths).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P78 row), §3 (bebop-lane gate-0 / P85 / C3), §5 (OD-3/OD-6).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `cross-branch-todo-map-2026-07-10.md` (bebop files → `/root/bebop-repo`, push `openbebop`);
  `worktree-remote-push-collision-avoidance-2026-07-18.md` (push after each milestone);
  `performance-priority-over-minimal-change-2026-07-17.md` (pursue real compounding perf gains).

## 13. Instructions for the executing worker (zero prior context — standard §2 item 18)

**Repo:** `/root/bebop-repo/bebop2` (NOT dowiz). **Push:** `openbebop`. **Blocked from committing
until** the bebop gate-0 (C3/OD-3 + P85/OD-6) clears — you may write, test locally, and review now.

1. **B3** — `proto-wire/src/sync_pull.rs`: delete the `self.leaves.sort_unstable();` line in `add`
   (`:452`); in `root` (`:457`), sort the `level` clone right after `:461`. Add the RED tests (§4.1)
   next to the existing `MerkleLog` tests; capture the golden root from HEAD **before** editing.
2. **B4** — `delivery-domain/src/hub_ring.rs`: rewrite `ranked` (`:52-60`) as the Schwartzian
   precompute mirroring `proto-cap/src/matcher.rs:63-73`; rewrite `owner_hub` (`:78-80`) as a `max_by`
   scan. Add the RED tests (§4.2); if P76 has not un-gated the module, add them under plain
   `#[cfg(test)]` (not `kernel-rlib`) so they run. Capture golden `ranked`/`owner` output from HEAD first.
3. Add the 4 bench groups (§3) into the bebop criterion harness P82 owns (or `verify_lane.rs`'s
   sibling), under P75's `<group>/<n>` id schema.
4. `cargo test -p bebop-delivery-domain -p bebop-proto-wire` (add `--features kernel-rlib` until P76)
   fully green; existing AC-11 + anti-entropy tests stay green (D5/D-NOREG).
5. Add REGRESSION-LEDGER entries for the two identity tests.
6. **Do NOT** claim a B4 speedup the bench doesn't show — report the honest mechanism win (§7).
7. **Do NOT** touch `hrw_weight`, the Merkle fold algorithm, `SyncNode::pull`, `crates/bebop`, or any
   wire byte.
