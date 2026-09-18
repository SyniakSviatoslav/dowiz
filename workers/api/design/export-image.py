#!/usr/bin/env python3
"""Render Figma nodes to files in public/kit/img/ through the REST images API.

For the artwork a sprite cannot carry: photographs, the QR, illustrations that
are a hundred vector paths rather than an icon. `/v1/images` renders any node at
a scale, which is how these arrive as the file's own pixels instead of something
drawn by hand.

Usage:  . /root/.figma_token
        python3 export-image.py 1:5701=qr 1:2107=onboarding-device --scale 2
"""
import json, os, sys, urllib.request, urllib.parse

KEY = '0CR9FTDXuyuHE6c34EmOR9'
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'public', 'kit', 'img')
TOKEN = os.environ.get('FIGMA_TOKEN')
if not TOKEN:
    sys.exit('FIGMA_TOKEN is not set: `. /root/.figma_token` first')

pairs, scale, fmt = [], '2', 'png'
for a in sys.argv[1:]:
    if a.startswith('--scale='): scale = a.split('=', 1)[1]
    elif a.startswith('--format='): fmt = a.split('=', 1)[1]
    elif '=' in a: pairs.append(a.split('=', 1))
if not pairs:
    sys.exit('nothing to export')

os.makedirs(OUT, exist_ok=True)
ids = ','.join(p[0] for p in pairs)
url = (f'https://api.figma.com/v1/images/{KEY}?ids={urllib.parse.quote(ids)}'
       f'&scale={scale}&format={fmt}')
req = urllib.request.Request(url, headers={'X-Figma-Token': TOKEN})
body = json.load(urllib.request.urlopen(req))
if body.get('err'):
    sys.exit(f'figma: {body["err"]}')

for nid, name in pairs:
    src = (body.get('images') or {}).get(nid)
    if not src:
        print(f'  MISS {nid} -> {name}: no image returned')
        continue
    dest = os.path.join(OUT, f'{name}.{fmt}')
    with urllib.request.urlopen(src) as r, open(dest, 'wb') as f:
        f.write(r.read())
    print(f'  {name}.{fmt}  {os.path.getsize(dest)} bytes')
