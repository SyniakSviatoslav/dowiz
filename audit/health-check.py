import urllib.request, json
req = urllib.request.Request('https://dowiz.fly.dev/health')
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read())
print('Status:', data['status'])
for k,v in data['checks'].items():
    s = v.get('status','?')
    d = v.get('detail','')[:120] if v.get('detail') else ''
    print(f'  {k}: {s} {d}')
