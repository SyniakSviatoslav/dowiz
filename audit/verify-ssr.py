import urllib.request
try:
    resp = urllib.request.urlopen(urllib.request.Request('https://dowiz.fly.dev/s/demo'), timeout=15)
    html = resp.read().decode()
    print(f'SSR /s/demo: {resp.status} ({len(html)} bytes)')
    lang_sq = 'YES' if 'lang="sq"' in html else 'NO'
    print(f'lang="sq": {lang_sq}')
    dsq = 'YES' if 'data-text-sq' in html else 'NO'
    den = 'YES' if 'data-text-en' in html else 'NO'
    print(f'data-text-sq: {dsq}')
    print(f'data-text-en: {den}')
    hsts = resp.getheader('Strict-Transport-Security')
    csp = resp.getheader('Content-Security-Policy') or 'absent'
    print(f'HSTS: {hsts or "absent"}')
    print(f'CSP: {csp[:100]}')
    cc = resp.getheader('Cache-Control') or 'absent'
    print(f'Cache-Control: {cc}')
except Exception as e:
    print(f'SSR error: {e}')
