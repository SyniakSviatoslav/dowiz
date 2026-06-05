import urllib.request, json, uuid

req = urllib.request.Request('https://dowiz.fly.dev/api/dev/mock-auth', method='POST', data=b'{}')
req.add_header('Content-Type', 'application/json')
try:
    resp = urllib.request.urlopen(req, timeout=10)
    data = json.loads(resp.read())
    print(f'MOCK AUTH: {resp.status}')
    tok = data.get('access_token', 'NONE')
    print(f'  token: {tok[:60]}...')
    print(f'  userId: {data.get("userId","NONE")}')
except urllib.error.HTTPError as e:
    print(f'MOCK AUTH: {e.code} -> {e.read().decode()[:200]}')

print()

order_body = json.dumps({
    'locationId': '1f609add-062a-4bb5-89bf-d695f963ede6',
    'type': 'delivery',
    'items': [{'productId': '1b4e1275-3f37-47e5-8652-1ebd6c8de04a', 'quantity': 1}],
    'customer': {'phone': '+355600000001', 'name': 'Test'},
    'delivery': {'pin': {'lat': 41.3275, 'lng': 19.8187}, 'address_text': 'Test Street'},
    'payment': {'method': 'cash'},
    'idempotency_key': str(uuid.uuid4()),
}).encode()

req = urllib.request.Request('https://dowiz.fly.dev/api/orders', method='POST', data=order_body)
req.add_header('Content-Type', 'application/json')
try:
    resp = urllib.request.urlopen(req, timeout=10)
    data = json.loads(resp.read())
    print(f'ORDER CREATE: {resp.status}')
    print(f'  orderId: {data.get("id","NONE")}')
    print(f'  status: {data.get("status","NONE")}')
    print(f'  token: {bool(data.get("access_token"))}')
except urllib.error.HTTPError as e:
    body = e.read().decode()
    print(f'ORDER CREATE: {e.code}')
    print(f'  {body[:400]}')
