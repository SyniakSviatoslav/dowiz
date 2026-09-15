#!/usr/bin/env python3
"""Port the live client data off the old platform into a hub bootstrap bundle.

The old Fly deployment still serves Dubin & Sushi's real catalogue, and it is the
only authoritative copy: the repo's design/dubin-sushi-menu.json is a 2026-07
snapshot and the local vendor .mjs is another. Rather than pick between two
stale copies, take what production actually answers with today.

Emits the bundle `POST /api/bootstrap` consumes. Ids are PRESERVED, so a row
stays matchable against the old system while both are alive.

  python3 scripts/import_from_staging.py > bundle.json
  curl -X POST "$HUB/api/bootstrap" \
       -H "x-dowiz-bootstrap: $BOOTSTRAP_SECRET" \
       -H 'content-type: application/json' --data @bundle.json
"""
import json, sys, urllib.request

BASE = "https://dowiz-staging.fly.dev"
SLUG = "demo"


def get(path):
    with urllib.request.urlopen(BASE + path, timeout=60) as r:
        return json.load(r)


def udeg(x):
    """Degrees -> integer micro-degrees. The same rule the order aggregate
    follows: nothing stored may depend on float rounding at read time."""
    return None if x is None else int(round(float(x) * 1_000_000))


info = get(f"/public/locations/{SLUG}/info")
menu = get(f"/public/locations/{SLUG}/menu")

location = {
    "id": info["id"],
    "slug": info["slug"],
    "name": info["name"],
    "phone": info["phone"],
    "address": info.get("address"),
    "status": info.get("status", "closed"),
    "closes_at": info.get("closesAt"),
    "timezone": info.get("timezone") or "Europe/Tirane",
    "delivery_eta": info.get("deliveryEta") or "30-45",
    # Money is integer minor units everywhere. int() here is a refusal to carry
    # a float into a store that must fold identically on every node.
    "delivery_fee": int(info.get("deliveryFeeFlat") or 0),
    "free_delivery_threshold": (
        int(info["freeDeliveryThreshold"]) if info.get("freeDeliveryThreshold") is not None else None
    ),
    "min_order": int(info.get("minOrderValue") or 0),
    "currency_code": info.get("currency_code", "ALL"),
    "menu_version": int(menu.get("menu_version") or 1),
    "supported_locales": json.dumps(menu.get("supported_locales") or ["sq"]),
    "default_locale": menu.get("default_locale", "sq"),
    "lat_udeg": udeg(info.get("lat")),
    "lon_udeg": udeg(info.get("lng")),
    "delivery_paused": 0,
    "weekly_hours": info.get("weeklyHours") or [],
}

categories, products = [], []
for ci, cat in enumerate(menu.get("categories", [])):
    categories.append({
        "id": cat["id"],
        "name": cat["name"],
        "sortOrder": int(cat.get("sort_order", ci)),
    })
    for pi, p in enumerate(cat.get("products", [])):
        products.append({
            "id": p["id"],
            "categoryId": cat["id"],
            "name": p["name"],
            "description": p.get("description"),
            "price": int(p["price"]),
            "available": bool(p.get("available", True)),
            "unavailableNote": None,
            "imageUrl": p.get("imageUrl"),
            "allergens": p.get("allergens") or [],
            "prepTimeMinutes": p.get("prep_time_minutes"),
            "sortOrder": int(p.get("sortOrder", pi)),
        })

bundle = {"location": location, "categories": categories, "products": products}
json.dump(bundle, sys.stdout, ensure_ascii=False, indent=1)
sys.stderr.write(
    f"location={location['name']!r} categories={len(categories)} products={len(products)}\n"
)
