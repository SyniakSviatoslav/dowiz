#!/usr/bin/env python3
"""Port the live client data off the old platform into the new D1 schema.

The old Fly deployment still serves Dubin & Sushi's real catalogue. It is the
only authoritative copy -- the repo's design/dubin-sushi-menu.json is a 2026-07
snapshot and the local vendor .mjs is another. Rather than pick one, take what
production actually answers with today.

Emits SQL to stdout. Ids are PRESERVED so the port is 1:1 and a row can still be
matched against the old system while both exist.
"""
import json, sys, urllib.request

BASE = "https://dowiz-staging.fly.dev"
SLUG = "demo"

def get(path):
    with urllib.request.urlopen(BASE + path, timeout=60) as r:
        return json.load(r)

def q(v):
    if v is None: return "NULL"
    if isinstance(v, bool): return "1" if v else "0"
    if isinstance(v, (int,)): return str(v)
    if isinstance(v, float):
        raise SystemExit("refusing to emit a float: money and coords are integers here")
    return "'" + str(v).replace("'", "''") + "'"

def udeg(x):
    """Degrees -> integer micro-degrees. Same rule as the order aggregate: the
    stored value must not depend on float rounding at read time."""
    return "NULL" if x is None else str(int(round(float(x) * 1_000_000)))

info = get(f"/public/locations/{SLUG}/info")
menu = get(f"/public/locations/{SLUG}/menu")
NOW = "strftime('%s','now')*1000"

out = []
out.append("DELETE FROM modifiers; DELETE FROM modifier_groups; DELETE FROM products; DELETE FROM categories; DELETE FROM locations;")
out.append(
  "INSERT INTO locations (id,slug,name,phone,address,status,closes_at,timezone,delivery_eta,"
  "delivery_fee,min_order,currency_code,menu_version,supported_locales,default_locale,"
  "lat_udeg,lon_udeg,delivery_paused,created_at_ms,updated_at_ms,weekly_hours,"
  "free_delivery_threshold,tax_rate_micro,price_includes_tax,currency_minor_unit,google_rating) VALUES ("
  + ",".join([
      q(info["id"]), q(info["slug"]), q(info["name"]), q(info["phone"]), q(info.get("address")),
      q(info.get("status","closed")), q(info.get("closesAt")), q(info.get("timezone") or "Europe/Tirane"),
      q(info.get("deliveryEta") or "30-45"),
      q(int(info.get("deliveryFeeFlat") or 0)), q(int(info.get("minOrderValue") or 0)),
      q(info.get("currency_code","ALL")), q(int(menu.get("menu_version") or 1)),
      q(json.dumps(menu.get("supported_locales") or ["sq"])), q(menu.get("default_locale","sq")),
      udeg(info.get("lat")), udeg(info.get("lng")),
      "0", NOW, NOW,
      q(json.dumps(info.get("weeklyHours") or [])),
      q(int(info["freeDeliveryThreshold"])) if info.get("freeDeliveryThreshold") is not None else "NULL",
      q(int(round(float(info.get("taxRate") or 0) * 1_000_000))),
      q(bool(info.get("priceIncludesTax", True))),
      q(int(info.get("currency_minor_unit") or 0)),
      q(str(info["googleRating"])) if info.get("googleRating") is not None else "NULL",
  ]) + ");")

loc = info["id"]
np = nc = nm = 0
for ci, cat in enumerate(menu.get("categories", [])):
    out.append("INSERT INTO categories (id,location_id,name,sort_order,created_at_ms) VALUES ("
        + ",".join([q(cat["id"]), q(loc), q(cat["name"]), q(int(cat.get("sort_order", ci))), NOW]) + ");")
    nc += 1
    for pi, p in enumerate(cat.get("products", [])):
        out.append("INSERT INTO products (id,location_id,category_id,name,description,price,available,"
            "image_url,primary_media_id,allergens,calories,prep_time_minutes,attributes,sort_order,"
            "created_at_ms,updated_at_ms) VALUES ("
            + ",".join([
                q(p["id"]), q(loc), q(cat["id"]), q(p["name"]), q(p.get("description")),
                q(int(p["price"])), q(bool(p.get("available", True))),
                q(p.get("imageUrl")), q(p.get("primary_media_id")),
                q(json.dumps(p.get("allergens") or [])),
                q(int(p["calories"])) if p.get("calories") is not None else "NULL",
                q(int(p["prep_time_minutes"])) if p.get("prep_time_minutes") is not None else "NULL",
                q(json.dumps(p.get("attributes") or {})),
                q(int(p.get("sortOrder", pi))), NOW, NOW,
            ]) + ");")
        np += 1
        for gi, g in enumerate(p.get("modifier_groups") or []):
            out.append("INSERT INTO modifier_groups (id,product_id,name,min_select,max_select,display_type,sort_order) VALUES ("
                + ",".join([q(g["id"]), q(p["id"]), q(g.get("name","")),
                            q(int(g.get("min_select",0))), q(int(g.get("max_select",1))),
                            q(g.get("display_type")), q(int(g.get("sort_order", gi)))]) + ");")
            for mi, m in enumerate(g.get("modifiers") or []):
                out.append("INSERT INTO modifiers (id,group_id,name,price,available,sort_order) VALUES ("
                    + ",".join([q(m["id"]), q(g["id"]), q(m.get("name","")),
                                q(int(m.get("price",0))), q(bool(m.get("available",True))),
                                q(int(m.get("sort_order", mi)))]) + ");")
                nm += 1

print("\n".join(out))
sys.stderr.write(f"location={info['name']!r} categories={nc} products={np} modifiers={nm}\n")
