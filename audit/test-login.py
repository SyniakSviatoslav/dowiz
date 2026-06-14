import urllib.request, json
body = json.dumps({'email': 'test@dowiz.com', 'password': 'test123456'}).encode()
req = urllib.request.Request('https://dowiz.fly.dev/api/auth/local/login', method='POST', data=body)
req.add_header('Content-Type', 'application/json')
try:
    resp = urllib.request.urlopen(req, timeout=10)
    data = json.loads(resp.read())
    print(f'LOGIN: {resp.status}')
    tok = data.get('access_token', 'NONE')
    print(f'  token: {tok[:60]}...')
    print(f'  refresh: {bool(data.get("refresh_token"))}')
except urllib.error.HTTPError as e:
    body = e.read().decode()[:300]
    print(f'LOGIN: {e.code} -> {body}')
