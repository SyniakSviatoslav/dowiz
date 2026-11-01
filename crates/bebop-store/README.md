<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-store

An embedded key-value store and signed event log that live in one file.

## What the file promises

The file reads with no server and no install, and a reader that checks it refuses damage by name
instead of reading it as data. A commit either finished or is not there, and a log record can carry
a signature (a *seal*) that makes a later edit visible. A file can also carry evidence from outside
itself of when a record existed, and an archive file can carry its own reader.

**This is pre-1.0 and not published to any registry.** The container format is frozen at version 2.
The signed layouts above it changed as recently as 2026-10-31, and the Rust API can still change.
Everything below describes what the code in this repository does today, and each promise has an
example that runs it.

## What it does not promise

These limits are part of the contract.

- **A proof is not truth.** From `bebop-log/src/doc/verify.rs`: a seal counted `ok` "means the holder
  of key K signed this tip at seq s of its chain and nothing under the tip has changed since. It
  does NOT say the event happened, that a quantity is true, or that `claimed_ms` is right."
- **Seals are not checked by the MIT reader.** It carries no signature algorithm, and it says so:
  "seals COUNTED, not CHECKED: this reader carries no signature algorithm (MIT, no ML-DSA); a count
  is not a proof -- bring a verifier to the same seam, or the format text's independent readers".
  See [Signing a file](#signing-a-file).
- **A level 1 anchor needs a Bitcoin confirmation.** An OpenTimestamps proof says nothing until its
  transaction is in a block and the auditor compares that block's header. Until then it is
  `Unchecked`, and `Unchecked` is never a pass.
- **The clock in a seal is a claim.** `claimed_ms` is the device's own clock, signed and repeated
  and never believed.
- **Two devices merge as M2-min, and no wider.** `Doc::merge_into` joins two copies of one genesis
  with no server, keeping every record and seal, and names each conflict without resolving it. A
  cut, an erasure or a revocation on one side only is refused by name; so are one device forked
  across the copies and two seal chains of one key. Under a cut a conflict list cannot be re-derived:
  the answer is `NOT CHECKED`, never clean. See format §14.
- **Opening a file checks only what you ask for.** `Options::default()` opens at `Verify::Anchor`,
  which proves the last commit. Older commits are trusted, and a damaged old object can read as a
  refusal or as a wrong answer, until you open at `Verify::Reachable` or call `Db::verify`.
- **Nothing is encrypted.** The file holds your data as plain bytes.

## Quickstart

```rust
use bebop_store::{Db, Durability, Options, Refusal, Verify};

fn main() -> Result<(), Refusal> {
    let path = std::env::temp_dir().join(format!("quickstart-{}.bebop", std::process::id()));
    let path = path.to_str().expect("a UTF-8 temp path");
    {
        let db = Db::create_path(path, Options::default())?;
        let mut w = db.begin_write()?;
        w.put(b"order/1", b"new")?;
        w.put(b"order/2", b"paid")?;
        w.commit(Durability::Data)?; // back when it is on the device
    } // dropping the Db releases the file and its lock
    {
        let db = Db::open_path(path, Options::default().verify(Verify::Reachable))?;
        let r = db.begin_read();
        for entry in r.prefix(b"order/")? {
            let (k, v) = entry?;
            println!("{} = {}", String::from_utf8_lossy(&k), String::from_utf8_lossy(&v));
        }
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{path}.lock"));
    Ok(())
}
```

This block is compiled and run by `cargo test --doc`. A refusal is a value with a name, never a
panic: see `Refusal`, and the numbers every reader shares in
[`docs/STATUS-CODES.md`](../../docs/STATUS-CODES.md).

The crates are not on crates.io, so depend on a checkout by path:

```toml
[dependencies]
bebop-store = { path = "../dowiz/crates/bebop-store" }
```

## Guarantee levels

Each level is optional and works on the same file. No blockchain is ever required, there is no
token, and the core holds one event, "this root was recorded here", with adapters outside it.

| Level | What it adds | Status today |
|---|---|---|
| **0, the file** | Signed per-device chains, verified from the file and public keys alone. A file that reads with no install. | Built. Merging two copies is planned for this level and **not built**. |
| **1, the anchor** | Evidence from outside the file that a record existed by a moment, kept in the file: OpenTimestamps (Bitcoin), a time-stamp authority's signature, or Roughtime as a lower bound. | The format and the checking seam are built here. The adapters are in `dowiz-hub`, which is AGPL (see below). |
| **2, the settlement** | A chain or witnesses fixing the order between organisations that do not trust each other, at the moments they connect. | Not built. |

The examples show the mechanics of each row they cover, and each one fails loudly if its promise
does not hold.

| Promise | Example |
|---|---|
| A changed byte is refused by name | [`refuse_damage`](examples/refuse_damage.rs) |
| An unfinished commit is not there | [`crash_fallback`](examples/crash_fallback.rs) |
| An edit under a seal is seen (level 0) | [`sealed_history`](examples/sealed_history.rs) |
| Keys are records; a revoke invalidates later seals (level 0) | [`revoke_key`](examples/revoke_key.rs) |
| An anchor nobody checked counts as nothing (level 1) | [`time_anchor`](examples/time_anchor.rs) |
| History leaves only under a signed declaration | [`cut_history`](examples/cut_history.rs) |
| An erasure is declared, never hidden | [`forget_a_person`](examples/forget_a_person.rs) |
| The file reads with nothing installed | [`read_without_library`](examples/read_without_library.rs) |
| An archive carries its own reader | [`archive`](examples/archive.rs) |

Run one with `cargo run --example refuse_damage`. The sealed examples use a stand-in named
`AnyoneCanForge`, which is **not a signature**: anyone can make one. They prove the layout and the
counters, and say so.

## Verify a file with no install

Each of these is one file. Copy it anywhere; it reads only the path you give it.

```sh
node tools/verify/bebop-verify.mjs your.bebop --dump --verify
python3 tools/verify/bebop-verify.py your.bebop --dump --verify
```

Both print one status line, and with `--verify` the counters of a signed file. `status=0` means
every object the root reaches matches its checksum, the last commit matches its anchor, and the
layout rules hold. It does not mean the content is true. In the counters, `checked=0` means the
seals were counted and not checked.

An archive file also carries its own reader: `node examples/read.mjs archive.bebop`. That reader is
testimony, not authority, because a file cannot vouch for itself. Compare its sha256 with a
published one, or read the file with one of the verifiers above.

## Signing a file

**To sign, you bring ML-DSA-65.** There is no signature code in the MIT crates. `bebop_store::evlog::doc`
defines the traits `Signer`, `Verifier` and `Ids`, and you implement them. `dowiz-core` has an
ML-DSA-65 implementation, and `dowiz-hub` has the implementations of those traits and the anchor
adapters, but both are **AGPL-3.0-or-later**, and linking them brings the AGPL's terms to your
program. `dowiz-hub` also has `doc_verify`, an AGPL program you can run beside your own code. It
checks ML-DSA-65 seals and its three anchor kinds, and it reads key operations and declarations in
the record framing `dowiz-hub` writes.

| You want to | You need | Licence of what you take |
|---|---|---|
| Read and write KV files | this crate | MIT OR Apache-2.0 |
| Read a signed file, count its seals | `bebop-wasm-reader`, or `tools/verify` | MIT OR Apache-2.0 |
| Write a signed file | this crate, plus your own ML-DSA-65 behind `Signer` | MIT OR Apache-2.0, plus yours |
| Check seals | your own ML-DSA-65 behind `Verifier` | MIT OR Apache-2.0, plus yours |
| Check anchors | your own adapter behind `AnchorCheck`, or `dowiz-hub`'s | yours, or AGPL |

**Interoperability warning.** Seals are made with ML-DSA `Sign_internal` and checked with
`Verify_internal` (FIPS 204 algorithms 7 and 8) directly over the seal message, with no context
string or domain prefix. A library that exposes only the external `ML-DSA.Sign` signs a different
message, and its signatures will not verify against these seals. The message and the actor id are
in [FORMAT §9.4](../../docs/FORMAT-BEBOP-IMAGE-v2.md).

## The crates

| crate | what | no_std |
|---|---|---|
| `bebop-crc` | zlib CRC-32 | yes |
| `bebop-image` | the format: cells, superblocks, PartTab, objects, crc | yes |
| `bebop-kv` | the KV layout (v1 to v4) and its zero-copy reader | yes |
| `bebop-log` | the chained event log, and the signed DOC root over it | yes |
| `bebop-io` | `Db`, transactions, devices, recovery, crash harness | no (std, wasm32 ok) |
| `bebop-store` | this facade, plus `merkle`, `nodekey` (SHA-256), `proj`, `zset` | no |
| `bebop-ffi` | the C ABI (cdylib) and, through it, Python | no |
| `bebop-wasm-reader` | the reader compiled to wasm32, no imports | no |

`bebop-store` re-exports `bebop-image`, `bebop-kv` (as `kv`), `bebop-log` (as `evlog`) and `bebop-io`
(as `iodev`, plus `Db`, `ReadTxn`, `WriteTxn`, `Scan`, `Refusal`, `Options`, `Verify`, `Durability`,
`Batch` and `Bytes` at the top level). It re-exports and never wraps: a wrapper is code the wasm
reader would have to carry. Python goes through the C ABI: `bindings/python/bebop_store.py`.

The DAG runtime in this crate (`proj`, `zset`) belongs to the language runtime that shares this
format. It is not part of the public library.

## Five readers, one answer

The same image is read by five independent readers: `bebop.bin`
(`bebop-lang/selfhost/std/kv.bp`), this crate natively, this crate as wasm32,
`crates/bebop-wasm/oracle.py`, and the C ABI through Python. Each must give the same `(n, root)` or
the same refusal, and `tools/gates/conformance.sh` holds them to it.

## The format

[`docs/FORMAT-BEBOP-IMAGE-v2.md`](../../docs/FORMAT-BEBOP-IMAGE-v2.md) is normative. **It is written
in Ukrainian**, and no English text of it exists yet. §12 of that document lists what a third-party
implementer would still have to take from the code.

Changes are in [`CHANGELOG-store.md`](../../CHANGELOG-store.md), which also states the versioning
rule.

## Licence

MIT OR Apache-2.0, at your option. The repository around this crate is AGPL-3.0-or-later; the store
crates listed above are not, and none of them links an AGPL crate outside its tests.
