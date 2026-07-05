import urllib.request, json

# Deep API check - test all known endpoints from contract map
endpoints = [
    ('GET', '/health', None),
    ('GET', '/s/demo', None),
    ('GET', '/s/demo/cart', None),
    ('GET', '/s/demo/checkout', None),
    ('GET', '/public/locations/demo/menu', None),
    ('GET', '/public/locations/demo/theme.css', None),
    ('GET', '/s/demo/manifest.webmanifest', None),
    ('GET', '/api/push/vapid-public-key', None),
    ('POST', '/api/telemetry', '{"action":"page_view","slug":"demo"}'),
    ('GET', '/auth/google', None),
    ('GET', '/auth/local/login', None),
    ('GET', '/api/orders', None),
    ('POST', '/api/orders', '{"test":true}'),
    ('GET', '/api/owner/locations/11111111-1111-1111-1111-111111111111/dashboard/snapshot', None),
    ('GET', '/api/courier/me', None),
    ('GET', '/api/customer/locations/demo/otp/send', None),
    ('GET', '/api/dev/mock-auth', None),
    ('GET', '/robots.txt', None),
]

for method, path, body in endpoints:
    url = f'https://dowiz.fly.dev{path}'
    try:
        req = urllib.request.Request(url, method=method)
        if body:
            req.data = body.encode()
            req.add_header('Content-Type', 'application/json')
        resp = urllib.request.urlopen(req, timeout=10)
        print(f'{resp.status} {method} {path}')
    except urllib.error.HTTPError as e:
        print(f'{e.code} {method} {path}')
    except Exception as e:
        print(f'ERR {method} {path}: {e}')
