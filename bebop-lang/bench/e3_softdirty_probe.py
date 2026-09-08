#!/usr/bin/env python3
# ROADMAP E3 probe: is /proc/self/clear_refs=4 (soft-dirty) real on this box?
# Refutes E3. Four checks; the POSITIVE CONTROL (type 1) is what makes the negative binding.
import os, re, ctypes, sys
PS = os.sysconf('SC_PAGE_SIZE')
libc = ctypes.CDLL("libc.so.6", use_errno=True)
libc.mmap.restype = ctypes.c_void_p
libc.mmap.argtypes = [ctypes.c_void_p,ctypes.c_size_t,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_long]
PM = os.open("/proc/self/pagemap", os.O_RDONLY)
def pmread(base,n):
    os.lseek(PM,(base//PS)*8,os.SEEK_SET); b=os.read(PM,n*8)
    return [int.from_bytes(b[i*8:i*8+8],'little') for i in range(len(b)//8)]
def clear(t):
    fd=os.open("/proc/self/clear_refs",os.O_WRONLY); n=os.write(fd,("%d\n"%t).encode()); os.close(fd); return n
def smaps_referenced(base):
    # the VMA CONTAINING base (an anon mmap can be merged into a neighbour)
    hit=False
    for L in open("/proc/self/smaps"):
        m=re.match(r'^([0-9a-f]+)-([0-9a-f]+) ',L)
        if m: hit = int(m.group(1),16) <= base < int(m.group(2),16)
        elif hit and L.startswith("Referenced:"): return int(L.split()[1])

print("== 0. clear_refs input validation (does the write reach the real handler?)")
for t in (1,4,9):
    try: print("   type %d -> accepted, n=%d" % (t, clear(t)))
    except OSError as e: print("   type %d -> %s" % (t,e))

print("== 1. POSITIVE CONTROL: clear_refs type 1 (referenced) via smaps")
NP=512; b=libc.mmap(None,NP*PS,3,0x22,-1,0); buf=(ctypes.c_char*(NP*PS)).from_address(b)
for i in range(NP): buf[i*PS]=b'\x01'
print("   Referenced before=%s kB" % smaps_referenced(b)); clear(1)
print("   Referenced after clear=%s kB" % smaps_referenced(b))
for i in range(0,NP,4): buf[i*PS+8]=b'\x02'
print("   Referenced after touching 1/4=%s kB  (expect %d)" % (smaps_referenced(b), NP*PS//1024//4))
libc.munmap(ctypes.c_void_p(b),NP*PS)

print("== 2. pagemap offset control: [mapped, HOLE, mapped]")
b=libc.mmap(None,3*PS,3,0x22,-1,0); buf=(ctypes.c_char*(3*PS)).from_address(b)
buf[0]=b'\x01'; buf[2*PS]=b'\x01'; libc.munmap(ctypes.c_void_p(b+PS),PS)
print("   present bits =", [(e>>63)&1 for e in pmread(b,3)], "(expect [1,0,1])")

print("== 3. THE GATE: soft-dirty bit 55 over the 2x2, 64 MiB each, 1/3 of pages written")
NP=16384; SZ=NP*PS
for name,fl,fb in [("MAP_PRIVATE|ANON",0x22,0),("MAP_SHARED |ANON",0x21,0),
                   ("MAP_PRIVATE|FILE",0x02,1),("MAP_SHARED |FILE",0x01,1)]:
    fd=-1
    if fb:
        p=os.path.join(sys.argv[1] if len(sys.argv)>1 else ".","e3_%s.dat"%name[4:10].strip())
        fd=os.open(p,os.O_RDWR|os.O_CREAT|os.O_TRUNC,0o600); os.ftruncate(fd,SZ)
    b=libc.mmap(None,SZ,3,fl,fd,0); buf=(ctypes.c_char*SZ).from_address(b)
    for i in range(NP): buf[i*PS]=b'\x01'          # fault in: present + clean
    clear(4)
    T=set(range(0,NP,3))
    for i in T: buf[i*PS+64]=b'\x02'               # write exactly 1/3
    e=pmread(b,NP)
    sd={i for i,v in enumerate(e) if (v>>55)&1}
    pres=sum(1 for v in e if (v>>63)&1)
    print("   %-16s present %5d/%d  softdirty %5d  FN %5d (%.1f%%)  FP %5d (%.1f%%)  ent[0]=%016x"
          % (name,pres,NP,len(sd),len(T-sd),100.0*len(T-sd)/len(T),len(sd-T),100.0*len(sd-T)/(NP-len(T)),e[0]))
    libc.munmap(ctypes.c_void_p(b),SZ)
    if fd>=0: os.close(fd); os.unlink(p)

print("== 4. no relocated flag: does ANY bit 48..62 separate written from unwritten?")
NP=4096; b=libc.mmap(None,NP*PS,3,0x22,-1,0); buf=(ctypes.c_char*(NP*PS)).from_address(b)
for i in range(NP): buf[i*PS]=b'\x01'
clear(4)
for i in range(0,NP,3): buf[i*PS+64]=b'\x02'
e=pmread(b,NP)
print("   distinct pagemap entries over %d pages: %s" % (NP,sorted({hex(v) for v in e})))
print("== 5. whole address space + VmFlags")
clear(4); x=bytearray(8<<20)
for i in range(0,len(x),PS): x[i]=7
tot=seen=0
for L in open("/proc/self/maps"):
    m=re.match(r'^([0-9a-f]+)-([0-9a-f]+) ',L)
    if not m: continue
    a,z=int(m.group(1),16),int(m.group(2),16); n=(z-a)//PS
    if n==0 or n>200000: continue
    try: v=pmread(a,n)
    except Exception: continue
    tot+=len(v); seen+=sum(1 for q in v if (q>>55)&1)
print("   %d pages examined, %d with bit55 set" % (tot,seen))
vf=[L for L in open("/proc/self/smaps") if L.startswith("VmFlags")]
print("   VMAs advertising 'sd' (VM_SOFTDIRTY): %d of %d" % (sum(1 for L in vf if re.search(r'\bsd\b',L)),len(vf)))
