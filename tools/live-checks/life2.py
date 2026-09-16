"""The other lifecycles: the shelf, the refusal, and a closed venue."""
import json, time, urllib.request, urllib.error
B = "https://dowiz-api.sviatoslavsyniak.workers.dev"; SLUG = "dubin-sushi"
ok = fail = 0
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
            b=f.read()
            try: return f.status, json.loads(b or b"null")
            except Exception: return f.status, b[:200].decode("utf-8","replace")
    except urllib.error.HTTPError as e:
        b=e.read()
        try: return e.code, json.loads(b or b"null")
        except Exception: return e.code, b[:200].decode("utf-8","replace")
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

def check(n,c,d=""):
    global ok,fail
    if c: ok+=1; print(f"    ✓ {n}")
    else: fail+=1; print(f"    ✗ {n}   {d}")

own = token(call("POST","/api/auth/login",{"email":"ana@dubin.al","password":"dubin-owner"}), "access_token")

print("ІНВЕНТАР")
code,_ = call("POST","/api/owner/supplies",{"id":"salmon","name":"Лосось","unit":"г","lowAt":500},own)
check("інгредієнт заведено", code==200)
code,v = call("GET","/api/owner/stock",tok=own)
check("полиця читається", code==200 and isinstance(v, dict), f"HTTP {code}: {str(v)[:120]}")
if not isinstance(v, dict):
    raise SystemExit("полиця не віддалась — далі перевіряти нічого")
# DELTAS, NOT ABSOLUTES. The shelf is a running ledger: asserting "it holds
# 2000" passes once and fails for ever after, which is a test that measures how
# many times it has been run rather than whether the arithmetic is right.
base = next((s for s in v.get("supplies",[]) if s["id"]=="salmon"), {}).get("onHand", 0)
check("  стартовий рівень прочитано", isinstance(base, int), str(base))
code,_ = call("POST","/api/owner/stock/received",{"item":"salmon","qty":2000},own)
check("прийом записано", code==200)
_,v = call("GET","/api/owner/stock",tok=own)
s1 = next(s for s in v["supplies"] if s["id"]=="salmon")
check("  +2000 на полиці", s1["onHand"]==base+2000, f'{base} → {s1["onHand"]}')
code,_ = call("POST","/api/owner/stock/wasted",{"item":"salmon","qty":300,"reason":"spoiled"},own)
check("списання записано", code==200)
_,v = call("GET","/api/owner/stock",tok=own)
s2 = next(s for s in v["supplies"] if s["id"]=="salmon")
check("  −300 після списання", s2["onHand"]==base+1700, str(s2["onHand"]))
check("  поріг не спрацював", s2["low"] is False, f'{s2["available"]} > {s2["lowAt"]}')
# A stocktake REPLACES the level with what was counted, whatever the ledger
# believed: the shelf is the authority, and the difference is recorded as one.
code,_ = call("POST","/api/owner/stock/stocktake",{"item":"salmon","observed":40},own)
check("перелік записано", code==200)
_,v = call("GET","/api/owner/stock",tok=own)
s3 = next(s for s in v["supplies"] if s["id"]=="salmon")
check("  перелік переписує рівень", s3["onHand"]==40, str(s3["onHand"]))
check("  і тепер мало", s3["low"] is True, f'{s3["available"]} <= {s3["lowAt"]}')
code,v = call("POST","/api/owner/stock/received",{"item":"nosuch","qty":1},own)
check("невідомий інгредієнт відхилено", code==404, f"{code}")
code,v = call("POST","/api/owner/stock/teleported",{"item":"salmon","qty":1},own)
check("вигаданий рух відхилено", code==400, f"{code}")

print("\nВІДМОВА ЗАМОВЛЕННЯ")
_, o = call("POST", f"/api/public/locations/{SLUG}/orders", {
    "items":[{"product_id":"item-02","modifier_ids":[],"quantity":1}],
    "contact":{"name":"Відмова","phone":"+355690005555"},
    "fulfilment":{"kind":"pickup"}})
oid = o["id"]; ctok = o["access_token"]
code,v = call("POST", f"/api/owner/orders/{oid}/action",
              {"location_id":"dubin-durres","action":"reject","reason":"немає риби"},own)
check("відхилено", code==200 and v.get("status")=="REJECTED", f"{code} {str(v)[:80]}")
_,seen = call("GET", f"/api/order/{oid}", tok=ctok)
check("клієнт бачить причину", seen.get("rejection_reason")=="немає риби", str(seen.get("rejection_reason")))
_,d = call("GET","/api/owner/analytics?days=7",tok=own)
check("відхилене не в виручці", d.get("rejected",0)>=1, str(d.get("rejected")))

print("\nЗАЧИНЕНИЙ ЗАКЛАД")
code,_ = call("POST","/api/owner/location",{"location_id":"dubin-durres","status":"closed"},own)
check("зачинено", code==200)
code,v = call("POST", f"/api/public/locations/{SLUG}/orders", {
    "items":[{"product_id":"item-01","modifier_ids":[],"quantity":1}],
    "contact":{"name":"Пізно","phone":"+355690006666"},
    "fulfilment":{"kind":"pickup"}})
check("замовлення відхилено", code >= 400, f"HTTP {code} {str(v)[:70]}")
_,menu = call("GET", f"/api/public/locations/{SLUG}/menu")
check("вітрина каже «зачинено»", menu["location"]["status"]=="closed")
check("  і каже чому", menu["location"].get("closedReason")=="manual", str(menu["location"].get("closedReason")))
code,_ = call("POST","/api/owner/location",{"location_id":"dubin-durres","status":"open"},own)
check("відчинено назад", code==200)
print(f"\n{ok} пройшло, {fail} впало")
