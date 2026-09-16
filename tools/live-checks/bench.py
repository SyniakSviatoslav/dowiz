"""Server time per endpoint, against the deployment.

READ THE NUMBERS AS A SHAPE, NOT AS A STOPWATCH. This box is a PRoot container
on a phone: the network baseline alone swings sixty milliseconds between runs,
which is larger than most of the differences worth chasing. `healthz` is
measured every time and subtracted precisely so the noise is visible rather
than folded into the answer.

What the numbers are good for is COUNTING ROUND TRIPS. A D1 query from this
Worker is roughly sixty milliseconds, so an endpoint that drops by that much has
lost one -- and that is a change you can also verify by reading the code, which
is the check that actually settles it.
"""
import json, time, statistics, urllib.request
B="https://dowiz-api.sviatoslavsyniak.workers.dev"
H={"accept":"application/json","user-agent":"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"}
def t(p, tok=None, n=7):
    out=[]
    for _ in range(n):
        time.sleep(0.5)
        r=urllib.request.Request(B+p)
        for k,v in H.items(): r.add_header(k,v)
        if tok: r.add_header("authorization","Bearer "+tok)
        a=time.perf_counter()
        try:
            with urllib.request.urlopen(r,timeout=60) as f: f.read()
        except Exception: continue
        out.append(time.perf_counter()-a)
    return statistics.median(out) if out else float('nan')
r=urllib.request.Request(B+"/api/auth/login", data=json.dumps({"email":"ana@dubin.al","password":"dubin-owner"}).encode(), method="POST")
for k,v in {**H,"content-type":"application/json"}.items(): r.add_header(k,v)
with urllib.request.urlopen(r,timeout=60) as f: tok=json.loads(f.read())["access_token"]
base=t("/healthz")
print(f"  {'база (healthz)':<40} {base*1000:6.0f} мс")
for p in ["/api/public/locations/dubin-sushi/menu","/api/owner/dashboard","/api/owner/orders",
          "/api/owner/analytics?days=30","/api/owner/stock","/api/owner/features","/api/owner/customers"]:
    m=t(p,tok)
    print(f"  {p:<40} {m*1000:6.0f} мс  ·  сервер ~{max(0,(m-base))*1000:.0f} мс")
