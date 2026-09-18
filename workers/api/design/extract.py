#!/usr/bin/env python3
"""Print one Figma frame's layout from the offline document dump.

WHY THIS EXISTS. The Figma MCP server allows 20 reads per month on the Starter
plan and they are spent. One of those reads was `get_metadata` on the page,
which returned the WHOLE document: every node's id, name, type, position and
size, and -- because Figma names a text layer after its content -- every string
on every screen. 16,193 nodes, 3,837 of them text.

So the geometry and the copy for the remaining frames are already here. What the
dump does NOT carry is per-node colour and font. Those come from the design
system instead: the two palettes are exact (lib/figma.css, read from the file's
own "Color" frames) and the component vocabulary was established against eight
full design-context pulls, so a 1e1e1e card and an a8a8a8 subtitle are not
guesses -- they are the values every one of those eight frames used.

Usage:  python3 extract.py 1:7208            # one frame, nested
        python3 extract.py --list            # every top-level frame
        python3 extract.py --find Receipt    # frames whose name matches
"""
import json, sys, re, os

DUMP = os.environ.get('FIGMA_DUMP') or os.path.join(os.path.dirname(__file__), 'document.json')

def load():
    with open(DUMP, encoding='utf-8') as f:
        d = json.load(f)
    return "".join(x.get('text', '') for x in d) if isinstance(d, list) else d

TAG = re.compile(
    r'<(?P<tag>[a-z]+) id="(?P<id>[^"]+)" name="(?P<name>[^"]*)"'
    r'(?: x="(?P<x>-?[\d.]+)" y="(?P<y>-?[\d.]+)")?'
    r'(?: width="(?P<w>[\d.]+)" height="(?P<h>[\d.]+)")?'
    r'(?P<selfclose>\s*/)?>')

def walk(txt, start):
    """Yield (depth, tag, id, name, x, y, w, h) for the subtree rooted at start."""
    i = txt.index(f'id="{start}"')
    i = txt.rindex('<', 0, i)
    depth = 0
    while i < len(txt):
        m = TAG.match(txt, i)
        if m:
            yield depth, m.group('tag'), m.group('id'), m.group('name'), \
                  m.group('x'), m.group('y'), m.group('w'), m.group('h')
            if not m.group('selfclose'):
                depth += 1
            i = m.end()
            continue
        if txt.startswith('</', i):
            depth -= 1
            if depth <= 0:
                return
            i = txt.index('>', i) + 1
            continue
        i += 1

def main():
    txt = load()
    if '--list' in sys.argv:
        for m in re.finditer(r'^  <(?:frame|instance) id="([^"]+)" name="([^"]*)"'
                             r' x="[-\d.]+" y="[-\d.]+" width="([\d.]+)" height="([\d.]+)"',
                             txt, re.M):
            print(f'{m.group(1):<10} {m.group(3):>6}x{m.group(4):<6} {m.group(2)}')
        return
    if '--find' in sys.argv:
        needle = sys.argv[sys.argv.index('--find') + 1].lower()
        for m in re.finditer(r'<(?:frame|instance) id="([^"]+)" name="([^"]*)"', txt):
            if needle in m.group(2).lower():
                print(f'{m.group(1):<12} {m.group(2)}')
        return

    node = sys.argv[1]
    # Coordinates in the dump are relative to the immediate parent, except on a
    # top-level frame, where they are the canvas position and mean nothing here.
    first = True
    for depth, tag, nid, name, x, y, w, h in walk(txt, node):
        pos = ''
        if x is not None and not first:
            pos = f'@{float(x):>5.0f},{float(y):<5.0f}'
        first = False
        size = f'{float(w):>5.0f}x{float(h):<5.0f}' if w else ' ' * 11
        mark = {'text': 'T', 'vector': 'v', 'ellipse': 'o', 'line': '-',
                'star': '*', 'instance': 'I'}.get(tag, ' ')
        print(f'{"  " * depth}{mark} {size} {pos}  {name}')

main()
