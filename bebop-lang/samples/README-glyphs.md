# Bebop Glyphic Surface: Hello, Fibonacci, Sum

## Side-by-Side: Hello World

**ASCII (samples/hello.bp)**
```bebop
fn main() -> i64 {
  let buf = zeros(16);
  let n = str_len("Hello, world!");
  let i = 0;
  while i < n {
    let _ = buf[i] = char("Hello, world!", i);
    let i = i + 1;
    0
  };
  let _ = buf[n] = 10;
  let _ = sys_write(1, buf, n + 1);
  0
}
```

**Glyph (samples/hello_glyph.bp)**
```bebop
★ main() → ℤ {
  ◐ buf = zeros(16);
  ◐ n = str_len("Hello, world!");
  ◐ i = 0;
  ↺ i < n {
    ◐ _ = buf[i] = char("Hello, world!", i);
    ◐ i = i + 1;
    0
  };
  ◐ _ = buf[n] = 10;
  ◐ _ = sys_write(1, buf, n + 1);
  0
}
```

## Glyphs Used Across All Three Samples

| Glyph | ASCII | Category | Meaning |
|-------|-------|----------|---------|
| ★ | fn | decl | function declaration |
| ◐ | let | stmt | variable binding |
| ↺ | while | stmt | while loop |
| ℤ | i64 | type | 64-bit integer (Bebop's only numeric type) |
| → | -> | arrow | return type annotation |

## Verification Commands

All three samples compile and run identically from ASCII and glyph surfaces.

### Compile
```bash
bash tools/cc.sh ./bebop.bin samples/hello.bp /tmp/hello.bin
bash tools/cc.sh ./bebop.bin samples/fib.bp /tmp/fib.bin
bash tools/cc.sh ./bebop.bin samples/sum.bp /tmp/sum.bin
```

### Run
```bash
./seed/build/seed /tmp/hello.bin  # Prints "Hello, world!" then "0"
./seed/build/seed /tmp/fib.bin    # Prints "832040" (fib(30))
./seed/build/seed /tmp/sum.bin    # Prints "15" (sum 1..5)
```

### Round-Trip Proof (Glyph → ASCII identity)
```bash
python3 tools/glyphfmt.py --ascii samples/hello_glyph.bp | cmp -s - samples/hello.bp && echo "RT_OK_hello"
python3 tools/glyphfmt.py --ascii samples/fib_glyph.bp | cmp -s - samples/fib.bp && echo "RT_OK_fib"
python3 tools/glyphfmt.py --ascii samples/sum_glyph.bp | cmp -s - samples/sum.bp && echo "RT_OK_sum"
```

### Binary Identity Proof (ASCII and glyph compile to the same bytes)
```bash
for n in hello fib sum; do
  python3 tools/glyphfmt.py --ascii samples/${n}_glyph.bp > /tmp/$n.back.bp
  bash tools/cc.sh ./bebop.bin samples/$n.bp      /tmp/$n.a.bin
  bash tools/cc.sh ./bebop.bin /tmp/$n.back.bp    /tmp/$n.b.bin
  a=$(md5sum < /tmp/$n.a.bin | cut -c1-8); b=$(md5sum < /tmp/$n.b.bin | cut -c1-8)
  [ "$a" = "$b" ] && echo "BINEQ $n $a $b SAME" || echo "BINEQ $n $a $b DIFF"
done
```
Measured 2026-09-13 on the promoted compiler: `hello a706a240 SAME`, `fib eefe8b65 SAME`,
`sum d792bc7a SAME`.

## Canonical Surface

**Glyphs are the canonical surface; ASCII is a lossless projection (ROADMAP T84).** Every glyph
twin successfully round-trips through the transliterator (`glyphfmt.py`) and compiles to
byte-identical binaries. The samples demonstrate that the glyphic surface and its ASCII
fallback are two perspectives on the same programs.

*Note:* The terminal characters rendered above are placeholder glyphs pending δ-outline
(vector outline) rendering in a dedicated editor.
