import urllib.request, json
# Test the public menu API endpoint which calls read_public_menu_all_locales indirectly
# Actually, the public menu API uses read_public_menu, not all_locales
# Let's try a direct approach - check the public menu API carefully
import time
time.sleep(5)
req = urllib.request.Request('https://dowiz.fly.dev/public/locations/demo/menu')
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read())
print('default_locale:', data.get('default_locale'))
print('supported_locales:', data.get('supported_locales'))
print('currency:', data.get('currency'))
print('location keys:', list(data.get('location', {}).keys()))
print('categories count:', len(data.get('categories', [])))
