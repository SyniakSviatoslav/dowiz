# BLUEPRINT — Item 66: Periodic Durable-Log Scrub (ZFS-scrub slice)

- **Date:** 2026-07-19 · **Tier:** small journaling-FS gap (roadmap §K) · **Status:** BLUEPRINT
  (planning artifact, no code) — **GATED on item 64** (see §6).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 66
  (lines 1138–1147) + §K dependency line (1251 `66 after 64`); `docs/audits/hardening/CHECKLIST.md`.
  Ground truth for code citations: this worktree at HEAD `6701bbb6f`.
- **Upstream:** item 64 (composition root — wires the store this scrub walks); the EXISTING checksums
  it re-verifies — the durable EventLog SHA3 content-id chain
  (`kernel/src/event_log.rs:481` `verify_chain`) and the FDR ring CRC32 per-line checksums
  (`kernel/src/fdr/ring.rs:65` `crc32`, `:227` `recover`); the `Alarm` record kind
  (`kernel/src/fdr/schema.rs:189` `Kind::Alarm`, item-40 semantics).
- **Composes with:** item 54's Sentinel / integrity-alarm seam (roadmap §J, spec-level, no blueprint yet).

---

## 0. Scope in one paragraph — read this first

This is the ZFS-**scrub** slice and nothing more: an idle-cadence background pass that walks the
durable `EventLog` and the closed FDR segments and **re-verifies the checksums that already exist**,
to catch latent at-rest bit-rot *before* a read needs the data. On non-ECC local storage a silent bit
flip in a closed segment is invisible until something tries to trust it; a scrub is proportionate
defense-in-depth (roadmap line 1140). **No new checksum primitive, no new dependency** — the SHA3
content-ids and CRC32 line-checks are already computed and stored; the scrub only *re-reads and
compares* them on a schedule. A mismatch emits exactly one FDR `Alarm` (hardware-fault evidence).

## 1. Goal / non-goals

**Goal.** A `fn scrub_once(...)` that (a) walks the durable `EventLog` chain re-verifying each event's
stored content-id against a recomputation from its body, (b) walks the closed FDR segment lines
re-verifying each line's CRC32, and (c) on any mismatch emits one `Kind::Alarm` FDR record carrying
the location of the corruption; called on an idle cadence from a NAMED constant with one authority
site (P3 rate discipline).

**Non-goals.** No repair/self-heal (detection + alarm only — the evidence trail must never be
*modified* by the subject of the evidence, per §L item 74 clause 4). No new hashing primitive. No new
dependency. Not the FDR recovery path (`recover` runs once at boot; the scrub runs repeatedly at
idle). Does not change any checksum, does not touch the write path, does not add a field to any
record (the `Alarm` kind and its stamp already exist).

## 2. Verified current state (grounded)

| Fact | Citation (live HEAD) |
|---|---|
| Durable EventLog has a SHA3 content-id chain + read-back integrity walk | `kernel/src/event_log.rs:481` (`verify_chain` — recomputes `event_id()` per event, follows `prev` to genesis, returns typed `ChainDefect` on mismatch), `:524` (`verify_chain_before_trust`) |
| The exact corruption class targeted (torn write / bad sector between hash and persist) | `event_log.rs:472-475` (doc: "corruption *between* hash and persist … this walk is the only observer that catches it"); `chaos.rs` `CorruptPayload` injection exercises it |
| Durable store keeps bodies retrievable (scrub can walk them) | `kernel/src/hydra.rs:925` (`by_id: HashMap<[u8;32], MeshEvent>`), on-disk lines re-parsed by `serde_json_like_parse` (`hydra.rs:972`) |
| FDR closed segments are CRC32-per-line | `kernel/src/fdr/ring.rs:8-9` (each line suffixed with CRC32 of payload), `:65` (`crc32`), `:263` (recovery validates `crc32(payload) == want`) |
| FDR recovery is READ-ONLY (precedent: never mutate segments under inspection) | `fdr/ring.rs:227-229` ("Never truncates — safe to run against a crashed writer's segments") |
| `Alarm` record kind exists (item-40 hardware-fault evidence semantics) | `kernel/src/fdr/schema.rs:189` (`Kind::Alarm`) |

## 3. Implementation plan (numbered)

Lives in a NEW module (proposed `kernel/src/fdr/scrub.rs`) next to `ring.rs`; called by item 64's
composition root on an idle timer.

1. **EventLog leg.** `scrub_once` walks the durable chain reusing the *same* recomputation
   `verify_chain` (`event_log.rs:481`) already performs — recompute each event's content-id from its
   stored body and compare against the id it is stored under. A `HashMismatch`/`BrokenPrev`
   `ChainDefect` is a detected at-rest corruption. (The scrub can either call `verify_chain` directly
   for a whole-chain pass, or iterate `by_id` for an incremental per-event pass — the incremental form
   is preferred for idle-cadence low-impact scanning; §7 flags the choice.)
2. **FDR segment leg.** Walk each *closed* segment file line by line, recompute `crc32(payload)`
   (`ring.rs:65`) and compare against the stored suffix — the exact check `recover` (`:263`) does at
   boot, run repeatedly at idle against closed (not the active) segment. Read-only, never truncates —
   inherits `recover`'s discipline (`:227`).
3. **Alarm on mismatch.** Any mismatch emits ONE `Kind::Alarm` FDR record (`schema.rs:189`) whose
   payload names the corruption location (event content-id or segment+line offset) — hardware-fault
   evidence, item-40 semantics. An uncorrupted store scrubs **silent** (no record — a clean scrub is
   not an event).
4. **Named cadence constant, one authority site.** `const SCRUB_INTERVAL: Duration` (or a tick count)
   defined once, pinned by a test (P3 rate discipline, the item-6 pin-test idiom). The scrub is
   idle-triggered by the composition root's loop; the interval is config-overridable but the *default*
   is a single named constant.
5. **Cost containment.** The scrub is bounded work per pass (a segment cap / event batch) so an idle
   scan never taxes a hot path — this is the ZFS-scrub "background, low-priority" posture, stated
   explicitly.

## 4. Required tests / proofs (CHECKLIST.md 5-point mapping)

The scrub is a read-only verification pass over existing checksums — not new algorithmic hot-path
code, and not a timing surface:

- **Item 1 (oracle).** The **planted-at-rest-corruption red→green** (the load-bearing proof, roadmap
  line 1145): flip a byte in a closed FDR segment / a persisted EventLog body, run `scrub_once`,
  assert it detects the mismatch and writes the `Kind::Alarm` record. Reuse `chaos.rs`'s
  `CorruptPayload` injection (already exercises the exact class). Then the negative oracle: an
  uncorrupted store scrubs **silent** (no `Alarm` emitted).
- **Item 3 (debug cross-check).** `debug_assert!` that the recomputed checksum comparison in the
  scrub matches the write-path checksum computation for the same bytes — a zero-release-cost sanity
  that the scrub re-uses, not re-implements, the checksum.
- **Item 2 (dudect):** **N/A(not-a-secret-timing-path)** — the scrub reads at-rest evidence with no
  secret input; nothing to time. Recorded, not faked.
- **Item 4 (asm spot-check):** **N/A(no-branch-free-crypto)** — CRC32/SHA3 already carry their own
  coverage under their existing HOT-PATHS rows; the scrub adds no new arithmetic hot path.
- **Item 5 (formal proof):** **N/A(re-verification-not-new-property)** — the checksum properties are
  already proven under `event_log.rs`/`fdr/` rows; the scrub is a scheduler around them.

**Manifest.** The scrub touches `kernel/src/event_log.rs` (a HOT-PATHS `@ZONE`, `HOT-PATHS.tsv:32`) and
adds `kernel/src/fdr/scrub.rs`; register a row and bump `event_log::` `min_tests` for the new
planted-corruption test so a deletion goes RED (item-6 anti-forgery floor). Cadence constant pinned by
its own test.

## 5. Falsifiable acceptance criteria

1. A planted at-rest corruption in a closed FDR segment is detected by the next `scrub_once` and
   writes a `Kind::Alarm` record naming the location. **Falsifier:** the scrub misses it, or writes no
   alarm → FAIL.
2. A planted corruption in a persisted EventLog body is detected via content-id recomputation.
   **Falsifier:** the scrub misses a `HashMismatch` the `verify_chain` walk would catch → FAIL.
3. An uncorrupted store scrubs silent (zero `Alarm` records). **Falsifier:** a clean store emits an
   alarm → FAIL.
4. The scrub never truncates/mutates a segment or event body (read-only, inherits `recover`'s
   discipline). **Falsifier:** any write to the inspected data → FAIL.
5. The cadence constant is a single named authority site, pinned by a test. **Falsifier:** a second
   cadence literal exists → FAIL.
6. `cargo tree -e no-dev` byte-unchanged (grep confirms only existing CRC32/SHA3 used). **Falsifier:**
   a new checksum crate added → FAIL.

## 6. Dependency gates (honest)

- **Gated on item 64.** Roadmap line 1138: "scrubbing an unwired store is pointless." Until item 64's
  composition root actually constructs a durable `FileEventStore` in production (item 2's defect,
  today constructed only in test code — see BLUEPRINT-ITEM-64 §0), there is no live durable store for
  the scrub to walk. **This item cannot be dispatched before item 64 lands.**
- **Composes with item 54** (Sentinel / integrity-alarm seam, roadmap §J, no blueprint yet): the
  `Alarm` this scrub emits is a natural producer for item 54's integrity-alarm consumer. Item 54 is
  not a *blocker* — the `Kind::Alarm` record already exists (`schema.rs:189`); item 54 layers a
  consumer on top. If item 54 has not landed, the alarm is still emitted and recoverable from the ring.
- **FDR leg is buildable against live code now** (the CRC32 machinery is landed); only the EventLog
  leg needs item 64's wired durable store.

## 7. Operator-decision points (flagged)

- **Whole-chain vs incremental scan.** `verify_chain` walks the entire chain each call (O(n)); an
  idle-cadence scrub of a large log may prefer an incremental cursor (scan a batch per pass). Which to
  ship is an operator/executor call weighing scan-latency vs idle-CPU; recommend incremental with a
  batch bound (§3 step 5). Flagged.
- **Cadence value.** The default `SCRUB_INTERVAL` is a real operational tradeoff (detection latency vs
  idle wakeups) — a named constant, but its *value* is an operator ruling, not an engineering default.
