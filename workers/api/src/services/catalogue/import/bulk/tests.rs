//! F1 over a REAL catalogue image: the file is read by the hub's parser and
//! written by the console's own writers, and the dish's numbers follow.

use super::*;

const NOW: i64 = 1_790_078_400_000;

const SUPPLIES: &str = "emri;lloji;njësia;kalori;proteina;yndyra;karbohidrate;kosto;për;minimumi\n\
    Oriz sushi;food;kg;130;2,7;0,3;28;380;1 kg;5 kg\n\
    Nori;food;g;35;5,8;0,3;5,1;4.500,00;1 kg;200 g\n\
    Salmon;food;kg;208;20;13;0;2.400,00;;3 kg\n\
    Kuti;box;unit;;;;;35;;50\n\
    Sake;food;cup;;;;;;;\n";

const RECIPES: &str = "dish,ingredient,qty,unit\n\
    Maki Salmon,Oriz sushi,90,g\nMaki Salmon,Nori,4,g\nMaki Salmon,Salmon,35,g\nMaki Salmon,Kuti,1,unit\n\
    Maki Tuna,Oriz sushi,90,g\n";

fn venue() -> Catalog {
    let mut cat = Catalog::create().expect("an empty catalogue");
    cat.set_location(&json!({ "id": "v1", "currency_code": "ALL", "menu_version": 4 }).to_string());
    cat.set_product("maki-salmon", &json!({ "id": "maki-salmon", "name": "Maki Salmon", "price": 650, "weightG": 200, "nutrition": { "kcal": 999 } }).to_string());
    cat.set_product("maki-cucumber", &json!({ "id": "maki-cucumber", "name": "Maki Cucumber", "price": 550 }).to_string());
    cat
}

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

fn supplied() -> Catalog {
    let mut cat = venue();
    let (d, _) = read(&cat, SUPPLIES, Kind::Supplies, CostScale::Major, NOW);
    assert_eq!(apply_supplies(&mut cat, &d, false), Ok(4), "{:?}", d.warnings);
    cat
}

#[test]
fn a_supplies_file_lands_through_the_forms_writer() {
    let cat = supplied();
    let salmon: Value = serde_json::from_str(&cat.supply("salmon").unwrap()).unwrap();
    // "2.400,00" per kg is 240 lek per 100 g; 3 kg of threshold is 3000 g.
    assert_eq!((salmon["unit"].clone(), salmon["costPerBasis"].clone(), salmon["lowAt"].clone()), (json!("g"), json!(240), json!(3000)));
    assert_eq!((salmon["active"].clone(), salmon["nutritionConfirmed"].clone()), (json!(true), json!(false)));
    let kuti: Value = serde_json::from_str(&cat.supply("kuti").unwrap()).unwrap();
    assert_eq!((kuti["kind"].clone(), kuti["unit"].clone(), kuti["costPerBasis"].clone()), (json!("packaging"), json!("unit"), json!(35)));
    // The cup is a warning naming its row, and nothing was written for it.
    assert!(cat.supply("sake").is_none());
    let (d, rows) = read(&venue(), SUPPLIES, Kind::Supplies, CostScale::Major, NOW);
    assert!(d.warnings.iter().any(|w| w.contains("row 6") && w.contains("cup")), "{:?}", d.warnings);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["new"], json!(true));
}

#[test]
fn a_vanished_supply_is_retired_only_when_asked() {
    let mut cat = supplied();
    let (d, _) = read(&cat, "name,unit\nSalmon,kg\n", Kind::Supplies, CostScale::Major, NOW);
    assert_eq!(d.retired.len(), 3, "{:?}", d.retired);
    apply_supplies(&mut cat, &d, false).unwrap();
    let nori: Value = serde_json::from_str(&cat.supply("nori").unwrap()).unwrap();
    assert_eq!(nori["active"], json!(true), "not asked: kept");
    apply_supplies(&mut cat, &d, true).unwrap();
    let nori: Value = serde_json::from_str(&cat.supply("nori").unwrap()).unwrap();
    assert_eq!(nori["active"], json!(false), "asked: retired, never deleted");
}

#[test]
fn a_recipes_file_lands_as_bom_and_the_dish_numbers_follow() {
    let mut cat = supplied();
    let (d, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    assert!(d.warnings.iter().any(|w| w.contains("Maki Tuna") && w.contains("not on the menu")), "{:?}", d.warnings);
    // The dry run says what would change, and changes nothing.
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0]["before"]["lines"].clone(), rows[0]["after"]["lines"].clone()), (json!(0), json!(4)));
    assert_eq!(product(&cat, "maki-salmon")["bom"], Value::Null, "a dry run writes nothing");

    assert_eq!(apply_recipes(&mut cat, &d), Ok(1));
    let p = product(&cat, "maki-salmon");
    let reserved: Vec<(String, i64)> = dowiz_hub::stock::bom_of(&p.to_string()).into_iter().map(|l| (l.supply, l.qty)).collect();
    assert_eq!(reserved, vec![("oriz-sushi".into(), 90), ("nori".into(), 4), ("salmon".into(), 35), ("kuti".into(), 1)]);
    // 90 g rice 117 + 4 g nori 1.4 + 35 g salmon 72.8 = 191.2 kcal.
    assert_eq!(p["nutrition"]["kcal"], json!(191));
    assert_eq!(p["weightG"], json!(129), "the food lines' grams");
    // Per line, rounded: rice 34.2 -> 34, nori 18, salmon 84, the box 35.
    assert_eq!(p["cost"], json!(171));
    assert_eq!(p["nutritionComplete"], json!(true));
    assert_eq!(rows[0]["after"]["kcal"], p["nutrition"]["kcal"], "the preview is the write");
    // The dish nobody named is untouched; the menu version moved once.
    assert_eq!(product(&cat, "maki-cucumber")["bom"], Value::Null);
    let loc: Value = serde_json::from_str(&cat.location().unwrap()).unwrap();
    assert_eq!(loc["menu_version"], json!(5));
}

#[test]
fn the_same_file_twice_lands_the_same_recipe() {
    let mut cat = supplied();
    let (d, _) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    let once = product(&cat, "maki-salmon");
    apply_recipes(&mut cat, &d).unwrap();
    assert_eq!(product(&cat, "maki-salmon"), once);
}

#[test]
fn recipes_before_supplies_import_nothing_and_say_so() {
    let mut cat = venue();
    let (d, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    assert!(rows.is_empty() && d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("supplies file first")), "{:?}", d.warnings);
    assert_eq!(apply_recipes(&mut cat, &d), Ok(0));
}

// ── A VENUE THE SIZE OF DUBIN (2026-09-24) ──────────────────────────────────
// 165 dishes at the live record size (348 B mean, measured from the venue's
// own `/api/owner/products`), 75 supplies and 73 recipes of six lines, through
// the real parser and the real writers. The stored line used to repeat a
// snapshot of its supply (~159 B a line): this menu modelled to 1245 per mille
// and could not be saved at all.

/// A dish shaped like the live ones. The 73 with a recipe are the sushi, with
/// a description and an ingredient list (477 B, their live mean); the other
/// 92 are drinks and snacks without (246 B), so the menu's mean is the live
/// 348 B.
fn live_dish(i: usize) -> Value {
    let mut d = json!({ "allergens": null, "available": true, "categoryId": "futomaki",
            "id": format!("item-{i:03}"), "imageUrl": format!("/media/{:064x}.jpg", i * 7919),
            "name": format!("Sake Futomaki {i}"), "price": 1000, "sortOrder": i, "weightG": 243 });
    if i <= 73 {
        d["description"] = json!("oriz sushi, nori, salmon, krem djathi, kastravec, tobiko");
        d["ingredients"] = json!(["oriz sushi", "nori", "salmon", "krem djathi", "kastravec", "tobiko"]);
        d["nutrition"] = json!({ "approx": true, "carbs": 45, "fat": 18, "kcal": 408, "protein": 17 });
        d["tags"] = json!(["salmon"]);
    }
    d
}

/// The live venue: its 165 dishes, and the rest of its catalogue (location,
/// categories, texts) as one record of the size that brings the image to the
/// 513 per mille `/api/owner/health` reported before any recipe.
pub(super) fn dubin() -> Catalog {
    let mut cat = Catalog::create().expect("an empty catalogue");
    let rest = "x".repeat(7_200);
    cat.set_location(&json!({ "id": "v1", "currency_code": "ALL", "menu_version": 1, "rest": rest }).to_string());
    for i in 1..=165 {
        let d = live_dish(i);
        cat.set_product(d["id"].as_str().unwrap(), &d.to_string());
    }
    cat
}

pub(super) fn dubin_supplies() -> String {
    let mut s = String::from("id,name,kind,unit,kcal,protein,fat,carbs,cost,per,category,low_at\n");
    for i in 0..75 {
        let (kind, unit, per) = if i >= 72 { ("packaging", "pcs", "1") } else { ("food_ingredient", "g", "1 kg") };
        s += &format!("supply-{i:02}x,Supply {i:02}x,{kind},{unit},145,2.6,0.3,32,1800,{per},Peshk,3000\n");
    }
    s
}

pub(super) fn dubin_recipes() -> String {
    let mut s = String::from("dish,ingredient,qty,unit\n");
    for d in 1..=73 {
        for l in 0..6 {
            s += &format!("item-{d:03},supply-{:02}x,{},g\n", (d * 5 + l) % 72, 20 + l * 7);
        }
    }
    s
}

fn per_mille(cat: &mut Catalog) -> i64 {
    let bytes = cat.to_bytes().expect("the catalogue is saved");
    Catalog::load(&bytes).unwrap().usage().used_per_mille()
}

#[test]
fn a_full_menu_with_recipes_fits_the_catalogue() {
    let mut cat = dubin();
    let mean = cat.products().iter().map(|(_, j)| j.len()).sum::<usize>() / 165;
    assert!((340..=360).contains(&mean), "the live dishes are 348 B on average; these are {mean}");
    let before = per_mille(&mut cat);
    // 513 per mille of the old 1 MiB ceiling = 51 of dowiz_hub::CEILING_BYTES (10 MiB).
    assert!((50..=52).contains(&before), "the live venue reads 51 per mille; this one reads {before}");
    let (d, _) = read(&cat, &dubin_supplies(), Kind::Supplies, CostScale::Major, NOW);
    assert_eq!(apply_supplies(&mut cat, &d, false), Ok(75), "{:?}", d.warnings);
    let (d, _) = read(&cat, &dubin_recipes(), Kind::Recipes, CostScale::Major, NOW);
    assert_eq!(apply_recipes(&mut cat, &d), Ok(73), "{:?}", d.warnings);
    let after = cat.to_bytes().map(|b| Catalog::load(&b).unwrap().usage().used_per_mille());
    assert!(matches!(after, Ok(n) if n < 90), "73 recipes must leave room: {after:?}");
}

/// PRINTS the modelled per mille for dubin-sushi from the inventory lane's real
/// files, through the real parser and writers. IGNORED because it reads files
/// outside the repo; it panics (never passes quietly) when they are missing:
///   DUBIN_INV=<dir: supplies.csv, recipes.csv> DUBIN_BACKUP=<dir: owner-products.json, owner-health.json> \
///   cargo test --lib dubin_per_mille -- --ignored --nocapture
#[test]
#[ignore]
fn dubin_per_mille_from_the_real_files() {
    let dir = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{k} is not set"));
    let file = |d: &str, f: &str| std::fs::read_to_string(format!("{d}/{f}")).unwrap_or_else(|e| panic!("{d}/{f}: {e}"));
    let (inv, backup) = (dir("DUBIN_INV"), dir("DUBIN_BACKUP"));
    let products: Value = serde_json::from_str(&file(&backup, "owner-products.json")).unwrap();
    let health: Value = serde_json::from_str(&file(&backup, "owner-health.json")).unwrap();
    let live = health["images"]["catalog"]["usedCells"].as_i64().unwrap();
    let mut cat = Catalog::create().unwrap();
    for p in products["products"].as_array().unwrap() {
        cat.set_product(p["id"].as_str().unwrap(), &p.to_string());
    }
    // The rest of the live image (location, categories, texts) as one record
    // of the size that brings this one to what /api/owner/health measured.
    let loc = |n: usize| json!({ "id": "v1", "currency_code": "ALL", "rest": "x".repeat(n) }).to_string();
    cat.set_location(&loc(0));
    let menu = cat.projected().unwrap().usage.used_cells;
    cat.set_location(&loc((live - menu).max(0) as usize));
    let before = cat.projected().unwrap().usage;
    let (d, _) = read(&cat, &file(&inv, "supplies.csv"), Kind::Supplies, CostScale::Major, NOW);
    let supplies = apply_supplies(&mut cat, &d, false).unwrap();
    let with_supplies = cat.projected().unwrap().usage.used_per_mille();
    let (d, _) = read(&cat, &file(&inv, "recipes.csv"), Kind::Recipes, CostScale::Major, NOW);
    let recipes = apply_recipes(&mut cat, &d).unwrap();
    let after = cat.projected().unwrap();
    println!(
        "dubin: live {live} cells, modelled {} = {} per mille; +{supplies} supplies -> {with_supplies}; +{recipes} recipes -> {} cells = {} per mille, fits {} ({} warnings)",
        before.used_cells, before.used_per_mille(), after.usage.used_cells, after.usage.used_per_mille(), after.fits, d.warnings.len()
    );
}

// ── the dry run says whether it fits (2026-09-24) ───────────────────────────

/// Dubin with its supplies in, and `blob` bytes of something else besides.
fn nearly_full(blob: usize) -> Catalog {
    let mut cat = dubin();
    let (d, _) = read(&cat, &dubin_supplies(), Kind::Supplies, CostScale::Major, NOW);
    apply_supplies(&mut cat, &d, false).unwrap();
    cat.set_product("zz-blob", &json!({ "id": "zz-blob", "name": "Blob", "blob": "b".repeat(blob) }).to_string());
    cat
}

/// The largest blob (to 4 KiB) that leaves dubin-with-supplies at or under
/// `per_mille` of its ceiling.
fn blob_under(per_mille: i64) -> usize {
    let at = |blob: usize| nearly_full(blob).projected().ok().filter(|p| p.fits).map(|p| p.usage.used_per_mille());
    let (mut lo, mut hi) = (0usize, 2 * dowiz_hub::CEILING_BYTES);
    while hi - lo > 4096 {
        let mid = (lo + hi) / 2;
        if at(mid).is_some_and(|n| n <= per_mille) { lo = mid } else { hi = mid }
    }
    lo
}

#[test]
fn a_dry_run_that_would_not_fit_says_so() {
    // Filled to just under the ceiling (found, not hard-coded, so the test
    // follows dowiz_hub::CEILING_BYTES), so 73 recipes tip it over.
    let mut cat = nearly_full(blob_under(990));
    let (d, _) = read(&cat, &dubin_recipes(), Kind::Recipes, CostScale::Major, NOW);
    assert_eq!(d.recipes.len(), 73, "{:?}", d.warnings);
    let room = projected(&mut cat, &d, Kind::Recipes, false);
    assert_eq!(room["fits"], json!(false), "{room}");
    let (now, then) = (room["nowPerMille"].as_i64().unwrap(), room["perMille"].as_i64().unwrap());
    assert!(now < 1000 && then > 1000, "{room}");
    let said = room["said"].as_str().unwrap();
    assert!(said.contains("would not fit") && said.contains(&format!("{then} per mille")), "{said}");
    assert_eq!(product(&cat, "item-001")["bom"], Value::Null, "measured on a copy: nothing written");
    // TWIN: the same file on the venue as it is fits, and says nothing.
    let mut cat = nearly_full(0);
    let room = projected(&mut cat, &d, Kind::Recipes, false);
    assert_eq!((room["fits"].clone(), room["said"].clone()), (json!(true), Value::Null), "{room}");
    assert!(room["perMille"].as_i64().unwrap() < 90, "{room}");
}

#[test]
fn a_dry_run_past_two_ceilings_says_so_without_a_number() {
    let mut cat = venue();
    let mut d = RecipeDraft::default();
    let (big, _) = read(&cat, "name,unit\nSalt,g\n", Kind::Supplies, CostScale::Major, NOW);
    d.supplies = big.supplies;
    d.supplies[0].name = "n".repeat(22_000_000);
    let room = projected(&mut cat, &d, Kind::Supplies, false);
    assert_eq!((room["fits"].clone(), room["perMille"].clone()), (json!(false), Value::Null), "{room}");
    assert!(room["said"].as_str().unwrap().contains("more than twice its ceiling"), "{room}");
}

/// Refused for another reason than room: Apply names it; the dry run does
/// not pretend to have measured.
#[test]
fn a_draft_apply_would_refuse_is_not_measured() {
    let mut cat = venue();
    let (mut d, _) = read(&cat, "name,unit\nSalt,g\n", Kind::Supplies, CostScale::Major, NOW);
    d.supplies[0].id = String::new();
    let room = projected(&mut cat, &d, Kind::Supplies, false);
    assert_eq!((room["fits"].clone(), room["perMille"].clone(), room["said"].clone()), (Value::Null, Value::Null, Value::Null));
}

#[test]
fn a_catalogue_that_cannot_be_saved_now_says_so() {
    let mut cat = Catalog::create().unwrap();
    cat.set_product("huge", &"h".repeat(11_000_000));
    let room = projected(&mut cat, &RecipeDraft::default(), Kind::Recipes, false);
    assert_eq!(room["fits"], json!(false));
    assert!(room["said"].as_str().unwrap().contains("cannot be read to measure"), "{room}");
}

#[test]
fn supply_in_carries_weight_per_unit() {
    let mut cat = venue();
    let (d, rows) = read(&cat, "name,unit,weight per unit\nKuti,pcs,12\nSalmon,kg,\n", Kind::Supplies, CostScale::Major, NOW);
    assert_eq!((supply_in(&d.supplies[0]).weight_per_unit, rows[0]["weightPerUnit"].clone()), (Some(12.0), json!(12.0)));
    apply_supplies(&mut cat, &d, false).unwrap();
    let kuti: Value = serde_json::from_str(&cat.supply("kuti").unwrap()).unwrap();
    assert_eq!(kuti["weightPerUnit"], json!(12.0));
    // TWIN: no weight given, none stored -- not even a null.
    assert_eq!(supply_in(&d.supplies[1]).weight_per_unit, None);
    assert!(!cat.supply("salmon").unwrap().contains("weightPerUnit"));
}

/// The preview shows each line with its supply's name, as the console draws it.
#[test]
fn the_preview_names_each_line() {
    let cat = supplied();
    let (_, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    assert_eq!((rows[0]["bom"][2]["supply"].clone(), rows[0]["bom"][2]["name"].clone()), (json!("salmon"), json!("Salmon")));
}

#[test]
fn no_room_is_a_warning_and_the_reason_apply_is_refused() {
    let mut warnings = vec!["row 3: something".to_string()];
    let room = json!({ "fits": false, "said": "this import would not fit" });
    assert_eq!(room::refusal(&room, &mut warnings).as_deref(), Some("this import would not fit"));
    assert_eq!(warnings.len(), 2);
    // TWINS: room, and a draft that was not measured, refuse nothing.
    for room in [json!({ "fits": true }), json!({ "fits": null })] {
        assert_eq!(room::refusal(&room, &mut warnings), None);
    }
    assert_eq!(warnings.len(), 2);
}

/// The owner reads a lean recipe with names and numbers; `?id=` narrows it.
#[test]
fn the_owner_reads_each_stored_line_with_its_supply() {
    let mut cat = supplied();
    let (d, _) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    let stored = product(&cat, "maki-salmon");
    assert_eq!(stored["bom"][0], json!({ "supply": "oriz-sushi", "qty": 90 }), "stored lean");
    let all = owner_view(&cat, None);
    assert_eq!(all.len(), 2);
    let one = owner_view(&cat, Some("maki-salmon"));
    assert_eq!(one.len(), 1);
    assert_eq!((one[0]["bom"][0]["name"].clone(), one[0]["bom"][0]["kcal"].clone()), (json!("Oriz sushi"), json!(117.0)));
    assert_eq!(one[0]["bom"][3]["kind"], json!("packaging"));
    assert!(owner_view(&cat, Some("nope")).is_empty());
}

/// The preview is the write: a weight the owner typed (`weightDerived:false`)
/// is shown kept, as Apply keeps it.
#[test]
fn the_preview_keeps_what_the_owner_typed_as_apply_does() {
    let mut cat = supplied();
    cat.set_product("maki-salmon", &json!({ "id": "maki-salmon", "name": "Maki Salmon", "price": 650, "weightG": 200, "weightDerived": false }).to_string());
    let (d, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    assert_eq!((rows[0]["after"]["weightG"].clone(), product(&cat, "maki-salmon")["weightG"].clone()), (json!(200), json!(200)));
}
