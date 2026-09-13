# The Bebop Braille Surface (T84)

The canonical surface of Bebop is **Unicode Braille Patterns** (U+2800-U+28FF), a contiguous 256-cell bijection over the byte range.

## Hello World

| ASCII | Braille |
|-------|---------|
| `// hello.bp (ASCII) and hello.brl (braille) are ONE program in two spellings;` | `⠌⠌⠀⠓⠑⠇⠇⠕⠲⠃⠏⠀⠣⡁⡎⡉⡊⡊⠜⠀⠁⠧...` |
| `fn main() -> i64 {` | `⠋⠝⠀⠍⠁⠊⠝⠣⠜⠀⠤⠨⠀⠊⢋⢙⠀⡣` |

## Dot Numbering (2×4 Grid)

```
  1 4
  2 5
  3 6
  7 8
```

**Bit values by position:**
- dot1 = 0x01, dot4 = 0x08
- dot2 = 0x02, dot5 = 0x10
- dot3 = 0x04, dot6 = 0x20
- dot7 = 0x40, dot8 = 0x80

Cell value is the bitwise OR of the dots present. Example: dot1+dot2+dot4 = 0x01|0x02|0x08 = 0x0B.

## Encoding Rules

**Grade-1 braille letters** (200 years old):
- a-z: Standard braille (26 values, e.g., 'a'=0x01, 'b'=0x03, ..., 'z'=0x35)
- A-Z: Lowercase + dot7 (capital prefix)
- 0-9: digit prefix (dot8) + letter (0='j', 1='a', ..., 9='i')

**Punctuation** fixed table:
- Space: 0x00 | Underscore: 0x38
- Parentheses: (0x23, )0x1C | Brackets: [0xA3, ]0x9C | Braces: {0x63, }0x5C
- Comma: 0x02 | Semicolon: 0x06 | Colon: 0x12 | Period: 0x32
- Plus: 0x16 | Minus: 0x24 | Multiply: 0x21 | Divide: 0x0C | Modulo: 0x29
- Equals: 0x36 | Ampersand: 0x2F | Pipe: 0x33 | Caret: 0x18

**Newline** passes through unchanged; line structure survives.

## Why Braille, Not Glyphs

The retired glyph alphabet (★ ◐ ↺ ∧ ⇒) was replaced due to three measurable facts:

1. **East-Asian Width.** All 256 braille cells are width-1 (U+2800-U+28FF is Ambiguous/Neutral, rendered as width-1 in both Latin and CJK locales). The glyph set had 20 of 35 glyphs marked AMBIGUOUS (width 1 Latin / width 2 CJK), rendering 1 column in ASCII editors and 2 in CJK terminals — incompatible with a checksum ledger.

2. **Collision-free bijection.** Braille is a contiguous 256-cell block that maps byte→cell directly (cell = U+2800 + byte). No hand-checking needed; collisions are impossible by construction. The glyph table needed manual review and had three collisions (★ and two variants assigned the same glyph).

3. **Pixel-perfect bitmap.** A braille cell IS the 2×4 dot bitmap that BEBOP-GLYPH-ALPHABET.md's law demanded as "the delta-outline on a pixel grid". The placeholder is now the thing itself.

## Reproduction

Encode a `.bp` file to braille:
```bash
python3 /root/dowiz/.claude/lanes/glyph-kit/braille.py --text <file.bp> > <file.brl>
```

Decode braille back to ASCII:
```bash
python3 /root/dowiz/.claude/lanes/glyph-kit/braille.py --text --decode <file.brl> > <file.bp>
```

Round-trip verification (all six samples, 2026-09-13):
```bash
# Verify known-good files
cmp samples/hello.brl /root/dowiz/.claude/lanes/glyph-kit/hello.brl  # rc=0
cmp samples/fib.brl /root/dowiz/.claude/lanes/glyph-kit/fib.brl      # rc=0
cmp samples/sum.brl /root/dowiz/.claude/lanes/glyph-kit/sum.brl      # rc=0

# Compile and run ASCII versions
bash tools/cc.sh ./bebop.bin samples/hello.bp /tmp/brlsamp.hello.a.bin
timeout 30 ./seed/build/seed /tmp/brlsamp.hello.a.bin  # Output: Hello, world!

# Decode braille, verify cmp, compile, compare binaries
python3 /root/dowiz/.claude/lanes/glyph-kit/braille.py --text --decode samples/hello.brl > /tmp/hello.decoded.bp
cmp -s /tmp/hello.decoded.bp samples/hello.bp  # rc=0
bash tools/cc.sh ./bebop.bin /tmp/hello.decoded.bp /tmp/hello.decoded.bin
md5sum /tmp/brlsamp.hello.a.bin /tmp/hello.decoded.bin  # Both a706a2409e96bc2d3eb62ddf9a27f7f0
```

**Measured on 2026-09-13:**
- hello: md5 a706a2409e96bc2d3eb62ddf9a27f7f0 (SAME)
- fib: md5 eefe8b65ff1d72fe1bd8880ee2c53e39 (SAME)
- sum: md5 d792bc7a3f471eaefe9c41fec856c7b4 (SAME)
- gcd: md5 801a874c23ef604e4bef5f9f85224429 (SAME)
- collatz: md5 045713acc9a227f00411e996834f672c (SAME)
- bits: md5 a416c8ea402505f14e83e9693e989a97 (SAME)
