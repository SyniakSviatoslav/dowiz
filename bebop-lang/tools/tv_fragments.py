#!/usr/bin/env python3
"""tv_fragments.py -- F5 translation-validation gate (ROADMAP.md row F5).

STATUS: NOT IMPLEMENTED. This script is the blueprint's step 0
(docs/blueprints/F4-fragment-validation.md, section 5): it refuses to
report a number it has not measured. It has NO rc=0 path and never prints
the word PASS.

What it does:
  1. loads <a.bin> the way tools/check_abi.py:62 does (LE64 entry footer,
     code_end = word after the last `ret`) -- a malformed file is FAIL, rc=1;
  2. looks for the trace SIDE FILE <a>.trace next to <a.bin> (blueprint
     section 4.1: a side file like `<out>.use`, NOT a zone inside the .bin --
     the .bin's md5 must stay unchanged by the trace);
  3. reports:
       absent  -> tv_fragments: NOT MEASURED (no trace ...)         rc=1
       present -> tv_fragments: NOT MEASURED (trace present ...)    rc=1
     because the validator (blueprint step 3) does not exist. Reading the
     trace is deferred until step 1 fixes its format; a reader written
     before the writer would be written twice.

History: the previous contents (committed 4286ec1) scanned the .bin for a
marker word 0xF5F5F5F5 that bebop.bp never emits, printed `PASS (0/0)` when
it found nothing, "validated" fragments by recomputing an FNV digest of the
words it had just read (a check that cannot fail), and carried an AArch64
decoder that agreed with objdump on 48/308 of the emitter's base words
(ret->b, ldr->eor_reg, cmp->str_imm, brk->b, movz->add_imm). Measured and
removed 2026-09-13 (lane tvfrag). The decoder is NOT to be revived as a
bit-field decoder: the design (section 4.4) is an objdump-verified inventory
table over the base words, and aarch64-linux-gnu-objdump is on the box.

Usage: python3 tools/tv_fragments.py <a.bin>
Exit: 1 always, until step 3 lands (there is nothing to pass yet).
"""
import hashlib, os, struct, sys

BLUEPRINT = "docs/blueprints/F4-fragment-validation.md"


def load_bin(path):
    """-> (n_words, entry_word, code_end). Same rule as tools/check_abi.py:62."""
    b = open(path, "rb").read()
    if len(b) < 16 or len(b) % 4:
        raise ValueError(f"size {len(b)} (need >= 16 and a multiple of 4)")
    e = struct.unpack("<Q", b[-8:])[0]
    W = struct.unpack(f"<{(len(b) - 8) // 4}I", b[:-8])
    if e % 4 or e // 4 >= len(W):
        raise ValueError(f"entry byte offset {e} outside code ({len(W)} words)")
    rets = [i for i, w in enumerate(W) if w == 0xD65F03C0]
    if not rets:
        raise ValueError("no ret word")
    return len(W), e // 4, rets[-1] + 1, hashlib.md5(b).hexdigest()[:8]


def trace_path(bin_path):
    """<out>.trace next to <out>.bin (blueprint 4.1; `<out>.use` precedent, bebop.bp:7365)."""
    root, ext = os.path.splitext(bin_path)
    return (root if ext == ".bin" else bin_path) + ".trace"


def main(argv):
    if len(argv) != 2:
        print("usage: tv_fragments.py <a.bin>   (see " + BLUEPRINT + ")", file=sys.stderr)
        return 2
    bin_path = argv[1]
    try:
        n_words, entry, code_end, md5 = load_bin(bin_path)
    except (OSError, ValueError) as e:
        print(f"tv_fragments FAIL: {bin_path}: {e}")
        return 1

    tp = trace_path(bin_path)
    ident = f"{os.path.basename(bin_path)} md5 {md5}, {n_words} words, code_end {code_end}"
    if not os.path.exists(tp):
        print(f"tv_fragments: NOT MEASURED (no trace: {tp} absent; the emitter writes "
              f"no trace yet -- {BLUEPRINT} step 1) [{ident}]")
        return 1
    size = os.path.getsize(tp)
    print(f"tv_fragments: NOT MEASURED (trace present: {tp}, {size} bytes; validator "
          f"not implemented -- {BLUEPRINT} step 3; 0 of ? fragments validated) [{ident}]")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
