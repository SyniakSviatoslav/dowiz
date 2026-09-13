#!/usr/bin/env python3
"""glyphfmt — bidirectional ASCII<->glyph transliteration for Bebop (T84 G2/G3 draft).
Token-aware: never rewrites inside string literals, char literals or // comments."""
import sys

KW = [("module","◈"),("struct","◇"),("contract","⊙"),("kernel","⎔"),("return","↯"),
      ("while","↺"),("break","⊘"),("match","❖"),("enum","△"),("test","⚗"),
      ("else","⋄"),("then","⊸"),("use","⇐"),("let","◐"),("str","▤"),("i64","ℤ"),
      ("fn","★"),("if","◒"),("in","∴")]
OPS = [("->","→"),(">>>","⋙"),(">>","≫"),(">=","≥"),("<<","≪"),("<=","≤"),
       ("==","≡"),("=>","⇒"),("!=","≠"),("&&","∧"),("||","∨"),
       ("!","¬"),("^","⊻"),("*","×"),("/","÷"),("%","∤")]
IDC = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")

def convert(src, to_glyph=True):
    kw  = KW if to_glyph else [(g,a) for a,g in KW]
    ops = OPS if to_glyph else [(g,a) for a,g in OPS]
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        # --- inert regions: copied byte-for-byte ---
        if c == '"':
            j = i+1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            out.append(src[i:min(j+1,n)]); i = min(j+1,n); continue
        if c == "'":
            j = i+1
            while j < n and src[j] != "'":
                j += 2 if src[j] == '\\' else 1
            out.append(src[i:min(j+1,n)]); i = min(j+1,n); continue
        if src.startswith("//", i):
            j = src.find("\n", i); j = n if j < 0 else j
            out.append(src[i:j]); i = j; continue
        # --- word-boundary keywords ---
        if c in IDC:
            j = i
            while j < n and src[j] in IDC: j += 1
            word = src[i:j]
            hit = next((b for a,b in kw if a == word), None)
            out.append(hit if hit else word); i = j; continue
        # --- non-ident glyph keywords (glyph->ascii direction) ---
        hit = next(((a,b) for a,b in kw if len(a) and src.startswith(a,i) and a[0] not in IDC), None)
        if hit:
            a,b = hit
            nxt = src[i+len(a):i+len(a)+1]
            out.append(b + (" " if (b and b[-1] in IDC and nxt and nxt in IDC) else "")); i += len(a); continue
        # --- operators, longest match first ---
        hit = next(((a,b) for a,b in ops if src.startswith(a,i)), None)
        if hit:
            a,b = hit; out.append(b); i += len(a); continue
        out.append(c); i += 1
    return "".join(out)

if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else "--glyph"
    data = open(sys.argv[2]).read() if len(sys.argv) > 2 else sys.stdin.read()
    sys.stdout.write(convert(data, to_glyph=(mode == "--glyph")))
