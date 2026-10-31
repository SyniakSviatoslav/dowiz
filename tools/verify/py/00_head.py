#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""bebop-verify (Python half): ONE FILE, STANDARD LIBRARY ONLY, no repo, no wasm, no C.

    python3 bebop-verify.py <image> [--dump] [--verify]

Prints ONE line, the line every reader of docs/STATUS-CODES.md prints --
    kv status=<s> n=<entries> root=<fold>     log status=<s> n=<records> root=<fold>
    doc status=<s> n=<records> root=<log fold>                  (W-MG-M5, FORMAT §9.4)
-- exits with the status (0 = read), and says the status in words on stderr. `--dump` prints
every entry (key order) or every record (oldest first; a DOC's heads first). `--verify` prints
a DOC's counters on stdout:
    verify records= chained= legacy= redacted= broken= forks= seals= checked= heads= keys= admitted=

WHAT IT CHECKS, in the library's order (docs/FORMAT-BEBOP-IMAGE-v2.md; bebop-image
`pick_checked_in` §3.1 -> 1 and `last_commit_in` §7 -> 10 / 41; then the root's family: bebop-kv
`Kv::load_checked` v1-v3 and `TreeIn::verify` v4 -- every reached object's crc -> 10, any claim or
rule that does not hold -> 2, a version above 4 that is not a DOC -> 12; bebop-log `EvLog::chain_crc`
-> 10, the count -> 4; bebop-log `Doc::open` + `doc::verify` §9.4 -- the root, its log, HEADS, KEYS,
SPEC crc -> 10, then the counters). A green line says the bytes are whole and the layout holds; it
is not a proof that the content is true. A DOC's seals are COUNTED, never checked: this file has
no signature algorithm (`checked=0`). This is a THIRD implementation beside the Rust library (the
wasm half) and crates/bebop-wasm/oracle.py, sharing code with neither; conformance.sh and
tools/verify/gate.sh hold all of them to one answer on every fixture.

GENERATED: tools/verify/assemble.sh concatenates tools/verify/py/*.py into this one file so the
deliverable stays a single download and each source part stays under 300 lines. Edit the parts.
"""
import hashlib
import struct
import sys
import zlib

MAGIC = 3554557610294396226
M32, M64 = 0xFFFFFFFF, 0xFFFFFFFFFFFFFFFF
DIGEST_KV_ROOT, DIGEST_EVLOG_ROOT = 610082063, 0x45564C47
DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL = 284439384, 3702325768, 44497123
# FORMAT §9.4 / §9.5 (W-MG-M1, M3, M1B, M5): the DOC with cell 9 = ref SEG (digest 2619226223; 3669646107 was
# the M1 layout with cell 9 reserved, of which no image exists).
DIGEST_DOC, ROOT_VERSION_DOC, DOC_CELLS = 2619226223, 5, 10
DIGEST_SEAL, DIGEST_HEADS, DIGEST_KEY, DIGEST_KEYS = 1951851249, 4237402659, 2540354000, 1448248784
# SEG rows carry `scheme` since W-MG-M3b (12 + 10m cells, digest 2192499951; 3288852188 was the 9-cell row).
DIGEST_SEG, DIGEST_ANCH, DIGEST_SPEC, DIGEST_BLOB = 2192499951, 180762881, 849619745, 799774958
SEG_HEAD, SEG_ROW, SCHEME_CHAINED = 12, 10, 1
TRUNCATE_MARK, FORGOTTEN_MARK = b"bebop-truncate/1", b"bebop-forgotten/1"
FNV_OFFSET, FNV_PRIME = 0xCBF29CE484222325, 0x100000001B3
MAX_VALUE_BYTES = 1 << 30
MAX_SIG_BYTES = MAX_PK_BYTES = 65536
MAX_BLOB_BYTES = 16 << 20
FLAG_SHA256, NONCE_LEN = 2, 16
WORDS = {0: "OK", 1: "NO_SUPERBLOCK: no valid superblock, or the live one refused structurally",
         2: "NOT_A_KV: no root, or a KV claim that does not hold, or a DOC whose SPEC or v1-v3 TREE this reader does not read",
         4: "TRUNCATED: the log's chain does not deliver the root's count",
         10: "BAD_CRC: an object the read reached fails its crc", 12: "UNKNOWN_VERSION: the KV root names a version above 4 and is not a DOC",
         41: "LAST_COMMIT: the last commit does not hash to its anchor and every object in it checks",
         75: "CUT_UNDECLARED: a log segment left this file (a SEG under the DOC) and no TRUNCATE record on the kept chain names it by root and range -- records were dropped without the cut being declared"}
UNCHECKED = ("seals COUNTED, not CHECKED: this reader carries no signature algorithm (stdlib, no ML-DSA); "
             "a count is not a proof -- bring a verifier, or the format text's independent readers")


class Refuse(Exception):
    """The status, as `args[0]`."""


def fnv(h, data):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & M64
    return h


def to_i64(u):
    return u - (1 << 64) if u >= 1 << 63 else u


def lossy(kb):
    return kb.decode("utf-8", "replace").encode("utf-8")


class Image:
    """The cells; 0 past the end (FORMAT §1). Every claim read out of a cell is bounded here."""

    def __init__(self, raw):
        self.n = len(raw) // 8
        self.b = raw[: self.n * 8]

    def cell(self, i):
        return struct.unpack_from("<q", self.b, 8 * i)[0] if 0 <= i < self.n else 0

    def crc(self, off, cnt):
        inside = max(0, min(cnt, self.n - off)) if off < self.n else 0
        return zlib.crc32(self.b[8 * off: 8 * (off + inside)] + bytes(8 * (cnt - inside)))

    def obj_len(self, o):
        return self.cell(o) & M32

    def obj_cells(self, o):
        return max(0, min(self.obj_len(o), self.n - (o + 2)))

    def digest(self, o):
        return (self.cell(o) & M64) >> 32

    def crc_ok(self, o):
        ln = self.obj_len(o)
        return o + 2 + ln <= self.n and self.crc(o + 2, ln) == (self.cell(o + 1) & M64) >> 32

    def need_crc(self, o):
        if not self.crc_ok(o):
            raise Refuse(10)

    def get(self, o, i):
        return self.cell(o + 2 + i)

    def follow(self, o, i):
        off = self.get(o, i)
        return None if off == 0 or o + off < 0 or o + off + 2 > self.n else o + off

    def back(self, o, i):
        """A ref that must point strictly backward (v4 rule 1)."""
        off = self.get(o, i)
        if off >= 0 or o + off < 0:
            raise Refuse(2)
        return o + off

    def bytes_at(self, at, ln):
        if at + ln > len(self.b):
            raise Refuse(2)
        return self.b[at: at + ln]

    def packed(self, o, at, ln):
        """`ln` bytes packed eight to a cell from payload cell `at` of `o`, bounded by the object."""
        have = max(0, min((ln + 7) // 8, self.obj_cells(o) - at))
        return self.b[(o + 2 + at) * 8: (o + 2 + at + have) * 8][:ln]

    def id_at(self, o, at):
        """32 bytes as four cells (zero where the object is short)."""
        return self.packed(o, at, 32).ljust(32, b"\0")
