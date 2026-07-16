# BLUEPRINT P12 — DURABLE STORAGE, DEPLOY & OPS FLOOR

> *Phase 12 of the R2 master roadmap. "Persist only what Phase 7 made correct."*
> Anchors: **D1, S1, S2, E11, E26, E27, E28, E29, E30, E48, E49, E50, F37, F38**.
> Depends on: **Phase 7 (money-law closure)** — HARD. Parallel-safe with: Phase 9, 10, 11.
> Scope note (ARCHITECTURE.md §0 SCOPE RULE): every gate/schedule here is a **canonical-repo /
> operator-build fence**, never a runtime control on hubs. pgrust promotion to *primary* stays a
> per-hub choice (M5); this phase only proves the *fallback* works and the *default* is durable.
> All claims below carry file:line evidence or an honest NOT BUILT, re-verified against the live
> tree 2026-07-16. Sources: R1-C §K5 + §2.6, R1-E §E48/E49/E50/E11, direct reads.

---

## 1. Current-state evidence (read these two findings first)

**FINDING A — the automated hygiene loop is DEAD (silent regression).** `tools/deep-clean` is
built, offline-buildable (`Cargo.toml`: `rusqlite 0.31 bundled`), and was verified working
(subcommands `vacuum|clean|prune|all`, hard deny-list at `src/main.rs:22` structurally excluding
`.env*`/`*secret*`/`backups/cold/`, JSONL audit log to `/root/.backups/clean-log/`). Two Hermes
cronjobs are registered (`deep-clean-daily`, `deep-clean-weekly-audit`). **But `hermes cron
status` returns `✗ Gateway is not running — cron jobs will NOT fire`** (verified this session; the
`hermes cron list` view repeats `⚠ Gateway is not running — jobs won't fire automatically`). The
daily disk-hygiene that MEMORY.md records as "created + running" has never fired on schedule. Every
"automated hygiene" claim in the repo is currently false. This is the highest-signal find of the
phase: an operator who believes hygiene is running is accumulating disk pressure blind.

**FINDING B — the pgrust backup/fallback story has never been exercised end-to-end.** The systemd
unit `deploy/pgrust.service` sets `ExecStart=/usr/local/bin/pgrust --config /etc/pgrust/pgrust.toml`,
but `/usr/local/bin/pgrust` **does not exist** (verified: `ls` → No such file; not on `PATH`). The
feature-gated `PgEventStore` seam (`event_log.rs:16-20` `innovate:` marker) and the sqlx adapter
compile only under `--features pgrust` and have **never run against a real pgrust server**.
`internal-retrieval-living-memory-blueprint.md:88-90` records pgrust upstream as **~67% Postgres-
compatible, extensions incomplete**. So E26/E11 ("pgrust as backup/fallback") is spec + adapter +
unit-for-an-absent-binary — three parts of a story whose middle (a running binary) is missing.

**The rest of the current-state ledger:**

- **E28 durable event replay — nothing to replay from.** `kernel/src/event_log.rs:162` defines the
  `EventStore` trait; the only implementation is the in-memory `MemEventStore` (`:187`), explicitly
  "non-durable and not shared across processes — single-node local-first only" (`:18`). The pure
  replay reducer (`fold_transitions`) works, but a `kill -9` loses the entire log. There is **no
  durable store to replay from.**
- **E29 disk BlockStore — in-memory only.** `kernel/src/backup.rs:29` defines `BlockStore`
  (`put`/`get`/`len`, sha3-keyed `Hash = [u8;32]`); the only impl is `MemStore` (`:45`, a
  `HashMap<Hash, Vec<u8>>`). `BackupOrgan` (`:111`) does CDC-chunk → dedup → bit-exact `restore`
  (`:169`) with Verified-by-Math round-trip tests, but the module header itself notes "a real
  append-log / R2 backend can drop in later" — the disk adapter is that missing drop-in.
- **E27/E50/F38 COLD archives — real but drill-less.** `/root/.backups/cold/` holds genuine
  archives (`buckets-c-2026-07-16.tar.zst` 3.9 GB, `claude-projects-…` 415 MB, `state-db-…` 503 MB,
  plus `state-db-preprune-1236.db`). They were produced by **terminal ops, one-way**. `zstd` is
  present (`/usr/bin/zstd`). **No restore has ever been drilled** — no script, no runbook, no drill
  log. An untested backup is not a backup; the 3-2-1-1-0 discipline's final leg ("0 errors on
  restore-verify") is unmet.
- **E30/F37 deep-clean — see Finding A** (built; schedule lives only on the Hermes host, not in the
  repo, so a fresh hub cannot reproduce it).
- **E49 OpenTofu — zero.** No `*.tf`/`*.tofu` file anywhere; no `tofu`/`terraform` binary on the
  host. R1-C §2.6 flags the originally-planned `backend "pg"` remote-state approach **AT RISK**:
  OpenTofu's pg backend needs Postgres **advisory locks** (`pg_advisory_lock`) for state locking,
  and pgrust is ~67%-compatible and not installed — the lock behavior must be *verified* before
  that backend is safe, or a different backend chosen.
- **S1/S2 systemd — only for the ghost binary.** The single unit in-repo is `deploy/pgrust.service`
  (for the absent binary). There is **no unit/timer for anything that actually runs** (the
  `native-spa-server` binary, the deep-clean job). S2 (modular monolith, single host) holds by
  construction; the microVM launcher is a fail-closed KVM probe only (`isolation/microvm.rs`,
  `innovate:` VMM launch), fleet size = 1, so `fleet>5` is NOT met — probe-gated is honest.
- **E48 H8 runbook — ALREADY BUILT, reuse.** Confirmed via R1-E §E48: `docs/red-team/2026-07-13/
  H8-SECRET-SCRUB-RUNBOOK.md` exists (status CLOSED, evidence-rich) alongside
  `docs/ops/P1-PAUSE-SECRET-PUSH-RUNBOOK.md`. **This phase does NOT rebuild it** — it is cited as
  the model for the restore/pgrust drill runbooks (§3, §5) and as the secret-hygiene precondition
  for archiving (archives must never carry live secrets; the deny-list at `deep-clean/src/main.rs:22`
  already enforces the exclusion, H8 documents the scrub discipline).

---

## 2. The disk-backed `BlockStore` adapter (E29)

**Design.** Add a `FileBlockStore` implementing the existing `BlockStore` trait (`backup.rs:29`) —
zero trait changes, so `BackupOrgan<S>` and every round-trip test keep working with `S =
FileBlockStore`. Layout is content-addressed under a root dir, identical keying to the in-memory
store (sha3-256 block id, `Hash = [u8;32]`):

```
<root>/blocks/<hex[0:2]>/<hex[2:4]>/<hex>        one file per unique block, named by its sha3 id
<root>/manifests/<name>.manifest                 ordered block-id list (Manifest, backup.rs:82)
```

- **`put(id, bytes) -> bool`**: fan-out by the first two id bytes (65 536-way sharding avoids a
  single hot directory). Idempotent and crash-atomic: write to `<root>/tmp/<id>.partial`, `fsync`,
  then `rename` into place (POSIX rename is atomic on the same filesystem). If the final path
  already exists, it is a dedup no-op → return `false` (mirrors `MemStore` semantics exactly). The
  content-address IS the integrity check: on `get`, re-hash the bytes and compare to the filename;
  a mismatch is a fail-closed `Err`/`None`, never a silent serve of corrupt data.
- **`get(id) -> Option<&[u8]>`**: the trait returns a borrowed slice, which a disk store cannot
  hand back from a `read` without owning a buffer. Resolve by having `FileBlockStore` hold a small
  bounded LRU read-cache (`HashMap<Hash, Vec<u8>>`) so the borrow is valid, OR — cleaner — add a
  sibling `get_owned(&self, id) -> Option<Vec<u8>>` default method to the trait and have
  `BackupOrgan::restore` prefer it. **DECART note:** the borrowed-slice signature is the one seam
  that forces a decision; pick the additive `get_owned` default (no breakage to `MemStore`, no new
  dep) over an LRU (which adds eviction complexity). Record it inline.
- **No new dependency.** Uses `std::fs` + the kernel's own `sha3_256` (`event_log.rs:30`). Consistent
  with M6/V2 (zero-dep at the storage boundary) and D6 (content-addressing).

**Falsifiable acceptance:** the four existing `backup.rs` round-trip tests
(`restore_is_byte_identical`, `one_byte_edit_dedups_over_90pct`, `identical_rebackup_fully_dedups`,
`missing_block_fails_closed`) pass verbatim against `FileBlockStore`; a block whose on-disk bytes
are flipped 1 bit is rejected at `get` (fail-closed, not served); a `kill -9` between `.partial`
write and `rename` leaves NO half-written block visible to `get` (the temp file is invisible;
`put` is all-or-nothing).

---

## 3. COLD-archive `restore-verify` design (E27/F38 + pgrust drill E50/E11)

**Where it lives.** Extend `tools/deep-clean` with two new subcommands (it already owns
`/root/.backups/`, has the deny-list, and logs JSONL to `/root/.backups/clean-log/`) — or a sibling
`tools/cold-archive` crate if the operator prefers separation of "delete" from "preserve". Prefer
extension: one binary, one audit-log convention, one deny-list.

- **`archive <src> <dest.tar.zst>`**: tar the source, pipe through `zstd` (present at
  `/usr/bin/zstd`; shell it, or add the `zstd` Rust crate behind a DECART — shelling avoids a new
  dep and is the ponytail choice). On completion, compute and write a `.sha3` sidecar of the archive
  bytes. Refuses to run if any path in `src` matches the deny-list (no `.env*`/secret material ever
  enters a COLD archive — reuses the `excluded()` gate at `main.rs:59`).
- **`restore-verify <dest.tar.zst>`**: the load-bearing new capability — a **drill**, not a
  one-way step. Steps: (1) verify the archive's `.sha3` sidecar matches the file on disk;
  (2) decompress + untar into a scratch dir (`/root/.backups/restore-drill/<ts>/`); (3) for a
  SQLite payload (the `state-db` archive), open the restored `.db` and run
  `PRAGMA integrity_check` → must equal `ok`; (4) for a file tree, re-hash every file and compare
  against a manifest captured at archive time → **byte-identical or FAIL**; (5) append a drill log
  line to `/root/.backups/clean-log/restore-drill-<ts>.jsonl` recording `archive`, `sha3_match`,
  `integrity_check`, `bytes_verified`, `result`; (6) delete the scratch dir. The drill is
  **read-only against production** — it restores to scratch, never over live data.

**The pgrust restore leg (E50/E11) — explicitly included.** A separate drill sub-mode
`restore-verify --pgrust <dump>`: (1) requires a running pgrust (see §5); (2) restores a logical
dump into a *scratch* pgrust database (never the live one); (3) runs an integrity probe appropriate
to pgrust's ~67%-compat surface — at minimum `SELECT count(*)` parity against the source event count
and a re-fold of the restored event log to confirm the tip event-id matches; (4) logs the same
JSONL shape with `integrity_check=ok` on success. This is the ONLY thing that converts "pgrust is
our backup" from an assertion into a demonstrated fact. Model the runbook on the already-built H8
runbook (§1 Finding, E48): dated, evidence-rich, status field.

**Falsifiable acceptance:** `restore-verify` on a freshly-made COLD archive produces a byte-identical
restore and a drill-log line with `integrity_check=ok`; corrupting 1 byte of the archive makes the
sha3-sidecar check FAIL loudly (never a silent bad restore); the drill log **explicitly includes a
pgrust leg line with `integrity_check=ok`**. This closes the 3-2-1-1-0 "0 errors" leg with an
automated check, not a manual one.

---

## 4. The durable `EventStore` design + the Phase-7-then-file-then-pg sequencing (E28)

### 4.1 Why Phase 7 MUST land first (hard constraint, argued out)

The blueprint's own ordering is not bureaucratic — it is a correctness gate. R1-C §0.2 documents a
live **dedup-id divergence**: `commit_after_decide` (`event_log.rs:283-303`) computes
`let id = ev.event_id()` at `:292` **before** delegating to `append`, and `append` (`:257-273`)
*re-binds* `ev.prev` to the current tip at `:261-265` and *then* re-hashes at `:266`. So the
duplicate check at `:293` tests a **different id** than the one actually stored. Concrete failure:
replay a zero-`prev` event onto a non-empty log → `contains()` misses → `decide` re-runs → a
**second** event is committed. The existing idempotency test only covers the genesis (empty-log)
case, so it stays green over the bug.

Today this bug is *ephemeral* — `MemEventStore` dies with the process, so a duplicated tip is lost
on restart. The instant we add a **durable** store, replay becomes load-bearing, and replaying a
log that contains a wrongly-deduplicated event would **durably persist the duplicate** and any
money event chained after it. Persisting a corrupt log is strictly worse than not persisting at all:
the corruption becomes the source of truth, survives restarts, and replicates. Therefore the durable
`EventStore` **must not be built until Phase 7 (S9/S5, P0-A2) has bound `prev` before the dedup
check and shipped the RED replay-on-non-empty-log test.** This phase consumes Phase 7's fix; it does
not work around it. (This is the sole reason P12 depends on P7 and is why P12 is Wave-2, not Wave-0.)

### 4.2 Build the file-JSONL store FIRST, promote to pg only after pgrust is proven

**Step 1 — `FileEventStore` (JSONL append log), the durable default.** Implement the `EventStore`
trait (`event_log.rs:162`) over an append-only JSONL file, one canonical-encoded `MeshEvent` per
line, keyed by content-id in a sidecar index (or scanned on load). Crash-safety contract: `append`
does `write` + `fsync` of the line **before** returning `Committed(id)` and before updating the
in-memory `tip`. An event is "acknowledged" only after its bytes are durably on disk. On restart,
the store rebuilds `contains`/`tip` by scanning the JSONL and re-folding — the tip event-id is a
pure function of the durable bytes.

- Torn-write handling: a `kill -9` mid-`write` can leave a trailing partial line. On load, a line
  that fails to parse or whose recomputed `event_id()` ≠ its recorded id is **truncated** (it was
  never acked, so no consumer ever saw its `Committed`). This is what makes "loses zero *acked*
  events" true: acked ⇒ fsynced ⇒ intact; not-fsynced ⇒ not-acked ⇒ safe to drop.
- No new dep: JSONL via the kernel's existing serialization + `std::fs`. Std-only, offline, matches
  D1 "native vectorless default" and M6.

**Step 2 — `PgEventStore`, ONLY after §5 proves pgrust.** The `innovate:` marker at
`event_log.rs:16-20` already names this as "the only seam." Promotion to pg is gated on: (a) §5
smoke-test green, (b) the advisory-lock falsifier (§7) if pg is also chosen as tofu state backend.
Until then, `PgEventStore` stays feature-gated and unshipped in the durable-default path. Per the
SCOPE RULE, a *hub* may still flip pg on at its discretion (M5) — the canonical build ships
`FileEventStore` as the durable default.

**Falsifiable acceptance:** a `kill -9` mid-`append` against `FileEventStore` loses **zero already-
acknowledged events**, and replay reproduces the **exact same tip event-id**; the RED replay-on-non-
empty-log test from Phase 7 is green (precondition, re-asserted here); `PgEventStore` is NOT enabled
in the default feature set until §5 passes.

---

## 5. pgrust install + smoke-test plan (E26/E11)

**Goal:** prove the fallback WORKS, end-to-end, once. Promotion to primary stays a hub choice (M5) —
out of scope. Steps:

1. **Install the binary** the `deploy/pgrust.service` unit already points at: obtain/build the
   pgrust binary, place at `/usr/local/bin/pgrust`, create the `pgrust` service user + `/var/lib/
   pgrust` data dir + `/etc/pgrust/{pgrust.toml,pgrust.env}` from the in-repo `deploy/pgrust.*`
   templates. (The unit hardening is already correct: `NoNewPrivileges`, `ProtectSystem=strict`,
   `CapabilityBoundingSet=` empty, `MemoryDenyWriteExecute=yes`.)
2. **Smoke-test as a service:** `systemctl enable --now pgrust`; confirm it binds
   `127.0.0.1:5432` (per `pgrust.env` `PGRUST_LISTEN`); confirm `PGRUST_RLS_CROSS_TENANT=deny` is
   honored (the DK-05 red-line).
3. **Exercise the adapter:** build the kernel with `--features pgrust`, run its `migrate()` DDL
   against the live server, write N events through `PgEventStore`, read them back, and confirm the
   re-folded tip event-id matches the `FileEventStore` tip for the same input (cross-store
   equivalence — the durable-default and the fallback must agree).
4. **Capability probe (feeds §7):** run `SELECT pg_advisory_lock(1)` / `pg_advisory_unlock(1)`
   against pgrust and record the exact result. This single probe decides the tofu state backend.
5. **Document as a runbook** in the H8 style (E48 model): dated, with the literal commands + output,
   status field. This runbook plus the §3 pgrust drill line together retire the "never exercised"
   gap.

**Falsifiable acceptance:** `systemctl status pgrust` is `active (running)` with a real binary;
`PgEventStore` round-trips N events with a tip-id matching `FileEventStore`; the advisory-lock probe
result is recorded (whichever way it lands). If pgrust cannot be obtained/built in-environment, the
honest outcome is a dated NOT-AVAILABLE note in the runbook + the tofu backend defaults to the
de-risked alternative (§7) — silence is not an option.

---

## 6. Systemd units/timers + Hermes cron revival (S1, E30/F37)

**Problem:** a freshly-provisioned hub cannot reproduce its own operational schedule from the repo,
because the only in-repo unit is for a binary that isn't installed, and the deep-clean schedule
lives only on the Hermes host (§1 Finding A). Fix both halves.

- **`deploy/native-spa-server.service`** — a systemd unit for the binary that actually runs today
  (the static SPA server, `Dockerfile:53` `FROM scratch` single binary). Same hardening template as
  `pgrust.service`: `NoNewPrivileges`, `ProtectSystem=strict`, `EnvironmentFile=`, dedicated user,
  `Restart=on-failure`.
- **`deploy/deep-clean.service` + `deploy/deep-clean.timer`** — a oneshot unit invoking
  `deep-clean all --commit --days 7` and a timer (`OnCalendar=*-*-* 04:37:00`, `Persistent=true`
  so a missed run fires on next boot) that mirrors the existing Hermes `deep-clean-daily` schedule.
  This makes the hygiene schedule **reproducible from canon**, independent of Hermes. A second
  `deep-clean-audit.timer` (weekly, dry-run) mirrors `deep-clean-weekly-audit`.
- **Revive the dead Hermes gateway (Finding A):** run `sudo hermes gateway install --system` so the
  already-registered `deep-clean-daily`/`deep-clean-weekly-audit` jobs actually fire. This is the
  trivial-but-load-bearing fix. **Both** paths (in-repo systemd timer AND revived Hermes gateway)
  are worth having: the systemd timer is the canonical, hub-reproducible schedule; the Hermes
  revival unblocks the *existing* automation immediately. The acceptance test targets the in-repo
  unit specifically (not the ad-hoc Hermes registration), so the schedule is provable from a clean
  checkout.

**Falsifiable acceptance:** `systemctl list-timers` shows `deep-clean.timer` registered **from the
in-repo unit file** (not only a Hermes-host registration), and the hygiene job **actually fires on
schedule** (demonstrated — e.g. a forced `systemctl start deep-clean.service` produces a fresh
`/root/.backups/clean-log/<ts>.jsonl` entry, and `hermes cron status` shows the gateway `running`
with next-run times). Configured-but-never-fired is a FAIL (that is exactly today's regression).

---

## 7. OpenTofu single-host module + the state-backend decision (E49)

**Module.** One `opentofu/` module provisioning the single host's reality as code: a
`dmacvicar/libvirt` `libvirt_domain` resource for one KVM microVM node (the standard module path;
host KVM is already probed by `isolation/microvm.rs`), plus the systemd units/env files from §6 as
managed artifacts. Kept minimal — this is the *floor*, not a fleet manager.

**The state-backend decision (reasoned, per R1-C §2.6 — this is a HARD sequencing constraint, not a
free choice):** the originally-planned `backend "pg"` remote-state approach is **AT RISK**. OpenTofu's
pg backend requires Postgres **advisory locks** (`pg_advisory_lock`, a per-database global mechanism)
for state locking; pgrust is **~67% Postgres-wire-compatible with incomplete extension support** and
(until §5) not installed. Proceeding blindly risks a state backend whose lock is a silent no-op —
concurrent `tofu apply` could corrupt state. Decision tree:

- **IF §5's advisory-lock probe shows `pg_advisory_lock` works correctly on pgrust** → `backend "pg"`
  is safe; use it, and record the probe output as the justification.
- **ELSE (probe fails, is a no-op, or pgrust unavailable)** → use a **de-risked alternative**:
  `backend "local"` (state file on the single host, committed-gitignored, backed up by the §3 COLD
  archiver) for a genuinely single-host floor, OR an S3-compatible backend (e.g. a self-hosted
  MinIO/Garage bucket with real object locking) if remote state is wanted. **Either way, write a
  dated DECART** stating which backend was chosen and why (the probe result), per E49.

There is no path where this phase "blindly proceeds with the original pg plan." It either verifies
pg is safe or picks the alternative — with a written DECART in both branches.

**microVM launcher stays probe-gated.** Confirm (do not build early): the actual VMM launch remains
the `innovate:` follow-up in `isolation/microvm.rs`; the tofu module provisions one node but the
in-process microVM *launcher* activates only when **fleet size exceeds 5** (S1/S2 canon threshold).
Fleet = 1 today, so probe-only is correct. Building the launcher now would violate the canon
threshold and YAGNI.

**Falsifiable acceptance:** `tofu apply` from a **clean checkout** creates one real microVM node, and
a subsequent `tofu plan` shows an **empty diff** (idempotent) — OR, if the operator declines
OpenTofu for now, a **dated DECART document** stands in place of the apply/plan proof, explaining
why. The state-backend choice carries its own DECART citing the §5 probe result.

---

## 8. Acceptance criteria (numbered checklist)

The phase is DONE when every line is demonstrated (not merely configured):

1. **BlockStore (E29):** the four `backup.rs` round-trip tests pass verbatim against
   `FileBlockStore`; a 1-bit on-disk corruption is rejected at `get` (fail-closed); a `kill -9`
   between `.partial` write and `rename` leaves no half-written block visible.
2. **EventStore durability (E28):** a `kill -9` mid-`append` against `FileEventStore` loses **zero
   already-acknowledged events**, and replay reproduces the **exact same tip event-id**.
3. **Phase-7 precondition:** the RED replay-on-non-empty-log test (P0-A2, `event_log.rs:283-303`
   dedup-id fix) is green; the durable store was built strictly AFTER it. `PgEventStore` is NOT in
   the default feature set until criterion 6 passes.
4. **COLD restore-verify (E27/F38):** a COLD archive → `restore-verify` is **byte-identical**, and
   the drill log shows `integrity_check=ok`; a 1-byte archive corruption fails the sha3-sidecar
   check loudly.
5. **pgrust restore drill (E50/E11):** the drill log **explicitly includes a pgrust leg** with
   `integrity_check=ok` (or a dated NOT-AVAILABLE note if the binary cannot be obtained).
6. **pgrust smoke (E26/E11):** `systemctl status pgrust` is `active (running)` with a real binary at
   `/usr/local/bin/pgrust`; `PgEventStore` round-trips N events with a tip-id matching
   `FileEventStore`; the `pg_advisory_lock` probe result is recorded.
7. **Systemd + cron (S1, E30/F37):** `systemctl list-timers` shows `deep-clean.timer` registered
   **from the in-repo unit file**; the hygiene job **actually fires** (fresh `clean-log` entry
   demonstrated); the Hermes gateway is revived (`hermes cron status` → running); a
   `native-spa-server.service` unit exists in-repo.
8. **OpenTofu (E49):** `tofu apply` from a clean checkout creates one microVM node and a subsequent
   `tofu plan` is **empty** (idempotent) — OR a dated DECART declines it. The state-backend choice
   (pg iff §5 probe passes, else local/S3) carries its own DECART.
9. **microVM gate (S1/S2):** the microVM *launcher* remains probe-gated and unbuilt (fleet=1 < 5);
   confirmed, not built early.
10. **E48 reuse:** the H8 secret-scrub runbook was confirmed present and REUSED as the runbook model
    (§3/§5), not rebuilt; no COLD archive carries deny-listed secret material.

**Anchor coverage:** D1 (§4.2 durable-default confirm) · S1 (§6) · S2 (§7 microVM gate) · E11 (§3,§5)
· E26 (§5) · E27 (§3) · E28 (§4) · E29 (§2) · E30 (§6) · E48 (§1,§3 reuse) · E49 (§7) · E50 (§3) ·
F37 (§6) · F38 (§3). All 14 anchors land; zero deferred outside the phase.

---

*Blueprint P12 complete. Builds nothing; specifies the durable-storage/deploy/ops floor so that
Phase 7's correctness is the only thing that gets persisted. Two flagged regressions to surface to
the operator immediately: the DEAD Hermes cron gateway (hygiene loop not firing) and the MISSING
pgrust binary (fallback never exercised). Sequencing hard constraints: P7-dedup-fix → FileEventStore
→ (pgrust-proven) → PgEventStore; and advisory-lock-probe → tofu state-backend choice.*
