import urllib.request, json

print('=== GET AUTH TOKEN ===')
req = urllib.request.Request('https://dowiz.fly.dev/api/dev/mock-auth', method='POST', data=b'{}')
req.add_header('Content-Type', 'application/json')
resp = urllib.request.urlopen(req, timeout=10)
auth = json.loads(resp.read())
token = auth['access_token']
locId = auth.get('activeLocationId', 'NONE')
print(f'Token: {token[:50]}...')
print(f'LocationId: {locId}')
print()

headers = {'Authorization': f'Bearer {token}'}

tests = [
    ('GET /api/owner/menu/categories', 'GET', '/api/owner/menu/categories'),
    ('POST /api/owner/menu/categories', 'POST', '/api/owner/menu/categories', {'name': 'TestCat'}),
    ('GET /api/owner/menu/products', 'GET', '/api/owner/menu/products'),
    ('GET /api/owner/orders', 'GET', '/api/owner/orders'),
    ('GET /api/owner/couriers', 'GET', '/api/owner/couriers'),
    ('GET /api/owner/brand', 'GET', '/api/owner/brand'),
    ('GET /api/owner/settings', 'GET', '/api/owner/settings'),
    ('POST /api/auth/local/login', 'POST', '/api/auth/local/login', {'email': 'test@test.com', 'password': '123456'}),
]

for name, method, path, *rest in tests:
    body = rest[0] if rest else None
    url = f'https://dowiz.fly.dev{path}'
    try:
        req = urllib.request.Request(url, method=method)
        req.add_header('Authorization', f'Bearer {token}')
        req.add_header('Content-Type', 'application/json')
        if body:
            req.data = json.dumps(body).encode()
        resp = urllib.request.urlopen(req, timeout=10)
        print(f'{resp.status} {method} {path}')
        if resp.status == 200:
            data = json.loads(resp.read())
            if isinstance(data, list):
                print(f'  -> {len(data)} items')
            elif isinstance(data, dict):
                print(f'  -> keys: {list(data.keys())[:5]}')
    except urllib.error.HTTPError as e:
        body = e.read().decode()[:200]
        print(f'{e.code} {method} {path} -> {body}')
    except Exception as e:
        print(f'ERR {method} {path} -> {e}')
