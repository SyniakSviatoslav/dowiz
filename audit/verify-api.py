import urllib.request, json

tests = [
    ('SSR menu sq', 'GET', '/s/demo', None, 200, 'lang="sq"'),
    ('SSR menu embed', 'GET', '/s/demo?embed=1', None, 200, 'embed-mode'),
    ('Public menu JSON', 'GET', '/public/locations/demo/menu', None, 200, 'Pepperoni'),
    ('VAPID key', 'GET', '/api/push/vapid-public-key', None, 200, 'publicKey'),
    ('Health', 'GET', '/health', None, 200, 'degraded'),
    ('Google OAuth', 'GET', '/auth/google', None, 200, 'accounts.google.com'),
    ('Dashboard (auth)', 'GET', '/api/owner/locations/1f609add-062a-4bb5-89bf-d695f963ede6/dashboard/snapshot', None, 401, 'Unauthorized'),
    ('Courier me (auth)', 'GET', '/api/courier/me', None, 401, 'Token'),
    ('Order POST (validate)', 'POST', '/api/orders', {'test':1}, 500, 'Internal'),
    ('Nonexistent route', 'GET', '/api/nonexistent', None, 404, 'Not found'),
]

for name, method, path, body, expect_code, expect_body in tests:
    try:
        url = f'https://dowiz.fly.dev{path}'
        req = urllib.request.Request(url, method=method)
        if body is not None:
            req.data = json.dumps(body).encode()
            req.add_header('Content-Type', 'application/json')
        resp = urllib.request.urlopen(req, timeout=10)
        content = resp.read().decode()
        ok = str(expect_code) in str(resp.status) and expect_body.lower() in content.lower()
        print(f'  {"PASS" if ok else "FAIL"}: {name} ({resp.status})')
        if not ok:
            print(f'    Expected: {expect_code} + "{expect_body}"')
            print(f'    Got: {resp.status} + "{content[:100]}"')
    except urllib.error.HTTPError as e:
        content = e.read().decode()
        ok = e.code == expect_code and expect_body.lower() in content.lower()
        print(f'  {"PASS" if ok else "FAIL"}: {name} ({e.code})')
        if not ok:
            print(f'    Expected: {expect_code} + "{expect_body}"')
            print(f'    Got: {e.code} + "{content[:100]}"')
