# The bebop image format, container version 2 - normative description

Status: **normative**, frozen 2026-10-27, English text added 2026-10-31. This is the English twin of
`docs/FORMAT-BEBOP-IMAGE-v2.md`, section for section; the two state the same rules. It describes the
bytes that the reference writers and readers produce and accept today.

**Authority.** This text is the contract for a third party and the reference code is informative: where the two differ that is a defect, to be reported to the maintainers of the reference repository with the section, the file or its sha256, and both answers, and §12 lists the ones known today.

Key words: **MUST** - breaking it makes an image invalid or a reader non-conforming; **MUST NOT** -
a prohibition; **MAY** - allowed. Identifiers are written as the code writes them. A name in
`backticks` that ends in `.rs`, `.bp` or `.py` is an informative pointer to the reference
implementation: nothing normative depends on opening it, and neither does any repository path. So is a qualified
name (`doc::id_message`, `chain::stock_flags`, `HubIds::body`, `ReaderIds::body`, `Verifier::actor_of`) and the reference tool
`doc_verify`: the rule stands here, and `Ids::chained(r)` means the id formula of §9.4 over record `r`. Everything a
reader needs is in this text, and §13 gives the answers a conforming reader must reach, with the sha256 of each file.

Notation: `a ‖ b` is byte concatenation; `LE8(x)` is `x` as 8 bytes, little-endian, two's complement;
`ceil(x/8)` rounds up; `[a, b)` is a half-open range; `0^32` is 32 zero bytes. "Packed" bytes sit eight
to a cell, little-endian (byte `i` is byte `i % 8` of cell `i / 8`), and a writer MUST zero-fill the last cell. A reader takes
the bytes its length names and MUST NOT refuse non-zero padding behind them.
A 32-byte value held in four cells (an id, a tip, a sha256) is packed the same way.

---

## 1. Cell, byte order, zero

- An image is a sequence of **cells** of 8 bytes; a cell is an `i64`, **little-endian**. Every index,
  length and offset in an image is in cells unless it says "bytes".
- A tail shorter than 8 bytes MUST be **ignored**, not padded: a cut image must not become valid.
- A reader that reads a cell past the end of the image MUST get **0**. That is already the format's
  null (`ref` 0 = no object, length 0 = no payload). A reader MUST NOT panic on, or allocate by, a
  value from the image without first bounding it by the image.
- **Integers.** Every expression over cell values in this text (`1024 + capacity`, `pt + 20`, `r + 2`, `this_obj + off`,
  `i + 2 + len`) is exact integer arithmetic. A reader with fixed-width integers MUST NOT let one wrap: an overflow is a
  value past every bound, so the check that guards it refuses, and a `ref` that overflows reads as null. A negative
  `capacity` counts as 0. A length or seq read "as unsigned" is 64 bits wide on every platform.
- **A field past `len`.** A field is read by its cell number; `len` bounds what this text says it bounds, namely the crc, a
  payload, table rows and the lengths it states. An object shorter than its layout is one a writer MUST NOT produce, and no
  rule refuses it unless a section says so: a reader MUST read such a field from the cell that lies there (the next
  object's header, or 0 past the image's end) and MUST NOT read it as 0 inside the image. The stated bounds: a KV root under 6
  cells has no version cell (§9.1), under 8 a v3 root is refused and a v4 root must be exactly 8; a log root under 8 cells has no
  version cell (§9.2); a record shorter than its head is refused (§9.2), and a v2 record under 16 cells has no actor; a DOC is exactly 10 cells, and SEAL, KEY, SEG, ANCH and SPEC have exact
  lengths (§9.4, §9.5).

## 2. Image layout

| Cells | Bytes | What |
|---|---|---|
| 0..15 | 0..127 | superblock A |
| 16..36 | 128..295 | the PartTab superblock A names (written by a commit **from** B, the commit that writes A) |
| 512..527 | 4096..4223 | superblock B |
| 528..548 | 4224..4391 | the PartTab superblock B names (written by a commit from A) |
| 1024.. | 8192.. | arena: a bump allocator of self-describing objects, append-only |

All other cells of pages 0 and 1 are zero. Each PartTab lies on the page of **its own** superblock,
and two consecutive generations (superblock + PartTab) lie on **different 4 KiB pages**, so tearing
one page does not destroy both generations. The superblock (128 bytes) and the PartTab (168 bytes)
each fit the first 512 bytes of their page. Whether that is one atomic sector on a given filesystem
is **not verified**; the only integrity argument is the crc (§6).

## 3. Superblock (16 cells, `SB_CELLS`)

| Cell | Name | Value |
|---|---|---|
| 0 | magic | `MAGIC` = "BEBOPST1" as an LE `i64` |
| 1 | `SB_VERSION` | container version, `FORMAT_VERSION` = 2 (all writers) |
| 2 | `SB_GEN` | generation; a fresh image is 0, each commit adds 1 |
| 3 | `SB_PARTTAB` | **absolute cell index of the PartTab** (not the data root); 0 = an empty image with no root |
| 4 | `SB_USED` | `arena_used`: absolute index of the first cell past the last object; fresh = 1024 |
| 5 | `SB_PROJTAB` | projection table (§9.3), 0 = none; a commit that does not know it carries it over |
| 6 | `SB_MIGRATION` | migration table of the bebop language writer, 0 = none; carried unchanged |
| 7 | `SB_LIVE` | live cells, headers included (§8; advisory) |
| 8 | `SB_SUPERSEDED` | cells superseded since the last compaction (§8; advisory) |
| 9 | `SB_ANC` | **anchor** of the last commit (§7); 0 = no anchor |
| 10 | `SB_COMMIT` | ref to a commit object of the bebop language writer; the Rust writer writes 0 |
| 11 | `SB_HEADS` | ref to a heads table of the bebop language writer; the Rust writer writes 0 |
| 12 | `SB_CAPACITY` | arena capacity in cells, counted from 1024 |
| 13, 14 | - | a writer MUST write 0. They are not part of validity: three of the reference readers ignore them, and the Python one refuses (§3.1) |
| 15 | `SB_CRC` | zlib crc32 of cells 0..14 (§6), stored as a non-negative `i64` |

**A valid superblock**: cell 0 = `MAGIC`, and the low 32 bits of cell 15 = crc32(cells 0..14), and all
16 cells are in the image. Only the low 32 bits of cell 15 are compared.

### 3.1 Picking the live superblock

A reader of bytes handed whole **never pads**: `n` is the cells that arrived, and no answer depends on `capacity`. A ref,
a root or an object past them is outside the image, and a cell read there is 0 (§1). `capacity` is the writer's: what a
writer re-opening a trimmed image restores before it writes (§8), never a claim on what a reader reads.

A superblock B whose 16 cells are past the end is simply not valid.

1. Validity by the rule above. Both valid → the higher generation (compared as `i64`); equal → A. One
   valid → that one. None → refuse (status 1). A reader MAY check cells 13-14 here: the Python reference reader
   answers status 5 when ANY valid superblock, live or not, has either cell non-zero, before every other check of
   this section, status 1 included. No fixture has such a superblock, and the Rust readers read on.
2. The chosen superblock (at cell `at`, with `pt` = its cell 3, `used` = its cell 4) is checked
   further, in this order, **with no fallback to the other one**; each failure is status 1:
   - `used` < 0 or `used` > `n`. This is a **cut** image, not a reason to read the previous generation.
   - `pt` < 0, or `pt` ≠ 0 and `pt + 20` ≥ `n`.
   - `pt` ≠ 0 and `pt` ≠ `at + 16`, the PartTab's **home** (§2). Normatively a live PartTab lies only on
     its superblock's page: the next commit from `at` writes its own PartTab at `(512 − at) + 16`, so a
     live PartTab anywhere else (for example 528 under a live A) would be overwritten before the commit
     point, and a crash in that window would lose generation k (§8). No writer produces such an image.
   - `pt` ≠ 0: the PartTab's `len` < 19, or the object does not fit the image; or its crc (§6) fails.
   - `r = cells[pt + 18]`: `r` < 0, or `r` ≠ 0 and `r + 2` ≥ `n`.
3. Falling back to the other generation is allowed **only** between superblocks with a bad magic or a
   bad cell 15. It MUST NOT happen for any reason in step 2.

## 4. PartTab (P = 1)

An object (§5) of 19 payload cells (`PARTTAB_CELLS` = 16 + 3·P), 21 with its header
(`PARTTAB_OBJ_CELLS`). A commit from the live superblock `sb` writes it into the **tail of the other
superblock's page** (the page of the new superblock): `pt = (512 − sb) + 16` (`PT_PAGE_OFFSET` = 16),
so 528 for a commit from A and 16 for a commit from B. The new superblock `512 − sb` names it in cell 3.

| Payload | Cell relative to `pt` | Value |
|---|---|---|
| h0 | `pt + 0` | `(digest_lo32 << 32) \| 19` |
| h1 | `pt + 1` | `(crc32(payload[0..19]) << 32) \| (gen mod 2³²)` |
| 0..15 | `pt + 2 .. pt + 17` | a copy of the **previous** live superblock (the one the commit started from), cells 0..15 |
| 15 | `pt + 17` | overwritten: crc32 over payload[0..19], computed while payload[15] still held the copied cell 15 |
| 16 | `pt + 18` = `PT_ROOT` | `root_p`: absolute index of the data root, 0 = none |
| 17 | `pt + 19` = `PT_CURSOR` | `used_p`: the writer's cursor = the new superblock's `arena_used` |
| 18 | `pt + 20` = `PT_GEN` | `gen_p` = the new superblock's generation |

- A writer takes the cursor and the generation **from the PartTab**, not from the superblock; with
  `pt` = 0, from cells 4 and 2 of the superblock.
- The PartTab digest: the Rust writer **carries** the previous PartTab's digest (0 if there was none,
  so in a Rust-created image it is 0 forever); the bebop language writer writes `st_digest("PartTab")`.
  A reader MUST NOT check the PartTab digest.
- No reader checks the inner crc in payload[15]; the crc that is checked is the one in h1 (§6).

## 5. Object

```
h0 = (layout_digest & 0xFFFF_FFFF) << 32 | len        len = payload cells, ≤ 2³²−1
h1 = (crc32(payload) << 32) | (generation & 0xFFFF_FFFF)
payload[0..len]
```

- Objects lie in the arena from `ARENA` = 1024 with no gaps, in allocation order. The next commit
  starts at the cursor (`used_p`).
- `len` is a **claim**: a reader MUST bound it by the image. An object that does not fit fails its crc
  check (status 10), and MUST NOT cause an allocation. Three walks ask something else first and name their own status for
  such an object: a PartTab is **1** (§3.1), a v3 chain record is **2** (§9.1), and a log record is **4** when the running sum
  passes the image (§9.2). Anywhere else a crc is asked, 10.
- **The crc covers the payload, not the header.** h0 is protected only by the anchor over the last commit, so a
  digest or length bit of an older object is caught by the read that depends on it, or not at all.
- **`ref`**: a payload cell holding a **signed offset in cells, relative to the first header cell of the
  object that contains it**: `target = this_obj + off`; 0 = null. The target MUST have both header
  cells in the image, otherwise the ref reads as null. The addition is overflow-checked. `ref` values
  are relative in every object below, including the data root; only superblock cells and `cells[pt + 18]`
  are absolute.
- The generation in h1 is the low 32 bits only (`MAX_GENERATION`, §10): the commit that wrote the object.
- **h0 and h1 MUST be read as unsigned.** A cell is an `i64`, so an object with a digest ≥ 2³¹ has a
  negative h0. digest = `(h0 as u64) >> 32`, length = `h0 & 0xFFFF_FFFF`, and likewise the crc in h1. An
  arithmetic shift gives a wrong digest.
- **`st_digest(s)`** is the last four bytes of `sha256(s)` (bytes 28..31) as a big-endian `u32`, over
  the ASCII string `s`. §10 lists every layout string. Two digests are **not** derived this way, they
  are ASCII labels: the EvLog root `0x45564C47` and the EvLog record `0x45565F52` (§9.2).
- **Who checks the digest.** A reader of KV v1-v3 MUST NOT require the KV root's digest: the layout and version
  come from cell 5 of the root (§9.1); a v4 root's digest is rule 0 of §9.1, after the version. DOC layouts (§9.4, §9.5) are checked by digest, because a root is
  a DOC only if digest, length and both version cells all agree.

## 6. CRC

- zlib CRC-32 (reflected polynomial 0xEDB88320, init `0xFFFFFFFF`, final xor `0xFFFFFFFF`) over the
  **raw LE bytes of the cells**: `zlib.crc32(struct.pack('<%dq' % n, *cells))`.
- Superblock: crc over cells 0..14 → cell 15.
- Object: crc over the `len` payload cells → the high 32 bits of h1.
- Anchor: crc over cells `[mark, arena_used)` (§7).

## 7. The anchor `anc` (cell 9)

```
anc = (mark << 32) | crc32(cells[mark .. cursor))
```

`mark` is the cursor at the start of the commit, `cursor` the cursor after its last object (= the new
`arena_used`). The range covers **only the arena**; the PartTab has its own crc.

- Both the bebop language writer and the Rust writer write it. The container version stays 2.
- `mark` MUST be read with a **logical** shift (`(anc as u64) >> 32`).
- The `mark` field is 32 bits: a commit starting past `MAX_ANCHOR_MARK` writes `anc` = 0.
- A reader MUST accept both 0 and non-zero. The anchor is:
  `Absent` when `anc` = 0; `Holds` when `1024 ≤ mark ≤ arena_used`, `arena_used` ≤ the image's cells, and
  crc32 of the range = the low 32 bits of `anc`; otherwise `Torn`. `Torn` with the mark outside those
  bounds is called *torn, no range*.
- Picking (§3.1) does **not** check the anchor; a read does, right after the pick.
- The anchor lies inside the superblock's crc: changing cell 9 alone makes the superblock invalid.

**The last-commit rule, one for all readers.** After the pick and the PartTab crc, every reader asks
the anchor:

| Anchor | A reader of **bytes handed whole** | A reader with **recovery authority** over a device (a file it may fall back in) |
|---|---|---|
| `Holds` | the last commit is proved; a lazy open (below) MAY stop here, older commits trusted | the same |
| `Absent` | proves and refuses nothing; the reader checks what it reaches | a bounded walk of the arena |
| `Torn` | **a refusal, never a fallback**: *torn, no range* → `LAST_COMMIT` 41 at once. Otherwise walk the objects of `[mark, arena_used)` (next = `i + 2 + len`); the first whose payload fails its crc, or does not fit the image's cells (not `arena_used`) → `BAD_CRC` 10; if every one checks (a re-sealed object, a changed header) → `LAST_COMMIT` 41 | hides this superblock, walks the **whole** arena of the other one, and opens generation k−1 |

Bytes handed whole were not torn by a power cut: they are corrupt, and generation k−1 is not what the
holder stored. That is why a reader of bytes MUST NOT fall back. Fixtures: `kv4stale` (a re-sealed object
under a stale anchor → 41), `kv4old` (a corrupt object of an **older** commit → 10 at a full read).

**The documented guarantee of a lazy open.** A corrupt object of an older commit is **not** caught by an
open that stops at the anchor, by design. It is caught by a read of what the root reaches, if the root
reaches it, and always by a walk of the whole arena.

## 8. Commit protocol

**To a file**: stage the commit in memory (the PartTab into the other superblock's page, then the other
superblock with `anc`), then

1. `pwrite(cells[mark..cursor))` → `fdatasync`;
2. `pwrite(PartTab, 21 cells)` → `fdatasync`;
3. `pwrite(superblock, 16 cells)` → `fdatasync` - **the commit point**.

A crash between any two steps leaves generation k readable: nothing the old superblock names is
overwritten (the arena is append-only; the new PartTab lies on a different page from the old one).

**Without a filesystem**: staging only; atomicity belongs to whoever stores the bytes.

**The bebop language writer, mmap**: `msync([mark, cursor))` → write PartTab and superblock →
`msync(pages 0-1)`; two barriers.

**Creation**: superblock A (gen 0, `pt` 0, `used` 1024, capacity `size/8 − 1024`, crc), the rest zero;
the file is written, then the file and its directory are `fsync`ed. Below `MIN_BYTES`: refusal.

**What a writer must keep, and a reader must not rely on.**

- A commit from superblock `sb` writes the new superblock `512 − sb` with: cells 0, 1 = magic, 2; cell 2 =
  `gen + 1`; cell 3 = the new `pt`; cell 4 = the cursor; cell 5 = the projection table (carried unless
  this commit wrote one); cell 6 carried; cell 7, cell 8 as below; cell 9 = the anchor; cells 10, 11 = 0
  (Rust: carrying a ref would name the wrong generation); cell 12 carried; cells 13, 14 = 0; cell 15 =
  crc. A commit that would pass `MAX_GENERATION` MUST be refused.
- The cursor MUST lie in `[1024, 1024 + capacity]` and MUST NOT be behind cell 4: otherwise the image is
  refused before any write.
- `alloc(len)`: `0 ≤ len ≤ MAX_OBJ_CELLS`, and `cursor + 2 + len ≤ 1024 + capacity` or the arena is full.
- `SB_LIVE`' = `live + allocated − dead`; `SB_SUPERSEDED`' = `superseded + dead`, both starting at 0.
  `allocated` is `Σ(2 + len)` over the objects the commit allocated (the PartTab is not allocated and does
  not count). `dead` (`sup_delta`) is `Σ(2 + len)` over the old objects the new generation no longer
  reaches: a whole KV rewrite counts the old root, four arrays and delta records; a delta append the old
  root and the records its keys replace; a v4 commit the rewritten path, old values and old root; a log
  append the old root; a DOC commit the old HEADS, KEYS when replaced, and DOC. These two cells are
  accounting that compaction policy reads. A reader MUST NOT derive a read's answer from them.
- `SB_CAPACITY` is set at creation and carried by every commit. **Trimming**: an image MAY be stored cut
  at `max(arena_used, 1024)` cells (no readable superblock: stored whole); capacity is not changed. A reader reads what arrived (§3.1); a writer re-opening such an
  image restores the zero tail before it writes, sized by the capacity (`Store::image_bytes`), and past its ceiling, by default
  `MAX_PAD_CELLS`, it MUST NOT grow to a capacity claim: the claim is not an instruction to allocate.

## 9. Layouts over the container

The root of a layout is `cells[pt + 18]`. A layout's version is a field in its root; an unknown version
is a **refusal**, not a guess.

### 9.0 Reading an image: order and status

A reader of bytes MUST do these steps in this order. The numbers are the status codes every reader
answers with; status 0 is `OK`.

1. Pick the superblock (§3.1). Refused → **1** `NO_SUPERBLOCK`.
2. Ask the anchor (§7, last-commit rule). → **10** `BAD_CRC` or **41** `LAST_COMMIT`.
3. `pt` = 0 or root = 0 → **2** `NOT_A_KV` (an image with no root).
4. Dispatch on the root object's header, before any of its crc: if it is a DOC (digest `DIGEST_DOC`, `len` = 10, the
   object fits, cells 5 and 7 both = 5), §9.4. Else if its digest is `0x45564C47`, a log, §9.2. Else a KV, §9.1. So a
   root with a DOC digest that does not fit goes down the KV path and fails its crc there (10), and a root whose
   digest was damaged fails on the path it lands on: the digest is never the reason for a refusal at this step.

| Code | Name | Meaning |
|---|---|---|
| 0 | `OK` | read; `n` = entries or records, `root` = the fold |
| 1 | `NO_SUPERBLOCK` | no valid superblock, or the live one refused structurally (§3.1). Never a fallback |
| 2 | `NOT_A_KV` | a superblock but no root, or a slice, chain or v4 claim that does not hold, or a KV base not strictly ascending, or a DOC part this reader does not read or that does not hold (a SEG with `last` < `first`, the walk bound spent) |
| 3 | `NOT_A_LOG` | asked for the log of an image whose root is neither a log nor a DOC |
| 4 | `TRUNCATED` | a log's chain does not deliver the root's count, or never ends, or holds a record shorter than its head |
| 5 | `NULL_ARG` | an ABI null pointer; the Python reference reader also uses 5 for non-zero superblock cells 13-14 (§3.1) |
| 10 | `BAD_CRC` | an object the read reached fails its crc |
| 12 | `UNKNOWN_VERSION` | a KV root names a version above 4 and is not a DOC, or a log root names a version above 2 (§9.2) |
| 41 | `LAST_COMMIT` | the anchor does not hold and every object of its range checks |
| 75 | `CUT_UNDECLARED` | `verify` on a DOC: the file is whole, but a SEG is named by no TRUNCATE on the kept chain (§9.4) |
| 76 | `UNPINNED` | a host only (§9.5): the embedded reader is not the one the host pins; it is not run |
| 77 | `MODULE_REFUSED` | a host only (§9.5): the embedded module has an import, a start section, or no single bounded memory |
| 78 | `HOST_FAILED` | a host only (§9.5): the file could not be read, or the reader trapped or ran out of its clock or memory; no verdict |

The readers report `kv status=<s> n=<n> root=<fold>`, `log status=…` or `doc status=…`, with `n=0 root=0`
on a refusal, and exit with `<s>`. `<fold>` is printed as a signed `i64` (the `u64` hash reinterpreted in two's complement).

**`verify` is a second question, asked of a DOC that opened.** Status 75 is a status of `verify`, never of the open. The open
line of such a file MUST stay `doc status=0 n=<records> root=<log fold>`; the next line MUST read `verify status=75` in place of
the counters, with no counter printed; and the exit code MUST be 75. Any other refusal `verify` gives after a good open
takes the same form, `verify status=<s>` and exit `<s>`. At the ABI the same file answers `bw_open` 0, `bw_verify(1)` 75 and
`bw_counts` -75.
**The kind.** There are three words, `kv`, `log`, `doc`, and the ABI's numbers for them (§9.5.1): the KV version 1..4,
5 for a DOC, 16 for a log. A kind exists only for an opened image: a refusal has none (`bw_kind` = 0). A refused line MAY
start with any of the three words (the reference readers print `kv`), a consumer MUST NOT read a kind from it, and
conformance compares the status, `n` and `root` alone.
Codes 40-60 and 70-74 belong to a database or ABI layer and never come from a read of bytes as above.
A KV-only reader MAY stop at step 4 and refuse a DOC root as 12 (cell 5 reads as a version above 4);
§13 gives both answers for the fixture `doc`.

**The walk bound.** One number bounds every walk over objects: `n`, the cells that arrived (§3.1). Each object a walk reads costs `2 + c`, `c` being its `len` bounded by the cells behind it, paid before it is read. The objects of an honest file are disjoint, so no honest file spends it; a file that does has named something twice or laid objects over each other. Spent: a v3 chain → 2 (§9.1), a v4 walk, nodes and values → 2 (§9.1), a log walk → 4 (§9.2), a SEG chain → 2 and an ANCH chain ends without a refusal (§9.4). A read's walks each start from `n`; all the walks of one `verify` share one bound (§9.4). A count of steps (`step_cap`) is a separate rule, stated where it applies.

### 9.1 KV

Digests: root `DIGEST_KV_ROOT`, arrays and records `DIGEST_ARR_I64`. A reader of v1-v3 MUST NOT check the digest of
the root, of an array or of a delta record: those objects are found by position, and only their crc is checked.
A writer MUST keep the base's keys **sorted** by raw bytes, strictly ascending, and a reader MUST refuse a base that is not, with or without a chain.

**Version**: when the root has ≥ 6 payload cells and `payload[5]` > 0, the version is `payload[5]`;
otherwise it is 1. Above 4 → status 12, checked after the root's crc and before any of its cells is
followed.

- **v1** - root of 5 cells `{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB}`. KIDX and VIDX are `2n` cells
  of `(offset, len)` (at least 1 cell); KBLOB and VBLOB hold **one byte per cell**, the low byte of the
  cell (at least 1 cell).
- **v2** - root of 6 cells, adding `VERSION = 2`; offset and len are in **bytes**; blobs are packed,
  `max(1, ceil(bytes/8))` cells.
- **v3** - root of 8 cells `{.. v2 .., VERSION = 3, ref DELTA, D}`: a v2 base plus a chain of records,
  newest first; `D` is a **claim** of the chain's length. **Cell 0 of a v3 root is the BASE's `n` and is
  copied unchanged by every delta commit**: the number of entries of the map is what the replay leaves,
  not cell 0.
  A record (digest `arr i64`): `{ref NEXT (older, 0 = none), OP (1 put, 2 remove), KLEN, VLEN, key
  ceil(KLEN/8) cells, value ceil(VLEN/8) cells}`, packed. Its length MUST equal exactly
  `4 + ceil(KLEN/8) + ceil(VLEN/8)`, and a remove has `VLEN = 0`.
- **Read of v1-v3, in order**: (1) crc of the root → 10; (2) the version; (3) crc of each of the four
  arrays a non-null ref reaches → 10; (4) v3: walk the chain from `ref DELTA` with no crc yet, and refuse with **2**
  on: a
  root under 8 cells; `D` < 0; more or fewer than `D` records; a record whose `len` does not fit the image (a record cut by the image's end is therefore **2**, never
  10), a bad length, an unknown OP,
  KLEN or VLEN < 0, or a remove with VLEN ≠ 0; the walk bound spent (§9.0, that also ends a looping chain); (5) crc of every record, newest first → 10; (6) decode, refusing with **2** on:
  a null array ref; `n` < 0; `2n` above either index's cell count; any offset or length negative, or a
  slice past its blob (blob size = its cells, ×8 from v2); the entries together claiming more bytes than
  the two blobs hold; a v1, v2 or v3 base that is not strictly ascending in raw bytes. So with two defects the order
  is arrays' crc, chain shape, records' crc, decode: a null array ref behind a failing crc elsewhere answers 10.
- **The map** = the base, then the records from the **oldest**: a put inserts or overwrites, a remove
  deletes (a remove of an absent key changes nothing). Entries stay in byte order of keys. With an empty chain nothing is
  replayed: the map is the base, already ascending by step 6, and `n` is cell 0.
- **Fold**: FNV-1a 64 (basis `0xCBF29CE484222325`, prime `0x100000001B3`, xor the byte then multiply mod
  2⁶⁴) over `LE8(len(key)) ‖ key ‖ LE8(len(value)) ‖ value`, entry by entry in the map's order. It runs over
  the raw bytes of keys and values, which a reader MUST NOT decode or replace, so v1, v2, v3 and v4 of the same entries give one root. An empty map folds to the basis.
- **Compaction** writes v2 into a fresh image (doubling from 16 KiB up to the ceiling), trimmed. The
  delta writer compacts when the chain would pass `MAX_DELTAS`, when `superseded · DEAD_DIV > live`, when
  the base is v1, or when the arena is full.

- **v4** - a copy-on-write B+tree. The root has 8 cells and the same `DIGEST_KV_ROOT`:
  `{n, 0, 0, 0, 0, VERSION = 4, ref TREE, DEPTH}`. Cells 1-4 are **zero**, so a reader that ignores the
  version finds no arrays and refuses. `DEPTH = 0` is the empty map (`TREE = 0`, `n = 0`). Objects:
  LEAF (`DIGEST_KV4_LEAF`) `[META = 1<<56 | m] [K_i, ref VAL_i]… [keys]`;
  NODE (`DIGEST_KV4_NODE`) `[META = 2<<56 | hc<<48 | m] [ref child_i, K_i, hc hash cells]… [keys]`, where
  `K_i` is the FIRST key of child i; VAL (`DIGEST_KV4_VAL`) `[vlen] [value, packed]`.
  In META: kind = bits 56..63, `hc` = bits 48..55, `m` = bits 0..47. `K = (byte offset in the key area << 16)
  | length`, read as unsigned. The key area starts right after the slots, at payload cell `1 + m·(2 + hc)`
  (`hc` = 0 in a leaf; a leaf with `hc` ≠ 0 is refused); keys are packed, in slot order with no gap.
  **The hash cells**: every writer writes `hc = 0`. A reader MUST use `hc` to compute the slot stride
  `2 + hc`, and MUST NOT interpret or check the hash cells. They are reserved space, not a Merkle tree:
  the tree is not hashed, its integrity is the crc of each object.
  **The rules a reader refuses on**, each **2** unless it says otherwise:
  (0) the root: digest `DIGEST_KV_ROOT`, `len` = 8, cells 1-4 zero, `n` ≥ 0, `0 ≤ DEPTH ≤ 16`; `DEPTH = 0` requires `TREE = 0` and
  `n = 0`;
  (1) every ref inside the tree, the root's `ref TREE` included, is strictly BACKWARD (`off < 0`, `target ≥ 0`; a commit
  allocates values, leaves, nodes, the root last), which ends any walk. A target below 1024 is not refused by this rule: it
  meets the crc and digest of the level that reads it;
  (2) a leaf lies at exactly `DEPTH` levels below the root (the top node is level `DEPTH`, leaves level 1): an
  object of the wrong digest, or wrong META kind, at a level is refused; `1 ≤ m ≤ 65535`, and `1 + m·(2 + hc)`
  ≤ `len`; a reachable NODE has `m ≥ 2`;
  (3) keys ascend strictly within an object and across the whole tree, and separator i = child i's first key;
  (4) `n` is a claim: the count of the leaves' entries MUST equal it;
  (5) lengths are exact: each key inside the key area, each key at the running offset, the object exactly
  `1 + m·(2 + hc) + ceil(key bytes/8)` cells; a VAL has digest `DIGEST_KV4_VAL` and exactly `1 + ceil(vlen/8)` cells with
  `0 ≤ vlen ≤ MAX_VALUE_BYTES`.
  **Order.** The root's crc, then the version, then rule 0, then a depth-first walk from the top node. At each
  object: its crc (10); then its digest, META and length; then ALL its slots, keys and refs, and the separator against
  the parent; only then its children in slot order. In a leaf, for each slot in order: the running key order, the VAL's
  crc (10), the VAL's shape (2). `n` is compared last. A reader MAY decode slot by slot instead: that changes only
  WHICH refusal an image with two defects gets, never whether it is refused, and the fixtures have one defect each. A lazy open (§7) checks only the root's shape.
  **Write**: a commit copies only the path from each changed leaf to the root; untouched subtrees are
  shared with the previous generation by address. Compaction bulk-loads live entries into a fresh v4 image,
  only when `superseded · tree_dead_div > live` (default 1), when the arena is full, or on request. An
  index API (`key(i)`) on v4 is refused; scans walk the tree. Migration from v1-v3 is an explicit call; an
  open never migrates.

### 9.2 EvLog

Digests are ASCII labels, not `st_digest`: root `0x45564C47`, record `0x45565F52`. Root v1 is 7 cells
`{n, ref LAST, has_tip, tip0..3}`; v2 is 8 cells, adding `VERSION`. Version: when the root has ≥ 8 cells
and `payload[7]` > 0, the version is `payload[7]`; otherwise 1. `tip` is 32 bytes in four cells, in force
when `has_tip` ≠ 0.

A v2 record, `12 + (4 if an actor) + ceil(payload_len/8)` cells: `actor_seq` (0), `payload_len` in bytes (1),
`ref prev` (2), `id0..3` (3-6), `prev_id0..3` (7-10), `flags` (11), `[actor_pubkey 4]` (from 12, present when
flags bit 0 is set and the object has ≥ 16 cells; absent = 0^32), then the payload packed. `ref prev` names
the previous record in the file, 0 in the oldest; `prev_id` is the previous record's id, 0^32 in the
oldest. A v1 record, `15 + payload_len` cells: `{actor_seq, payload_len, ref prev, id 4, prev_id 4,
actor_pubkey 4, payload one byte per cell}`, with no flags.

**Read of a log**, from its root (the image's, or a DOC's `ref LOG`): (1) crc of the root → 10; (2) walk from
`ref LAST` through `ref prev` to null, newest first. Before each record count one step and pay the walk bound (§9.0):
more than `step_cap` steps, or the bound spent, means the chain never ends → **4**. `step_cap` = `floor(cells / 14) + 1`, `cells` being `n`
of §3.1, here and in every walk of §9.4. An object shorter than a record's head (12 cells in v2, 15 in v1) → **4**, a record cut short: no field of it is read from the cells behind it.
Each record's crc → 10, after that record's step. A reader MUST NOT check the record digest `0x45565F52`, nor the digest of a
DOC's `ref LOG` root: both are found by position, and only their crc is checked. (3) The count MUST equal root cell 0,
else **4**. A record's payload is bounded by its object: `payload_len` is read as unsigned, so a negative one is
huge, and a `payload_len` larger than the object holds reads as the bytes that are there, never as an error here and
never as 0 bytes; the fold runs over the bytes actually read, and `len(payload)` in it is their count. A v1 payload byte is the cell's low byte. A record whose crc fails is refused here; only a loader with a
quarantine policy of its own serves the rest. A root naming a version above 2 MUST be refused before the walk, with status **12**: that is
the word the vocabulary has, and the hub's loaders refuse it by that name (`UnknownVersion`).

**Log fold**: FNV-1a 64 (§9.1) over `LE8(actor_seq) ‖ LE8(len(payload)) ‖ payload`, from the oldest record.
`n` = the records. A DOC's root for a reader is the fold of its log.

**`flags`**: bit 0 = `actor_pubkey` present; bit 1 (`FLAG_SHA256`) = the id is sha256 over `doc::id_message` (§9.4, W-MG-HOLES) and the
payload starts with a 16-byte nonce (base64url text in a writer's hands, opaque to a reader); bit 2 = the
nonce was derived from the record, not drawn from entropy. Bits 3-63 are zero and a reader passes them
through. Bit 1 = 0 is a record written before that scheme: its id is the FNV-1a ×4 `legacy_id` below, and the
bytes of old images do not change. The scheme moves forward once: within one device a legacy record newer
than a record that moved the device is `broken`, and §9.4 says what moves one. `legacy_id(b)` is four `u64` LE concatenated (32 bytes), lane `i` ∈ 0..3:
FNV-1a 64 over `b` with basis `0xCBF29CE484222325 ^ (i · 0x9E3779B97F4A7C15 mod 2⁶⁴)`; `b` =
`prev_id ‖ payload` (2026-09-20 until the scheme moved) or `payload` (earlier). A plain log reader does
not hash ids; the DOC verifiers do (§9.4).

### 9.3 PROJTAB / PROJ / OUT

From superblock cell 5, an **absolute** index (below 1024 = no table):

```
PROJTAB  [m, (key64, ref PROJ, kind) × m]      sorted by key64; digest st_digest("PROJTAB{i64,[i64,ref PROJ,i64]}") = 2699843079
PROJ     [key64, kind, code, fmt, input_gen, tip0, tip1, ref OUT, fn_key]   9 cells; fmt = 1; digest st_digest("PROJ{i64,i64,i64,i64,i64,i64,i64,ref OUT,i64}") = 313398166
OUT      [value, count]                         digest DIGEST_OUT
```

For every kind `input_gen` = the low 32 bits of h1 of the input's root. Every offset in a PROJ and in a fingerprint is an
**absolute** cell index, not a `ref`. `key64` and `fn_key` are opaque to a
reader of the file. A table MAY hold several rows, of one kind or of many; a reader of the file MUST consider each
row on its own, and MUST NOT take a row's position or kind for the answer to a read. `code` is the producer's code version. A memo is
checked by digest, length, crc and the kind's OUT rule; a memo that fails reads as absent and the owner
recomputes. This is the layout of a DAG runtime; it is not part of the public library's API.

Kinds 3 and 4. `PROJ` (9 cells, `fmt` = 1), `PROJTAB` and kinds 1, 2 are unchanged. The layout of the `OUT`
that a `PROJ` points at depends on the kind:

| Kind | Input | OUT | Digest | Payload length |
|---|---|---|---|---|
| 1 LOG, 2 KV | log / KV | `OUT [value, count]` | `DIGEST_OUT` 514297014 | exactly 2 |
| 3 BLOB | log | `OUTB [value, count, nbytes, bytes packed]` | `DIGEST_OUTB` 4157677073 | exactly `3 + ceil(nbytes/8)`, `nbytes ≥ 0` |
| 4 ZSET | KV v1/v2/v3 | `OUTZ [value, count, m, w, (c0..c(w−1), weight) × m]` | `DIGEST_OUTZ` 85289679 | exactly `4 + m·(w+1)`, `m ≥ 0`, `1 ≤ w ≤ MAX_WIDTH` (64) |

A reader that knows only kinds 1/2 sees a foreign digest and refuses the row, then recomputes; it does
**not** misread. A row of an unknown kind is refused as a memo, so it reads as absent. A memo never moves a read's status, `n`
or `root`: §9.0 does not read this table.

- **Kind 1.** `value` = the log fold (§9.2); `count` = root cell 0; `tip0, tip1` = id0, id1 of the newest record.
- **Kind 2.** `value` = the KV fold (§9.1); `count` = root cell 0; `tip0` = the root's absolute offset; `tip1` = the root's whole h1 cell.
- **Kind 3.** `value` = FNV-1a 64 over the bytes; `count` = the log's record count; the tail of the last
  cell is zero. Tip as in kind 1. Step: the tip is found in the chain and `count + k` = the log's length →
  `step(old bytes, the k newer records from the oldest)`; the law is `step(full(a), b) == full(a ++ b)`.
  What the bytes mean is the producer's. The fixture's fold is LogSummary: `LE8(records) ‖ LE8(total payload
  bytes) ‖ LE8(log fold)`, 24 bytes.
- **Kind 4.** `value` is a Z-set digest: FNV-1a 64 over the rows **ascending** (signed `i64`,
  lexicographically by cell), each row contributing `LE8(c0) ‖ … ‖ LE8(c(w−1)) ‖ LE8(weight)`, hashed as one
  run from the basis; rows of weight 0 are not written. `count` = cell 0 of the KV root. A KV row is
  **two cells**, `K64('O' ‖ LE8(len(key)) ‖ key)`, `K64('O' ‖ LE8(len(value)) ‖ value)`, weight 1, so `w = 2`
  for the identity query. `K64(frame) = (crc32(frame) << 32) | (len(frame) & 0xFFFF_FFFF)`, as `i64`.
  - **tip0** = the offset of the newest delta record (0 = no chain).
  - **tip1** = a FINGERPRINT: FNV-1a 64 over the pairs `(offset, h1)` as LE8, first of the four base arrays
    (refs 1..4 of the root), then of each chain record **from the oldest up to tip0** inclusive.
  - Hit: `code`, `input_gen`, tip and `count` match (and `w` = the query's width). Step: tip0 is in the
    current chain and the fingerprint of its prefix equals tip1 → `OUT + Q(Z-set of the k newer records)`, `Q`
    linear. Otherwise a full recompute.
  - The fingerprint exists because a base rewritten in place moves the arrays' offsets, and a record
    re-sealed at the same generation changes h1 (the crc is its high half). Kind 2 is keyed on the root alone and
    does not see this. **NOT COVERED, stated:** a crc32 collision in such a rewrite.

### 9.4 DOC / SEAL / HEADS / KEY / KEYS

Root-version **5** = DOC. Root-version numbers are **one space across all layouts**: a KV reader reads
root cell 5 as a KV version and refuses above 4 (§9.1); a log reader reads cell 7. So a DOC carries 5 **in
both** cells, and a reader that does not know DOC refuses it: status 12 from a KV reader. A log reader
refuses versions above `MAX_VERSION` = 2.

A 32-byte field below takes 4 cells. Cell numbers are payload cells.

```
DOC   10 cells, digest st_digest("DOC{i64,ref EVLOG,ref KV,ref HEADS,ref KEYS,i64,ref SPEC,i64,ref ANCH,ref SEG}") = 2619226223
      0 FLAGS (bit0 ids = sha256, bit1 tree = fold(log), bit2 self-describing)
      1 ref LOG (EvLog v2 root, 0 = none)   2 ref TREE (a KV root, 0 = none; which versions a reader reads: "Reading a DOC")
      3 ref HEADS   4 ref KEYS   5 VERSION = 5   6 ref SPEC (0 = none)   7 VERSION = 5   8 ref ANCH (0 = none)
      9 ref SEG (the newest log segment, 0 = the log was never cut)
SEAL  {VERSION=1 (0), actor_id (1-4), seq (5), tip (6-9), claimed_ms (10), ref PREV_SEAL (11), ref TIME (12, 0),
      alg (13), sig_len (14), sig packed (15..)}
      15 + ceil(sig_len/8) cells (ML-DSA-65: 429); digest st_digest("SEAL{i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,ref SEAL,ref TIME,i64,i64,[i64]}") = 1951851249.
      One per device commit. sig_len ≤ 65536.
      Signed message: "bebop-seal/1" ‖ actor_id ‖ LE8(seq) ‖ tip ‖ LE8(claimed_ms) ‖ prev_tip (the tip of this
      device's previous seal, 0^32 for the first). Changing the message means SEAL v2.
      With alg = 65 the signature is ML-DSA-65 `Sign_internal` / `Verify_internal` (FIPS 204, algorithms 7 and 8)
      DIRECTLY over this message, without the `0x00 ‖ len(ctx) ‖ ctx` prefix of the outer `ML-DSA.Sign`: a library
      that offers only the outer interface signs a different message and will not verify such a seal. The
      reference writer signs with `rnd = 0^32`; verification does not depend on `rnd`. The same holds for anchor kind 1.
HEADS {m, (actor_id 4, seq, tip 4, ref SEAL) × m}  ascending by actor_id; 1 + 10m cells;
      digest st_digest("HEADS{i64,[i64,i64,i64,i64,i64,i64,i64,i64,i64,ref SEAL]}") = 4237402659
KEY   {actor_id (0-3), alg (4), valid_from (5), revoked_at (6), ref ADMIT_REC (7), pk_len (8), pk packed (9..)}
      9 + ceil(pk_len/8) cells (ML-DSA-65: 253); pk_len ≤ 65536;
      digest st_digest("KEY{i64,i64,i64,i64,i64,i64,i64,ref EV,i64,[i64]}") = 2540354000.
      revoked_at = 0: not revoked; k > 0: seals with seq > k are invalid; -1 (REVOKED_ALL): revoked after 0, no seal is
      valid. This cell is an INDEX of the REVOKE records and nobody signs it. ADMIT_REC = 0: a genesis key.
KEYS  {m, (actor_id 4, ref KEY) × m}  ascending; 1 + 5m cells;
      digest st_digest("KEYS{i64,[i64,i64,i64,i64,ref KEY]}") = 1448248784
SEG   {VERSION=1 (0), first (1), last (2), root (3-6), ref PREV_SEG (7), ref BLOB (8, 0), redacted (9), declared (10),
      m (11), (actor_id 4, seq, tip 4, scheme) × m (12..)}
      12 + 10m cells; digest st_digest("SEG{i64,i64,i64,i64,i64,i64,i64,ref SEG,ref BLOB,i64,i64,i64,[i64,i64,i64,i64,i64,i64,i64,i64,i64,i64]}") = 2192499951.
      `scheme` carries the rule "the id scheme moves forward once" across a cut: 1 if, before the cut, the device
      already had a CHAINED (sha256) record, 0 while all its records were legacy.
      first/last are positions of the cut records in the FILE, from its first record; they left `last − first + 1`
      records. root is the RFC 9162 tree over their ids in file order (leaf = sha256(0x00 ‖ id)). BLOB = 0 (the records
      live in the archive that gave up the cut). redacted/declared are the tombstones and the FORGOTTEN sums under
      the cut. The rows are the frontier at the cut: (seq, tip) of each device's newest cut record = the prev_tip of
      its oldest kept seal (a seal with no `ref PREV_SEAL` reads prev_tip from here).
```

**Identity of a device (`alg`, `actor_id`).** The registry of `alg` numbers has one entry: **65** = ML-DSA-65
(pk 1952 bytes, signature 3309 bytes). Any other number, or a key or signature of another length, MUST NOT
verify: such a seal counts bad, never a guess at another scheme. `actor_id` is a 32-byte name; the writer
convention is `actor_id = sha256(pk)`, and `actor_pubkey` of a record = `actor_id`, while the pk itself is held
once, in KEY. A verifier with an algorithm recomputes it (`Verifier::actor_of`, below); a reader with none cannot judge it.

**Record framing.** The framing of a record's payload is one rule for every verifier:

1. **Nonce.** When `FLAG_SHA256` is set the first 16 bytes of the payload are the nonce and what follows is the
   tail. A flagged payload shorter than 16 bytes has an empty tail. Otherwise the tail is the whole payload.
2. **Kind byte.** A leading `D` (0x44, a declaration) or `K` (0x4B, a key operation) of the tail is skipped; the rest
   is the **body**. Any other first byte: the tail is the body. Every mark below starts with `b`, so the
   two forms cannot be confused: records with a kind byte (the reference hub) and without (library fixtures) both
   parse. Other kind bytes (the hub's order events, 1..=8) carry no declaration, and their bodies match no mark.
3. **Tombstone.** A record is recognised as a tombstone by the **high bit (0x80) of the kind byte**, the first byte of
   the tail: behind the nonce when `FLAG_SHA256` is set, the first payload byte when it is not. An empty tail is no
   tombstone. A writer that redacts a record MUST set that bit, and on a flagged record MUST also erase the nonce to
   sixteen ASCII `0` bytes, because a kept id would otherwise confirm a guessed payload by recomputation. A reader
   MUST NOT read the erased nonce as the mark: a legacy record has no nonce to erase, and a record with an erased nonce and no bit is an edit, **broken**.

Key-op bodies: `ADMIT = "bebop-admit/1" ‖ actor_id ‖ LE8(alg) ‖ LE8(valid_from) ‖ LE8(pk_len) ‖ pk`,
`REVOKE = "bebop-revoke/1" ‖ actor_id ‖ LE8(after_seq)` (signed),
`TRUNCATE = "bebop-truncate/1" ‖ root ‖ prev_root ‖ LE8(first) ‖ LE8(last) ‖ LE8(redacted) ‖ LE8(declared) ‖ seg_hash (32)`,
`FORGOTTEN = "bebop-forgotten/1" ‖ LE8(n_redacted) ‖ subject (32) ‖ (id (32) ‖ digest (32)) × n_redacted`. A body shorter than its
fields is not that body, and a FORGOTTEN whose `n_redacted` is not the number of rows it holds is not a FORGOTTEN. An ADMIT
needs `0 ≤ pk_len ≤ 65536`. Bytes behind the last field of an ADMIT, REVOKE or TRUNCATE MUST be ignored: only a FORGOTTEN is exact.
In a hub's file only records of kinds `K` (key op) and `D` (declaration) have a body: `HubIds::body` answers empty for any other, or a
98-byte order id would read as a declaration. A reader with no signature code (`ReaderIds::body`, Python) leaves another kind's body
alone: it starts at its kind byte 1..8 and matches no mark, and a body with no kind byte (bebop-log's own fixtures) starts at its mark.
All refs in a commit point back: record → KEY → KEYS → SEAL → HEADS → log root → DOC.

**Cut.** The container has no compaction of a log, so a cut is a rebuild into a fresh image: the oldest records,
all but the `keep` newest, leave under a SEG; kept records are carried verbatim at new offsets; seals of kept
records verbatim, with `ref PREV_SEAL` relinked; KEY with `ref ADMIT_REC` remapped; old SEGs chained. The old
image is the archive, and its ids recompute `root`. A file after a cut is UNDECLARED until the cutter commits a
record with a TRUNCATE body, covered by a seal. ANCH objects are all carried. An anchor whose `over` went under a
segment reads as `under_cut`: not bad (the id may have been in the log), not ok (the file cannot show it), but
unchecked, naming the segments; its proof is still checked, and the ARCHIVE places it (`over` among the cut ids
whose tree is the segment's root). A cut is refused when: it would cross the ADMIT of a key still in KEYS; it would
take a **REVOKE** record (the revocation is that record, and a frontier row is signed by nobody, so it cannot be carried;
the writer refuses by the mark, not by the seal, because it has no verifier); there is nothing to cut; the id scheme
has no tree; the image carries a TREE or SPEC (those belong to the layer that owns them).
In-place redaction for a DOC is a copy with payloads replaced, ids, links, seals, anchors and segments verbatim;
the redactor then commits a signed FORGOTTEN. A tombstone over a REVOKE record is refused the same way. A copy or an
archive carries REVOKE records and KEYS verbatim, so the verdict travels with the file.

**Reading a DOC** (§9.0 step 4 reached): (1) crc of the root → 10; (2) the log as §9.2; (3) crc of HEADS and KEYS when
named → 10; (4) SPEC when named: crc → 10, and not a SPEC (digest, 8 cells, VERSION = 1) → 2. The SPEC's sha256 is a claim:
a read and `verify` MUST NOT check it, and §9.5 binds a host before it executes; (5) TREE when named: crc → 10, then by cell 5 of
the object: 4 is a v4 tree as §9.1, which a DOC reader MUST read; 1-3 a reader MAY refuse with 2, and the reference reader does (§12, item 1);
anything else is 2, never 12. Digest and root rules are those of §9.1;
(6) SEG chain from `ref SEG` through `ref PREV_SEG`: crc → 10; not a SEG (digest, version, exact length) → 2; `last` < `first` as `u64` → 2;
more steps than `step_cap`, or the walk bound spent → 2; (7) the ANCH chain from `ref ANCH` through `ref PREV_ANCH`: an object that is not an ANCH (digest, VERSION = 1, `0 ≤
proof_len ≤ MAX_PROOF_BYTES`, exact length) ENDS the walk without a refusal, as do `floor(cells / 11) + 1` steps and the walk bound spent; the crc of
each ANCH read → 10. Only after that walk is a crc checked: a foreign object at the end is not one. Status 0 reports `doc status=0 n=<records>
root=<log fold>`; with `ref LOG` = 0 there are no records, so `n` = 0 and `root` = the FNV basis as a signed `i64`
(-3750763034362895579). HEADS and KEYS read by digest: a table of a foreign digest reads as empty, `m` is clamped to the
rows its object holds, and a KEYS row with a null ref is skipped. A foreign KEY object is skipped (digest, exact
length, `0 ≤ pk_len ≤ 65536`).

A KEY object's crc is not read: a read and `verify` MUST NOT refuse on it. A key's actor is the KEY object's `actor_id`, and the
actor cells of its KEYS row MUST NOT be used. A `ref ADMIT_REC` that reads as null under §5, dangling included, makes a genesis key: that
one unsigned cell is why `Trust::Genesis` proves nothing. Of several HEADS rows for one actor the first in table order answers
for it, and of several KEYS rows the first. Only bit 1 of FLAGS changes an answer (`tree_unsigned`): a reader MUST NOT derive anything from
bits 0 and 2, and a record's scheme comes from that record's own flags. An anchor changes no status and no counter: "bad" and `under_cut`
are verdicts of an anchor check, which a blind reader does not run; it reports what it read as present.

**Record id (W-MG-HOLES, 2026-10-31).** `id = sha256("bebop-record/1" ‖ LE8(flags) ‖ actor (32) ‖ LE8(seq) ‖ prev (32) ‖ payload)`,
`doc::id_message`, with `flags` excluding `FLAG_ACTOR`. M0 hashed `prev ‖ payload`, and no live record carries that. A redaction keeps
the id: flags, actor, seq and `prev` do not move and the redacted bit is the kind byte inside the payload, so a tombstone's id
stops checking anything, and the tombstone's cells are bound by the `digest` in its FORGOTTEN. A record with `FLAG_SHA256` is never tested
by the legacy rule. The stock log carries no actor and no flags on the wire: actor = 0, seq = position counted from 0, flags derived from the
text (`chain::stock_flags`: `FLAG_NONCE_DERIVED` when the nonce equals the one derived from `prev` and the body). Legacy (FNV) records
carry none of these guarantees: their actor, seq and flags are free (§14).

**Segment hash (W-MG-HOLES).** `chain_hash` folds oldest to newest: `h = hash("bebop-seg/1" ‖ h_below ‖ root ‖ blob≠0 (1 byte) ‖ first ‖ last ‖
redacted ‖ declared ‖ m ‖ (actor (32), LE8(seq), tip (32), LE8(scheme)) × m)`, numbers LE8, `h` under the oldest = 0^32, hash = sha256. The TRUNCATE
signs the newest hash, so no number `verify` sums from any segment and no frontier row stands unsigned. The records that left are not under this
hash: `root` names their ids and a tombstone keeps its id, so a tombstone's bytes under a cut are bound only by a FORGOTTEN still in the
file; the archive is what shows them.

**`verify`: the counters.** Over the records oldest first, grouped by device (`actor_pubkey`). Refusals first: the
steps above, and then CUT_UNDECLARED (below). `records` = the log's records; `forks` = `(actor, seq)` pairs
that occur more than once. For each device, for each of its records in file order, exactly one of:

- `prev` test, first: a device's first record passes if the device's newest frontier row (newest segment first, and no
  older row counts) sits at `seq − 1` modulo 2⁶⁴ and its tip equals `prev`, or if no segment names the device at all; a later
  record passes if `prev` = the id of the device's previous record. Fail → **broken**. So the first record of a device with no
  segment is not tested against anything, and one whose row sits at another seq is broken.
- `id == Ids::chained(record)` → **chained**, and the device has moved. It is tested on every record, flagged or not. Only
  this result moves a device: a tombstone does not, flagged or not, so a legacy record after a tombstone stays **legacy** while the
  device has not moved.
- `legacy`: `FLAG_SHA256` clear, and `id` = `legacy_id(prev ‖ payload)` or `legacy_id(payload)`. → **legacy**, or **broken**
  when the device has moved. A device starts as moved if its newest frontier row (newest segment first) has
  `scheme` = 1.
- tombstone mark, and the link holds: the device's next record has `prev` = this id, or, for its last record, the
  device's HEADS row tip (no row: the log's tip, root cells 3-6 when `has_tip` ≠ 0) = this id. → **redacted**.
- otherwise **broken**.

**The seq of a device.** A device's seqs are 1, 2, 3 ... along its chain, and every rule that reads a seq (`valid_at`, "covers") trusts that. It is judged for an actor with a KEY row; any other actor signed nothing and may carry any number. Walk the device's records in file order, keeping `last` (the seq of the previous record, none at the start) and `soft` (the seq of the previous legacy record, none at the start). A record is **dense** when: its actor has no KEY row; or its `(actor, seq)` occurs more than once, which is a fork, counted in `forks` and not again here; or `last` exists and seq = `last` + 1 with no wrap; or `last` does not exist and any of: the device has a frontier row, seq = 1, `soft` exists and seq = `soft` + 1 with no wrap, or this is the device's first record and its `prev` is neither 0^32 nor the id of any record in the file. After a record classed legacy, `last` is none and `soft` is its seq; after any other record `last` is its seq, broken ones included, so one hole reads as one. A record whose `prev` test holds and whose id equals `Ids::chained` is **chained** only when dense; otherwise it is **broken**, it still moves the device, and a seal whose tip is that id is bad. A tombstone is **redacted** only when dense or when `last` does not exist. A legacy id does not cover its seq, so a legacy record is not judged: a device with no chained record has no bound seq at all (§14).

**What a number may decide.** Only a record counted **chained** (`prev` holds, id holds, dense) may admit a key or declare a TRUNCATE, FORGOTTEN or REVOKE. `valid_at` compares the seal's own seq, which the seal's tip binds to a record whose id covers that seq, and with dense seqs `seq ≤ covered` means at or below that seal on the device's chain.

**Bounds of one `verify`.** A verifier reads each object once, from the walk bound of §9.0, before copying it; `check_anchors` pays from its own. It holds at most 8 × the file size of heap above the image (measured worst 3 ×: 1.58 MB, 7 000 records of 7 000 actors; refused shapes 1-2 × at 1 MB). Time is O(n log n) in the file for reading, indexing and hashing, plus settle rounds × admission passes × distinct seals, each a map lookup; each signature is checked at most once, and a round past the first needs a real signature, 3 309 B each.

The tombstone's link test is one rule for every device, and it is REQUIRED, fallback included: a device with no HEADS row
uses the log's tip. Status 75 and all other refusals come before the counters are reported.

`seals`: for each HEADS row in table order, walk from `ref SEAL` through `ref PREV_SEAL`; a null `ref SEAL` (§5) starts no walk and
counts nothing. A seal is one object and counts once, however many rows reach it: a walk that meets a seal already read stops there,
and one that meets a seal it read itself (a ring) also counts one bad. An object that fails its crc, or is not a SEAL (digest, VERSION = 1,
`0 ≤ sig_len ≤ 65536`, exact length), ends the walk and counts one bad, once, and no seal. `seals` = the seals read + those bad.
All the walks of one `verify` (log records, HEADS, KEYS, SEGs, KEYs, SEALs) share one walk bound (§9.0); a file that spends it is refused, `verify status=2`.
A seal is **ok** when all hold, otherwise **bad**: for the first seal of a row, whichever walk read it, actor, seq and tip equal the row's; the tip names a record
at `(actor, seq)`; `Verifier::actor_of(pk)` equals the actor id, roots included, because the KEY row is unsigned and `Trust::Only` brings ids, not keys
(a verifier with no such rule, `None`, skips this); the actor is admitted; its key is valid at seq (`revoked_at` = 0, or `revoked_at` > 0 and `seq ≤ revoked_at` as unsigned; the `revoked_at` that
Revocation derives, never the table's), its `alg` equals the
seal's, and the signature verifies over the message with `prev_tip` = the previous seal's tip, else the device's frontier
tip at `seq − 1`, else 0^32.

**"Covers".** An actor's ok seals **cover** seq `s` when the highest seq among that actor's ok seals is ≥ `s`. It is a
high-water mark: seals need not exist for consecutive seqs, and one seal covers every lower seq of its device.

**Admission** is a fixpoint over signed records, never the table alone: start from the trusted keys (below). A key is
admitted when its `ref ADMIT_REC` names a record whose body is an ADMIT with the same actor, alg, valid_from and pk,
and that record's writer is admitted, has ok seals covering that record's seq, and holds a key valid at that seq.
Repeat until nothing grows, recounting the seals. A reader with no signature algorithm has no ok seal, so
nothing is ever admitted beyond the start.

**Trust, the most important sentence in this document.** `Trust::Genesis` starts from the keys the file itself calls
genesis (`ref ADMIT_REC` = 0), in a table nobody signs, so a forger writes his own key in as a root and everything below
holds for him: under it, an ok result means only "signed by the keys this file names as its roots", which is all a
self-contained read can say and is NOT evidence of who wrote the file. `Trust::Only` starts from exactly the actor ids a
verifier brings from outside the file, which nobody who edits the file can change: every statement below about what
a thief cannot do is a statement under `Trust::Only`, and under no other trust. A verifier that wants to know who signed
MUST bring its roots.

**Revocation.** `revoked_at` is signed by nobody, so `verify` derives it from the records. A REVOKE is **effective** under the
rule for declarations (below): classed **chained**, no tombstone mark, its actor admitted, its seq covered by that
actor's ok seals. The effective `revoked_at` of a key is the narrowest of the effective REVOKEs naming it: start at 0, and
for each `after_seq`, take `a = after_seq` if `after_seq` > 0, otherwise `a = REVOKED_ALL` (-1), then `revoked_at = a` if it was
0, otherwise `min(revoked_at, a)`, signed. So `after_seq` 0 becomes -1, because 0 already means "not revoked" and a key that never
sealed must be revocable, and a top-bit `after_seq` is negative and becomes -1 too. No REVOKE widens a key. A key with `revoked_at` = -1 is
valid at no seq; only a `revoked_at` > 0 is compared with `seq`, as unsigned. A REVOKE naming no key in KEYS changes nothing.

**Who may revoke whom:** ANY key valid at its own seq may revoke ANY key, itself and a root included. There are no roles, and a
stolen key can do this too. A REVOKE is **final**: a writer MUST refuse an ADMIT of a key already revoked in the table
(`Readmit`), and a new device takes a new key. The table is an index: a key whose table cell differs from the derived value,
either way, counts in `keys_disagree`, never as a refusal, because the records answer in full without the table and a
refusal would let one forged cell hide every other counter.

A REVOKE reaches back: `after_seq` may sit below seals that carried an ADMIT or another REVOKE, which fall with them, and the
order of a file across devices is signed by nobody. So the answer is an alternating fixpoint, independent of that order.
`round(F)` applies the REVOKEs in `F`, runs admission to its fixpoint, and returns the REVOKEs effective under that result.
Start with `sure` empty. Repeat: `r = round(sure)`; if `r` = `sure`, stop with that round. Else `maybe = r`, and `s = round(maybe)`;
if `s` has no more members than `sure`, stop with the round of `maybe`, every REVOKE of `maybe` staying IN FORCE (the narrowest
reading), and `revokes_unsettled` = |`maybe`| − |`sure`|. Else `sure = s`. Unsettled means the records contradict themselves (two keys
that revoked each other back past their REVOKEs, a key that declared itself dead before its own seal). That is not
resolved here: a seal is bad as a result, so `holds` is false, and a person decides.

**A reader with no signature algorithm (blind)** judges no seal, so it takes a REVOKE on its chain alone (classed **chained**, no
tombstone mark): `keys_disagree` compares `revoked_at` with the narrowest `after_seq` of those records, by the rule above.
That is the records' claim against the table, NOT a proof: a REVOKE a verifier would void counts here. `checked` = 0, and `holds`
is false.

**Who may erase whom.** The sum in `holds` alone would let any FORGOTTEN balance any tombstone, and a tombstone is one bit anyone can
set and cannot say what it was. So each live tombstone, a record classed **redacted**, must be listed by a counted FORGOTTEN of the record's AUTHOR or of a ROOT (a trusted key), with
`digest` = `Ids::chained` over the tombstone as it now stands, and each listed id still in the log must be a tombstone. `tombs_foreign` counts one per
tombstone that fails this and one per listed live record that is no tombstone. A listed id not in the log left under a cut. When an id is listed twice the
last row in file order rules; that is this rule's alone, and `declared` below sums every row. A blind reader takes its roots as the library does under `Trust::Genesis`, the keys with no ADMIT
record: not 0 means the file's own FORGOTTENs do not account for that tombstone; 0 proves no authority, because it judges no seal and the roots are in a table
nobody signs, so a forger with no key can write both halves.

**Declarations.** A TRUNCATE or FORGOTTEN **counts** only when its record is classed **chained** (so its `prev` test held) with no
tombstone mark, and its actor is admitted with an ok seal covering its seq. A verifier that carries no signature
algorithm cannot judge seals, so it takes a declaration on its chain alone, which is the first half of that sentence;
`holds` stays false for it. FORGOTTEN of a revoked key declares nothing to a verifier that judges seals; a blind reader does
not apply revocation to a declaration. The NEWEST SEG MUST be named by a counted TRUNCATE on all six fields
(root, prev_root = the root of the SEG at `ref PREV_SEG` or 0^32, first, last, redacted, declared; numbers compared as
`u64`) and on `seg_hash` = `chain_hash` of the whole chain, else refuse **75** `CUT_UNDECLARED`. The same hash covers every older SEG, so a later cut may
take their TRUNCATEs.

`segments` = SEGs; `cut` = Σ(last − first + 1); `cut_redacted` = Σ redacted; `declared` = Σ n_redacted of counted
FORGOTTENs + Σ declared of SEGs. These three are `u64` sums that saturate at i64::MAX on every reader width: a file's claim neither wraps nor depends on the platform. `heads`, `keys`: table rows read (KEYS rows that resolve to a KEY); `genesis`: keys with no
ADMIT record, which is all that can be counted with no signature code; `admitted`: keys admission reached, reported only by a
verifier that judges signatures (the reference auditor's `doc_verify` tool).

**Vouched for: `unsealed`.** Collect the tip INSIDE every ok seal (a blind reader: inside every seal it read), never a HEADS
row's tip: HEADS names where to start reading seals, and nobody signs it. From each tip follow `prev` by id, across devices and
over seq gaps, stopping at an id already visited, one visited set for all the walks of the file. A seal that fails its crc or
is not a SEAL gives no tip. A record is **vouched for** when such a walk reaches it. An id that is a record of the file continues the walk even
when it is also a frontier tip; ids are a map, so of two records with one id the walk follows the newer in file order. A walk that leaves
the file anywhere but at 0^32 or a frontier tip of any segment counts one. `unsealed` = records not vouched for (records − distinct ids visited, so each repeated id counts one) + such walks + records of a key above that key's own ok seals (a device's first record chains onto the log, so another device's tip reaches a record its author did not cover: a REVOKE with a rolled-back HEADS row; a blind reader judges no seal and does not count this term; an actor with no KEY row does not fall under it). Seals
need not cover consecutive seqs: one seal over a tip vouches for every record under it. A redaction keeps its id and `prev`, so
it stays on the path.

`holds` = broken 0, seals_bad 0, forks 0, seals_ok > 0, redacted + cut_redacted = declared, keys_disagree 0, tombs_foreign 0, unsealed 0
**and** tree_unsigned 0: every record vouched for, every such chain ending at 0^32 or at a frontier tip, and no tree outside the signatures.
`tree_unsigned` = 1 when the DOC names a TREE or sets `FLAG_TREE_IS_FOLD`: `verify` never reads a tree and no signature covers one, so `holds` speaks for the log alone
and is false on such a file until a signature does. `revokes_unsettled` is not
a term: a contradiction always leaves a seal bad. This does **not** detect a tail removed together with its seals and its HEADS
row: what remains is a shorter true file (§14). Only anchors or a second file see that.

Printed line, in this order: `records= chained= legacy= redacted= broken= forks= seals= checked= heads= keys= genesis= segments=
cut= cut_redacted= declared= keys_disagree= unsealed=`, where `seals` = ok + bad and `checked` = 1 if signatures were judged, else 0.

**What is not in the line.** The line and the ABI's counters are closed: `tombs_foreign`, `tree_unsigned`, `revokes_unsettled`, `admitted` and
the split of `seals` are in neither, and a reader MUST NOT add a field to the line. Five terms of `holds` can be read off the line; `seals_ok` > 0,
`seals_bad` 0, `tombs_foreign` 0 and `tree_unsigned` 0 cannot. A verifier that judges signatures MUST report all four. A blind reader's `holds` is false
whatever they are, because it has no ok seal, so it need not compute them; one that does MUST print each on a line of its own on its
diagnostics channel, stderr, as `name=<n>`, never on stdout. The reference Python reader prints `tombs_foreign=` there, and a host of the ABI has neither value.

**What an ok seal says.** The holder of key K signed this tip at seq s of its chain, and nothing under the tip
has changed since. It does NOT say the event happened, that a quantity is true, or that `claimed_ms` is right.
Time comes only from an anchor. A crc failure in the log or the tables is a refusal before any counting. A `forks`
count is a named finding, not the death of the identity. Signatures are a seam: a reader MAY carry no algorithm,
and then every seal is COUNTED and none is CHECKED, and it MUST say so in words wherever it prints the count, on a line of its own
beside the printed line. `checked=0` alone does not say so, and the words MUST NOT go inside the line, which stays exact.

**ANCH - level 1 time anchors.**

```
ANCH  {VERSION=1 (0), kind (1), over (2-5), time_ms (6), ref PREV_ANCH (7), proof_len (8), proof packed (9..)}
      9 + ceil(proof_len/8) cells; digest st_digest("ANCH{i64,i64,i64,i64,i64,i64,i64,ref ANCH,i64,[i64]}") = 180762881.
      DOC cell 8 names the NEWEST; each names the previous one (0 for the first), a chain like the seals.
      over = the id of a record of this log (the tip at the anchor); an anchor whose over names no record is bad
      before any adapter runs. time_ms = the writer's claim; the proof is checked by an adapter. MAX_PROOF_BYTES = 1 MiB.
      kind 1 platform TSA mark: proof = pk(1952) ‖ sig(3309), ML-DSA-65 over
             "bebop-anchor/1" ‖ LE8(kind) ‖ over ‖ LE8(time_ms); checked against platform keys the AUDITOR brings,
             never from the file.
      kind 2 OpenTimestamps: proof = an .ots file whose digest = over; the check computes the ops up to a Bitcoin
             attestation and compares with the block's Merkle root from the auditor's header source.
      kind 3 Roughtime: proof = nonce(64) ‖ reply; nonce = over ‖ sha256("bebop-roughtime/1" ‖ over).
```

The core does not interpret a proof. The verdict is Ok (not-after, not-before, what), Bad, or Unchecked: a
kind with no adapter, no key, still pending at a calendar, or a hash the build lacks is Unchecked with a reason,
never ok. Writing an anchor is one transaction (ANCH + a new DOC; log and seals unchanged). Copying a DOC into
a bigger image re-signs nothing.

### 9.5 SPEC / BLOB - a file that reads itself

This is the **archive** level, not every image: a working image keeps `DOC.SPEC = 0`. An archive file is written
FRESH and carries SPEC as **the first object of the arena** (cell `ARENA` = 1024); DOC names it in cell 6 and sets
FLAGS bit 2. A writer refuses a store whose arena is not empty: "first" is a promise to a host that cannot parse.

```
SPEC  {VERSION=1 (0), ref WASM (1), ref TEXT (2), sha256 (3-6), abi (7)}   8 cells; digest st_digest("SPEC{i64,ref BLOB,ref BLOB,i64,i64,i64,i64,i64}") = 849619745
BLOB  {len, bytes packed}                  1 + ceil(len/8) cells; digest st_digest("BLOB{i64,[i64]}") = 799774958; MAX_BLOB_BYTES = 16 MiB
```

`WASM` is a reader (wasm32, no imports, ABI §9.5.1); `TEXT` is this document in one of its two languages; `sha256` is a
**claim of the file** about the embedded reader; `abi` is the reader's ABI version. SPEC's refs point **forward** (the
blobs follow it): "backward only" is a rule of KV v4, not of the container.

**Bootstrap with no parser, six lines:** h0 at cell 1024 → high 32 bits = 849619745, otherwise this is no archive (2);
cell 1027 (payload[1]) = offset → `W = 1024 + offset`; cell `W+2` = the length `L` in bytes; the reader's bytes are
`[(W+3)·8, (W+3)·8 + L)`; cells 1029..1032 are the claimed sha256. A cell that becomes an offset or a length past the file refuses (2).
Then, in this order, before anything runs. A host MUST compute sha256 over the bytes it read and **refuse if they differ from the claim** (the reference host: 4). It MUST check the module's outline with no parser but its own: wasm 1, no import section with an entry, no start section, exactly one memory, unshared, with a maximum; otherwise **77** `MODULE_REFUSED`, not instantiated. The live superblock (§3.1) names a DOC, and that DOC's `ref SPEC` MUST be cell 1024, or the file runs a reader it does not declare → 2. The reader's sha256 MUST equal the one the host pins; the file agreeing with itself is no verify, so any other reader is **76** `UNPINNED` and is not run. A host MAY offer to run it anyway on the operator's word: every line then says the verdict is the file's own code, and the exit code is never 0. The module runs with no imports, under a clock and a memory ceiling, and no byte of it is printed unescaped; a trap, the clock or the ceiling is **78** `HOST_FAILED`, no verdict.

**Trust.** The embedded reader is *testimony*: it runs sandboxed with no I/O, and its sha256 is named
here and in the reader's release; the host prints both numbers side by side. A reader without ML-DSA *counts* seals, it does not
*check* them (`checked = 0`), and says so in words (§9.4).

**Cost:** `8·((2+8) + (2+1+⌈wasm/8⌉) + (2+1+⌈text/8⌉))` bytes per archive FILE, zero on a working image.

#### 9.5.1 Reader ABI (ABI 2)

`bw_version`=2, `bw_alloc/bw_free`, `bw_open(ptr,len)→status`, `bw_kind`→ 1..4 KV | **5 DOC** | 16 LOG,
`bw_len`, `bw_root(out)`, `bw_get`, `bw_scan_next` (a KV, or a DOC's v4 tree), `bw_log_len`/`bw_log_at`
(a LOG or a DOC's log: `seq8 ‖ plen8 ‖ id32 ‖ prev32 ‖ actor32 ‖ payload`), `bw_heads(out,cap)` → rows
`actor32 ‖ seq8 ‖ tip32`, `bw_verify(depth)` (0 anchor, 1 reachable + `verify`, 2 whole arena),
`bw_counts(out,cap)` → 18 `i64` cells `records chained legacy redacted broken seals forks heads keys genesis
checked holds segments cut cut_redacted declared keys_disagree unsealed`, cells numbered from 0 like every cell in this text and in
this order, which is not the printed line's (cell 9, `genesis`, was named `admitted`; cells 16 and
17 were appended, so older cells keep their places), `bw_anchors(out,cap)` → rows `kind8 ‖ over32 ‖ time_ms8 ‖
proof_len8` (present, not verified), `bw_spec(out,cap)` → `abi8 ‖ sha256 32 ‖ format text`, `bw_status_text(code)`,
`bw_unchecked_text`. A buffer too small gets the needed length and nothing is written; a refusal is `-status`.
Other ABI statuses: 70 `NOT_OPEN`, 71 `NOT_FOUND`, 73 `OUT_OF_RANGE`, 74 `NOT_A_DOC`.

## 10. Constants and limits

A limit refuses **at the bound + 1** and accepts at the bound.

| Name | Value | Where / what |
|---|---|---|
| `MAGIC` | 3554557610294396226 | "BEBOPST1" LE |
| `FORMAT_VERSION` | 2 | cell 1 |
| `SB_A` | 0 | superblock A |
| `SB_B` | 512 | superblock B = byte 4096 |
| `SB_CELLS` | 16 | cells in a superblock |
| `ARENA` | 1024 | first arena cell = byte 8192 |
| `MIN_BYTES` | 8200 | `(ARENA + 1) · 8`: the superblocks + one arena cell |
| `PARTTAB_CELLS` | 19 | PartTab payload, `16 + 3·1` |
| `PARTTAB_OBJ_CELLS` | 21 | with its header |
| `PT_PAGE_OFFSET` | 16 | `pt = (512 − sb) + 16` |
| `PT_ROOT` | 18 | `pt + 18` = root |
| `PT_CURSOR` | 19 | `pt + 19` = cursor |
| `PT_GEN` | 20 | `pt + 20` = generation |
| `SB_ANC` | 9 | anchor cell |
| `MAX_OBJ_CELLS` | 4294967295 | `len` field of h0 (32 bits) |
| `MAX_GENERATION` | 4294967295 | h1 holds gen mod 2³²; a commit that would pass it is refused |
| `MAX_ANCHOR_MARK` | 4294967295 | `mark` field of the anchor; past it `anc` = 0 |
| `MAX_KEY_BYTES` | 65535 | KV writers refuse above |
| `MAX_VALUE_BYTES` | 1073741824 | 1 GiB |
| `MAX_PAD_CELLS` | 67108864 | 512 MiB: the default writer ceiling, and nothing else (§8); no reader pads |
| `MAX_DELTAS` | 32 | longest chain before compaction |
| `DEAD_DIV` | 4 | compaction at `superseded · 4 > live` |
| `DIGEST_KV_ROOT` | 610082063 | `st_digest` of the KV root |
| `DIGEST_ARR_I64` | 4290599237 | `st_digest("arr i64")` |
| `VERSION_TREE` | 4 | version of a KV v4 root (§9.1) |
| `DIGEST_KV4_LEAF` | 284439384 | v4 leaf |
| `DIGEST_KV4_NODE` | 3702325768 | v4 node |
| `DIGEST_KV4_VAL` | 44497123 | v4 value |
| `MAX_TREE_DEPTH` | 16 | `DEPTH` of a v4 root; more is refused |
| `FANOUT` | 32 | the WRITER's v4 fanout (a reader takes `m` from the node), allowed 4..=65535 |
| `DIGEST_OUT` | 514297014 | kinds 1/2 (§9.3) |
| `DIGEST_OUTB` | 4157677073 | kind 3 (§9.3) |
| `DIGEST_OUTZ` | 85289679 | kind 4 (§9.3) |
| `MAX_WIDTH` | 64 | widest OUTZ row |

Bounds not tested, stated: `MAX_PAD_CELLS` at the bound itself, `MAX_VALUE_BYTES` at its real value (checked by
length), `MAX_WIDTH` at 64/65, and an image above 512 MiB through a file is **not measured**.

**Layout strings** of every digest not spelled out above (§5, `st_digest`):

| Digest | String |
|---|---|
| `DIGEST_KV_ROOT` | `KV{i64,ref [i64],ref [i64],ref [i64],ref [i64]}` |
| `DIGEST_ARR_I64` | `arr i64` |
| `DIGEST_KV4_LEAF` | `KV4L{i64,[i64]}` |
| `DIGEST_KV4_NODE` | `KV4N{i64,[i64]}` |
| `DIGEST_KV4_VAL` | `KV4V{i64,[i64]}` |
| `DIGEST_OUT` | `OUT{i64,i64}` |
| `DIGEST_OUTB` | `OUTB{i64,i64,i64,[i64]}` |
| `DIGEST_OUTZ` | `OUTZ{i64,i64,i64,i64,[i64]}` |

## 11. Divergences

Recorded, because a reader of old material meets them:

| # | Was | Now |
|---|---|---|
| D1 | cells 10/11 read as if the Rust writer wrote them | only the bebop language writer does. Rust writes 0 and does **not** carry them: a Rust commit over an image with that chain zeroes it, on purpose (a carried ref would name the wrong generation) |
| D2 | the Rust writer wrote `anc` = 0 | it writes the anchor (§7); old images with 0 are read by all |
| D4 | `create` had no fsync | file and directory are fsynced; a kill-anywhere test for the Rust path **still does not exist** |
| D6 | keys compared as bytes in one reader and decoded lossily as UTF-8 in another | **recorded, not changed**: on a non-UTF-8 key two readers would diverge. No writer writes one (keys are `&str`) |
| D7 | the 2³² generation limit was unwritten | `MAX_GENERATION`, refused at commit |
| D8 | a reader did not check the PartTab crc | every reader does (§3.1) |

Other points: the PartTab digest is 0 in Rust-created images and `st_digest("PartTab")` from the bebop language
writer (§4). The name `Superblock.root` in the Rust API is cell 3, the **PartTab** index, not the data root.

## 12. Can a third party implement a reader from this text (closed 2026-10-31)

Seven gaps were listed on 2026-10-31 and are closed: language (this file), outside references (§9.1-§9.5 carry their rules, §9.0
the status table, pointers are informative), the fixtures as a contract (§13), the writer (§8), `hc` (§9.1), the tombstone mark (§9.4), `actor_id` (§9.4).

Open, each true today:

1. **A v1-v3 TREE in a DOC.** A reader MAY refuse one with 2 (§9.4), and the reference reader does. Nothing writes a TREE. No fixture.
2. **Status of a DOC failure.** Status 2 here is named NOT_A_KV for a DOC part; the name is historical.
3. **Cells 13-14.** One reader refuses them, the others ignore them (§3.1). Making that one rule is a code change.

## 13. Conformance set

These fixtures are part of the contract: `crates/bebop-wasm/fixtures/<name>.store`, in the reference repository, with
`<name>.expected` beside them; the path is informative, the `sha256` column and the values here are the contract. A conforming **KV reader** MUST reach the `status`, `n` and `root` of every `kv` row.
A conforming **log reader** MUST reach them on rows of family `log`, a **DOC reader** on `doc`; a reader that does not
read a family MUST refuse by name rather than answer. `sha256` identifies the file. The other files of that directory
(`key`, `verify`, `decide`, `frozen`) belong to other layers.

| Fixture | Family | Bytes | status | n | root | sha256 (first 16 hex) |
|---|---|---|---|---|---|---|
| `empty` | kv | 8192 | 2 | - | - | `256a4772f6d04064` |
| `kv` | kv | 9360 | 0 | 5 | -211109995167561145 | `322b2eafebfb351c` |
| `kv0` | kv | 8352 | 0 | 0 | -3750763034362895579 | `3c9c800f4d56a079` |
| `kv2` | kv | 8736 | 0 | 5 | -211109995167561145 | `538253621e2ce292` |
| `kv3` | kv | 9200 | 0 | 5 | -4092201869538170098 | `82a4f54ce9acfd35` |
| `kv3c` | kv | 9688 | 0 | 27 | -6264351955216461874 | `e53ca317c93bf13a` |
| `kv3max` | kv | 13504 | 0 | 27 | -6264351955216461874 | `cbd63a520cc97f6f` |
| `kv4zero` | kv | 8272 | 0 | 0 | -3750763034362895579 | `5e0063bf5aa54379` |
| `kv4small` | kv | 9952 | 0 | 27 | -6264351955216461874 | `827c78b2b3872a24` |
| `kv4deep` | kv | 34872 | 0 | 150 | 7496409543980330172 | `cc366299ccfa733b` |
| `kv4del` | kv | 46224 | 0 | 38 | -1005982070270437490 | `58038f20900dc70a` |
| `kv4stub` | kv | 9200 | 2 | - | - | `f4029ad6c08f1ef2` |
| `kv4badd` | kv | 34872 | 2 | - | - | `16bea70cba0630dd` |
| `kv4badf` | kv | 34872 | 2 | - | - | `bda465907187fb6d` |
| `kv4badn` | kv | 34872 | 10 | - | - | `0e8ea2d6c5c881de` |
| `kv4badv` | kv | 34872 | 10 | - | - | `9bd4c462892453f5` |
| `kv4old` | kv | 34872 | 10 | - | - | `656c6caf2f98331a` |
| `kv4stale` | kv | 34872 | 41 | - | - | `c71e6bc8f461cc95` |
| `ptaway` | kv | 9200 | 1 | - | - | `16ccf3c6e093617b` |
| `ptbad` | kv | 9688 | 1 | - | - | `9f9c144f67ed17fd` |
| `torn` | kv | 13504 | 0 | 26 | -5357977772297964327 | `cfba168ebba053da` |
| `proj4` | kv | 9800 | 0 | 5 | -444871924492142954 | `37a2a0c84e6828fb` |
| `proj` | log | 13360 | 0 | 9 | 6344481284825166345 | `b40e2519e0776f3b` |
| `proj4.log` | log | 13424 | 0 | 9 | 6344481284825166345 | `cd888900a0832779` |
| `doc` | doc | 10376 | 0 | 3 | -5898116667070633908 | `c0a6fa19a88e7634` |
| `anch` | doc | 37072 | 0 | 4 | 4203703982444439664 | `d701979a9deb537f` |
| `tomb-sha` | doc | 10640 | 0 | 4 | 2912520156778742020 | `b7e346a7b74fba10` |
| `tomb-legacy` | doc | 10592 | 0 | 4 | 2385174699030489450 | `c47fb175735ed2fe` |
| `seg2` | doc | 10624 | 0 | 3 | 2237665766422822682 | `f53ea34a266ef8c2` |
| `seg2-forged` | doc | 10624 | 75 | 3 | 2237665766422822682 | `e2e4c81f72d8aa41` |
| `forged-tail` | doc | 10768 | 0 | 4 | 9058683980081876988 | `fb6741a5acb5cc04` |
| `keys-disagree` | doc | 12248 | 0 | 5 | 2233471922488689952 | `4965ea8f78fae881` |
| `cut-undeclared` | doc | 9680 | 75 | 2 | 936859044395617976 | `db5094856033cfd9` |
| `log-v1` | log | 8976 | 0 | 3 | 5846225103546468352 | `f1a4ea0cf755c6ba` |
| `log-v3` | log | 8552 | 12 | - | - | `3f7bf288a84c3e96` |
| `kv-unsorted` | kv | 8400 | 2 | - | - | `6ce0aeb177b618d9` |

`seg2-forged` and `cut-undeclared` read as a DOC, status 0 with the `n` and `root` above, and the status in their row is that of `verify`: 75. `doc_verify=` in each `.expected` is the exact counter line of the others. `log-v3` and `kv-unsorted` are refused at the read.

`doc` as a DOC verifier, with no signature algorithm, MUST print exactly: `records=3 chained=0 legacy=0 redacted=0 broken=3 forks=0 seals=3 checked=0 heads=1 keys=1 genesis=1 segments=0 cut=0 cut_redacted=0 declared=0 keys_disagree=0 unsealed=0`. This fixture is
written with a test-only id mix in place of sha256, which this text does not define and no other file uses, so its ids answer neither `chained` nor
`legacy`. A reader MUST count those three records **broken** and MUST NOT look for a scheme under which they hold: the conformance
value is `broken=3`. A KV-only reader answers `doc` with status 12.

`anch` is a real sealed hub, cut once, and MUST print exactly: `records=4 chained=4 legacy=0 redacted=0 broken=0 forks=0 seals=4 checked=0 heads=1 keys=1 genesis=1 segments=1 cut=9 cut_redacted=2 declared=2 keys_disagree=0 unsealed=0`.

The memo lines bind only a reader that claims to read memos. Memos: `proj` kind 1, value 6344481284825166345; `proj4.log` kind 3, value -9190210101691969827, `root` = FNV-1a 64 over the LogSummary
derived from the log; `proj4` kind 4, value 8768581835520683532, `root` = the Z-set digest derived from the replayed entries. In all
three a reader that derives `root` from the input MUST find it equal to the memo.

## 14. Limits: what one file cannot decide

Nothing above decides these, and a reader MUST NOT claim that it does.

- **Trust.** Under `Trust::Genesis` a good result means "signed by the keys this file names as its roots" and nothing more: the
  roots are in a table nobody signs. Who signed is known only under `Trust::Only`, from roots a verifier brings (§9.4).
- **Rollback.** The file as it stood before a REVOKE, or a revoker's newest records dropped together with their seals and HEADS
  row, is a shorter true file. One file cannot know a newer one exists. `unsealed` catches a HEADS row dropped alone,
  never a tail removed with its seals and its row. Level 1 anchors (an anchor over a dropped record names nothing) or a comparison
  with a second file are what see it.
- **A stolen root.** Roots have no rank between them and nothing outranks one, so the thief of a root can tombstone or cut a REVOKE
  and sign the declaration. A witness, or the audit of the archive that holds the record under the root the TRUNCATE names, sees that.
  What a file does hold, under `Trust::Only`: a thief with an admitted, non-root key cannot hide a REVOKE inside it, because a tombstone
  stops at `tombs_foreign`, and a cut through a REVOKE takes that key's older ADMIT with it, so its TRUNCATE declares nothing.
- **A blind reader** counts; it does not check. `checked` = 0 and `holds` is false, whatever else it prints.
- **Removing an anchor.** Nobody signs the ANCH chain: DOC.ANCH = 0 gives a file with no anchors and no trace. Anchors prove when present; their absence says nothing.
- **Swapping files of one owner.** No seal names the file, so two hubs of one key both hold. Which file this is, is a question for whoever brings the root (first id, tip, anchor). A file id in the first seal would not help: it helps only if the verifier brings the expected id, and that verifier can already bring the journal's first id (after a cut, `root`).
- **Order across devices.** Nobody signs it: `prev` links only a device's own records and a device's first record; the fold reads file order.
- **Legacy records.** Id v2 does not bind them: actor, seq and `FLAG_NONCE_DERIVED` of an FNV record are free, and FNV preimages are untested. Live data is all legacy, so a `seal_hub` of a real venue is exposed on its pre-seal records until those records leave the file.
- **A tombstone's bytes under a cut.** `root` names ids and a tombstone keeps its id, so those bytes are bound only by a FORGOTTEN still in the file; the archive shows them (§9.4).
- **Time.** `claimed_ms` is a claim; only an anchor's proof bounds time, and only with a verifier that holds what the proof needs.
