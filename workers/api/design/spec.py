#!/usr/bin/env python3
"""Print a complete spec for one Figma frame: geometry, colour, type, radius.

Reads design/file.json -- the whole document pulled once through the REST API
(`GET /v1/files/:key`), which carries what the MCP metadata dump did not: fills,
strokes, effects and typography per node.

Usage:  python3 spec.py 1:7208            # one frame
        python3 spec.py --list            # every top-level frame
        python3 spec.py --find Receipt
        python3 spec.py 1:7208 --max 400  # cap the number of lines
"""
import json, sys, os

HERE = os.path.dirname(os.path.abspath(__file__))
DOC = json.load(open(os.path.join(HERE, 'file.json'), encoding='utf-8'))
PAGE = DOC['document']['children'][0]

# The two palettes, so a hex prints as the token a screen should actually use
# rather than as a number somebody has to look up.
TOKENS = {
    '#EA4F16': 'orange', '#FB5B21': 'orange(light)', '#FCAF23': 'yellow',
    '#121212': 'bg', '#1E1E1E': 'surface', '#2A2A2A': 'surface-2', '#3A3A3A': 'surface-3',
    '#A8A8A8': 'text-2', '#FFFFFF': 'white', '#010101': 'ink', '#F6F6F6': 'bg(light)',
    '#2C2C2C': 'stroke', '#3C3C3C': 'stroke-2', '#E0E0E0': 'icon-strong', '#B0B0B0': 'icon',
    '#8A8A8A': 'grey', '#D9D9D9': 'photo-placeholder',
}

def hexof(c):
    if not c: return None
    r, g, b = (round(c.get(k, 0) * 255) for k in 'rgb')
    return f'#{r:02X}{g:02X}{b:02X}'

def paint(fills):
    out = []
    for f in fills or []:
        if f.get('visible') is False: continue
        t = f.get('type')
        if t == 'SOLID':
            h = hexof(f.get('color'))
            a = f.get('opacity', f.get('color', {}).get('a', 1))
            name = TOKENS.get(h, h)
            out.append(name if a in (1, None) else f'{name}@{a:.2f}')
        elif t and t.startswith('GRADIENT'):
            stops = [hexof(s.get('color')) for s in f.get('gradientStops', [])]
            out.append('grad(' + '→'.join(x for x in stops if x) + ')')
        elif t == 'IMAGE':
            out.append('image')
    return ','.join(out)

def radius(n):
    if n.get('rectangleCornerRadii'):
        vals = n['rectangleCornerRadii']
        return 'r' + ('/'.join(str(round(v)) for v in vals) if len(set(vals)) > 1
                      else str(round(vals[0])))
    r = n.get('cornerRadius')
    return f'r{round(r)}' if r else ''

def typo(n):
    s = n.get('style') or {}
    if not s.get('fontSize'): return ''
    w = s.get('fontWeight', 400)
    lh = s.get('lineHeightPx')
    bits = [f"{round(s['fontSize'])}/{w}"]
    if lh: bits.append(f'lh{round(lh)}')
    if s.get('italic'): bits.append('italic')
    return ' '.join(bits)

def find(node, nid):
    if node.get('id') == nid: return node
    for c in node.get('children', []):
        hit = find(c, nid)
        if hit: return hit
    return None

# Device chrome: the phone's own status bar and home indicator. It is never
# drawn -- a web page does not paint the OS -- so it is pruned rather than
# scrolled past every time.
CHROME = ('status bar', 'homeindicator', 'home indicator', 'battery', 'wifi',
          'cellular connection', 'group 1000003294', 'group 80216')

def dump(node, depth, parent_box, out, cap):
    if len(out) >= cap: return
    if node.get('name', '').strip().lower() in CHROME and '--chrome' not in sys.argv:
        return
    b = node.get('absoluteBoundingBox') or {}
    pos = ''
    if b and parent_box:
        pos = f"@{round(b['x']-parent_box['x']):>4},{round(b['y']-parent_box['y']):<4}"
    size = f"{round(b['width']):>4}x{round(b['height']):<4}" if b.get('width') else ' ' * 9
    bits = [x for x in [
        radius(node),
        ('fill:' + paint(node.get('fills'))) if paint(node.get('fills')) else '',
        ('stroke:' + paint(node.get('strokes')) + f":{round(node.get('strokeWeight') or 0)}")
            if paint(node.get('strokes')) else '',
        typo(node),
        'blur' if any(e.get('type') == 'BACKGROUND_BLUR' for e in node.get('effects') or []) else '',
        'shadow' if any(e.get('type') == 'DROP_SHADOW' for e in node.get('effects') or []) else '',
    ] if x]
    mark = {'TEXT': 'T', 'VECTOR': 'v', 'ELLIPSE': 'o', 'LINE': '-', 'INSTANCE': 'I',
            'RECTANGLE': '▭', 'STAR': '*'}.get(node.get('type'), ' ')
    label = node.get('characters') or node.get('name', '')
    label = label.replace('\n', ' ⏎ ')[:60]
    out.append(f"{'  ' * depth}{mark} {size} {pos}  {label}"
               + (f"   [{' '.join(bits)}]" if bits else ''))
    for c in node.get('children', []):
        dump(c, depth + 1, b or parent_box, out, cap)

def main():
    if '--list' in sys.argv:
        for f in PAGE['children']:
            b = f.get('absoluteBoundingBox') or {}
            print(f"{f['id']:<10} {round(b.get('width',0)):>5}x{round(b.get('height',0)):<5} {f['name']}")
        return
    if '--find' in sys.argv:
        needle = sys.argv[sys.argv.index('--find') + 1].lower()
        for f in PAGE['children']:
            if needle in f['name'].lower():
                b = f.get('absoluteBoundingBox') or {}
                print(f"{f['id']:<10} {round(b.get('width',0)):>5}x{round(b.get('height',0)):<5} {f['name']}")
        return
    if '--text' in sys.argv:
        # Just the copy, in reading order: enough to build a form or a list
        # without scrolling past a hundred vector nodes.
        node = find(PAGE, sys.argv[1])
        if not node: sys.exit(f'no such node: {sys.argv[1]}')
        def texts(n):
            if n.get('name', '').strip().lower() in CHROME: return
            if n.get('type') == 'TEXT':
                st = n.get('style') or {}
                print(f"{round(st.get('fontSize', 0)):>3}/{st.get('fontWeight', 0):<3} "
                      f"{paint(n.get('fills')) or '':<12} {(n.get('characters') or '').strip()}")
            for c in n.get('children', []): texts(c)
        texts(node)
        return
    cap = int(sys.argv[sys.argv.index('--max') + 1]) if '--max' in sys.argv else 100000
    node = find(PAGE, sys.argv[1])
    if not node:
        sys.exit(f'no such node: {sys.argv[1]}')
    out = []
    dump(node, 0, None, out, cap)
    print('\n'.join(out))

main()
