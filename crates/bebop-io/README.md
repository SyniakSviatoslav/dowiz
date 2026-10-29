<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-io

The embedded KV database over a bebop image: `Db`, snapshot `ReadTxn`s, the single `WriteTxn`,
ordered `Scan`s, and `Refusal`, which names every reason the library says no. It also holds the
devices an image is written through, and the enumerated crash harness that checks the commit
protocol. Needs `std`. Builds for wasm32. `FileDevice` is unix-only.

```rust
use bebop_io::{Db, Durability, Options};

let db = Db::create_path("shop.bebop", Options::default())?;
let mut w = db.begin_write()?;
w.put(b"order/0001", b"new")?;
w.commit(Durability::Data)?;              // power-loss safe when it returns

let r = db.begin_read();                  // a snapshot: later commits do not change it
assert_eq!(&*r.get(b"order/0001")?.unwrap(), b"new");
for e in r.prefix(b"order/")? { let (k, v) = e?; /* key order */ }
```

A complete program is in `examples/kv.rs` at the repository root:
`cd crates/bebop-ffi && cargo run --release --example kv`.

## The model

- **One writer, many readers.** `begin_write` returns `Refusal::WriterBusy` at once if a writer
  is already open; it never waits. Across processes, `FileDevice` holds `<path>.lock`
  (`Refusal::Locked { by: pid }`).
- **Snapshot reads.** A `ReadTxn` holds one generation's bytes. A value it returns is borrowed
  from them for v2/v3 images.
- **Writes touch nothing until commit.** `abort` costs nothing. `into_batch` keeps the writes to
  resume later.
- **Durability is asked for, never downgraded.** `None` survives a process crash. `Data` uses two
  barriers and survives a power cut. Asking a device for more than it can give is
  `Refusal::Durability`.
- **Group commit.** Call `WriteTxn::stage()` N times, then one `Db::flush(Durability::Data)`.
- **Recovery on open.** A torn last commit falls back to the previous generation on a file
  (`OnTorn::Fallback`). For bytes handed over whole it is refused (`Db::from_bytes`,
  `OnTorn::Refuse`).
- **Verification depth.** `Verify::Anchor` is the default on open and covers the last commit.
  `Reachable` checks every object the root reaches. `WholeArena` checks the whole arena.
  `Db::verify(depth)` can be run at any time.
- **A failed barrier poisons the writer** (`Refusal::Poisoned`) until `Db::reopen`. After an
  fsync error nothing is known about the writes.

`Detached` is a device with no storage. The caller persists `Db::snapshot_bytes()` itself, which is
how a Cloudflare Durable Object holds an image.

## Other languages

`crates/bebop-ffi` exposes this API as a C ABI. `bindings/python/bebop_store.py` wraps it with ctypes.

## Licence

MIT OR Apache-2.0.
