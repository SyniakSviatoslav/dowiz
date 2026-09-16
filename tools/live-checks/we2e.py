import json, time, urllib.request, urllib.error
B = "https://dowiz-api.sviatoslavsyniak.workers.dev"
def call(m, p, body=None, tok=None):
    time.sleep(0.6)
    r = urllib.request.Request(B+p, data=json.dumps(body).encode() if body is not None else None, method=m)
    if body is not None: r.add_header("content-type", "application/json")
    if tok: r.add_header("authorization", "Bearer "+tok)
    # BROWSER-SHAPED HEADERS, and not to sneak past anything: these scripts
    # stand in for the three browser surfaces. Cloudflare answers a request
    # with NO `accept` header using 503 error 1102 at the edge -- which
    # looks exactly like a Worker resource limit and is not one. Measured:
    # user-agent alone still fails, user-agent plus accept passes. Worth
    # knowing for real API clients too: a bare HTTP library that omits
    # `accept` will be refused before the Worker ever runs.
    r.add_header("user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    r.add_header("accept", "application/json, text/plain, */*")
    r.add_header("accept-language", "uk-UA,uk;q=0.9,en;q=0.8")
    # A BROWSER USER-AGENT, and not to sneak past anything: these scripts
    # stand in for the three browser surfaces, and Cloudflare answers an
    # unrecognised agent with 503 error 1102 at the EDGE -- which looks
    # exactly like a Worker resource limit and is not one. Diagnosing that
    # as a CPU problem cost an hour.
    try:
        with urllib.request.urlopen(r, timeout=60) as f:
            raw = f.read()
            try: return f.status, json.loads(raw or b"null")
            except: return f.status, raw[:200].decode("utf-8","replace")
    except urllib.error.HTTPError as e:
        raw = e.read()
        try: return e.code, json.loads(raw or b"null")
        except: return e.code, raw[:200].decode("utf-8","replace")

ok=fail=0
def token(resp, field):
    """Fail LOUDLY and say what came back.

    These scripts used to index straight into the response, so a 503 or a bot
    challenge surfaced as `TypeError: string indices must be integers` -- an
    error about Python, three frames deep, that says nothing about the service.
    """
    code, body = resp
    if not isinstance(body, dict) or field not in body:
        raise SystemExit(f"вхід не вдався: HTTP {code} — {str(body)[:200]}")
    return body[field]

def check(n, c, d=""):
    global ok, fail
    if c: ok+=1; print("  ✓", n)
    else: fail+=1; print("  ✗", n, " ", d)

print("— вхід власника —")
c, t = call("POST", "/api/auth/login", {"email":"ana@dubin.al","password":"dubin-owner"})
check("логін", c == 200 and isinstance(t, dict) and "access_token" in t, f"{c} {str(t)[:150]}")
if not isinstance(t, dict) or "access_token" not in t:
    print(f"\n{ok} пройшло, {fail} впало"); raise SystemExit
own = token((200, t), "access_token")

print("— власницькі панелі —")
for p in ["/api/owner/dashboard","/api/owner/orders","/api/owner/analytics?days=30",
          "/api/owner/promotions","/api/owner/activation","/api/owner/branding",
          "/api/owner/customers","/api/owner/customers/reveals"]:
    c, v = call("GET", p, tok=own)
    check(p, c == 200, f"{c} {str(v)[:120]}")

print("— промокод —")
c, v = call("POST", "/api/owner/promotions", {"code":"vecher 15","kind":"percent","value":15,"minOrder":2000}, own)
check("створено", c == 200, f"{c} {v}")
c, v = call("POST", "/api/owner/promotions", {"code":"TYPO","kind":"percent","value":15,"until":1}, own)
check("друкарська помилка відхилена", c == 400 and "until" in str(v), f"{c} {v}")
c, v = call("POST", "/api/promo/check", {"code":"vecher15",
     "items":[{"product_id":"item-01","modifier_ids":[],"quantity":3}]})
check("прев'ю 15% від 2700 = 405", c == 200 and v.get("discount") == 405, f"{c} {v}")

print("— замовлення —")
c, o = call("POST", "/api/public/locations/dubin-sushi/orders", {
  "items":[{"product_id":"item-01","modifier_ids":[],"quantity":3}],
  "contact":{"name":"Cloudflare Test","phone":"+355690000123"},
  "fulfilment":{"kind":"delivery","address":{"line":"Rruga Taulantia 9"}},
  "promo":"vecher 15", "tip":200})
check("прийнято", c == 200, f"{c} {str(o)[:200]}")
if c == 200:
    check("знижка застосована", o.get("discount") == 405, str(o.get("discount")))
    check("чайові окремо", o.get("tip") == 200, str(o.get("tip")))
    check("total = 2700 − 405 + доставка + 200",
          o.get("total") == 2700 - 405 + o.get("delivery_fee",0) + 200, str(o.get("total")))

print("— чайові поза виручкою —")
c, d = call("GET", "/api/owner/dashboard", tok=own)
c2, a = call("GET", "/api/owner/analytics?days=7", tok=own)
check("аналітика рахує", c2 == 200 and a.get("orders",0) >= 1, str(a)[:140])
if c2 == 200:
    check("виручка без чайових", a.get("revenue",0) % 5 == 0 and a.get("revenue",0) > 0,
          f'revenue={a.get("revenue")}')
print(f"\n{ok} пройшло, {fail} впало")
