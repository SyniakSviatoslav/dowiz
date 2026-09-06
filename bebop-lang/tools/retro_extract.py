import json,sys,glob,os,re,collections,datetime
KW=re.compile(r'pitfall|cost|wasted|again|phantom|mistake|wrong|retract|sorry|\bbug\b|SIGBUS|exit 95|rc=90|killed|\b144\b|NO FIXPOINT|battery: RED|DIVERGE|CRASH|transient|not reproducible',re.I)
UKW=re.compile(r'знову|ще раз|я ж казав|чому|не роби|перестань|стоп|повторюєш|забув|знов|нахуя|блять|бляха|скільки раз|казав',re.I)
files=sorted(glob.glob('/root/.claude/projects/-root/*.jsonl'),key=os.path.getsize,reverse=True)
mode=sys.argv[1] if len(sys.argv)>1 else 'inv'
def text_of(c):
    if isinstance(c,str): return c
    out=[]
    for b in c or []:
        if isinstance(b,dict):
            if b.get('type')=='text': out.append(b.get('text',''))
            elif b.get('type')=='tool_result':
                cc=b.get('content')
                if isinstance(cc,str): out.append(cc)
                elif isinstance(cc,list):
                    for x in cc:
                        if isinstance(x,dict) and x.get('type')=='text': out.append(x.get('text',''))
    return '\n'.join(out)
for f in files:
    if len(sys.argv)>2 and sys.argv[2] not in f: continue
    n=os.path.basename(f)[:8]
    ts=[];tools=0;errs=[];errtexts=collections.Counter();cmds=collections.Counter();kwmsgs=[];umsgs=[];ufr=[];bashcmds=[]
    gitcommits=0;first=None
    with open(f) as fh:
        for line in fh:
            try: d=json.loads(line)
            except: continue
            t=d.get('timestamp')
            if t: ts.append(t)
            m=d.get('message') or {}
            role=m.get('role') or d.get('type')
            c=m.get('content')
            if d.get('type')=='assistant' and isinstance(c,list):
                for b in c:
                    if b.get('type')=='tool_use':
                        tools+=1
                        inp=b.get('input',{})
                        if b.get('name')=='Bash':
                            cmd=inp.get('command','')
                            bashcmds.append((t,cmd))
                            cmds[cmd.strip()[:120]]+=1
                            if 'git commit' in cmd: gitcommits+=1
                    elif b.get('type')=='text':
                        tx=b.get('text','')
                        if KW.search(tx): kwmsgs.append((t,tx))
            if d.get('type')=='user' and isinstance(c,list):
                for b in c:
                    if b.get('type')=='tool_result' and b.get('is_error'):
                        errs.append(1)
                        et=text_of([b])[:160].replace('\n',' ')
                        errtexts[et]+=1
                    elif b.get('type')=='text':
                        tx=b.get('text','')
                        if first is None: first=tx
                        umsgs.append((t,tx))
                        if UKW.search(tx): ufr.append((t,tx))
            elif d.get('type')=='user' and isinstance(c,str):
                if first is None: first=c
                umsgs.append((t,c))
                if UKW.search(c): ufr.append((t,c))
    if not ts: continue
    t0=min(ts);t1=max(ts)
    dt=(datetime.datetime.fromisoformat(t1.replace('Z','+00:00'))-datetime.datetime.fromisoformat(t0.replace('Z','+00:00')))
    # gaps
    tsd=sorted(datetime.datetime.fromisoformat(x.replace('Z','+00:00')) for x in ts)
    gaps=[(tsd[i+1]-tsd[i]).total_seconds()/60 for i in range(len(tsd)-1)]
    biggaps=sorted([g for g in gaps if g>20],reverse=True)[:3]
    if mode=='inv':
        print(f"{n} start={t0[:16]} dur={str(dt)[:7]} tools={tools} errs={len(errs)} commits={gitcommits} usermsgs={len(umsgs)} kwmsgs={len(kwmsgs)} frust={len(ufr)} gaps>20m={[round(g) for g in biggaps]}")
        print("  FIRST:",(first or '')[:300].replace('\n',' '))
        rep=[(k,v) for k,v in cmds.most_common(5) if v>2]
        if rep: print("  REPEATED:",rep)
        print("  TOPERR:",[ (k[:100],v) for k,v in errtexts.most_common(4)])
    elif mode=='kw':
        print("=====",n,t0[:16])
        for t,tx in kwmsgs:
            for ln in tx.split('\n'):
                if KW.search(ln): print(" ",t[11:16],ln.strip()[:260])
    elif mode=='user':
        print("=====",n,t0[:16])
        for t,tx in ufr: print(" ",t[11:16],tx.strip()[:400].replace('\n',' | '))
    elif mode=='allusers':
        print("=====",n,t0[:16])
        for t,tx in umsgs:
            if tx.startswith('<') : continue
            print(" ",t[11:16],tx.strip()[:200].replace('\n',' | '))
    elif mode=='errs':
        print("=====",n,t0[:16],len(errs))
        for k,v in errtexts.most_common(12): print(" ",v,k[:150])
