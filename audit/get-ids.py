import urllib.request, json
resp = urllib.request.urlopen('https://dowiz.fly.dev/public/locations/demo/menu')
data = json.loads(resp.read())
lid = data.get('location', {}).get('id', 'NONE')
print('Location ID:', lid)
print()
for cat in data.get('categories', []):
    for prod in cat.get('products', []):
        print(f"Product: {prod['id']} name={prod.get('available_names',{}).get('sq','?')} price={prod['price']}")
