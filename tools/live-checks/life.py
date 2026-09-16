"""The order's life, watched from all three surfaces at every step.

Not "did the API answer 200" but "does each role SEE the right thing" -- which
is the gap that once left the courier app showing 'you are offline' for every
courier while every endpoint returned 200.
"""
import json, time, urllib.request, urllib.error
B = "https://dowiz-api.sviatoslavsyniak.workers.dev"
SLUG = "dubin-sushi"
ok = fail = 0

def call(m, p, body=None, tok=None, raw=None, ctype="application/json"):
    time.sleep(0.6)
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    r = urllib.request.Request(B + p, data=data, method=m)
    if data is not None: r.add_header("content-type", ctype)
    if tok: r.add_header("authorization", "Bearer " + tok)
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
            b = f.read()
            try: return f.status, json.loads(b or b"null")
            except Exception: return f.status, b[:200].decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        b = e.read()
        try: return e.code, json.loads(b or b"null")
        except Exception: return e.code, b[:200].decode("utf-8", "replace")

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

def check(name, cond, detail=""):
    global ok, fail
    if cond: ok += 1; print(f"    ✓ {name}")
    else:    fail += 1; print(f"    ✗ {name}   {detail}")

print("РОЛІ")
own = token(call("POST", "/api/auth/login", {"email": "ana@dubin.al", "password": "dubin-owner"}), "access_token"); check("власник увійшов", bool(own))
eni = token(call("POST", "/api/courier/auth/login", {"phone": "+355691112233", "password": "courier-eni"}), "jwt"); check("кур'єр Eni увійшов", bool(eni))
_, c2 = call("POST", "/api/courier/auth/login", {"phone": "+355694445566", "password": "courier-blerim"})
blerim = c2.get("jwt"); check("кур'єр Blerim увійшов", bool(blerim), str(c2)[:80])

print("\n1. КЛІЄНТ ВІДКРИВАЄ ВІТРИНУ")
code, menu = call("GET", f"/api/public/locations/{SLUG}/menu")
L = menu["location"]
check("меню віддається", code == 200)
check("заклад відчинено", L["status"] == "open", L["status"])
check("тема закладу дійшла", (L.get("theme") or {}).get("seed") is not None)
check("прапорці дійшли", isinstance(L.get("features"), dict))
prods = [p for cat in menu["categories"] for p in cat["products"]]
check("страви є", len(prods) > 40, str(len(prods)))
check("алергени передано", any(p.get("allergens") is not None for p in prods))

print("\n2. КЛІЄНТ ЗАМОВЛЯЄ")
code, o = call("POST", f"/api/public/locations/{SLUG}/orders", {
    "items": [{"product_id": "item-01", "modifier_ids": [], "quantity": 2}],
    "contact": {"name": "Життєвий цикл", "phone": "+355690001234"},
    "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Taulantia 5"}},
    "tip": 100})
check("замовлення прийнято", code == 200, str(o)[:150])
oid = o.get("id"); ctok = o.get("access_token")
check("клієнт отримав свій ключ", bool(ctok))
check("статус PENDING", o.get("status") == "PENDING", str(o.get("status")))

print("\n   що бачить кожна роль одразу після замовлення:")
code, mine = call("GET", f"/api/order/{oid}", tok=ctok)
check("клієнт бачить своє замовлення", code == 200 and mine.get("id") == oid)
code, _ = call("GET", f"/api/order/{oid}")
check("без ключа — не бачить", code == 401, f"HTTP {code}")
_, olist = call("GET", "/api/owner/orders", tok=own)
row = next((x for x in olist["orders"] if x["id"] == oid), None)
check("власник бачить у черзі", row is not None)
check("  з іменем і телефоном", bool(row and row.get("contact", {}).get("phone")))
check("  з часом", bool(row and row.get("created_at_ms")))
_, tasks = call("GET", "/api/courier/tasks", tok=eni)
check("кур'єр НЕ бачить (ще не готове)",
      not any(x["id"] == oid for x in (tasks.get("available") or [])))
_, dash = call("GET", "/api/owner/dashboard", tok=own)
check("дашборд рахує як очікує", dash.get("pending", 0) >= 1, str(dash))

print("\n3. ВЛАСНИК ВЕДЕ ЗАМОВЛЕННЯ")
for verb, want in [("confirm", "CONFIRMED"), ("preparing", "PREPARING"), ("ready", "READY")]:
    code, v = call("POST", f"/api/owner/orders/{oid}/action",
                   {"location_id": "dubin-durres", "action": verb}, own)
    check(f"{verb} → {want}", code == 200 and v.get("status") == want, f"{code} {str(v)[:90]}")
    code2, seen = call("GET", f"/api/order/{oid}", tok=ctok)
    check(f"  клієнт бачить {want}", seen.get("status") == want, str(seen.get("status")))

print("\n4. КУР'ЄР")
call("POST", "/api/courier/shift", {"open": True}, eni)
call("POST", "/api/courier/shift", {"open": True}, blerim)
_, tasks = call("GET", "/api/courier/tasks", tok=eni)
check("на зміні", tasks.get("onShift") is True)
check("бачить готове замовлення", any(x["id"] == oid for x in (tasks.get("available") or [])))
code, _ = call("POST", f"/api/courier/orders/{oid}/accept", {}, eni)
check("Eni взяв", code == 200)
code, v = call("POST", f"/api/courier/orders/{oid}/accept", {}, blerim)
check("Blerim відхилений — уже зайняте", code == 409, f"{code} {str(v)[:70]}")
_, t2 = call("GET", "/api/courier/tasks", tok=blerim)
check("  і воно зникло з його списку",
      not any(x["id"] == oid for x in (t2.get("available") or [])))
_, t3 = call("GET", "/api/courier/tasks", tok=eni)
check("у Eni воно в «моїх»", any(x["id"] == oid for x in (t3.get("mine") or [])))
mine_card = next((x for x in (t3.get("mine") or []) if x["id"] == oid), {})
check("  з адресою", bool(mine_card.get("address", {}).get("line")), str(mine_card.get("address")))
check("  з телефоном клієнта", bool(mine_card.get("contact", {}).get("phone")))
code, _ = call("POST", f"/api/courier/orders/{oid}/pickup", {}, eni)
check("забрав", code == 200)
_, seen = call("GET", f"/api/order/{oid}", tok=ctok)
check("клієнт бачить IN_DELIVERY", seen.get("status") == "IN_DELIVERY", str(seen.get("status")))
code, _ = call("POST", f"/api/courier/orders/{oid}/deliver", {"cash_collected": 1900}, eni)
check("доставив", code == 200)

print("\n5. ПІСЛЯ ДОСТАВКИ")
_, seen = call("GET", f"/api/order/{oid}", tok=ctok)
check("клієнт бачить DELIVERED", seen.get("status") == "DELIVERED")
code, _ = call("POST", f"/api/order/{oid}/feedback", {"text": "швидко, дякую"}, ctok)
check("відгук приймається", code == 200)
_, olist = call("GET", "/api/owner/orders", tok=own)
row = next((x for x in olist["orders"] if x["id"] == oid), {})
check("власник бачить відгук", (row.get("feedback") or {}).get("text") == "швидко, дякую")
check("чайові збереглись через усі переходи", row.get("tip") == 100, str(row.get("tip")))
_, e = call("GET", "/api/courier/earnings", tok=eni)
check("кур'єр бачить чайові окремо", e["today"]["tips"] >= 100, str(e["today"]))
_, h = call("GET", "/api/courier/history", tok=eni)
check("у історії кур'єра", any(x["id"] == oid for x in h.get("history", [])))
_, cust = call("GET", "/api/owner/customers", tok=own)
check("клієнт у списку, приховано",
      any("•" in (x.get("phone") or "") for x in cust.get("customers", [])))

print(f"\n{ok} пройшло, {fail} впало")
