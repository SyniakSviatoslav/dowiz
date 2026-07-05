import urllib.request, json

req = urllib.request.Request("https://dowiz.fly.dev/health")
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read())
print(json.dumps(data["checks"]["postgres"], indent=2))
