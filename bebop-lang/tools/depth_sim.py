#!/usr/bin/env python3
"""SP-temp-depth symbolic simulator for emitted .full streams.
Usage: depth_sim.py <file.full>  -> flags fns with unbalanced temp stack
(decodes: b/b.cond/cbz/cbnz/bl/ret, add/sub imm on sp, stp/ldp pre/post).
Merge-point depth conflicts and nonzero-at-ret are candidate emitter bugs."""
import sys
toks=open(sys.argv[1]).read().split()
n=int(toks[0]); W=[int(t)&0xffffffff for t in toks[1:1+n]]
_offl=[l for l in open(sys.argv[1]) if l.startswith('OFF')]
OFF=[int(x) for x in _offl[0].split()[2:]] if _offl else [0]
def sim(span):
    L=len(span); depths=[None]*L; depths[0]=0; work=[0]; problems=[]
    while work:
        i=work.pop(); d=depths[i]
        if i>=L or d is None: continue
        w=span[i]; op=w>>26
        if w==0xD65F03C0:
            if d!=0: problems.append(("ret",i,d))
            continue
        nxt=[i+1]
        if op==0x05:
            imm=w&0x3FFFFFF
            if imm&0x2000000: imm-=0x4000000
            t=i+imm; nxt=[t] if 0<=t<L else []
        elif (w&0xFF000010)==0x54000000 or (w>>24) in (0x34,0x35,0xB4,0xB5):
            imm=(w>>5)&0x7FFFF
            if imm&0x100000: imm-=0x200000
            t=i+imm; nxt=[i+1]+([t] if 0<=t<L else [])
        else:
            rd=w&0x1F; rn=(w>>5)&0x1F
            # sub/add sp,#imm{,lsl #12}: sh bit 22 means imm<<12 (a full slot
            # count of imm*4096/16); the old model shifted by 1 — symmetric in
            # prologue/epilogue so it never flagged, but count it true now.
            if (w&0xFFC00000)==0xD1000000 and rn==31 and rd==31:
                d+=((w>>10)&0xFFF)<<(12*((w>>22)&1))>>4
            elif (w&0xFFC00000)==0x91000000 and rn==31 and rd==31:
                d-=((w>>10)&0xFFF)<<(12*((w>>22)&1))>>4
            elif w==0xA9BF7BFD: d+=1
            elif w==0xA8C17BFD: d-=1
        for t in nxt:
            old=depths[t]
            if old is None: depths[t]=d; work.append(t)
            elif old!=d: problems.append(("merge",t,old,d))
    return problems
for k in range(len(OFF)):
    s=OFF[k]; e=OFF[k+1] if k+1<len(OFF) else n
    p=sim(W[s:e])
    if p: print("fn#%d @%d: %s"%(k,s,p[:6]))
