#!/usr/bin/env python3
"""Enrich a live hub's catalogue with what the menu site knows and what it
does not: the other two languages for every heading, dish and ingredient
list, the dish's tags, and an APPROXIMATE nutrition line per portion.

    set -a && . /root/.dowiz_owner && set +a
    python3 scripts/enrich_menu.py --hub https://sushi-durres.dowiz.org \
        --location-id sushi-durres --dry-run
    python3 scripts/enrich_menu.py --hub https://sushi-durres.dowiz.org \
        --location-id sushi-durres --name "Dubin & Sushi" --delivery-fee 300

Four writes, each idempotent:

  1. POST /api/owner/i18n          category names (uk/en), dish names where the
                                   venue's own is Albanian, descriptions and
                                   ingredient lists (uk/en)
  2. POST /api/owner/products/:id  tags, approximate nutrition and weight
  3. POST /api/owner/location      the venue's name and delivery terms (only
                                   when asked for on the command line)
  4. POST /api/owner/branding      the venue's paper and gold (--brand)

THE NUTRITION IS AN ESTIMATE AND IS MARKED AS ONE. The venue has published no
figures; a customer asked for "approximately how many calories", so each dish
is summed from a per-ingredient table of typical portion grams and per-100 g
values, and the record carries `"approx": true`, which every surface renders
as "≈". A dish whose ingredients are unknown gets a figure only when its name
alone is enough (a 330 ml beer, a double espresso) -- otherwise it gets none,
because a made-up number about food is worse than no number.
"""
import argparse, json, os, re, sys, urllib.request, urllib.error

SITE = "https://sushi-durres-menu.netlify.app/"
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
    except Exception as e:
        return 0, str(e).encode()


def site_data(src):
    if src.startswith("http"):
        code, raw = http("GET", src)
        if code != 200:
            sys.exit(f"enrich: {src} → {code}")
        raw = raw.decode("utf-8", "replace")
    else:
        raw = open(src, encoding="utf-8").read()
    m = re.search(r'<script id="menu-data" type="application/json">(.*?)</script>', raw, re.S)
    if not m:
        sys.exit("enrich: the page has no <script id=\"menu-data\"> block")
    return json.loads(m.group(1))


def unhtml(s):
    return (s or "").replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", '"')


# ── names the venue wrote in Albanian (or, twice, in Ukrainian) ──────────────
# The rolls carry the names the venue trades under and stay as they are in
# every language. These are the ones a reader of the other two languages
# cannot follow.
NAMES = {
    "item-44": {"en": "French fries", "uk": "Картопля фрі"},
    "item-45": {"en": "Rice chips", "uk": "Рисові чипси"},
    "item-78": {"en": "Tea (bag)", "uk": "Чай пакетований"},
    "item-79": {"en": "Double espresso", "uk": "Подвійний еспресо"},
    "item-80": {"en": "Organic tea", "uk": "Органічний чай"},
    "item-81": {"en": "Dubai chocolate", "uk": "Дубайський шоколад"},
    "item-82": {"en": "Hot chocolate", "uk": "Гарячий шоколад"},
    "item-83": {"en": "Decaf espresso", "uk": "Еспресо без кофеїну"},
    "item-84": {"en": "Decaf cappuccino", "uk": "Капучино без кофеїну"},
    "item-85": {"en": "Frappé", "uk": "Фрапе"},
    "item-86": {"en": "Ginseng coffee, small", "uk": "Кава з женьшенем, мала"},
    "item-87": {"en": "Ginseng coffee, large", "uk": "Кава з женьшенем, велика"},
    "item-88": {"en": "Caffè corretto", "uk": "Кафе корретто"},
    "item-89": {"en": "Latte", "uk": "Лате"},
    "item-90": {"en": "Turkish coffee", "uk": "Кава по-турецьки"},
    "item-91": {"en": "Turkish coffee, double", "uk": "Кава по-турецьки, подвійна"},
    "item-92": {"en": "Cocoa, small", "uk": "Какао, мале"},
    "item-93": {"en": "Cocoa", "uk": "Какао"},
    "item-94": {"en": "Cappuccino (sachet)", "uk": "Капучино (пакетик)"},
    "item-95": {"en": "Cappuccino", "uk": "Капучино"},
    "item-96": {"en": "Caffè americano", "uk": "Американо"},
    "item-97": {"en": "Cappuccino, chocolate or vanilla", "uk": "Капучино шоколадне / ванільне"},
    "item-98": {"en": "Lajthiza water", "uk": "Вода Lajthiza"},
    "item-99": {"en": "Glina water", "uk": "Вода Glina"},
    "item-100": {"en": "Water 0.5 l", "uk": "Вода 0,5 л"},
    "item-101": {"en": "Vitamin water", "uk": "Вітамінна вода"},
    "item-102": {"en": "Schweppes", "uk": "Schweppes"},
    "item-103": {"en": "B52", "uk": "B52"},
    "item-104": {"en": "Bitter", "uk": "Біттер"},
    "item-105": {"en": "Bravo", "uk": "Bravo"},
    "item-106": {"en": "Cola", "uk": "Кола"},
    "item-107": {"en": "Crodino", "uk": "Crodino"},
    "item-108": {"en": "Fanta", "uk": "Fanta"},
    "item-109": {"en": "Lemon soda", "uk": "Лимонна содова"},
    "item-110": {"en": "Red Bull", "uk": "Red Bull"},
    "item-111": {"en": "Iced cappuccino", "uk": "Холодне капучино"},
    "item-112": {"en": "Iced cocoa", "uk": "Холодне какао"},
    "item-113": {"en": "Iced tea", "uk": "Холодний чай"},
    "item-114": {"en": "Iced chocolate", "uk": "Холодний шоколад"},
    "item-115": {"en": "Pacha", "uk": "Pacha"},
    "item-116": {"en": "Monin tea", "uk": "Чай Monin"},
    "item-117": {"en": "Amaro Lucano", "uk": "Amaro Lucano"},
    "item-118": {"en": "Amaro Montenegro", "uk": "Amaro Montenegro"},
    "item-119": {"en": "Amaro del Capo", "uk": "Amaro del Capo"},
    "item-120": {"en": "The Bush rum", "uk": "Ром The Bush"},
    "item-121": {"en": "Chivas Regal 12", "uk": "Chivas Regal 12"},
    "item-122": {"en": "Chivas Regal 18", "uk": "Chivas Regal 18"},
    "item-123": {"en": "Disaronno", "uk": "Disaronno"},
    "item-124": {"en": "Fernet-Branca", "uk": "Fernet-Branca"},
    "item-125": {"en": "Bombay gin", "uk": "Джин Bombay"},
    "item-126": {"en": "Gordon's gin", "uk": "Джин Gordon's"},
    "item-127": {"en": "Hendrick's gin", "uk": "Джин Hendrick's"},
    "item-128": {"en": "Jack Daniel's", "uk": "Jack Daniel's"},
    "item-129": {"en": "Jägermeister", "uk": "Jägermeister"},
    "item-130": {"en": "Johnnie Walker Red Label", "uk": "Johnnie Walker Red Label"},
    "item-131": {"en": "Johnnie Walker Black Label", "uk": "Johnnie Walker Black Label"},
    "item-132": {"en": "Limoncello", "uk": "Лімончело"},
    "item-133": {"en": "Metaxa 12", "uk": "Metaxa 12"},
    "item-134": {"en": "Metaxa 7", "uk": "Metaxa 7"},
    "item-135": {"en": "Metaxa 5", "uk": "Metaxa 5"},
    "item-136": {"en": "Sambuca", "uk": "Самбука"},
    "item-137": {"en": "Absolut vodka", "uk": "Горілка Absolut"},
    "item-138": {"en": "Smirnoff vodka", "uk": "Горілка Smirnoff"},
    "item-139": {"en": "Vecchia Romagna", "uk": "Vecchia Romagna"},
    "item-140": {"en": "Ballantine's", "uk": "Ballantine's"},
    "item-141": {"en": "J&B", "uk": "J&B"},
    "item-142": {"en": "Ouzo", "uk": "Узо"},
    "item-143": {"en": "Raki", "uk": "Ракія"},
    "item-144": {"en": "Skënderbeu punch", "uk": "Пунш Skënderbeu"},
    "item-145": {"en": "Glen Grant", "uk": "Glen Grant"},
    "item-146": {"en": "Fernet Skënderbeu", "uk": "Fernet Skënderbeu"},
    "item-147": {"en": "Glass of red wine", "uk": "Келих червоного вина"},
    "item-148": {"en": "Glass of white wine", "uk": "Келих білого вина"},
    "item-149": {"en": "Skënderbeu cognac", "uk": "Коньяк Skënderbeu"},
    "item-150": {"en": "White wine 100 ml", "uk": "Біле вино 100 мл"},
    "item-151": {"en": "Red wine 100 ml", "uk": "Червоне вино 100 мл"},
    "item-152": {"en": "Bavaria", "uk": "Bavaria"},
    "item-153": {"en": "Korça", "uk": "Korça"},
    "item-154": {"en": "Peroni", "uk": "Peroni"},
    "item-155": {"en": "Corona", "uk": "Corona"},
    "item-156": {"en": "Heineken", "uk": "Heineken"},
    "item-157": {"en": "Paulaner, large", "uk": "Paulaner, велике"},
    "item-158": {"en": "Paulaner, small", "uk": "Paulaner, мале"},
    "item-159": {"en": "Stella Artois", "uk": "Stella Artois"},
    "item-160": {"en": "Kriko, small", "uk": "Kriko, мале"},
    "item-161": {"en": "Kriko, large", "uk": "Kriko, велике"},
    "item-162": {"en": "Orange juice", "uk": "Апельсиновий фреш"},
    "item-163": {"en": "Apple juice", "uk": "Яблучний фреш"},
    "item-164": {"en": "Mixed juice", "uk": "Фреш мікс"},
    "item-165": {"en": "Pomegranate juice", "uk": "Гранатовий фреш"},
    "item-41": {"en": "Hot Set", "uk": "Гарячий сет"},
    "item-42": {"en": "Party Set", "uk": "Сет для компанії"},
}

# ── per-ingredient table: grams in a roll portion, then kcal/protein/fat/carbs per 100 g ──
# Typical values from public food composition tables, rounded. A roll is eight
# pieces, about 250 g; a bowl about 350 g; a soup about 400 g.
ING = {
    "sushi rice":        (130, 150, 2.7, 0.8, 33),
    "nori":              (3,   35,  6,   0.3, 5),
    "cucumber":          (20,  15,  0.7, 0.1, 3.6),
    "avocado":           (30,  160, 2,   15,  9),
    "salmon":            (50,  200, 20,  13,  0),
    "grilled salmon":    (50,  210, 22,  13,  0),
    "salmon mix":        (50,  220, 16,  16,  2),
    "cream cheese":      (30,  340, 6,   34,  4),
    "cheese mix":        (30,  330, 12,  30,  3),
    "tobiko":            (10,  70,  13,  1.5, 1),
    "sesame":            (5,   570, 17,  50,  23),
    "shrimp":            (50,  100, 22,  1,   0),
    "panko shrimp":      (60,  250, 14,  12,  22),
    "tuna":              (50,  130, 28,  1,   0),
    "crab mix":          (50,  150, 8,   10,  8),
    "surimi":            (50,  100, 8,   1,   14),
    "spicy mayo":        (15,  600, 1,   65,  4),
    "kewpie mayo":       (15,  700, 1.5, 76,  1),
    "sweet chili":       (15,  200, 0.5, 0.5, 48),
    "lemon-kimchi sauce":(15,  150, 1,   10,  14),
    "truffle sauce":     (15,  400, 2,   40,  6),
    "unagi sauce":       (10,  250, 2,   0,   60),
    "soy sauce":         (8,   50,  8,   0,   5),
    "egg":               (30,  150, 12,  10,  1),
    "crispy onion":      (10,  450, 6,   30,  40),
    "caramelized onion": (15,  100, 1,   4,   16),
    "mango":             (25,  60,  0.8, 0.4, 15),
    "torched gouda":     (30,  350, 25,  27,  2),
    "tempura batter":    (30,  320, 6,   14,  40),
    # bowls / soups
    "chuka":             (40,  110, 1.5, 4,   16),
    "dashi broth":       (300, 10,  1,   0,   1),
    "dry meat broth":    (300, 30,  4,   1,   1),
    "sesame oil":        (5,   880, 0,   100, 0),
    "wakame":            (5,   45,  3,   0.6, 9),
    "miso paste":        (15,  200, 12,  6,   26),
    "fish sauce":        (5,   35,  5,   0,   4),
    "udon noodles":      (120, 130, 3.5, 0.5, 27),
    "noodles":           (120, 140, 5,   1,   28),
    "shiitake mushrooms":(30,  35,  2,   0.5, 7),
    "cherry tomatoes":   (30,  18,  1,   0.2, 4),
    "tahini paste":      (15,  600, 17,  54,  21),
    "tofu":              (50,  75,  8,   4,   2),
    "radish":            (15,  16,  0.7, 0.1, 3.4),
    "green onion":       (5,   30,  2,   0.2, 7),
    "hondashi":          (3,   200, 25,  1,   25),
    "fried chicken":     (80,  250, 22,  14,  9),
    "marinated egg":     (50,  150, 12,  10,  2),
    # cocktails (ml as grams)
    "ice":               (0,   0,   0,   0,   0),
    "gin":               (50,  230, 0,   0,   0),
    "vodka":             (50,  230, 0,   0,   0),
    "white rum":         (50,  230, 0,   0,   0),
    "rum":               (50,  230, 0,   0,   0),
    "tequila":           (50,  230, 0,   0,   0),
    "bourbon whiskey":   (50,  250, 0,   0,   0),
    "sugar syrup":       (15,  260, 0,   0,   65),
    "elderflower syrup": (15,  260, 0,   0,   65),
    "cranberry syrup":   (15,  260, 0,   0,   65),
    "lemon juice":       (20,  25,  0.4, 0,   8),
    "lime juice":        (20,  25,  0.4, 0,   8),
    "fresh lime":        (10,  30,  0.7, 0.2, 10),
    "lime":              (10,  30,  0.7, 0.2, 10),
    "soda water":        (80,  0,   0,   0,   0),
    "tonic water":       (100, 34,  0,   0,   9),
    "cola":              (100, 42,  0,   0,   10.6),
    "triple sec":        (20,  300, 0,   0,   30),
    "coffee liqueur":    (20,  300, 0,   0,   45),
    "aperol":            (50,  150, 0,   0,   20),
    "prosecco":          (100, 70,  0,   0,   1),
    "passion fruit puree":(30, 60,  1,   0.5, 13),
    "pineapple juice":   (60,  50,  0.4, 0,   12),
    "orange juice":      (50,  45,  0.7, 0.2, 10),
    "orange":            (20,  47,  0.9, 0.1, 12),
    "orange slice":      (10,  47,  0.9, 0.1, 12),
    "coconut cream":     (30,  330, 3,   34,  6),
    "espresso":          (30,  2,   0.1, 0,   0),
    "egg white":         (20,  50,  11,  0,   0.7),
    "strawberries":      (30,  32,  0.7, 0.3, 8),
    "mint":              (0,   0,   0,   0,   0),
    "mint leaves":       (0,   0,   0,   0,   0),
    "basil leaves":      (0,   0,   0,   0,   0),
}
# Categories that change the portion. Bowls carry more rice and more fish
# than a roll; nigiri and maki are small; a bowl of soup is mostly broth.
PORTION = {
    "bowls": {"sushi rice": 160, "salmon": 80, "tuna": 80, "shrimp": 80, "crab mix": 80, "avocado": 40, "cucumber": 40},
    "nigiri": {"sushi rice": 30, "salmon": 25, "tuna": 25, "shrimp": 25, "nori": 0},
    "maki": {"sushi rice": 90, "salmon": 30, "tuna": 30, "shrimp": 30, "cucumber": 30, "surimi": 30, "cream cheese": 25},
}
# Dishes with no ingredient list whose name alone says what they are.
BY_NAME = [
    (r"dopio kafe|kafe turke dopio", 10), (r"kafe turke|dekafeinato$|caffe amerikano|espresso", 5),
    (r"kapucino ftohte", 120), (r"kapucino|dekafeinato kapucino", 80), (r"latte", 120),
    (r"cokollate dubai", 300), (r"cokollat|cokollate", 220), (r"frappe", 150),
    (r"ginseng i vogel", 60), (r"ginseng i madh", 100), (r"korreto", 70),
    (r"kakao ftohte", 180), (r"kakao e vogel", 120), (r"kakao", 150),
    (r"caj|tea", 2), (r"lajthiza|glina|kond", 0), (r"uje vitamin", 30), (r"zhveps", 45),
    (r"b52", 180), (r"bitter", 90), (r"bravo", 110), (r"^cola", 105), (r"crodino", 60), (r"fanta", 110),
    (r"lemon soda", 90), (r"red bull", 110), (r"pacha", 100),
    (r"amaro|fernet", 100), (r"rum|whisk|chivas|jack|label|ballantin|j&b|glen", 100), (r"disaronno", 120),
    (r"gin ", 95), (r"jager", 110), (r"limoncel", 110), (r"metaxa|konjak|vechia", 100), (r"sambuca", 130),
    (r"vodka", 90), (r"uzo|raki", 95), (r"ponc", 120), (r"vere|wine", 85),
    (r"paulaner m|kriko m", 210), (r"bavaria|korca|peroni|corona|heineken|paulaner|stella|kriko", 140),
    (r"portokalli", 110), (r"molle", 115), (r"miks", 115), (r"shege", 130),
    (r"картопля фрі", 310), (r"рисові чипси", 260),
]
HOT_CATS = {"hot", "volcano"}


def estimate(item, by_name_items):
    """(nutrition dict, weight_g) or (None, None)."""
    cats = item.get("categories") or []
    primary = cats[0] if cats else ""
    ings = [unhtml(x).lower().strip() for x in (item["translations"].get("en") or {}).get("ingredients") or []]
    if primary == "sets" and ings:
        # A set is the rolls it lists; each is summed on its own.
        total = [0, 0, 0, 0]; grams = 0; found = 0
        for ing in ings:
            half = ing.endswith(" 1/2")
            base = ing[:-4] if half else ing
            ref = by_name_items.get(base)
            if not ref:
                continue
            n, g = estimate(ref, {})
            if not n:
                continue
            f = 0.5 if half else 1.0
            found += 1; grams += g * f
            total[0] += n["kcal"] * f; total[1] += n["protein"] * f; total[2] += n["fat"] * f; total[3] += n["carbs"] * f
        if not found:
            return None, None
        return {"kcal": round(total[0]), "protein": round(total[1]), "fat": round(total[2]), "carbs": round(total[3]), "approx": True}, round(grams)
    if ings:
        total = [0.0, 0.0, 0.0, 0.0]; grams = 0; known = 0
        override = PORTION.get(primary, {})
        for ing in ings:
            row = ING.get(ing)
            if not row:
                continue
            g = override.get(ing, row[0])
            known += 1; grams += g
            total[0] += row[1] * g / 100; total[1] += row[2] * g / 100; total[2] += row[3] * g / 100; total[3] += row[4] * g / 100
        if primary in HOT_CATS:
            row = ING["tempura batter"]; g = row[0]; grams += g
            total[0] += row[1] * g / 100; total[1] += row[2] * g / 100; total[2] += row[3] * g / 100; total[3] += row[4] * g / 100
        if not known:
            return None, None
        return {"kcal": round(total[0]), "protein": round(total[1]), "fat": round(total[2]), "carbs": round(total[3]), "approx": True}, round(grams)
    name = unhtml(item["name"]).lower()
    for pat, kcal in BY_NAME:
        if re.search(pat, name):
            return {"kcal": kcal, "approx": True}, None
    return None, None


def build(data):
    cats, items = data["categories"], data["items"]
    entries = []
    for c in cats:
        for loc in ("uk", "en"):
            v = unhtml((c.get("title") or {}).get(loc))
            if v:
                entries.append({"entity": "category", "id": c["id"], "locale": loc, "field": "name", "value": v})
    by_name = {unhtml(i["name"]).lower(): i for i in items}
    products = {}
    for it in items:
        pid = it["id"]
        for loc in ("uk", "en"):
            tr = (it.get("translations") or {}).get(loc) or {}
            ings = [unhtml(x) for x in tr.get("ingredients") or []]
            if ings:
                entries.append({"entity": "product", "id": pid, "locale": loc, "field": "description", "value": ", ".join(ings)})
                entries.append({"entity": "product", "id": pid, "locale": loc, "field": "ingredients", "value": json.dumps(ings, ensure_ascii=False)})
            nm = NAMES.get(pid, {}).get(loc)
            if nm:
                entries.append({"entity": "product", "id": pid, "locale": loc, "field": "name", "value": nm})
        nutrition, weight = estimate(it, by_name)
        payload = {"tags": [t for t in (it.get("filters") or []) if t and t != "all"]}
        if nutrition:
            payload["nutrition"] = nutrition
        if weight:
            payload["weight_g"] = weight
        products[pid] = payload
    return entries, products


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=SITE)
    ap.add_argument("--hub", default=os.environ.get("HUB", ""))
    ap.add_argument("--location-id", required=True)
    ap.add_argument("--name", default=None)
    ap.add_argument("--delivery-fee", type=int, default=None)
    ap.add_argument("--free-over", type=int, default=None, help="free delivery from this subtotal; -1 clears")
    ap.add_argument("--min-order", type=int, default=None)
    ap.add_argument("--brand", default=None, help="primary,ink,paper hex triple, e.g. #c9a35a,#f1e8d8,#0b1717")
    ap.add_argument("--out", default=None, help="write i18n.json / products.json here as well")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--skip-products", action="store_true")
    a = ap.parse_args()

    data = site_data(a.src)
    entries, products = build(data)
    n_nut = sum(1 for p in products.values() if "nutrition" in p)
    print(f"i18n entries {len(entries)}  products {len(products)}  with nutrition {n_nut}")
    if a.out:
        os.makedirs(a.out, exist_ok=True)
        json.dump(entries, open(os.path.join(a.out, "i18n.json"), "w"), ensure_ascii=False, indent=1)
        json.dump(products, open(os.path.join(a.out, "products.json"), "w"), ensure_ascii=False, indent=1)
    if a.dry_run or not a.hub:
        for pid in ("item-05", "item-37", "item-52", "item-58", "item-65", "item-79", "item-156"):
            print(f"  {pid}: {json.dumps(products.get(pid), ensure_ascii=False)}")
        print("dry run: nothing written")
        return 0

    email, password = os.environ.get("OWNER_EMAIL"), os.environ.get("OWNER_PASSWORD")
    if not (email and password):
        sys.exit("enrich: set OWNER_EMAIL and OWNER_PASSWORD")
    code, body = http("POST", f"{a.hub}/api/auth/login",
                      json.dumps({"email": email, "password": password, "location_id": a.location_id}).encode(),
                      {"content-type": "application/json"})
    if code != 200:
        sys.exit(f"enrich: owner login failed {code}: {body[:200].decode('utf-8','replace')}")
    auth = {"authorization": "Bearer " + json.loads(body)["access_token"], "content-type": "application/json"}

    # 1. the other languages, in chunks the route accepts
    written = 0; refused = []
    for i in range(0, len(entries), 400):
        chunk = entries[i:i + 400]
        code, resp = http("POST", f"{a.hub}/api/owner/i18n",
                          json.dumps({"location_id": a.location_id, "entries": chunk}).encode(), auth)
        if code != 200:
            sys.exit(f"enrich: i18n chunk {i} → {code} {resp[:300].decode('utf-8','replace')}")
        d = json.loads(resp); written += d.get("written", 0); refused += d.get("refused", [])
    print(f"translations: {written} written, {len(refused)} refused" + (f" e.g. {refused[:3]}" if refused else ""))

    # 2. tags, nutrition, weight -- one request per dish
    if not a.skip_products:
        ok = bad = 0
        for pid, payload in products.items():
            code, resp = http("POST", f"{a.hub}/api/owner/products/{pid}",
                              json.dumps({"location_id": a.location_id, **payload}).encode(), auth)
            if code == 200:
                ok += 1
            else:
                bad += 1; print(f"  {pid} → {code} {resp[:120].decode('utf-8','replace')}")
        print(f"products: {ok} ok, {bad} refused")

    # 3. the venue's name and delivery terms
    loc = {}
    if a.name: loc["name"] = a.name
    if a.delivery_fee is not None: loc["delivery_fee"] = a.delivery_fee
    if a.min_order is not None: loc["min_order"] = a.min_order
    if a.free_over is not None: loc["free_delivery_threshold"] = None if a.free_over < 0 else a.free_over
    if loc:
        code, resp = http("POST", f"{a.hub}/api/owner/location",
                          json.dumps({"location_id": a.location_id, **loc}).encode(), auth)
        print(f"location {loc} → {code} {resp[:160].decode('utf-8','replace')}")

    # 4. the venue's colours
    if a.brand:
        primary, ink, paper = [x.strip() for x in a.brand.split(",")]
        code, resp = http("POST", f"{a.hub}/api/owner/branding?location_id={a.location_id}",
                          json.dumps({"primary": primary, "ink": ink, "paper": paper, "typePair": "classic", "radius": 8}).encode(), auth)
        print(f"branding → {code} {resp[:200].decode('utf-8','replace')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
