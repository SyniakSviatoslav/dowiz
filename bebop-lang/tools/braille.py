#!/usr/bin/env python3
"""Bebop Braille surface (T84, braille revision).

The canonical source character is a Unicode Braille Pattern (U+2800-U+28FF).
A cell is literally a 2x4 dot bitmap -- the delta-outline on a pixel grid that
BEBOP-GLYPH-ALPHABET.md's law demands -- and the block is a contiguous 256-cell
bijection over the byte range, all of it East-Asian width 1.

Two layers, both total and both lossless:
  --text  : the READABLE surface. Standard Grade-1 braille letters for a-z,
            dot7 for capitals, dot8 for digits, a fixed table for punctuation.
  --bytes : the SUBSTRATE. byte b -> U+2800+b. Bijective by construction;
            round-trip is a theorem, not a test.
Newline passes through unchanged in both, so line structure survives.
"""
import sys

B = 0x2800
def cell(v): return chr(B + v)

# Standard Grade-1 braille letters (200 years old; not invented here).
LETTER = dict(zip("abcdefghijklmnopqrstuvwxyz",
  [0x01,0x03,0x09,0x19,0x11,0x0B,0x1B,0x13,0x0A,0x1A,0x05,0x07,0x0D,
   0x1D,0x15,0x0F,0x1F,0x17,0x0E,0x1E,0x25,0x27,0x3A,0x2D,0x3D,0x35]))
DOT7, DOT8 = 0x40, 0x80

TEXT = {}
for ch, v in LETTER.items():
    TEXT[ch] = v                      # a-z
    TEXT[ch.upper()] = v | DOT7       # A-Z  (dot7 = capital)
for d, ch in enumerate("jabcdefghi"): # 0-9  (dot8 = digit)
    TEXT[str(d)] = LETTER[ch] | DOT8
TEXT.update({
  ' ':0x00, '_':0x38,
  '(':0x23, ')':0x1C, '{':0x63, '}':0x5C, '[':0xA3, ']':0x9C,
  ',':0x02, ';':0x06, ':':0x12, '.':0x32, '"':0x26, "'":0x04,
  '=':0x36, '+':0x16, '-':0x24, '*':0x21, '/':0x0C, '%':0x29,
  '!':0x2E, '<':0x22, '>':0x28, '&':0x2F, '|':0x33, '^':0x18,
})
BACK = {v: k for k, v in TEXT.items()}
assert len(BACK) == len(TEXT), "TEXT layer is not injective"

def enc(s, mode):
    out = []
    for ch in s:
        if ch == '\n': out.append('\n'); continue
        if mode == 'bytes': out.append(cell(ord(ch) & 0xFF)); continue
        if ch not in TEXT: raise SystemExit(f"no braille cell for {ch!r}")
        out.append(cell(TEXT[ch]))
    return ''.join(out)

def dec(s, mode):
    out = []
    for ch in s:
        if ch == '\n': out.append('\n'); continue
        v = ord(ch) - B
        if not 0 <= v < 256: raise SystemExit(f"not a braille cell: {ch!r}")
        out.append(chr(v) if mode == 'bytes' else BACK[v])
    return ''.join(out)

def art(v):
    """Render one cell as its 2x4 dot grid."""
    d = [(v>>i)&1 for i in range(8)]      # dot1..dot8
    rows = [(d[0],d[3]), (d[1],d[4]), (d[2],d[5]), (d[6],d[7])]
    return ["".join('#' if x else '.' for x in r) for r in rows]

if __name__ == "__main__":
    a = sys.argv[1:]
    mode = 'bytes' if '--bytes' in a else 'text'
    op   = dec if '--decode' in a else enc
    path = [x for x in a if not x.startswith('-')]
    sys.stdout.write(op(open(path[0]).read() if path else sys.stdin.read(), mode))
