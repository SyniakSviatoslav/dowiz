<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# einfile-image

The einfile format: a pointer-free file of 8-byte little-endian cells. Two superblocks, a
partition table (PartTab), and an append-only arena of self-describing objects, each sealed with a
crc32. Nothing in an image is an address. Every reference is an object-relative cell offset, so any
language can read and write the same file. The bebop language, Rust, wasm32 and Python all do.

`no_std` + `alloc`. The only dependency is `einfile-crc`. With the `std` feature you get file I/O
(`Store::create`, `Store::open`).

## What is in it

- `Store` (owned cells) and `View` (borrowed bytes), both behind the `Cells` trait.
- `pick_in` / `format::pick_checked_in` choose the live superblock: the valid one with the higher
  generation. A newer superblock that names an offset outside the image, or a PartTab that fails its
  crc, is a **refusal**, never a guess.
- Object access (`get_in`, `follow_in`, `obj_crc_ok_in`) and the commit staging every layout writes through.

The normative description is `docs/FORMAT-EINFILE-v2.md` in this repository, and the names and
limits are in `src/format.rs`.

## Where it sits

```
einfile-image  (format)  <-  einfile-kv (KV layout)  <-  einfile-io (Db, transactions, devices)  <-  einstore (facade)
             (format)  <-  einfile-log (event log)
```

Most users want `einfile-io` (or the `einstore` facade), not this crate.

## Licence

MIT OR Apache-2.0.
