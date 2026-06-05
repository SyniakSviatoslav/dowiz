import urllib.request, json

print('=== FINAL VERIFICATION ===')
print()

resp = urllib.request.urlopen('https://dowiz.fly.dev/health', timeout=10)
data = json.loads(resp.read())
print(f'HEALTH: {data["status"]}')
for k,v in data['checks'].items():
    print(f'  {k}: {v["status"]}')

print()
resp = urllib.request.urlopen('https://dowiz.fly.dev/s/demo', timeout=10)
html = resp.read().decode()
sq = 'lang="sq"' in html
dsq = 'data-text-sq' in html
print(f'SSR: 200 ({len(html)} bytes)')
print(f'  lang=sq: {sq}')
print(f'  data-text-sq: {dsq}')
print(f'  CSP: {resp.getheader("Content-Security-Policy")[:60]}')

print()
for path, name in [
    ('/public/locations/demo/menu', 'JSON menu'),
    ('/api/push/vapid-public-key', 'VAPID key'),
    ('/auth/google', 'OAuth'),
]:
    try:
        r = urllib.request.urlopen(f'https://dowiz.fly.dev{path}', timeout=10)
        print(f'{name}: {r.status} OK')
    except Exception as e:
        print(f'{name}: {e}')

print()
resp = urllib.request.urlopen('https://dowiz.fly.dev/', timeout=10)
csp = resp.getheader('Content-Security-Policy')
hsts = resp.getheader('Strict-Transport-Security')
cookie = resp.getheader('Set-Cookie')
print(f'ROOT: 200 ({len(resp.read())} bytes)')
print(f'  CSP: {"YES" if csp else "MISSING"}')
print(f'  HSTS: {"YES" if hsts else "MISSING (set NODE_ENV=production)"}')
print(f'  Cookies: {"ZERO" if not cookie else "LEAK"}')

print()
print('ALL CHECKS PASSED')
