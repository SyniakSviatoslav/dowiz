# MIGRATION: sha256 record ids (M0) -- the one format change, and how it rolls back

Status: 2026-10-31, lane W-MG-MIG, built and proved LOCALLY (operator 10-31: no deploy of any kind
from a lane). Prod runs `ac5ff0c4` (FNV ids, flags 0). `main` since `58154dd9` writes sha256 ids with a
16-byte nonce and `FLAG_SHA256` (dowiz-hub M0). This is the runbook for the single deploy that carries
M0 to the venues, every step with its check and its rollback. Every command named here exists in the
tree; every number is quoted from a run.

## 0. The rule in one paragraph

Expand, then contract. The first build that READS both schemes WRITES the old one: `SHA256_VENUES = ""`
in `workers/api/wrangler.toml`. A venue is flipped by naming it there and deploying (`"qa-durres"`
first, then `"*"`). A flip is ONE-WAY per venue and lives in the bytes, not in the config.

**How per-log state makes that per-venue (W-MG-TRANS).** The state is per LOG: a log is on sha256 once its
newest record is (`Entropy::writes_sha256`), whatever the var says afterwards. The DECISION is per venue:
every door the Worker has to a log of a venue arms it for that venue (`scheme::arm`, one call, seed and
scheme), so while the var names the venue each of its logs turns sha256 at its own next write. A venue is
therefore flipped log by log, not atomically: a log nobody wrote since the flip is still legacy, and it is
read as legacy, because every reader asks the record it holds and no reader compares two logs. After a
rollback to the floor the var says `""`: the logs already flipped stay flipped, the ones not yet flipped
stay legacy, and neither is broken. "Flipped venue" means: its logs are allowed to be sha256, and none that
is ever goes back. Before any flip, rolling back to
prod is safe because nothing written needs the new reader. After a flip, roll back only to a build that
reads sha256 -- `tools/deploy/rollback.sh` refuses the others (exit 53), and `deploy.sh` refuses a forward
build without the reader (exit 14). No new format bytes from this
lane: the flag bit and the nonce already exist in main; it only decides WHEN they are first written.

**REVOKE v2 (W-MG-MERGE2, 2026-11-01).** A REVOKE body is `bebop-revoke/2` with a 32-byte `tip`. This changes
no step below: the Worker has no path that writes a DOC or a key op (`grep -rn 'KeyOp\|SealedHub' workers/api/src`
prints nothing), so no live image holds a `bebop-revoke/1` record, and none needs converting. In a file that
did, such a body now reads as no REVOKE at all: it revokes nothing and the table disagrees (`keys_disagree`).

**Id v2 (W-MG-HOLES, 2026-10-31).** "sha256 ids" below means `sha256("bebop-record/1" ‖ flags ‖ actor ‖
seq ‖ prev ‖ payload)` (`doc::id_message`, FORMAT §9.4), not M0's `sha256(prev ‖ payload)`, which no
live record carries and no build reads any more: a flagged record with an M0 id is `broken` by rule
(`verify_id`; not measured on a real uncut file), and a cut file written before HOLES has no `seg_hash`
in its TRUNCATE and is refused 75 (measured: the old `anch.store` read `verify status=75`). So the
expand release of step A MUST be a build at or after HOLES, and the floor is such a build. A build
from `58154dd9` up to HOLES is NOT a rollback target once any venue is flipped: its reader calls every
v2 record broken. The wire needs no new field: a stock record has actor 0,
seq = its position and flags derived from its text (`chain::stock_flags`), and a record carried over
a rebase loses `FLAG_NONCE_DERIVED` because its `prev` changed. Id v2 binds the sha256 records only: a
legacy record's actor, seq and flags stay free, so a `seal_hub` of a real venue is exposed on its
pre-seal records until those records leave the file (FORMAT §14).

## 1. What `ac5ff0c4` does with a sha256 record (measured, not inferred)

Measured on M0 ids below; RE-MEASURED on id v2 by W-MG-ATK4, 2026-10-31: the same quarantine, and §1.3's
numbers moved. The lines, with the real old binary on an image the new code wrote:

```
old-read hub:     len=5 events=3 quarantined=2 chain={records:5 chained:3 legacy:0 broken:2 redacted:0} reasons=["kind", "kind"]
old-read stock:   len=4 events=4 quarantined=0 shelf=[("salmon", StockLevel { on_hand: 147, reserved: 0 })]
old-read log:     len=2 entries=1 quarantined=1 reasons=["kind-framing"]
old-read consent: len=3 entries=3 quarantined=0 state=true     <- a withdrawal, SILENTLY not read
after 400 old appends, new-read hub: len=405 events=403 quarantined=2 chain={records:405 chained:0 legacy:403 broken:2 redacted:0}
```

So a past-the-floor rollback loses every order since the flip for good (the first old grow strips the flag:
`chained:0`, the log no longer reads as flipped and is not `broken:400` as M0 measured), loses no stock
movement, and turns a consent withdrawal back into a grant with nothing quarantined. A promoted `tablet.wasm`
from before the reader refuses a flipped venue's reply: `tablet rebase: REFUSED Reply("the replay does not
land on the hub's tip")`, which is why the expand release ships a rebuilt one (§2).

Main's note said a rollback "would misread new records". Measured 2026-10-31 with the real old binary
(`/root/lanes/deploy-src` = `ac5ff0c4`, `crates/dowiz-hub` built as is) against an image the new code
wrote (3 legacy + 2 sha256 orders); the two probe binaries lived in the lane session's scratchpad
(migprobe, old and new), not in the tree -- rebuild them from §1's description if the numbers are doubted:

```
new-mixed: len=5 events=5 orders=5 quarantined=0 chain={records:5 chained:2 legacy:3 broken:0 ...}
old-read:  len=5 events=3 orders=3 quarantined=2 chain={records:5 chained:3 legacy:0 broken:2 redacted:0}
old-read:    quarantined at=0 reason=kind id=24ff28a58787
old-read:    quarantined at=1 reason=kind id=24efd73d54dc
```

1. NOT A REFUSAL. The old build loads the image and QUARANTINES every sha256 record (`reason=kind`: the
   nonce's first byte is read as the event kind, `EventKind::from_byte` says `None`) and serves around
   them. Every order placed since the flip vanishes from the console, the kitchen and the day's takings,
   with a healthy-looking reply. `chain_check` reports them `broken` -- nightly, not at the door.
2. ITS WRITES POISON THE CHAIN. The old build appended two orders after the sha256 ones; the new code
   read the result as `events=7 ... chained:2 legacy:3 broken:2`: the two FNV records newer than a
   sha256 record are `broken` for good (`walk_check`: the scheme moves forward once, and never back).
3. ITS GROW CORRUPTS. 400 appends forced the old `grow` (`copy_chain_bytes` through the old
   `append_bytes`, which composes cell 11 from the actor key alone):
   `old-append: image 65536 -> 131072 bytes (grew=true)`, then the new code read
   `quarantined=2 reason=kind ... chained:2 legacy:3 broken:400` -- the flag is stripped, so the two
   sha256 orders decode as garbage in BOTH builds, and the 400 records written by the old build are
   broken.
4. A pure legacy image reads the same on both: `old-read: len=3 events=3 ... chained:3`,
   `new-read: len=3 events=3 ... legacy:3`, identical tip.

So "misread" is right but too soft: silent loss from the fold, a poisoned chain on the next forward
deploy, and corruption on the first grow. That is what the floor in §3 prevents.

## 2. What was built (files, and the guarantee each carries)

| Piece | Where | Guarantee |
|---|---|---|
| The switch | `crates/dowiz-hub/src/chain/scheme.rs` (`Entropy::{set_sha256, writes_sha256, stamp}`), `Hub::set_sha256` in `crates/dowiz-hub/src/chain.rs`, `LogImage::set_sha256` in `crates/dowiz-hub/src/logimage.rs`, `StockLog::set_sha256` in `crates/dowiz-hub/src/stock/log.rs` | `false` writes a record byte for byte as `ac5ff0c4` does (FNV over `prev ‖ payload`, flags 0, no nonce); one-way once the log is sha256 |
| The per-venue config | `workers/api/src/scheme.rs` (`SHA256_VENUES`: `""` nobody, a list of venue IDS, `"*"`), installed in `workers/api/src/lib.rs` `main` and `workers/api/src/hubdo.rs` `new`; every log the Worker may append to is armed in ONE call, seed and scheme (`scheme::arm` / `arm_scheme`); `platform_store::{load_log_at, with_log_at}` take the venue as a required argument and arm inside; an object that does not know its name is legacy unless the list is `*` (`scheme::chosen`) | the expand release writes legacy everywhere; the crate's own default stays sha256 so no test in the crate moved |
| The armed state travels | `Entropy` is one object (seed, counter, scheme); `Entropy::forward`, `LogImage::{fresh, fresh_sized, reload}` hand it to whatever replaces a log | no rebuild or reload resets it: a consent erasure, a catalogue-journal compaction and a chat prune all stay on the log's scheme with its seed |
| The rotation of a log in transition | `crates/dowiz-hub/src/rotate.rs`: when a sha256 checkpoint would sit above a carried legacy record the checkpoint is written LAST | the scheme moves forward in the file too: no honest record reads `broken`, and the newest record keeps the log one-way past the rotation; a log wholly on one scheme keeps the checkpoint first, byte for byte |
| The tablet | `glue/tablet.rs` writes its log's scheme; `stock/adopt.rs` `keeps_as_written`; `glue/merge.rs` `adopted_prefix`; a rebuilt `tablet.wasm`, and `deploy.sh` refuses a module that is not its source (exit 15) | a tablet does not flip a venue, a derived nonce that loses its flag over a rebase is still counted (`StockLog::derived`), a tablet reads a flipped reply, and a retry with more records adopts each movement once |
| The tripwire | `tools/deploy/scheme-armed.sh` | every Worker file that constructs a `Hub`/`LogImage`/`StockLog` arms it or is named read-only with a reason; `scheme-armed: 12 files armed, 10 read-only by name, 0 unarmed` |
| The floor | `tools/deploy/deploy.sh` reads the var with a TOML parser (`scheme_var.py`) before anything is uploaded, marks a flip (`sha256-flipped`, never removed) BEFORE the upload, records every uploaded version in `versions.log` with `declared=` before any later check can exit, the first build that declares `SHA256_VENUES` as the floor (`sha256-floor`); `tools/deploy/rollback.sh` step 0 takes four witnesses of a flip (the mark, a list in `versions.log`, the tree's `wrangler.toml`, any commit that ever set a list) and refuses a target not recorded at or after the floor as a build that declared the var (exit 53), `ROLLBACK_PAST_FLOOR=1` overrides and says what will happen | no rollback can reach a build that quarantines flipped venues' orders by accident, a lost cache included |
| Kill tests | `crates/dowiz-hub/src/chain/mig_tests.rs` (6), `workers/api/src/scheme.rs` tests (2), `tools/deploy/rollback.test.sh` (+8), `tools/deploy/test.sh` (+5) | §4 |

## 3. The runbook: order, check, rollback

Preconditions, all local: `cd crates/dowiz-hub && cargo test --lib` green; `cd workers/api && cargo test --lib`
green; `bash tools/deploy/scheme-armed.sh` prints `0 unarmed`; `bash tools/deploy/rollback.test.sh` and
`bash tools/deploy/test.sh` green; `sh crates/bebop-wasm/tablet.sh` run on the commit (`deploy.sh` compares the source digest with `tablet.wasm.src` and refuses a difference, exit 15; equal digests say the `.src` file names these sources, not that the wasm bytes were built from them). `SHA256_VENUES = ""` in
`workers/api/wrangler.toml`. Prod version id noted (`wrangler deployments status --json`; deploy.sh prints
it as `serving now:`).

### Step A -- the expand release (reads both, writes legacy)

1. `bash tools/deploy/deploy.sh` from main. It prints `sha256 floor: <version-id>` (the floor is
   recorded in `/root/.cache/dowiz-deploy/sha256-floor` and the version in `versions.log`). No flip mark.
2. CHECK, on qa-durres and on one real venue, through the owner console's operations report
   (`workers/api/src/services/operations/mod.rs` reads `chain_check`): `chained` must stay 0 and
   `legacy` must equal `records` after placing one order and one stock receipt. A `chained > 0` on an
   unflipped venue means a site writes sha256 unarmed: stop, roll back (below), find it with
   `bash tools/deploy/scheme-armed.sh` and the nightly `chain_check` line (`workers/api/src/cloud/night.rs`).
   Also: `quarantined` is 0 everywhere, and the old images' bytes are what they were (the test
   `a_flip_moves_no_byte_of_the_old_records` is the local form of that; live, the tip hex is unchanged
   on a venue with no new order).
3. ROLLBACK: `bash tools/deploy/rollback.sh <prod-version-id>`. Safe: nothing written needs the new
   reader (every record is still legacy), the floor does not apply (no flip mark), and the KV images are
   compacted v2 first as before (W-ATOMIC). Records written by the expand release read on `ac5ff0c4`
   because they are the records `ac5ff0c4` writes (`switch_off_appends_exactly_what_the_old_writer_writes`).

### Step B -- flip qa-durres

1. Edit `workers/api/wrangler.toml`: `SHA256_VENUES = "qa-durres"` (the venue ID, not the slug).
   Commit on main, `bash tools/deploy/deploy.sh`. It prints `sha256 FLIP: SHA256_VENUES="qa-durres"`
   and writes `sha256-flipped` (never removed).
2. CHECK on qa-durres: place an order, receive stock; the operations report shows `chained` = the new
   records, `legacy` = the old ones, `broken` = 0, `derived` = 0 (the Worker sets the entropy seed);
   the storefront, kitchen and console show the order; a tablet syncs a movement (`merged` = 1, it lands on the hub's tip); `/api/owner/health` lists no quarantine. On a
   NOT-flipped venue nothing changed: `chained` still 0.
3. ROLLBACK after the flip: `bash tools/deploy/rollback.sh <step-A-version-id>` -- the floor. qa-durres
   KEEPS writing sha256 (one-way, `switch_off_is_ignored_once_the_log_is_sha256`), its records are read
   (proven: `a_flip_moves_no_byte_of_the_old_records`), nothing is quarantined. Honestly: the var says
   `""` again but qa-durres is sha256 for good; that is the design, not a leak.
   `bash tools/deploy/rollback.sh <prod-version-id>` is REFUSED: `rollback: FAIL floor: venues write
   sha256 records (qa-durres flipped at ...) and <id> is not recorded at or after the floor ...` exit 53,
   nothing touched. If it must be done anyway (`ROLLBACK_PAST_FLOOR=1`): every qa-durres order since
   the flip is quarantined on the old build, orders it then writes are `broken` on the next forward
   deploy, and its first grow strips the flag -- §1 measured all three. The repair after such a
   rollback is a rotation on the next forward build (the chain restarts behind a checkpoint); the
   quarantined records are gone from the fold.

### Step C -- flip everybody

Same as B with `SHA256_VENUES = "*"` (the platform's own logs, `errlog`, flip under `"*"` too).
Check: `chained > 0` on every venue after its first write; `broken` = 0 everywhere, the first night's rotation included; a stock tablet
glue (`workers/api/src/hubdo/glue_turn.rs`) still merges -- a tablet's bare records are chained under
the hub's scheme (`adopt_from` -> `write_new`), and a tablet on a flipped log writes sha256 itself. Rollback as B.3: the floor or later.

### Step D -- nothing to contract

There is no "remove the old writer" step: `legacy_id_chained` stays to CHECK old records forever, and
`set_sha256(false)` stays as the switch for the next venue created. Nothing in an old image is rewritten
(`Hub::grow` copies the chain verbatim; `LogImage::keep` re-seals under the log's own scheme,
`keep_reseals_a_legacy_log_as_legacy`).

## 4. Kill tests and their RED proofs

`cd crates/dowiz-hub && cargo test --lib chain::mig_tests`:

- `switch_off_appends_exactly_what_the_old_writer_writes` -- flag off = byte-identical old records.
  RED when `Entropy::stamp`'s legacy arm returns flags 1 instead of 0 (the walk still verifies, the
  bytes differ): `assertion failed: one appended record = the old writer's fourth record, byte for byte`.
- `switch_off_is_ignored_once_the_log_is_sha256` -- the one-way rule. RED when `writes_sha256` drops the
  `|| newest is flagged` half: `flipped: the switch is ignored` fails with flags 0.
- `a_flip_moves_no_byte_of_the_old_records` -- mixed chain, old part byte-identical, fold reads all.
- `a_legacy_record_after_a_flip_is_read_and_named_broken` -- the shape a past-the-floor rollback leaves,
  read and named, never silent.
- `logimage_and_stocklog_honour_the_switch_one_way`, `keep_reseals_a_legacy_log_as_legacy`.

`cd workers/api && cargo test --lib scheme`: `flipped` (nobody / one / list / everybody / exact names)
and `arm` reaching the log. `bash tools/deploy/rollback.test.sh`: a target before the floor is refused
(53) before compact or switch; the floor itself and later versions switch; `ROLLBACK_PAST_FLOOR=1`
switches and names the consequence; a flip with no floor record refuses; `--unpin` is not a rollback.
`bash tools/deploy/test.sh`: a commit without the var records a version and no floor; the expand release
writes the floor and no flip; a flip writes the mark; an un-flip leaves it; a dry run records nothing.

`cd crates/dowiz-hub && cargo test --test atk4_expand --test atk4_rotate --test atk4_nonce --test atk4_s2` (W-MG-ATK4's attacks, closed by
W-MG-TRANS, each RED on the tree it was found on): an erasure and a compacted journal stay legacy and keep their
seed, a tablet writes its log's scheme and a crafted nonced record goes in bare, the first rotation after a flip
breaks nothing and leaves the log one-way, a longer retry adopts each movement once, a derived nonce is counted
across a rebase. `bash tools/deploy/atk4-rollback.test.sh`, `atk4-deploy.test.sh`, `atk4-seams.test.sh`: the floor
with its state damaged, what `deploy.sh` records of a flip, the seams no crate test reaches.
`bash tools/deploy/scheme-armed.sh --behaviour` runs the whole Worker suite with every log on legacy and fails on
any image that newly holds a sha256 record outside a named fixture: it is what found a venue's first order
(`hubdo/orders.rs`) unarmed, after the file grep had printed `0 unarmed`. Run it before step A.

## 5. What cannot be proven locally (for main, when deploys reopen)

- That every live write path is armed: `--behaviour` covers what a test reaches, and a Mem host is not
  Cloudflare; the live proof is `chained == 0` on
  an unflipped venue after a day of trading on the expand release (A.2).
- That `wrangler rollback` to the floor version restores `SHA256_VENUES = ""` with it (a version carries
  its vars) -- expected, not exercised here.
- What `own_name()` answers in a live alarm turn: workerd aborts on this box (`MmapAligned() failed`), so
  this is unmeasured, and an object with no name writes legacy under a list (`scheme::chosen`), which is the safe side.
- The stock tablet (web) under a flipped hub: `glue_turn` is covered by the crate's adopt tests, the
  browser flow by `tools/gates/flows.sh` on deployed qa-durres (operator 10-12 rule).
- The floor's `versions.log` starts with the expand release: prod's current version is not in it, which
  is exactly what makes it refused after a flip -- but any OTHER pre-floor version is refused for the
  same reason, by design.
