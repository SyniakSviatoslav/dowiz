#!/usr/bin/env python3
"""Exact mirror of selfhost collect_fns: scan for 'fn <alpha>' outside
strings/comments; returns ordered fn-name list. Asserts alignment with the
OFF table of a compilewords artifact."""
import sys

def mirror(src):
    s = src
    n = len(s)
    j = 0
    names = []
    while j + 2 < n:
        c0, c1, c2 = ord(s[j]), ord(s[j+1]), ord(s[j+2])
        is_quote = c0 == 34
        is_comment = c0 == 47 and c1 == 47
        cafter = ord(s[j+3]) if j+3 < n else 0
        # compiler's is_alpha: ONLY lowercase a-z (97..122)
        is_fn = (c0 == 102 and c1 == 110 and c2 == 32 and 97 <= cafter <= 122)
        if is_fn:
            k = j + 3
            h = 0
            while k < n:
                c = ord(s[k])
                if (65<=c<=90) or (97<=c<=122) or c==95 or (48<=c<=57):
                    h = (h*131 + c) % (1<<64); k += 1
                else:
                    break
            # recover text name by rescanning ident chars
            k2 = j + 3
            while k2 < n:
                c = s[k2]
                if c.isalnum() or c == '_': k2 += 1
                else: break
            names.append((s[j+3:k2], h))
            j = j + 3 + (k2 - (j+3))
            continue
        if is_quote:
            k = j + 1
            while k < n and ord(s[k]) != 34:
                k += 1
            j = k + 1
            continue
        if is_comment:
            while j < n and ord(s[j]) != 10:
                j += 1
            continue
        j += 1
    return names

def parse_off(path):
    for line in open(path):
        if line.startswith('OFF'):
            parts = line.split()
            return [int(x) for x in parts[2:]]
    return None

if __name__ == '__main__':
    src = open(sys.argv[1]).read()
    art = sys.argv[2]
    want = sys.argv[3] if len(sys.argv) > 3 else None
    names = mirror(src)
    offs = parse_off(art)
    assert offs is not None, "no OFF line in artifact"
    print(f"names={len(names)} offs={len(offs)}")
    assert len(names) == len(offs), f"PAIRED-COUNT MISMATCH {len(names)} vs {len(offs)}"
    idx = [i for i, (nm, _) in enumerate(names) if nm == want]
    assert len(idx) == 1, f"'{want}' found {len(idx)} times"
    print(f"{want}: index={idx[0]} word_off={offs[idx[0]]} byte_off={offs[idx[0]]*4}")
