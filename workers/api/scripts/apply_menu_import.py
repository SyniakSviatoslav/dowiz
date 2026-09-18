#!/usr/bin/env python3
"""Apply an import produced by `import_menu_site.py` to a live hub.

Three writes, in the order that keeps the venue servable at every step:

  1. the catalogue        POST  /api/bootstrap                     (bootstrap secret)
  2. the photographs      POST  /api/owner/products/:id/image      (owner login)
  3. the other languages  PATCH /api/owner/products/:id            (owner login)

Nothing here guesses a credential. Set them in the environment:

    export HUB=https://sushi-durres.dowiz.org
    export BOOTSTRAP_SECRET=…          # only for step 1
    export OWNER_EMAIL=… OWNER_PASSWORD=…   # steps 2 and 3

    python3 scripts/apply_menu_import.py --dir ./import --photos ./refimg --dry-run
    python3 scripts/apply_menu_import.py --dir ./import --photos ./refimg

`--dry-run` performs every check and every read and writes NOTHING, so the whole
import can be rehearsed against the real hub before it changes anything.

PHOTOGRAPHS ARE CHECKED BEFORE THEY ARE SENT. A file that starts like an image
and stops -- a dropped download, a half-written file -- used to be accepted by
the hub, stored, and served with a 200 that every browser drew as nothing. The
hub now refuses those (`dowiz_hub::media::complete`), and this script refuses
them one step earlier, where the error can still name the file on disk.
"""
import argparse, json, os, sys, urllib.error, urllib.request

# Cloudflare's bot rules answer urllib's default User-Agent with a 403 whose
# body is "error code: 1010" -- the edge's own signature block, not the hub's
# 403. It looks exactly like a rejected secret and cost one debugging round, so
# every request from this script names a browser.
UA = ('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0 Safari/537.36')


def http(method, url, data=None, headers=None, timeout=120):
    h = {"user-agent": UA, "accept": "*/*", **(headers or {})}
    req = urllib.request.Request(url, data=data, method=method, headers=h)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:                      # a refused connection is not a 500
        return 0, str(e).encode()


def whole_image(b: bytes) -> tuple[bool, str]:
    """The same rule the hub applies, so a refusal happens here with a filename."""
    if b[:3] == b"\xff\xd8\xff":
        t = b.rstrip(b"\x00")
        if not t.endswith(b"\xff\xd9"):        return False, "jpeg with no end marker (truncated)"
        if b"\xff\xda" not in t:               return False, "jpeg with no scan"
        if not any(m in t for m in (b"\xff\xc0", b"\xff\xc1", b"\xff\xc2")):
            return False, "jpeg with no frame header"
        return True, "jpeg"
    if b[:8] == b"\x89PNG\r\n\x1a\n":
        return (b.rstrip(b"\x00").endswith(b"IEND\xae\x42\x60\x82"), "png")
    if b[:4] == b"RIFF" and b[8:12] == b"WEBP":  return True, "webp"
    if b[:6] in (b"GIF87a", b"GIF89a"):          return True, "gif"
    return False, "not a jpeg, png, webp or gif"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", default="./import")
    # Default to where the import step writes them, so the two halves of the
    # pipeline agree without a flag anybody has to remember.
    ap.add_argument("--photos", default=None)
    ap.add_argument("--hub", default=os.environ.get("HUB", ""))
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--skip-catalogue", action="store_true")
    a = ap.parse_args()
    if not a.hub:
        sys.exit("apply: set HUB (e.g. https://sushi-durres.dowiz.org)")

    if not a.photos:
        a.photos = os.path.join(a.dir, "photos")
    load = lambda n: json.load(open(os.path.join(a.dir, n), encoding="utf-8"))
    bundle, photos, i18n = load("bundle.json"), load("photos.json"), load("i18n.json")

    # ── Every photograph is read and checked BEFORE anything is written ──
    files, broken, missing = {}, [], []
    for pid, rel in photos.items():
        path = os.path.join(a.photos, pid + os.path.splitext(rel)[1])
        if not os.path.exists(path):
            missing.append(pid); continue
        b = open(path, "rb").read()
        ok, why = whole_image(b)
        (files.setdefault(pid, b) if ok else broken.append((pid, why)))
    print(f"photographs: {len(files)} whole, {len(broken)} broken, {len(missing)} missing")
    for pid, why in broken:
        print(f"  BROKEN {pid}: {why}")
    if broken:
        sys.exit("apply: refusing to upload a broken photograph — fix or remove it first")

    print(f"catalogue: {len(bundle['categories'])} categories, {len(bundle['products'])} products")
    print(f"translations: {len(i18n)} products")
    if a.dry_run:
        print("DRY RUN — nothing was written")
        return 0

    # 1. the catalogue
    if not a.skip_catalogue:
        secret = os.environ.get("BOOTSTRAP_SECRET")
        if not secret:
            sys.exit("apply: BOOTSTRAP_SECRET is not set (or pass --skip-catalogue)")
        code, body = http("POST", f"{a.hub}/api/bootstrap",
                          json.dumps(bundle).encode(),
                          {"content-type": "application/json", "x-dowiz-bootstrap": secret})
        print(f"bootstrap → {code} {body[:200].decode('utf-8','replace')}")
        if code != 200:
            sys.exit("apply: the catalogue was refused; nothing else was attempted")

    # owner session for steps 2 and 3
    email, password = os.environ.get("OWNER_EMAIL"), os.environ.get("OWNER_PASSWORD")
    if not (email and password):
        print("apply: no OWNER_EMAIL/OWNER_PASSWORD — photographs, translations, logo and colours skipped")
        return 0
    code, body = http("POST", f"{a.hub}/api/auth/login",
                      json.dumps({"email": email, "password": password}).encode(),
                      {"content-type": "application/json"})
    if code != 200:
        sys.exit(f"apply: owner login failed {code}: {body[:200].decode('utf-8','replace')}")
    token = json.loads(body)["access_token"]
    auth = {"authorization": f"Bearer {token}"}

    # 2. the photographs
    ok = bad = 0
    for pid, b in files.items():
        code, resp = http("POST", f"{a.hub}/api/owner/products/{pid}/image", b,
                          {**auth, "content-type": "application/octet-stream"})
        if code == 200: ok += 1
        else:
            bad += 1
            print(f"  photo {pid} → {code} {resp[:120].decode('utf-8','replace')}")
    print(f"photographs uploaded: {ok} ok, {bad} refused")

    # 3. the other languages
    ok = bad = 0
    loc = bundle["location"]["id"]
    for pid, by_locale in i18n.items():
        payload = {"location_id": loc, "translations": by_locale}
        # POST, not PATCH. The route is `.post_async("/api/owner/products/:id")`
        # and a PATCH to it is a 405 -- seventy-three of them, once each.
        code, resp = http("POST", f"{a.hub}/api/owner/products/{pid}", json.dumps(payload).encode(),
                          {**auth, "content-type": "application/json"})
        if code == 200: ok += 1
        else:
            bad += 1
            print(f"  i18n {pid} → {code} {resp[:120].decode('utf-8','replace')}")
    print(f"translations written: {ok} ok, {bad} refused")

    # 4. the venue's own mark
    #
    # LAST, DELIBERATELY. A logo on a venue whose catalogue was refused is a
    # brand on an empty shop; the order here is the order in which the venue
    # stays servable, and the mark is the only step whose failure changes
    # nothing a customer can order from.
    brand_path = os.path.join(a.dir, "brand.json")
    brand = json.load(open(brand_path, encoding="utf-8")) if os.path.exists(brand_path) else {}
    if brand.get("logoFile"):
        blob = open(os.path.join(a.dir, brand["logoFile"]), "rb").read()
        whole, why = whole_image(blob)
        if not whole:
            print(f"  logo: NOT UPLOADED — {why}")
        else:
            code, resp = http("POST", f"{a.hub}/api/owner/logo", blob,
                              {**auth, "content-type": "application/octet-stream"})
            print(f"logo → {code} {resp[:160].decode('utf-8','replace')}")

    # 5. the venue's own colours
    #
    # The hub DERIVES a whole theme from the seed and refuses one whose text
    # would be unreadable (409 with the contrast it measured). A refusal is
    # printed and not worked around: a venue's brand is not worth a customer
    # who cannot read the price.
    # 6. where the venue is, when it opens, and what Google says
    #
    # `place.json` comes from `harvest_google_place.mjs`. Only the fields that
    # were actually read are sent: the hub writes what it is given and leaves
    # the rest alone, so a harvest that could not find the telephone number does
    # not erase the one the owner typed.
    place_path = os.path.join(a.dir, "place.json")
    if os.path.exists(place_path):
        pl = json.load(open(place_path, encoding="utf-8"))
        payload = {}
        if pl.get("address"): payload["address"] = pl["address"]
        if pl.get("lat") is not None and pl.get("lng") is not None:
            payload["lat"], payload["lng"] = pl["lat"], pl["lng"]
        if pl.get("hoursMinutes"): payload["hours"] = pl["hoursMinutes"]
        goog = {k: pl[k] for k in ("url", "rating", "reviewCount", "reviews", "harvestedAt")
                if pl.get(k) is not None}
        if goog: payload["google"] = goog
        if payload:
            code, resp = http("POST", f"{a.hub}/api/owner/place", json.dumps(payload).encode(),
                              {**auth, "content-type": "application/json"})
            print(f"place {sorted(payload)} → {code} {resp[:160].decode('utf-8','replace')}")

    if brand.get("primary"):
        payload = {k: v for k, v in
                   (("primary", brand.get("primary")), ("ink", brand.get("ink")),
                    ("paper", brand.get("paper"))) if v}
        code, resp = http("POST", f"{a.hub}/api/owner/branding", json.dumps(payload).encode(),
                          {**auth, "content-type": "application/json"})
        print(f"branding {payload} → {code} {resp[:200].decode('utf-8','replace')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
