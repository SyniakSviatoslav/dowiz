#!/usr/bin/env python3
"""Derive word values for A7 handle forms using as + objdump."""
import subprocess, tempfile, os

def asm_to_words(asm_text):
    """Assemble asm_text and return list of 32-bit LE words."""
    with tempfile.NamedTemporaryFile(suffix='.s', mode='w', delete=False) as f:
        f.write(asm_text)
        sfile = f.name
    try:
        ofile = sfile.replace('.s', '.o')
        r = subprocess.run(['as', '-o', ofile, sfile], capture_output=True, text=True)
        if r.returncode != 0:
            return None, r.stderr
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

# crc32b handle form: prologue + loop using x1 as base, w3 for crc, w4 for byte
# cbz must use multiple of 4 offset; b uses multiple of 4
crc_handle_asm = """
.text
.globl _start
_start:
    lsr x1, x0, #32
    add x1, x17, x1
    and x2, x0, #0xffffffff
    movn w3, #0
    ldrb w4, [x1], #1
    cbz w4, 4
    crc32b w3, w3, w4
    b -4
    orn w0, wzr, w3
"""
words, err = asm_to_words(crc_handle_asm)
if words:
    print("crc32b handle words:", [hex(w) for w in words])
    print("Total words:", len(words))
else:
    print("crc32b handle asm failed:")
    print(err)
    # Try without crc32b instruction (simulator may not support it)
    # Just get the prologue + setup
    simple_asm = """
.text
.globl _start
_start:
    lsr x1, x0, #32
    add x1, x17, x1
    and x2, x0, #0xffffffff
    movn w3, #0
"""
words2, _ = asm_to_words(simple_asm)
print("Prologue+setup words:", [hex(w) for w in words2])

# Also derive the existing loop to confirm what it does
# ldrb w2,[x0],#1 = ?
ldr_asm = """
.text
.globl _start
_start:
    ldrb w2, [x0], #1
"""
w, _ = asm_to_words(ldr_asm)
print(f"ldrb w2,[x0],#1 = {hex(w[0]) if w else 'FAIL'}")

# crc32b w1,w1,w2 = ?
crc_asm = """
.text
.globl _start
_start:
    crc32b w1, w1, w2
"""
# This may fail if assembler doesn't support crc32b
w2, err2 = asm_to_words(crc_asm)
print(f"crc32b w1,w1,w2 = {hex(w2[0]) if w2 else 'FAIL: ' + err2.strip() if err2 else 'FAIL'}")

