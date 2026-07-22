//! postatch.rs — vendor menu authority: Dubin & Sushi (Durrës, Albania).
//!
//! Real vendor, parsed 2026-07-22 from https://sushi-durres-menu.netlify.app/
//! (the JSON in `<script id="menu-data">`). Source of truth: `design/dubin-sushi-menu.json`.
//! This module REPLACES the former "Pizza Roma" emoji demo with the actual vendor's
//! 59 trilingual (en/uk/sq) sushi-menu items across 15 categories + 10 filters.
//!
//! Money is exact integer minor units (Albanian Lek, "ALL"). Every price parses
//! to a non-negative `i64` lek amount by construction — zero floats, ever (matches
//! `money_guard.rs` and MANIFESTO C2). Drinks priced "Ask waiter" carry `price_minor:
//! 0` + `drink_ask: true`; they are PRESENTED as ask-price and never enter cart math.
//!
//! Offline-clean: no deps, no I/O, compiles on the default feature set. The menu is
//! a `static` table so the composer / checkout fragments read it with zero allocation.
//! The `SoundTrack`/`SdfShape`-bound fragment functions in `compose_ui.rs` will read
//! these items to build real product-card geometry (Sea & Sheet DZ-01…12 → actual
//! photos at `img/item-NN-*.webp`), replacing the placeholder boxes.

use crate::money_guard::Money;

/// ISO 4217 currency code for the vendor (Albanian Lek). Money is `i64` lek minor
/// units throughout; no fractional-ALL amount exists in this menu.
pub const VENDOR_CURRENCY: &str = "ALL";
/// Vendor display name (the real restaurant, not a demo alias).
pub const VENDOR_NAME: &str = "Dubin & Sushi";
/// Geolocation (Durrës, Albania) — for the contact / location screens + the courier
/// board map. Sourced from the vendor's schema.org JSON-LD `geo` block.
pub const VENDOR_LAT: f64 = 41.315_347;
pub const VENDOR_LON: f64 = 19.444_996_4;
/// WhatsApp/order phone (the vendor's real `tel:` link).
pub const VENDOR_PHONE: &str = "+355683085694";
/// Hours (vendor schema.org `openingHours`).
pub const VENDOR_HOURS: &str = "Mo-Su 10:00-22:00";

/// One menu item. Prices are integer Lek minor units; a drink whose price is
/// "Ask waiter" carries `drink_ask: true` and `price_minor: 0` (it is excluded
/// from cart totals and shown with a localized "ask" string by the presenter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub id: &'static str,
    pub name: &'static str,
    pub price_minor: i64,
    pub drink_ask: bool,
    pub categories: &'static [&'static str],
    pub filters: &'static [&'static str],
}

impl MenuItem {
    /// The decisive money amount for this item. `Money(0)` for ask-drinks; the
    /// composer must NOT tween this value (see `money_guard.rs`).
    pub fn price(&self) -> Money {
        Money(self.price_minor)
    }
}

/// One menu category (the vendor's 15). The composer's `Sea & Sheet` screens group
/// items by these. `title` keys are the i18n codes (en/uk/sq) resolved by the presenter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCategory {
    pub id: &'static str,
    /// i18n keys present in `design/dubin-sushi-menu.json` `categories[].title`.
    pub title_key: &'static str,
}

/// The vendor's full category list, in display order (matches the vendor's nav chips).
pub static CATEGORIES: &[MenuCategory] = &[
    MenuCategory {
        id: "chef",
        title_key: "chef",
    },
    MenuCategory {
        id: "cocktails",
        title_key: "cocktails",
    },
    MenuCategory {
        id: "sets",
        title_key: "sets",
    },
    MenuCategory {
        id: "bowls",
        title_key: "bowls",
    },
    MenuCategory {
        id: "nigiri",
        title_key: "nigiri",
    },
    MenuCategory {
        id: "philadelphia",
        title_key: "philadelphia",
    },
    MenuCategory {
        id: "california",
        title_key: "california",
    },
    MenuCategory {
        id: "futomaki",
        title_key: "futomaki",
    },
    MenuCategory {
        id: "signature",
        title_key: "signature",
    },
    MenuCategory {
        id: "hot",
        title_key: "hot",
    },
    MenuCategory {
        id: "volcano",
        title_key: "volcano",
    },
    MenuCategory {
        id: "vegetarian",
        title_key: "vegetarian",
    },
    MenuCategory {
        id: "snacks",
        title_key: "snacks",
    },
    MenuCategory {
        id: "premium",
        title_key: "premium",
    },
    MenuCategory {
        id: "maki",
        title_key: "maki",
    },
];

/// The vendor's filter list (the menu's horizontal "Popular / Salmon / …" chips).
pub static FILTERS: &[&str] = &[
    "all",
    "popular",
    "sets",
    "bowls",
    "salmon",
    "shrimp",
    "tuna",
    "hot",
    "vegetarian",
    "drinks",
];

/// The full vendor menu — 59 items, prices in integer Lek minor units.
/// Parsed from `design/dubin-sushi-menu.json` (the vendor's own `menu-data` JSON).
pub static MENU: &[MenuItem] = &[
    MenuItem {
        id: "item-01",
        name: "Sake Futomaki",
        price_minor: 900,
        drink_ask: false,
        categories: &["chef", "futomaki"],
        filters: &["popular", "salmon"],
    },
    MenuItem {
        id: "item-02",
        name: "Ebi Futomaki",
        price_minor: 850,
        drink_ask: false,
        categories: &["chef", "futomaki"],
        filters: &["popular", "shrimp"],
    },
    MenuItem {
        id: "item-03",
        name: "Sake Furai Futomaki",
        price_minor: 850,
        drink_ask: false,
        categories: &["futomaki"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-04",
        name: "Philadelphia Classic",
        price_minor: 1100,
        drink_ask: false,
        categories: &["philadelphia"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-05",
        name: "Philadelphia Premium",
        price_minor: 1400,
        drink_ask: false,
        categories: &["chef", "philadelphia", "premium"],
        filters: &["popular", "salmon"],
    },
    MenuItem {
        id: "item-06",
        name: "Red Pearl",
        price_minor: 950,
        drink_ask: false,
        categories: &["california"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-07",
        name: "Red Pearl Premium",
        price_minor: 1300,
        drink_ask: false,
        categories: &["california", "premium"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-08",
        name: "Sesame Sake",
        price_minor: 850,
        drink_ask: false,
        categories: &["chef", "philadelphia", "california"],
        filters: &["popular", "salmon"],
    },
    MenuItem {
        id: "item-09",
        name: "Ebi Cream",
        price_minor: 900,
        drink_ask: false,
        categories: &["philadelphia"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-10",
        name: "Ebi Cream Premium",
        price_minor: 1300,
        drink_ask: false,
        categories: &["philadelphia", "premium"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-11",
        name: "Coral Sake",
        price_minor: 1100,
        drink_ask: false,
        categories: &["california"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-12",
        name: "Coral Sake Premium",
        price_minor: 1400,
        drink_ask: false,
        categories: &["california", "premium"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-13",
        name: "Smoky Sunset",
        price_minor: 850,
        drink_ask: false,
        categories: &["california"],
        filters: &[],
    },
    MenuItem {
        id: "item-14",
        name: "Maguro Sesame",
        price_minor: 850,
        drink_ask: false,
        categories: &["california"],
        filters: &["tuna"],
    },
    MenuItem {
        id: "item-15",
        name: "Crunchy Ebi Sunset",
        price_minor: 850,
        drink_ask: false,
        categories: &["california"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-16",
        name: "California Classic",
        price_minor: 950,
        drink_ask: false,
        categories: &["chef", "california"],
        filters: &["popular"],
    },
    MenuItem {
        id: "item-17",
        name: "California Premium",
        price_minor: 1300,
        drink_ask: false,
        categories: &["california", "premium"],
        filters: &[],
    },
    MenuItem {
        id: "item-18",
        name: "Maguro Pearl",
        price_minor: 950,
        drink_ask: false,
        categories: &["california"],
        filters: &["tuna"],
    },
    MenuItem {
        id: "item-19",
        name: "Maguro Pearl Premium",
        price_minor: 1300,
        drink_ask: false,
        categories: &["california", "premium"],
        filters: &["tuna"],
    },
    MenuItem {
        id: "item-20",
        name: "Crispy Sunset",
        price_minor: 900,
        drink_ask: false,
        categories: &["california"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-21",
        name: "Sake Sunset",
        price_minor: 950,
        drink_ask: false,
        categories: &["chef", "california"],
        filters: &["popular", "salmon"],
    },
    MenuItem {
        id: "item-22",
        name: "Sake Sunset Premium",
        price_minor: 1300,
        drink_ask: false,
        categories: &["california", "premium"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-23",
        name: "Truffle Sake premium",
        price_minor: 1200,
        drink_ask: false,
        categories: &["chef", "signature", "premium"],
        filters: &["popular", "salmon", "shrimp"],
    },
    MenuItem {
        id: "item-24",
        name: "Hot Ebi",
        price_minor: 850,
        drink_ask: false,
        categories: &["chef", "hot"],
        filters: &["hot", "popular", "shrimp"],
    },
    MenuItem {
        id: "item-25",
        name: "Hot Smoky Sake",
        price_minor: 850,
        drink_ask: false,
        categories: &["hot"],
        filters: &["hot", "salmon"],
    },
    MenuItem {
        id: "item-26",
        name: "Hot Maguro",
        price_minor: 900,
        drink_ask: false,
        categories: &["hot"],
        filters: &["hot", "tuna"],
    },
    MenuItem {
        id: "item-27",
        name: "Sweet Chili Tiger",
        price_minor: 1200,
        drink_ask: false,
        categories: &["signature"],
        filters: &["salmon", "shrimp"],
    },
    MenuItem {
        id: "item-28",
        name: "Sweet Chili Tiger Premium",
        price_minor: 1500,
        drink_ask: false,
        categories: &["signature", "premium"],
        filters: &["salmon", "shrimp"],
    },
    MenuItem {
        id: "item-29",
        name: "Sake Volcano",
        price_minor: 1300,
        drink_ask: false,
        categories: &["volcano"],
        filters: &["hot", "salmon"],
    },
    MenuItem {
        id: "item-30",
        name: "Ebi Volcano",
        price_minor: 1000,
        drink_ask: false,
        categories: &["volcano"],
        filters: &["hot", "shrimp"],
    },
    MenuItem {
        id: "item-31",
        name: "Maki Salmon",
        price_minor: 600,
        drink_ask: false,
        categories: &["maki"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-32",
        name: "Maki Cream",
        price_minor: 500,
        drink_ask: false,
        categories: &["maki"],
        filters: &[],
    },
    MenuItem {
        id: "item-33",
        name: "Maki Shrimps",
        price_minor: 550,
        drink_ask: false,
        categories: &["maki"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-34",
        name: "Maki Cucumber",
        price_minor: 500,
        drink_ask: false,
        categories: &["maki"],
        filters: &[],
    },
    MenuItem {
        id: "item-35",
        name: "Maki Tuna",
        price_minor: 600,
        drink_ask: false,
        categories: &["maki"],
        filters: &["tuna"],
    },
    MenuItem {
        id: "item-36",
        name: "Maki Surimi",
        price_minor: 550,
        drink_ask: false,
        categories: &["maki"],
        filters: &[],
    },
    MenuItem {
        id: "item-37",
        name: "Set Philadelphia",
        price_minor: 4300,
        drink_ask: false,
        categories: &["sets"],
        filters: &["sets"],
    },
    MenuItem {
        id: "item-38",
        name: "Set Premium",
        price_minor: 5250,
        drink_ask: false,
        categories: &["sets"],
        filters: &["sets"],
    },
    MenuItem {
        id: "item-39",
        name: "Set 50/50",
        price_minor: 3100,
        drink_ask: false,
        categories: &["sets"],
        filters: &["sets"],
    },
    MenuItem {
        id: "item-40",
        name: "Set 1",
        price_minor: 2950,
        drink_ask: false,
        categories: &["sets"],
        filters: &["sets"],
    },
    MenuItem {
        id: "item-41",
        name: "Panko Shrimps",
        price_minor: 800,
        drink_ask: false,
        categories: &["snacks"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-42",
        name: "Salmon Nigiri",
        price_minor: 250,
        drink_ask: false,
        categories: &["nigiri"],
        filters: &["salmon"],
    },
    MenuItem {
        id: "item-43",
        name: "Tuna Nigiri",
        price_minor: 250,
        drink_ask: false,
        categories: &["nigiri"],
        filters: &["tuna"],
    },
    MenuItem {
        id: "item-44",
        name: "Shrimp Nigiri",
        price_minor: 250,
        drink_ask: false,
        categories: &["nigiri"],
        filters: &["shrimp"],
    },
    MenuItem {
        id: "item-45",
        name: "Smoky Gouda",
        price_minor: 900,
        drink_ask: false,
        categories: &["vegetarian"],
        filters: &["vegetarian"],
    },
    MenuItem {
        id: "item-46",
        name: "Okinawa Fresh",
        price_minor: 850,
        drink_ask: false,
        categories: &["vegetarian"],
        filters: &["vegetarian"],
    },
    MenuItem {
        id: "item-47",
        name: "Tuna Salmon Roll",
        price_minor: 1300,
        drink_ask: false,
        categories: &["signature"],
        filters: &["salmon", "tuna"],
    },
    MenuItem {
        id: "item-48",
        name: "Tuna Bowl",
        price_minor: 850,
        drink_ask: false,
        categories: &["bowls"],
        filters: &["bowls", "tuna"],
    },
    MenuItem {
        id: "item-49",
        name: "Crab Mix Bowl",
        price_minor: 850,
        drink_ask: false,
        categories: &["bowls"],
        filters: &["bowls"],
    },
    MenuItem {
        id: "item-50",
        name: "Salmon Bowl",
        price_minor: 850,
        drink_ask: false,
        categories: &["chef", "bowls"],
        filters: &["bowls", "popular", "salmon"],
    },
    MenuItem {
        id: "item-51",
        name: "Shrimp Bowl",
        price_minor: 850,
        drink_ask: false,
        categories: &["bowls"],
        filters: &["bowls", "shrimp"],
    },
    MenuItem {
        id: "item-52",
        name: "Green Mango",
        price_minor: 950,
        drink_ask: false,
        categories: &["vegetarian"],
        filters: &["vegetarian"],
    },
    MenuItem {
        id: "item-53",
        name: "Basil Smash",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-54",
        name: "Mohito Strawberry",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-55",
        name: "Mohito",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-56",
        name: "Margarita",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-57",
        name: "Espresso Martini",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-58",
        name: "Whiskey Sour",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
    MenuItem {
        id: "item-59",
        name: "Hugo",
        price_minor: 0,
        drink_ask: true,
        categories: &["cocktails"],
        filters: &["drinks"],
    },
];

/// Look up an item by id. `None` if unknown (the composer must never assume a
/// demo alias — an unknown id is an error, like the kernel FSM's forbidden jumps).
pub fn find(id: &str) -> Option<&'static MenuItem> {
    MENU.iter().find(|i| i.id == id)
}

/// Items in a category, in menu order. Empty slice if the category id is unknown.
pub fn by_category(category: &str) -> Vec<&'static MenuItem> {
    MENU.iter()
        .filter(|i| i.categories.contains(&category))
        .collect()
}

/// S4 — pre-burned category→item-index table for the static-slicing fast path.
/// Categories overlap (an item can be chef+futomaki), so a `const` map is not
/// buildable; the chef indices are burned here (verified against `design/dubin-
/// sushi-menu.json`) and returned as a static slice. Adding a new category's
/// burned indices: extend the match arm and the `category_index_test` gate.
/// innovate: ceiling — a `LazyLock`-backed index table is the upgrade trigger if
/// a fragment hot-path needs O(1) per-category lookup for ALL 15 categories.
pub const CHEF_INDICES: &[usize] = &[0, 1, 4, 7, 15, 20, 22, 23, 49];

/// S4 — a `&'static [MenuItem]` for the composer's `AppState.items` field.
/// `category="all"` → the full `MENU`. `"chef"` → the 9 Chef's Picks (via the
/// burned index table). Unknown category → empty (the composer's menu fallback
/// then composes the Chef's Picks by default, matching the customer landing).
///
/// innovate: ceiling — `CHEF_ITEMS` is a burned static built from `CHEF_INDICES`
/// because a `const` slice over `MENU` filtered by category is not expressible
/// (no `filter` in const fn). A `LazyLock` table for all 15 categories is the
/// upgrade trigger; today only `"chef"` is hot (the customer landing screen).
pub static CHEF_ITEMS: &[&'static MenuItem] = &[
    &MENU[0], &MENU[1], &MENU[4], &MENU[7], &MENU[15], &MENU[20], &MENU[22], &MENU[23], &MENU[49],
];

pub fn by_category_static(category: &str) -> &'static [&'static MenuItem] {
    match category {
        "all" => MENU.iter().collect::<Vec<&'static MenuItem>>().leak(),
        "chef" => CHEF_ITEMS,
        _ => &[],
    }
}

/// Items matching a filter ("popular" / "salmon" / …), in menu order.
pub fn by_filter(filter: &str) -> Vec<&'static MenuItem> {
    if filter == "all" {
        return MENU.iter().collect();
    }
    MENU.iter()
        .filter(|i| i.filters.contains(&filter))
        .collect()
}

/// The integer-Lek total of a cart: Σ (price_minor × qty) over priced items,
/// SKIPPING ask-drinks (price 0) exactly as the vendor's checkout does. Pure fn:
/// same (items, qtys) ⇒ identical `Money`. This is the cart total the real checkout
/// → kernel path computes; a fractional intermediate is structurally impossible
/// because every addend is an `i64 × u32` product summed into an `i64` accumulator.
///
/// Innovate: ceiling — overflow on a pathological cart (qty ≤ u32::MAX × 5250 lek
/// ≈ 22.6 trillion lek) is unreachable in practice but is guarded by checked_add
/// so a bug in the caller surfaces as `Err`, not as a silent wrap. Upgrade trigger:
/// if a set ever priced in a sub-minor fractional currency, move to the kernel's
/// `Money` double-entry ledger (which is already overflow-checked) and delete this.
pub fn cart_total(items: &[(&'static MenuItem, u32)]) -> Result<Money, &'static str> {
    let mut total: i64 = 0;
    for (item, qty) in items {
        if item.drink_ask {
            continue; // ask-price drinks never enter the numeric total
        }
        let addend = (item.price_minor)
            .checked_mul(*qty as i64)
            .ok_or("cart line overflow")?;
        total = total.checked_add(addend).ok_or("cart total overflow")?;
    }
    Ok(Money(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    // D-vendor-1 — the menu is the REAL vendor's 59 items, not a demo alias.
    #[test]
    fn menu_is_dubin_sushi_59_items() {
        assert_eq!(MENU.len(), 59, "vendor menu must be exactly 59 items");
        assert_eq!(VENDOR_NAME, "Dubin & Sushi");
        assert_eq!(VENDOR_CURRENCY, "ALL");
    }

    // D-vendor-2 — every menu category id the items reference exists in CATEGORIES.
    #[test]
    fn item_categories_all_registered() {
        let registered: Vec<&str> = CATEGORIES.iter().map(|c| c.id).collect();
        for item in MENU {
            for cat in item.categories {
                assert!(
                    registered.contains(cat),
                    "item {} references unknown category {}",
                    item.id,
                    cat
                );
            }
        }
    }

    // D-vendor-3 — every price is a non-negative integer (zero floats, ever).
    // Drinks priced "Ask waiter" are price 0 + drink_ask true; they must NOT be
    // negative, and a non-ask item at price 0 would be a data bug.
    #[test]
    fn prices_non_negative_integer_justifying_ask_marker() {
        for item in MENU {
            assert!(item.price_minor >= 0, "{}: price must be >= 0", item.id);
            if item.drink_ask {
                assert_eq!(
                    item.price_minor, 0,
                    "{}: ask-drink must be price 0",
                    item.id
                );
            } else {
                assert!(
                    item.price_minor > 0,
                    "{}: priced item must be > 0 lek",
                    item.id
                );
            }
        }
    }

    // D-vendor-4 — cart_total is EXACT integer arithmetic + order-independent
    // (Σ is associative/commutative over i64), matching the MANIFESTO C2 invariant.
    // The known vendor cart: {Sake Futomaki ×2, Maki Cream ×1} = 900×2 + 500.
    #[test]
    fn cart_total_exact_integer() {
        let cart = [
            (find("item-01").unwrap(), 2u32), // Sake Futomaki 900
            (find("item-32").unwrap(), 1u32), // Maki Cream 500
        ];
        assert_eq!(cart_total(&cart).unwrap(), Money(900 * 2 + 500));
        assert_eq!(cart_total(&cart).unwrap(), Money(2300));
    }

    // D-vendor-5 — ask-drinks are EXCLUDED from the numeric total (the vendor shows
    // "Ask waiter", never a 0-lek line in checkout). A cart of only drinks → Money(0)
    // but the presenter shows the ask string, not a zero total.
    #[test]
    fn ask_drinks_excluded_from_total() {
        let cart = [
            (find("item-53").unwrap(), 1u32), // Basil Smash (ask)
            (find("item-55").unwrap(), 3u32), // Mohito (ask)
        ];
        assert_eq!(cart_total(&cart).unwrap(), Money(0));
    }

    // D-vendor-6 — the overflow guards exist but are UNREACHABLE at the realistic
    // ceiling (innovate: see the comment on `cart_total`). Concretely: even a
    // `[u32::MAX copies of the priciest 5250-lek set]` line = 2.25e13 lek, far
    // under i64::MAX (9.2e18), and ≤59 such lines never sum past it. So this test
    // documents the REALISTIC ceiling (no overflow) rather than asserting an
    // unreproducible overflow; the checked_mul/checked_add remain defensive.
    #[test]
    fn cart_total_pathological_lines_never_overflow() {
        let priciest = find("item-38").unwrap(); // Set Premium, 5250
        let mut cart: Vec<(&MenuItem, u32)> = Vec::new();
        for _ in 0..59 {
            cart.push((priciest, u32::MAX));
        }
        // Σ ≈ 59 × 5250 × 4_294_967_295 ≈ 1.33e15 lek, well under i64::MAX.
        let total = cart_total(&cart).unwrap();
        assert!(total.0 > 0);
        assert!(total.0 < i64::MAX); // proves the ceiling holds; guards never fire here
    }

    // D-vendor-7 — by_category / by_filter return menu-order slices covering the
    // expected counts (e.g. "chef" has 10 picks, "sets" has 4 — verified live).
    #[test]
    fn category_and_filter_counts() {
        assert_eq!(by_category("chef").len(), 9, "Chef's Picks = 9");
        assert_eq!(by_category("sets").len(), 4, "Sets = 4");
        assert_eq!(by_category("nigiri").len(), 3, "Nigiri = 3");
        assert_eq!(by_category("maki").len(), 6, "Maki = 6");
        assert_eq!(by_filter("all").len(), 59);
        assert_eq!(by_filter("sets").len(), 4);
        assert!(by_filter("zzz").is_empty(), "unknown filter → empty");
        assert!(by_category("zzz").is_empty(), "unknown category → empty");
    }
}
