#!/usr/bin/env python3
"""Derive word values for A7 step 3 syscalls: sys_readbuf + sys_mapb."""
import subprocess, tempfile, os

def asm_to_words(asm_text):
    with tempfile.NamedTemporaryFile(suffix='.s', mode='w', delete=False) as f:
        f.write(asm_text)
        sfile = f.name
    try:
        ofile = sfile.replace('.s', '.o')
        r = subprocess.run(['as', '-o', ofile, sfile], capture_output=True, text=True)
        if r.returncode != 0:
            return None, 'ASSEMBLE ERROR: ' + r.stderr
        result = subprocess.run(['objdump', '-d', ofile], capture_output=True, text=True)
        words = []
        for line in result.stdout.split('\n'):
            if ':\t' in line and line.strip() and not line.startswith('Disassembly'):
                parts = line.strip().split()
                if len(parts) >= 2 and parts[0].endswith(':'):
                    hexword = parts[1]
                    if len(hexword) == 8:
                        words.append(int(hexword, 16))
        return words, None
    finally:
        for f in [sfile, ofile]:
            if os.path.exists(f):
                os.unlink(f)

print("=== sys_readbuf: new handle-returning version ===")
print("After vs_deliver(2): x0=fd, x1=len")
print("Goal: zeros(ceil(len/8)), read into x17+off, return (off<<32)|nread")
print()

# The new sys_readbuf — derive all words
sys_readbuf_asm = """\
.text
.globl _start
_start:
    // x0=fd, x1=len (from vs_deliver)
    mov x19, x0
    mov x20, x1
    add x21, x20, #7
    lsr x21, x21, #3
    lsl x1, x21, #3
    mov x22, x27
    add x23, x22, x1
    cmp x28, x23
    b.ls 2f
    add x27, x27, x1
    mov x3, x22
    b 1f
0:
    str xzr, [x3], #8
1:
    cmp x3, x23
    b.lt 0b
2:
    sub x1, x22, x17
    add x4, x17, x1
    mov x0, x19
    mov x3, x1
    mov x1, x4
    mov x2, x20
    mov x8, #63
    svc #0
    mov x23, x0
    lsl x0, x3, #32
    orr x0, x0, x23
"""
w, e = asm_to_words(sys_readbuf_asm)
if w:
    print("sys_readbuf words ({} total):".format(len(w)))
    for i, ww in enumerate(w):
        print("  em(insns, n, {});   // 0x{:08x}".format(ww, ww))
    print()
    print("Word count delta: +{} (26 - 9 old = +17)".format(len(w) - 9))
else:
    print("ERROR:", e)

print()
print("=== sys_mapb: mmap-file-as-handle version ===")
print("After vs_deliver(2): x0=path_handle, x1=map_len")
print("Goal: openat+mmap+close, return ((addr-x17)<<32)|len")
print()

sys_mapb_asm = """\
.text
.globl _start
_start:
    // x0=path_handle, x1=map_len (from vs_deliver)
    lsr x2, x0, #32
    add x2, x17, x2
    and x3, x0, #0xffffffff
    mov x4, x1
    mov x0, #-100
    mov x1, x2
    mov x2, x3
    mov x3, #0
    mov x8, #56
    svc #0
    mov x5, x0
    mov x0, #0
    mov x1, x4
    mov x2, #1
    mov x3, #2
    mov x4, x5
    mov x5, #0
    mov x8, #222
    svc #0
    mov x2, x0
    mov x0, x5
    mov x8, #57
    svc #0
    sub x0, x2, x17
    lsl x0, x0, #32
    orr x0, x0, x4
"""
w2, e2 = asm_to_words(sys_mapb_asm)
if w2:
    print("sys_mapb words ({} total):".format(len(w2)))
    for i, ww in enumerate(w2):
        print("  em(insns, n, {});   // 0x{:08x}".format(ww, ww))
    print()
    print("Word count: {} (new function)".format(len(w2)))
else:
    print("ERROR:", e2)

print()
print("=== Verify key syscalls ===")
for nr, name in [(56, "openat"), (57, "close"), (222, "mmap"), (63, "read")]:
    asm = ".text\n.globl _start\n_start:\n    mov x8, #{}\n".format(nr)
    w, _ = asm_to_words(asm)
    if w:
        print("  sys_{} (x8={}) = 0x{:08x} = {}".format(name, nr, w[0], w[0]))
    else:
        print("  sys_{} (x8={}): ERROR".format(name, nr))

print()
print("=== Constants ===")
for val, name in [(-100, "AT_FDCWD"), (1, "PROT_READ"), (2, "MAP_PRIVATE")]:
    asm = ".text\n.globl _start\n_start:\n    mov x0, #{}\n".format(val)
    w, _ = asm_to_words(asm)
    if w:
        print("  {} = 0x{:08x} = {}".format(name, w[0], w[0]))
    else:
        print("  {}: ERROR".format(name))
