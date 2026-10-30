# Status codes of the bebop image readers: one vocabulary

Status: **normative** from 2026-10-29 (lane W-ES-P4, R-EMBED-STORE §4.4 AC-ES-30/32, §5 row P4).
Every reader of a KV image answers with one number from this table. A reader whose native refusal
is not a number says here how it maps to one. The gate that holds them to it is
`tools/gates/conformance.sh`: 11 fixtures × 5 readers, plus `--prove`.

## 1. The readers

| Reader | Code | How it answers |
|---|---|---|
| native | `crates/bebop-wasm/fixtures/conform` (`bebop-conform read`) over `bebop-image` + `bebop-kv` | `kv status=<s> n=<n> root=<fold>`, mapping in §3 |
| wasm32 | `crates/bebop-wasm/src/abi.rs` `bw_kv`, run by `harness.mjs` | the `i32` it returns |
| python | `crates/bebop-wasm/oracle.py` | `kv status=<s> ...`, and the same number as its exit code |
| bebop.bin | `bebop-lang/selfhost/std/kv.bp` (`kv.bin n` / `kv.bin h`) | a value, or an exit code (trap), mapped in §3 |
| C ABI | `crates/bebop-ffi` `code_of`, via `bindings/python/bebop_store.py` | the `i32` every `bebop_*` call returns |

## 2. The table

Codes 1-12 are the reader codes and keep their `abi.rs` numbers on every surface. Codes 40-60 come
from the C ABI's database layer (open, recovery, writes). Nothing else may be given to a code below.

| Code | Name | Meaning | Who can give it |
|---|---|---|---|
| 0 | `OK` | read; `n` = entries, `root` = the FNV-1a 64 fold (FORMAT §9.1) | all |
| 1 | `NO_SUPERBLOCK` | no valid superblock, or the live one was refused structurally: `arena_used` outside, PartTab outside, **misplaced** (`ptaway`), **PartTab crc bad** (`ptbad`), root outside (FORMAT §3.1). This is never a fallback | all |
| 2 | `NOT_A_KV` | a superblock but no KV root (`empty`), or a slice/chain claim that does not hold | all |
| 3 | `NOT_A_LOG` | log reader only (`bw_log`) | wasm32 |
| 4 | `TRUNCATED` | log reader: a record cut short | wasm32 |
| 5 | `NULL_ARG` | a null pointer at the ABI. oracle.py also uses 5 for an S-1 break (superblock cells 13-14 non-zero), which no fixture has. **Two meanings, kept apart only by which surface gave them** | wasm32, C ABI, python |
| 6-9, 11 | — | the `nodekey` (6) and `proj` (7-9, 11) readers. Not KV codes | `bw_key`, `bw_proj`, `oracle.py --proj` |
| 10 | `BAD_CRC` | an object the read reached fails its crc (root, an array, a delta record) | native, wasm32, python, C ABI |
| 12 | `UNKNOWN_VERSION` | the KV root names a version above 3 (`kv4stub`) | all |
| 40 | `RECOVER` | the newest generation failed recovery and so did the fallback | C ABI |
| 41 | `LAST_COMMIT` | the last commit failed its proof and falling back was not allowed | C ABI |
| 42 | `OBJECT_BAD` | an object met while walking the whole arena is cut short or fails its crc | C ABI |
| 43 | `CORRUPT` | a cell the write path was about to trust disagrees with the image | C ABI |
| 44 | `KEY_TOO_LONG` | key over `MAX_KEY_BYTES` | C ABI |
| 45 | `VALUE_TOO_LONG` | value over `MAX_VALUE_BYTES` | C ABI |
| 46 | `ARENA_FULL` | no room left in the arena | C ABI |
| 47 | `LIMIT` | another format limit (FORMAT §10) | C ABI |
| 48 | `TOO_SMALL` | an image below `MIN_BYTES` | C ABI |
| 49 | `WRITER_BUSY` | a write transaction is already open | C ABI |
| 50 | `LOCKED` | another process holds the store's lock | C ABI |
| 51 | `POISONED` | an earlier write failed; reopen | C ABI |
| 52 | `EXISTS` | create over a device that already holds a KV image | C ABI |
| 53 | `IO` | the device said no | C ABI |
| 54 | `DURABILITY` | more durability asked than the device gives | C ABI |
| 55 | `PLAN` | the staged image cannot be flushed over the durable one | C ABI |
| 56 | `BAD_ARG` | an argument out of range | C ABI |
| 57 | `BUSY` | a handle still has open children | C ABI |
| 58 | `UNSUPPORTED` | this build cannot do that | C ABI |
| 59 | `PANIC` | a panic caught at the boundary | C ABI |
| 60 | `OTHER` | a `Refusal` variant newer than the ABI | C ABI |

## 3. Mappings that are not already this number

**native** (`bebop-conform read`): `pick_checked_in` refuses → 1; `Store::root()` is `None` → 2;
`Kv::load_checked`: `KvError::NotKv` → 2, `BadCrc` → 10, `UnknownVersion` → 12.

**C ABI** (`crates/bebop-ffi/src/lib.rs` `code_of`): `Refusal::Pick` → 1, `NotKv` → 2, `BadCrc` →
10, `UnknownVersion` → 12, and the rest 40-60 as in §2.

**bebop.bin** (`kv.bp`; `bebop-lang/docs/TRAPS.md`):

| bebop answer | Status |
|---|---|
| exit 0, a value | 0 (`n` from `kv.bin n`, `root` from `kv.bin h`) |
| exit 0, `-3` (a v3 root whose chain or base-order claims do not hold) | 2 |
| trap 79 (PartTab off its home, `st_open`) | 1 |
| trap 77 (live PartTab fails its crc; store.bp `st_open` (W-PRELUDE; was kv.bp `kv_precheck`, W-ES-P4)) | 1 |
| trap 76 (no PartTab, so no KV root; `kv_precheck`, W-ES-P4) | 2 |
| trap 78 (KV version above 3, `pj_kv_known`) | 12 |

bebop has **no `BAD_CRC`**. `kv.bp` does not check object crcs on read. `st_open` proves only the
last commit's range, through the anchor. A flipped bit there makes `st_open` fall back to the
previous generation (see the `--prove` row below), and a flipped bit in an older object is read
as data. Both are recorded here, not hidden. The answer still moves, so `--prove` measures it.

## 4. The measured matrix (2026-10-29, `conformance.sh --prove`, aarch64, every cell read)

| Fixture | Want | native | wasm32 | python | bebop.bin | C ABI |
|---|---|---|---|---|---|---|
| kv (v1) | 0 n=5 −211109995167561145 | = | = | = | = | = |
| kv2 (v2) | 0 n=5 −211109995167561145 | = | = | = | = | = |
| kv3 (v3, D=3) | 0 n=5 −4092201869538170098 | = | = | = | = | = |
| ptaway | 1 | 1 | 1 | 1 | 1 (trap 79) | 1 |
| kv4stub | 12 | 12 | 12 | 12 | 12 (trap 78) | 12 |
| kv3max (v3, D=32) | 0 n=27 −6264351955216461874 | = | = | = | = | = |
| kv3c (compacted) | 0 n=27 −6264351955216461874 | = | = | = | = | = |
| torn (SB torn) | 0 n=26 −5357977772297964327 | = | = | = | = | = |
| ptbad | 1 | 1 | 1 | 1 | 1 (trap 77) | 1 |
| empty | 2 | 2 | 2 | 2 | 2 (trap 76) | 2 |
| kv0 (empty KV) | 0 n=0 −3750763034362895579 | = | = | = | = | = |

`--prove`: bit 0 of cell 1677 of kv3max (the first value cell of the newest delta record) was
flipped. native, wasm32, python and C ABI → 10. bebop.bin → 0 n=26 −5357977772297964327, which is
generation 33. The anchor failed, so `st_open` fell back. All five answers moved.

## 5. Not covered

AC-ES-31 (the two writers byte-identical) is out of reach. `kv.bp` writes v2 (`i`) and never a
delta chain, so it cannot write kv3max. The conformance here is one-way: bebop reads what Rust
writes. Statuses 3-9, 11 and 40-60 come from no KV fixture. A test that enumerates `Refusal`
against this table (T-ES-32's second half) is for the owners of `bebop-ffi`/`abi.rs`. This lane
does not edit those crates.
