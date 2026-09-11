#!/usr/bin/env python3
"""Derive word values for A7 handle forms using as + objdump."""
import subprocess, tempfile, os, struct

def asm_to_words(asm_text):
    """Assemble asm_text and return list of 32-bit LE words."""
    with tempfile.NamedTemporaryFile(suffix='.s', mode='w', delete=False) as f:
        f.write(asm_text)
        sfile = f.name
    try:
        ofile = sfile.replace('.s', '.o')
        subprocess.run(['as', '-o', ofile, sfile], check=True, capture_output=True)
        result = subprocess.run(['objdump', '-d', ofile], capture_output=True, text=True, check=True)
        words = []
        for line in result.stdout.split('\n'):
            if ':\t' in line and line.strip() and not line.startswith('Disassembly'):
                # parse hex word
                parts = line.strip().split()
                if len(parts) >= 2 and parts[0].endswith(':'):
                    hexword = parts[1]
                    if len(hexword) == 8:
                        words.append(int(hexword, 16))
        return words
    finally:
        for f in [sfile, ofile]:
            if os.path.exists(f):
                os.unlink(f)

# Test char handle form: lsr xt,xs,#32 ; add xt,xt,xi ; ldrb wd,[x17,xt]
# Using x9 for xt, x0 for xs, x1 for xi, w0 for wd
char_asm = """
.text
.globl _start
_start:
    lsr x9, x0, #32
    add x9, x17, x9
    ldrb w0, [x9, x1]
"""
char_words = asm_to_words(char_asm)
print("char handle words:", [hex(w) for w in char_words])

# Test str_len handle form: and xd, xs, #0xffffffff
# Using x0 for xd, x0 for xs
str_len_asm = """
.text
.globl _start
_start:
    and x0, x0, #0xffffffff
"""
str_len_words = asm_to_words(str_len_asm)
print("str_len handle words:", [hex(w) for w in str_len_words])

# Test crc32b handle prologue: lsr x1,x0,#32 ; add x1,x17,x1 ; and x2,x0,#0xffffffff
prologue_asm = """
.text
.globl _start
_start:
    lsr x1, x0, #32
    add x1, x17, x1
    and x2, x0, #0xffffffff
"""
prologue_words = asm_to_words(prologue_asm)
print("crc32b prologue words:", [hex(w) for w in prologue_words])

# Test existing crc32b loop body (from emit_crc32b)
# movn w1,#0 = 0x12800001
# ldrb w2,[x0],#1 = 0x38401402  (wait, need to check: ldrb w2,[x0],#1)
# Actually the bp code uses raw pointers, let me verify the existing loop
# Current emit_crc32b emits: 310378497, 943723522, 872415330, 448938017, 402653181, 706806752
print("\nExisting crc32b words:", [hex(w) for w in [310378497, 943723522, 872415330, 448938017, 402653181, 706806752]])

# Verify what those existing words are
for w in [310378497, 943723522, 872415330, 448938017, 402653181, 706806752]:
    print(f"  {hex(w)} = {w}")

# Now derive the full crc32b handle form combining prologue + loop
# Prologue (3 words) + loop using x1 as base, x2 as len
# The loop: movn w3,#0; L: ldrb w4,[x1],#1; cbz w4,done; crc32b w3,w3,w4; b L; done: orn w0,wzr,w3
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
try:
    crc_handle_words = asm_to_words(crc_handle_asm)
    print("\ncrc32b handle words:", [hex(w) for w in crc_handle_words])
except Exception as e:
    print(f"\ncrc32b handle asm failed: {e}")

