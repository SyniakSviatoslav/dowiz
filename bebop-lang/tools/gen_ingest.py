#!/usr/bin/env python3
"""Scaled ingest-twin generator (ROADMAP A7 step 3, 2026-09-09).
100 MB twin deferred (time budget); this builds an N-byte file with the same
shape: lines `id,u,v,cell,label` (label 8-16 ASCII chars), LCG seed 12345.
Usage: python3 tools/gen_ingest.py <bytes> <out path>
"""
import sys

def lcg(st):
    return (st * 1103515245 + 12345) & 0x7fffffff

def main():
    target = int(sys.argv[1])
    out = sys.argv[2]
    st = 12345
    lid = 0
    with open(out, 'w') as f:
        n = 0
        while n < target:
            st = lcg(st); u = st % 100000
            st = lcg(st); v = st % 100000
            st = lcg(st); cell = st % 1000
            st = lcg(st); llen = 8 + st % 9
            lab = []
            for _ in range(llen):
                st = lcg(st); lab.append(chr(97 + st % 26))
            line = '%d,%d,%d,%d,%s\n' % (lid, u, v, cell, ''.join(lab))
            f.write(line)
            n += len(line)
            lid += 1
    print('wrote %d bytes %d lines -> %s' % (n, lid, out))

main()
