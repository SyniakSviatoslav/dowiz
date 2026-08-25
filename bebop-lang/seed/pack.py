#!/usr/bin/env python3
# pack.py <words.full|.bin-raw> <entry_byte_off> <out.bin>
# Reads decimal word stream (or raw), strips count/OFF lines, appends LE64 entry footer.
import re, sys, struct
src, entry, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
raw = open(src,'rb').read()
if b'OFF' in raw or not all((b==10 or 48<=b<=57) for b in raw[:min(8,len(raw))]):
    w=[int(l) for l in raw.decode().split('\n') if re.fullmatch(r'\d+',l.strip())]
    # drop leading count line when it matches remaining length
    if len(w)>=2 and w[0]==len(w)-1:
        w=w[1:]
else:
    w=list(struct.unpack('<%dQ'%(len(raw)//8), raw))
data=b''.join(v.to_bytes(4,'little') for v in w)
data+=struct.pack('<Q', entry)
open(out,'wb').write(data)
print(f"packed {len(w)} words + footer -> {out} ({len(data)}B), entry={entry}")
