#!/usr/bin/env python3
"""Validate BRAILLE-v1.tsv braille surface table."""

import sys
import unicodedata
import os
from pathlib import Path

B = 0x2800

def cell(v):
    """Convert hex value to braille cell."""
    return chr(B + v)

def read_tsv(path):
    """Read the TSV file and return list of rows (as dicts)."""
    rows = []
    with open(path) as f:
        header = f.readline().strip().split('\t')
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split('\t')
            row = dict(zip(header, parts))
            rows.append(row)
    return rows

def dots_to_hex(dots_str):
    """Convert dots string (e.g., '1-2-4') to hex value."""
    if not dots_str:
        return 0x00
    dots = list(map(int, dots_str.split('-')))
    v = 0
    for d in dots:
        if 1 <= d <= 8:
            v |= (1 << (d - 1))
    return v

def hex_to_dots(v):
    """Convert hex value to dots string."""
    dots = []
    for i in range(8):
        if v & (1 << i):
            dots.append(str(i + 1))
    return '-'.join(dots) if dots else ''

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

# Add missing printable ASCII
TEXT.update({
  '#':0x08, '$':0x10, '?':0x14, '@':0x20, '\\':0x2A, '`':0x2B, '~':0x2C,
})

def enc(s):
    """Encode ASCII string to braille."""
    out = []
    for ch in s:
        if ch == '\n': 
            out.append('\n')
            continue
        if ch not in TEXT:
            raise ValueError(f"no braille cell for {ch!r}")
        out.append(cell(TEXT[ch]))
    return ''.join(out)

def dec(s):
    """Decode braille string to ASCII."""
    out = []
    BACK = {v: k for k, v in TEXT.items()}
    for ch in s:
        if ch == '\n':
            out.append('\n')
            continue
        v = ord(ch) - B
        if not 0 <= v < 256:
            raise ValueError(f"not a braille cell: {ch!r}")
        if v not in BACK:
            raise ValueError(f"no text for braille cell 0x{v:02X}")
        out.append(BACK[v])
    return ''.join(out)

def run_checks(tsv_path):
    """Run all 7 checks."""
    rows = read_tsv(tsv_path)
    checks_passed = 0
    checks_failed = 0
    
    # Check 1: injective_cell — no braille cell appears on two rows
    seen_cells = {}
    fails = []
    for i, row in enumerate(rows):
        cell_ch = row['cell']
        if cell_ch in seen_cells:
            fails.append(f"Row {i}: cell {cell_ch!r} also at row {seen_cells[cell_ch]}")
        else:
            seen_cells[cell_ch] = i
    if fails:
        print(f"CHECK injective_cell FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK injective_cell PASS")
        checks_passed += 1
    
    # Check 2: injective_ascii — no ASCII character appears on two rows
    seen_ascii = {}
    fails = []
    for i, row in enumerate(rows):
        ascii_ch = row['ascii']
        # Handle <space> marker
        if ascii_ch == '<space>':
            ascii_ch = ' '
        if ascii_ch in seen_ascii:
            fails.append(f"Row {i}: ascii {ascii_ch!r} also at row {seen_ascii[ascii_ch]}")
        else:
            seen_ascii[ascii_ch] = i
    if fails:
        print(f"CHECK injective_ascii FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK injective_ascii PASS")
        checks_passed += 1
    
    # Check 3: in_block — every cell is in U+2800..U+28FF
    fails = []
    for i, row in enumerate(rows):
        cell_ch = row['cell']
        code = ord(cell_ch)
        if not (0x2800 <= code <= 0x28FF):
            fails.append(f"Row {i}: cell {cell_ch!r} (U+{code:04X}) outside block")
    if fails:
        print(f"CHECK in_block FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK in_block PASS")
        checks_passed += 1
    
    # Check 4: dots_match_hex — dots column converted to hex equals hex column
    fails = []
    for i, row in enumerate(rows):
        dots = row['dots']
        hex_str = row['hex']
        computed_hex = dots_to_hex(dots)
        expected_hex = int(hex_str, 16)
        if computed_hex != expected_hex:
            fails.append(f"Row {i}: dots {dots!r} -> 0x{computed_hex:02X}, expected {hex_str}")
    if fails:
        print(f"CHECK dots_match_hex FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK dots_match_hex PASS")
        checks_passed += 1
    
    # Check 5: width_one — every cell has east_asian_width in ('N','Na','H')
    fails = []
    for i, row in enumerate(rows):
        cell_ch = row['cell']
        width = unicodedata.east_asian_width(cell_ch)
        if width not in ('N', 'Na', 'H'):
            fails.append(f"Row {i}: cell {cell_ch!r} width={width}")
    if fails:
        print(f"CHECK width_one FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK width_one PASS")
        checks_passed += 1
    
    # Check 6: covers_source — split ASCII and non-ASCII coverage
    # Collect all .bp files and extract characters
    source_chars = set()
    bp_dirs = [
        '/root/dowiz/.claude/lanes/brltab/bench/parity_constructs',
        '/root/dowiz/.claude/lanes/brltab/samples',
    ]
    for bp_dir in bp_dirs:
        if os.path.isdir(bp_dir):
            for root, dirs, files in os.walk(bp_dir):
                for f in files:
                    if f.endswith('.bp'):
                        bp_path = os.path.join(root, f)
                        try:
                            with open(bp_path) as fp:
                                content = fp.read()
                                source_chars.update(content)
                        except Exception as e:
                            pass
    
    # Remove newline from the set (it passes through unchanged)
    source_chars.discard('\n')

    # Build the table of cells from the TSV (not the hardcoded TEXT)
    table_ascii = set()
    for row in rows:
        ascii_ch = row['ascii']
        if ascii_ch == '<space>':
            table_ascii.add(' ')
        else:
            table_ascii.add(ascii_ch)

    # Check coverage - split into ASCII and non-ASCII
    uncovered = source_chars - table_ascii
    ascii_uncovered = [ch for ch in uncovered if ord(ch) < 0x80]
    nonascii_uncovered = [ch for ch in uncovered if ord(ch) >= 0x80]
    
    if ascii_uncovered:
        print(f"CHECK covers_source FAIL ascii_uncovered={len(ascii_uncovered)} nonascii_uncovered={len(nonascii_uncovered)}")
        checks_failed += 1
    else:
        print(f"CHECK covers_source PASS ascii_uncovered=0 nonascii_uncovered={len(nonascii_uncovered)}")
        checks_passed += 1
    
    # Check 7: roundtrip_all — encoding and decoding returns same char
    fails = []
    for i, row in enumerate(rows):
        ascii_ch = row['ascii']
        if ascii_ch == '<space>':
            ascii_ch = ' '
        try:
            encoded = enc(ascii_ch)
            decoded = dec(encoded)
            if decoded != ascii_ch:
                fails.append(f"Row {i}: {ascii_ch!r} -> encode -> decode -> {decoded!r}")
        except Exception as e:
            fails.append(f"Row {i}: {ascii_ch!r} raised {e}")
    if fails:
        print(f"CHECK roundtrip_all FAIL {fails[0]}")
        checks_failed += 1
    else:
        print("CHECK roundtrip_all PASS")
        checks_passed += 1
    
    print(f"braille_check: {checks_passed} PASS {checks_failed} FAIL")
    return 0 if checks_failed == 0 else 1

if __name__ == '__main__':
    tsv_path = sys.argv[1] if len(sys.argv) > 1 else '/root/dowiz/.claude/lanes/brltab/docs/design/BRAILLE-v1.tsv'
    rc = run_checks(tsv_path)
    sys.exit(rc)
