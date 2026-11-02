<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# einfile-crc

zlib CRC-32 over bytes and over 8-byte cells: slicing-by-8 tables everywhere, and the CPU's own
instructions where it has them (AArch64 `crc32x`, x86_64 PCLMUL folding). `no_std`, no dependencies.
The `std` feature adds run-time CPU detection; without it only a feature enabled at compile time
takes the hardware path.

`unsafe` is confined to this crate, so `einfile-image` can stay `#![forbid(unsafe_code)]`.

See [`crates/einstore/README.md`](../einstore/README.md) for the library this belongs to.

## Licence

MIT OR Apache-2.0.
