# morph: atomic publish of a 32-cell kernel block (write tmp, close, rename over out) then read-back must be byte-identical; fold = pd*10 + ok
import os, tempfile
kern = bytes((i * i + i + 3) & 255 for i in range(32))  # sys_export stores the LOW BYTE of each cell
d = tempfile.mkdtemp()
tmp, out = os.path.join(d, 'tmp'), os.path.join(d, 'out')
pd = 1
try:
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
    if os.write(fd, kern) != 32: pd = 0
    os.close(fd)
    os.rename(tmp, out)
except OSError:
    pd = 0
try:
    with open(out, 'rb') as f: rd = f.read(33)
except OSError:
    rd = b''
ok = int(rd == kern)
for p in (tmp, out):
    if os.path.exists(p): os.unlink(p)
os.rmdir(d)
print(pd * 10 + ok)
