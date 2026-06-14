import urllib.request
for url in ['https://dowiz.fly.dev/', 'https://dowiz.fly.dev/s/demo', 'https://dowiz.fly.dev/health']:
    resp = urllib.request.urlopen(urllib.request.Request(url), timeout=10)
    hsts = resp.getheader('Strict-Transport-Security')
    csp = resp.getheader('Content-Security-Policy')
    xcto = resp.getheader('X-Content-Type-Options')
    print(f'{url}')
    print(f'  HSTS: {hsts or "MISSING"}')
    print(f'  CSP: {"YES" if csp else "MISSING"}')
    print(f'  X-CTO: {xcto or "MISSING"}')
    print()
