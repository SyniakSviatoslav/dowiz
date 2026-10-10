import urllib.request, json

req = urllib.request.Request("https://qa-durres.dowiz.org/health")
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read())
print(json.dumps(data["checks"]["postgres"], indent=2))
