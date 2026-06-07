import urllib.request, json

# Test: does the route work with valid data?
body = json.dumps({'email': 'test@dowiz.com', 'password': 'test123456'}).encode()
req = urllib.request.Request('https://dowiz.fly.dev/api/auth/local/login', method='POST', data=body)
req.add_header('Content-Type', 'application/json')
try:
    resp = urllib.request.urlopen(req, timeout=10)
    print(f'Valid body: {resp.status}')
    data = json.loads(resp.read())
    print(f'  Response: {json.dumps(data)[:200]}')
except urllib.error.HTTPError as e:
    body = e.read().decode()
    print(f'Valid body: {e.code}')
    print(f'  Response: {body[:500]}')
    # Also print response headers for debugging
    print(f'  Headers: {dict(e.headers)}')
