<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-store

The facade of the bebop store library. It re-exports `bebop-image`, `bebop-kv` (as `kv`),
`bebop-log` (as `evlog`) and `bebop-io` (as `iodev`, plus `Db`, `ReadTxn`, `WriteTxn`, `Scan`,
`Refusal`, `Options`, `Verify`, `Durability`, `Batch`, `Bytes`, ... at the top level). It re-exports
and never wraps: a wrapper is code the wasm reader would have to carry.

It also holds the DAG runtime (`proj`, `zset`, `nodekey`, `merkle`), and the tests that need more
than one of the four crates.

```rust
use bebop_store::{Db, Durability, Options};
let db = Db::create_path("shop.bebop", Options::default())?;
```

| crate | what | no_std |
|---|---|---|
| `bebop-image` | the format: cells, superblocks, PartTab, objects, crc | yes |
| `bebop-kv` | the KV layout (v1/v2/v3) and its zero-copy reader | yes |
| `bebop-log` | the chained event log | yes |
| `bebop-io` | `Db`, transactions, devices, recovery, crash harness | no (std, wasm32 ok) |
| `bebop-ffi` | the C ABI (cdylib) and, through it, Python | no |

## Five readers, one answer

The same image is read by five independent readers: bebop.bin (`selfhost/std/kv.bp`), this crate
natively, this crate as wasm32, `crates/bebop-wasm/oracle.py`, and the C ABI through Python. Each
must give the same `(n, root)`, or the same refusal. The first four are gated by
`crates/bebop-wasm/gate.sh`, the fifth by `tools/gates/ffi-readers.sh`.

## Licence

MIT OR Apache-2.0.
