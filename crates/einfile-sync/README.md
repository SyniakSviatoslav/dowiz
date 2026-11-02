<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# einfile-sync

One sync session between two copies of a signed file, with no server. It computes what each copy
lacks, moves that as bytes, and glues it through `Doc::tail_into`, which lays the same bytes as
merging the two whole files.

It does no I/O and has no clock, no runtime and no external crate. Bytes come in and bytes go out.
The caller carries them, and a carrier needs only to deliver bytes:

- `Peer::hello` and `Peer::on`: the session, one complete message at a time.
- `frames` and `Assembler`: any message as small frames, over a data channel or a stream alike, and
  where to resume after a break.
- `Peer::write_tail` and `Peer::read_tail`: a `.eintail`, for a carrier that never meets both sides
  at once.

A session broken at any step changes nothing it had not finished. Starting again is the recovery,
because taking the same tail twice writes nothing.

`Grants` folds the role records. It is a pure function over a file's records, and nothing in this
crate calls it on the way in: adoption gates are not here.

## What it is not

Not consensus, and not a way to decide who is right. Gluing detects a conflict and a person resolves
it. There is no authentication here: a caller who shows a `Front` to a stranger has shown which
devices are active.

## Licence

MIT OR Apache-2.0.
