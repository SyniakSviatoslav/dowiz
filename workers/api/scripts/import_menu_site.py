#!/usr/bin/env python3
"""Port the venue's published menu site into a hub bootstrap bundle.

Sushi Durrës publishes its real catalogue as a static site
(`sushi-durres-menu.netlify.app`): 165 dishes in 21 categories, priced, with
ingredient lists in Albanian, English and Ukrainian, and 77 photographs. The
hub's own copy had 18 dishes, 6 categories and no photograph at all, which is
why the storefront drew name-keyed plates where food should be.

That site carries its whole catalogue in one `<script id="menu-data">` block, so
it is read as DATA rather than scraped out of the markup.

    python3 scripts/import_menu_site.py --out ./import
    # → import/bundle.json   POST /api/bootstrap
    # → import/i18n.json     PATCH /api/owner/products/:id  (translations)
    # → import/photos.json   POST /api/owner/products/:id/image
    # → import/menu.json     the same catalogue in the hub's PUBLIC menu shape,
    #                        for serving to a local stand before anything is
    #                        written to a live venue.

WHAT THIS DELIBERATELY DOES NOT DO: declare allergens. The source has none, and
this product treats a guessed allergen as worse than a missing one -- a dish
listing "salmon" is not thereby declared to contain fish by anybody accountable.
Every imported dish arrives with allergens ABSENT, which every dowiz surface
renders as "not declared": the loud state, on purpose. The venue declares them
in the console, where the publish gate can hold them to it.
"""
import argparse, json, os, re, sys, urllib.request

SITE = "https://sushi-durres-menu.netlify.app/"
SLUG = "sushi-durres"
# Lek has no minor unit, so the printed figure IS the integer amount. Any other
# currency would need its own rule here rather than a multiply.
PRICE = re.compile(r"(\d[\d\s.,]*)\s*(lek|lekë|лек)", re.I)


# A User-Agent that names this tool. Python's default is `Python-urllib/3.x`,
# which the hub's edge answers with a 403 -- so the venue record came back
# "NOT FOUND" and the fixture quietly fell back to the design's dollars.
UA = {"user-agent": "dowiz-import/1.0 (+https://dowiz.org)"}


def fetch(url: str) -> bytes:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=90) as r:
        return r.read()


def site_data(src: str) -> dict:
    raw = fetch(src).decode("utf-8", "replace") if src.startswith("http") else open(src, encoding="utf-8").read()
    m = re.search(r'<script id="menu-data" type="application/json">(.*?)</script>', raw, re.S)
    if not m:
        sys.exit("import: the page has no <script id=\"menu-data\"> block")
    return json.loads(m.group(1))


def minor_units(text: str) -> int | None:
    """'1000 lek' -> 1000. Returns None rather than guessing."""
    m = PRICE.search(text or "")
    if not m:
        return None
    digits = re.sub(r"[^\d]", "", m.group(1))
    return int(digits) if digits else None


def tr(item: dict, locale: str, field: str):
    return ((item.get("translations") or {}).get(locale) or {}).get(field)


def joined(ingredients) -> str | None:
    if not ingredients:
        return None
    return ", ".join(str(x) for x in ingredients)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=SITE, help="the menu site, or a saved copy of its HTML")
    ap.add_argument("--slug", default=SLUG)
    ap.add_argument("--out", default="./import")
    ap.add_argument("--photos", default=None, help="directory of already-downloaded photographs")
    ap.add_argument("--hub", default=None,
                    help="a live hub to read the venue record from, so the fixture carries the "
                         "venue's real currency and delivery terms")
    args = ap.parse_args()

    data = site_data(args.src)
    items, cats = data.get("items", []), data.get("categories", [])
    if not items or not cats:
        sys.exit("import: the data block has no items or no categories")

    os.makedirs(args.out, exist_ok=True)

    # ── Categories, in the order the site lists them ──
    out_cats, order = [], {}
    for n, c in enumerate(cats):
        title = c.get("title") or {}
        order[c["id"]] = n
        out_cats.append({
            "id": c["id"],
            "name": title.get("sq") or title.get("en") or c["id"],
            "sortOrder": n,
        })

    # ── Products ──
    out_products, i18n, photos, skipped = [], {}, {}, []
    for n, it in enumerate(items):
        price = minor_units(tr(it, "sq", "price") or tr(it, "en", "price") or "")
        if price is None:
            skipped.append({"id": it["id"], "why": "no price could be read"})
            continue
        # A dish may be filed under several headings; the hub's record holds one,
        # and the FIRST is the site's own primary listing.
        cat = (it.get("categories") or [None])[0]
        sq = joined(tr(it, "sq", "ingredients"))
        out_products.append({
            "id": it["id"],
            "categoryId": cat,
            "name": it["name"],
            "description": sq,
            "price": price,
            "available": True,
            # Absent, not empty: see the module docstring.
            "allergens": None,
            "ingredients": tr(it, "sq", "ingredients") or None,
            "imageUrl": None,          # filled by the photo upload, never guessed
            "sortOrder": n,
        })
        for loc in ("uk", "en"):
            text = joined(tr(it, loc, "ingredients"))
            if text:
                i18n.setdefault(it["id"], {})[loc] = {"description": text}
        if it.get("image"):
            photos[it["id"]] = it["image"]

    bundle = {
        "location": {"id": args.slug, "slug": args.slug},
        "categories": out_cats,
        "products": out_products,
    }

    # ── The same catalogue in the PUBLIC menu shape ──
    # So the whole import can be looked at in a browser, on a local stand,
    # before one byte is written to a live venue.
    by_cat: dict[str, list] = {}
    for p in out_products:
        # The fixture points at the photograph where a local stand can serve it.
        # The BUNDLE deliberately leaves `imageUrl` null: in a real hub the URL
        # is the content hash the upload returns, and inventing one here would
        # publish a menu of broken images.
        shown = dict(p)
        if p["id"] in photos:
            shown["imageUrl"] = "/media/" + p["id"] + os.path.splitext(photos[p["id"]])[1]
        by_cat.setdefault(p["categoryId"], []).append(shown)
    # THE VENUE RECORD TRAVELS WITH THE FIXTURE.
    #
    # Currency and delivery terms belong to the venue, not to this catalogue, so
    # the fixture has to carry them or every price it renders falls back to the
    # design's dollars. Fetched ONCE, here, rather than by whatever serves the
    # fixture later: a stand that asks a hub per request is a stand that is
    # right most of the time, which is the worst kind.
    location = None
    if args.hub:
        try:
            raw = fetch(f"{args.hub}/api/public/locations/{args.slug}/menu?locale=sq")
            location = json.loads(raw).get("location")
            print("venue record: " + ("read from " + args.hub if location else "NOT FOUND"))
        except Exception as e:
            print(f"venue record: could not be read ({e}) -- the fixture will show the frame's prices")

    menu = {
        "location": location,
        "categories": [
            {"id": c["id"], "name": c["name"], "sortOrder": c["sortOrder"],
             "products": by_cat.get(c["id"], [])}
            for c in out_cats if by_cat.get(c["id"])
        ],
        "stripePublishableKey": None,
    }

    for name, payload in (("bundle.json", bundle), ("i18n.json", i18n),
                          ("photos.json", photos), ("menu.json", menu)):
        with open(os.path.join(args.out, name), "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=1)

    print(f"categories {len(out_cats)}  products {len(out_products)}  "
          f"photos {len(photos)}  translated {len(i18n)}  skipped {len(skipped)}")
    for s in skipped:
        print(f"  skipped {s['id']}: {s['why']}")
    print(f"NO ALLERGENS DECLARED for {len(out_products)} dishes — the venue must "
          f"declare them in the console; every surface shows 'not declared' until it does.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
