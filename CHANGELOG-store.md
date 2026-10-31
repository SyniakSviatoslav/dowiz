<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Changelog: the bebop store library

The crates `bebop-crc`, `bebop-image`, `bebop-kv`, `bebop-log`, `bebop-io`, `bebop-store`,
`bebop-ffi` and `bebop-wasm-reader`, and the verifiers in `tools/verify`. What the file promises and
what it does not: [`crates/bebop-store/README.md`](crates/bebop-store/README.md). The bytes:
[`docs/FORMAT-BEBOP-IMAGE-v2.md`](docs/FORMAT-BEBOP-IMAGE-v2.md).

## Versioning

Nothing has been released. These rules apply from the first release.

- **The crates move together.** They share one version number, and a release lists them all.
- **Crate versions and format versions are separate numbers.** The container version is cell 1 of
  the superblock (2 today). Each layout has its own root-version: KV 1 to 4, EvLog 1 and 2, DOC 5.
- **Old bytes stay readable.** A release does not stop reading an image an earlier release could
  write. Dropping a layout needs a major version and its own entry here.
- **A new layout gets a new root-version.** A reader that does not know a version refuses it by name
  (status 12, `UNKNOWN_VERSION`). It never guesses, so an old reader cannot misread a new file.
- **Status codes keep their numbers** ([`docs/STATUS-CODES.md`](docs/STATUS-CODES.md)). A code is
  added, never reused.
- **Before 1.0:** a minor version may change the Rust API or add a layout, and a patch version does
  neither. A layout with no production image may still change without a new root-version; this
  happened to DOC and SEG in October 2026 and the entry below says so. From 1.0 that stops.
- **An entry with a format change lists it first,** because a format change must update every
  reader in the same change.

The names `bebop-store`, `bebop-image`, `bebop-kv`, `bebop-log`, `bebop-io`, `bebop-ffi`, `bebop-crc`
and `bebop-wasm-reader` were free on crates.io on 2026-10-31. The name `bebop` belongs to an
unrelated project, a schema-based serialization format.

## Unreleased (0.1.0)

### Format

- **A signed file (root-version 5, DOC).** A DOC root names a log, a tree, a table of device heads
  and a table of keys. Each device commit adds one record and one seal over the device's chain tip,
  and readers older than this refuse the root by name. Old images are byte-identical.
- **Time anchors (ANCH).** Evidence from outside the file, kept as an opaque proof: a time-stamp
  authority's signature, an OpenTimestamps proof, or a Roughtime response.
- **Log segments (SEG), TRUNCATE and FORGOTTEN.** Records leave a signed file only under a segment
  declared by a signed TRUNCATE; a file whose segment nobody declared is refused. An erased record
  keeps its id and place, and a signed FORGOTTEN declares it.
- **Self-describing archives (SPEC, BLOB).** An archive file can carry its reader and the format text
  as its first objects, so that a host with no parser can find them.
- **Record ids.** A flag marks a record whose id is `sha256(prev ‖ payload)`, with a 16-byte nonce at
  the start of its payload. Older records keep their FNV ids and are counted as `legacy`.
- **KV v4,** a copy-on-write B+ tree, read by all five readers. Writing it is off by default.
- **The commit anchor.** Superblock cell 9 anchors the last commit. Images from before it hold 0 and
  still read. Every reader of bytes now applies one rule to a torn last commit: it refuses.
- **KV v3** (2026-10-06), a v2 base plus a delta chain.
- **KV and EvLog v2** (2026-09-28), bytes packed eight to a cell.
- **Changed before any production image held them:** DOC cell 9 became `ref SEG`, and a SEG row gained
  `scheme`. Both moved the layout's digest without a new root-version.

### Added

- `Db`, `ReadTxn`, `WriteTxn`, `Scan`, `Options`, `Verify`, `OnTorn`, and `Refusal`, a named value for
  every refusal. `Durability::Normal` batches commits to a checkpoint.
- `bebop_store::evlog::doc`: the DOC writer, `verify` and its counters, `check_anchors`, `Doc::cut_into`,
  `Doc::redact_into`, `Doc::archive_bytes`. The traits `Signer`, `Verifier`, `Ids` and `AnchorCheck`
  are how a caller brings signatures, hashes and anchor checks. The MIT crates hold no signature code.
- `bebop-io`'s `doc` feature: `docverify::verify_doc(bytes, ..)`, the last-commit rule and then the
  counters.
- `bebop-ffi`: a C ABI over `Db`, with refusals as integer codes plus a message.
  `bindings/python/bebop_store.py` is a ctypes module over it.
- `bebop-wasm-reader`: the reader in wasm32 under MIT OR Apache-2.0, with no imports.
- `tools/verify/bebop-verify.mjs` and `.py`: one-file verifiers with nothing to install.
- `examples/read.mjs`: reads an archive with the reader inside it.
- One runnable example per promise in `crates/bebop-store/examples`, a README for every crate, and
  licence texts in every crate.

### Changed

- `bebop-store` split into `bebop-image`, `bebop-kv`, `bebop-log` and `bebop-io`. It stays as a
  re-exporting facade, so existing paths keep working.

### Not in this version

- **Merging two copies of a file.** Not built.
- **Checking seals in the MIT reader.** It counts them and says so.
- **Level 2, settlement between organisations.** Not built.
