# MIGRATION: sha256 record ids (M0) -- the one format change, and how it rolls back

Status: 2026-10-31, lane W-MG-MIG, built and proved LOCALLY (operator 10-31: no deploy of any kind
from a lane). Prod runs `ac5ff0c4` (FNV ids, flags 0). `main` since `58154dd9` writes sha256 ids with a
16-byte nonce and `FLAG_SHA256` (dowiz-hub M0). This is the runbook for the single deploy that carries
M0 to the venues, every step with its check and its rollback. Every command named here exists in the
tree; every number is quoted from a run.

## 0. The rule in one paragraph

Expand, then contract. The first build that READS both schemes WRITES the old one: `SHA256_VENUES = ""`
in `workers/api/wrangler.toml`. A venue is flipped by naming it there and deploying (`"qa-durres"`
first, then `"*"`). A flip is ONE-WAY per venue and lives in the bytes, not in the config: the hub crate
keeps writing sha256 once a log's newest record is sha256, whatever the var says afterwards
(`crates/dowiz-hub/src/chain/scheme.rs`, `Entropy::writes_sha256`). Before any flip, rolling back to
prod is safe because nothing written needs the new reader. After a flip, roll back only to a build that
reads sha256 -- `tools/deploy/rollback.sh` refuses the others (exit 53). No new format bytes: the flag
bit and the nonce already exist in main; this change only decides WHEN they are first written.

## 1. What `ac5ff0c4` does with a sha256 record (measured, not inferred)

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
| The per-venue config | `workers/api/src/scheme.rs` (`SHA256_VENUES`: `""` nobody, a list of venue IDS, `"*"`), installed in `workers/api/src/lib.rs` `main` and `workers/api/src/hubdo.rs` `new`; every log the Worker may append to is armed (`scheme::arm` / `arm_scheme`) | the expand release writes legacy everywhere; the crate's own default stays sha256 so no test in the crate moved |
| The tripwire | `tools/deploy/scheme-armed.sh` | every Worker file that constructs a `Hub`/`LogImage`/`StockLog` arms it or is named read-only with a reason; `scheme-armed: 12 files armed, 10 read-only by name, 0 unarmed` |
| The floor | `tools/deploy/deploy.sh` step 5 records every uploaded version in `versions.log`, the first build whose `wrangler.toml` declares `SHA256_VENUES` as the floor (`sha256-floor`), a non-empty value as a flip (`sha256-flipped`, never removed); `tools/deploy/rollback.sh` step 0 refuses a target not recorded at or after the floor once a flip exists (exit 53), `ROLLBACK_PAST_FLOOR=1` overrides and says what will happen | no rollback can reach a build that quarantines flipped venues' orders by accident |
| Kill tests | `crates/dowiz-hub/src/chain/mig_tests.rs` (6), `workers/api/src/scheme.rs` tests (2), `tools/deploy/rollback.test.sh` (+8), `tools/deploy/test.sh` (+5) | §4 |

Not touched: `crates/dowiz-hub/src/rotate.rs` (W-MG-M3M4) still writes its checkpoint record as sha256
unconditionally -- the hand-back in the lane verdict names the three-line change (`stamp` instead of
`with_nonce`/`chain_id`/`flags_for`). Until it lands, a ROTATION on an unflipped venue writes one sha256
record into the fresh image and flips that venue. Rotation is the nightly archive step
(`workers/api/src/hubstore.rs` `rotate`), so the expand release must not run a rotation before that
hand-back is merged, or must be deployed with rotation off.

## 3. The runbook: order, check, rollback

Preconditions, all local: `cd crates/dowiz-hub && cargo test --lib` green; `cd workers/api && cargo test --lib`
green; `bash tools/deploy/scheme-armed.sh` prints `0 unarmed`; `bash tools/deploy/rollback.test.sh` and
`bash tools/deploy/test.sh` green; the rotate.rs hand-back merged (§2). `SHA256_VENUES = ""` in
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
   the storefront, kitchen and console show the order; `/api/owner/health` lists no quarantine. On a
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
Check: `chained > 0` on every venue after its first write; `broken` = 0 everywhere; a stock tablet
glue (`workers/api/src/hubdo/glue_turn.rs`) still merges -- a tablet's bare records are chained under
the hub's scheme (`adopt_from` -> `write_new`). Rollback as B.3: the floor or later.

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

## 5. What cannot be proven locally (for main, when deploys reopen)

- That every live write path is armed: the tripwire is file-level; the live proof is `chained == 0` on
  an unflipped venue after a day of trading on the expand release (A.2).
- That `wrangler rollback` to the floor version restores `SHA256_VENUES = ""` with it (a version carries
  its vars) -- expected, not exercised here.
- The stock tablet (web) under a flipped hub: `glue_turn` is covered by the crate's adopt tests, the
  browser flow by `tools/gates/flows.sh` on deployed qa-durres (operator 10-12 rule).
- The floor's `versions.log` starts with the expand release: prod's current version is not in it, which
  is exactly what makes it refused after a flip -- but any OTHER pre-floor version is refused for the
  same reason, by design.
