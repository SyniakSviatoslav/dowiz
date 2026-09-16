"""Delivery zones, colour extraction and voice — against the deployment."""
import json, time, urllib.request, urllib.error
B = "https://dowiz-api.sviatoslavsyniak.workers.dev"
H = {"content-type": "application/json", "accept": "application/json",
     "user-agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                   "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"}
ok = fail = 0

def call(m, p, body=None, tok=None):
    time.sleep(0.7)
    r = urllib.request.Request(
        B + p, data=json.dumps(body).encode() if body is not None else None, method=m)
    for k, v in H.items():
        r.add_header(k, v)
    if tok:
        r.add_header("authorization", "Bearer " + tok)
    try:
        with urllib.request.urlopen(r, timeout=60) as f:
            b = f.read()
            try: return f.status, json.loads(b or b"null")
            except Exception: return f.status, b[:160].decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        b = e.read()
        try: return e.code, json.loads(b or b"null")
        except Exception: return e.code, b[:160].decode("utf-8", "replace")

def token(resp, field):
    code, body = resp
    if not isinstance(body, dict) or field not in body:
        raise SystemExit(f"вхід не вдався: HTTP {code} — {str(body)[:200]}")
    return body[field]

def chk(n, c, d=""):
    global ok, fail
    if c: ok += 1; print(f"  ✓ {n}")
    else: fail += 1; print(f"  ✗ {n}   {d}")

own = token(call("POST", "/api/auth/login",
                 {"email": "ana@dubin.al", "password": "dubin-owner"}), "access_token")
jwt = token(call("POST", "/api/courier/auth/login",
                 {"phone": "+355691112233", "password": "courier-eni"}), "jwt")

print("ЗОНИ ДОСТАВКИ")
Z = {"kind": "circle", "lat": 41323000, "lon": 19445000, "radius_m": 3000}
code, v = call("POST", "/api/owner/zones", {"zones": [Z]}, own)
chk("коло збережено", code == 200 and v.get("zones") == 1, f"{code} {v}")
_, v = call("GET", "/api/public/reach?lat_udeg=41323500&lon_udeg=19445500")
chk("адреса всередині", v.get("reach") == "inside", str(v))
_, v = call("GET", "/api/public/reach?lat_udeg=41900000&lon_udeg=19900000")
chk("адреса поза зоною", v.get("reach") == "outside", str(v))
chk("  і сказано наскільки", isinstance(v.get("metresAway"), int), str(v.get("metresAway")))
_, v = call("GET", "/api/public/reach")
# UNKNOWN, not "outside": a customer who did not share a location has not been
# refused, they have not been asked. Answering "outside" would turn a missing
# fact into a rejection.
chk("без координат — невідомо", v.get("reach") == "unknown", str(v))
code, v = call("POST", "/api/owner/zones", {"zones": [{"kind": "circle", "lat": 1}]}, own)
chk("неповне коло відхилено", code == 400)
chk("  і сказано який формат", "radius_m" in str(v), str(v)[:120])
_, menu = call("GET", "/api/public/locations/dubin-sushi/menu")
chk("вітрина знає, що зони є", menu["location"]["hasDeliveryZones"] is True)
call("POST", "/api/owner/zones", {"zones": []}, own)

print("\nКОЛІР З ФОТО")
code, v = call("POST", "/api/owner/branding/extract",
               {"pixels": "d69a3d" * 40 + "061b1a" * 30 + "f5efe5" * 30}, own)
chk("зразки знайдено", code == 200 and len(v.get("swatches", [])) > 0, str(v)[:110])
chk("  кожен уже перевірено на контраст",
    all("passes" in s for s in v.get("swatches", [])), str(v)[:100])
code, _ = call("POST", "/api/owner/branding/extract", {"pixels": "не hex"}, own)
chk("сміття відхилено", code == 400, f"{code}")

print("\nГОЛОС")
_, v = call("POST", "/api/voice", {"transcript": "скільки замовлень"}, own)
chk("питання виконується одразу", v.get("action") == "status", str(v)[:110])
_, v = call("POST", "/api/voice", {"transcript": "підтверди останнє"}, own)
chk("дія потребує підтвердження", v.get("needsConfirmation") is True, str(v)[:110])
chk("  з читанням уголос", bool(v.get("readback")), str(v.get("readback")))
tok_v = v.get("token")
chk("  і з токеном", bool(tok_v))
_, v2 = call("POST", "/api/voice", {"confirm": tok_v}, own)
chk("підтвердження приймається", v2.get("understood") is True, str(v2)[:110])
# The token is bound to the speaker, so a courier cannot confirm an owner's
# proposal even holding the string.
_, v3 = call("POST", "/api/voice", {"confirm": tok_v}, jwt)
chk("чуже підтвердження відхилено", v3.get("understood") is False, str(v3)[:110])
# Anything that is not a command becomes a QUESTION for the assistant rather
# than a refusal: "I did not understand" is the answer of last resort, not the
# first one.
_, v4 = call("POST", "/api/voice", {"transcript": "чому так довго"}, own)
chk("не команда — це питання", v4.get("action") == "ask", str(v4)[:110])
code, _ = call("POST", "/api/voice", {"transcript": "скільки замовлень"})
chk("без входу — ні", code >= 401, f"{code}")
print(f"\n{ok} пройшло, {fail} впало")
