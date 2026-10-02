//! THE MOVE CANNOT CHANGE AN ANSWER (BN1, lane W-BN1B, 2026-10-02): the
//! twenty-three remaining handlers that pulled the catalogue image -- or the
//! order log -- across the Worker<->object hop, pinned on one fixture venue
//! BEFORE their derivations moved into the object (`/fold/catalogue?q=`,
//! `/fold/reveals|assist|graph|kitchen_facts|waste`, `POST /fold/bulk`, and
//! `/fold/venue` for the five that read the venue's record alone), through the
//! route seam (W-COV C2). The shape is `catalogue_routes/tests.rs`'s: each pin
//! is the handler's status and canonical body as the platform in memory
//! answered it, with the generated ids scrubbed. Where the derived value is
//! what a MODEL is shown (the two assistants, the social draft), the pin is
//! the payload the venue's model received, read back from the outbound hook.

use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn apex(path: &str) -> String {
    format!("https://{PLATFORM_HOST}{path}")
}

/// The fixture venue: a dish with a recipe, a translated name and a brand; a
/// supply received and partly binned; a waiter and a cook; one pickup order.
struct Venue {
    owner: String,
    owner_id: String,
    dish: String,
    waiter: String,
    waiter_id: String,
    cook: String,
    cook_id: String,
    order: String,
}

impl Venue {
    fn ids(&self) -> Vec<(String, &'static str)> {
        vec![
            (self.dish.clone(), "DISH"),
            (self.order.clone(), "ORDER"),
            (self.owner_id.clone(), "OWNER"),
            (self.waiter_id.clone(), "WAITER"),
            (self.cook_id.clone(), "COOK"),
        ]
    }
}

fn user_of(site: &Site, token: &str) -> String {
    match crate::auth::verify(&site.env(), token, site.now_ms).expect("a genuine token") {
        crate::auth::Claims::Owner { user_id, .. } => user_id,
        other => panic!("not an owner: {other:?}"),
    }
}

fn ok(r: &Reply, what: &str) {
    assert_eq!(r.status_code(), 200, "{what}: {}", r.body_str());
}

fn setting(site: &Site, t: &str, key: &str, value: &str) {
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("alpha", "/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"),
        &[],
    );
    ok(&r, key);
}

fn feature(site: &Site, t: &str, key: &str) {
    let r = site.run(
        crate::services::venue::settings::set_feature,
        post(&at("alpha", "/api/owner/features"), &json!({"key": key, "on": true})).bearer(t).on("alpha"),
        &[],
    );
    ok(&r, key);
}

/// The venue's model, answering every question with one line.
fn model(answer: &'static str) {
    answer_outbound(move |c| {
        let u = c.url().unwrap().to_string();
        if u.ends_with("/chat/completions") {
            Reply::from_json(&json!({"choices": [{"message": {"content": answer}}]}))
        } else if u.ends_with("/1789/media") {
            Reply::from_json(&json!({"id": "container-1"}))
        } else if u.ends_with("/1789/media_publish") {
            Reply::from_json(&json!({"id": "ig-post-1"}))
        } else {
            Reply::error("unexpected", 500)
        }
    });
}

fn ai_on(site: &Site, t: &str) {
    feature(site, t, "ai.enabled");
    setting(site, t, "ai.endpoint", "https://llm.example/v1");
    setting(site, t, "ai.model", "tiny");
}

fn fixture(site: &Site) -> Venue {
    let (owner, dish) = open_venue(site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("alpha", "/api/owner/supplies"), &json!({"id": "salmon", "name": "Salmon", "unit": "g", "costPerBasis": 180, "lowAt": 100})).bearer(&owner).on("alpha"),
        &[],
    );
    ok(&r, "supply");
    let r = site.run(
        crate::owner::update_product,
        post(&at("alpha", &format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "cooking_min": 12, "bom": [{"supply": "salmon", "qty": 100}]}))
            .bearer(&owner)
            .on("alpha"),
        &[("id", &dish)],
    );
    ok(&r, "recipe");
    for (kind, body) in [("received", json!({"item": "salmon", "qty": 5000})), ("wasted", json!({"item": "salmon", "qty": 100, "reason": "spoiled"}))] {
        let r = site.run(
            crate::services::operations::stock::stock_move,
            post(&at("alpha", &format!("/api/owner/stock/{kind}")), &body).bearer(&owner).on("alpha"),
            &[("kind", kind)],
        );
        ok(&r, kind);
    }
    let r = site.run(
        crate::owner::write_translations,
        post(
            &at("alpha", "/api/owner/i18n"),
            &json!({"location_id": "alpha", "entries": [{"entity": "product", "id": dish, "locale": "sq", "field": "name", "value": "Futomaki shqip"}]}),
        )
        .bearer(&owner)
        .on("alpha"),
        &[],
    );
    ok(&r, "translation");
    let r = site.run(
        crate::services::venue::brand::set_branding,
        post(&at("alpha", "/api/owner/branding"), &json!({"primary": "#aa3311"})).bearer(&owner).on("alpha"),
        &[],
    );
    ok(&r, "brand");
    let (waiter, waiter_id) = site.staff_full("alpha", &owner, "w@x.test", "waiter");
    let (cook, cook_id) = site.staff_full("alpha", &owner, "k@x.test", "kitchen");
    let placed = place_pickup(site, "alpha", &dish, 2);
    ok(&placed, "placed");
    let order = placed.body_value()["id"].as_str().expect("order id").to_string();
    let owner_id = user_of(site, &owner);
    Venue { owner, owner_id, dish, waiter, waiter_id, cook, cook_id, order }
}

/// Every generated id replaced by its name, tokens blanked; then the status
/// and the canonical body.
fn scrub(v: &mut Value, ids: &[(String, &'static str)]) {
    match v {
        Value::String(s) => {
            for (id, name) in ids {
                if s.contains(id.as_str()) {
                    *s = s.replace(id.as_str(), name);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| scrub(x, ids)),
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                if k == "access_token" || k == "jti" || k == "token" {
                    *x = json!("TOKEN");
                } else {
                    scrub(x, ids);
                }
            }
            let renamed: Vec<(String, String)> = m
                .keys()
                .filter_map(|k| ids.iter().find(|(id, _)| k == id).map(|(_, name)| (k.clone(), name.to_string())))
                .collect();
            for (from, to) in renamed {
                if let Some(x) = m.remove(&from) {
                    m.insert(to, x);
                }
            }
        }
        _ => {}
    }
}

fn pinned(r: &Reply, ids: &[(String, &'static str)]) -> String {
    let Ok(mut v) = serde_json::from_slice::<Value>(r.body()) else {
        return format!("{} {}", r.status_code(), r.body_str());
    };
    scrub(&mut v, ids);
    format!("{} {}", r.status_code(), v)
}

/// A body that is not JSON (the privacy notice is HTML): its status and digest.
fn pinned_digest(r: &Reply) -> String {
    format!("{} sha256:{}", r.status_code(), crate::auth::sha256_hex(&r.body_str()))
}

/// The last call the venue's model received, as the pin reads it.
fn model_saw(ids: &[(String, &'static str)]) -> String {
    let call = sent().into_iter().filter(|c| c.url().unwrap().as_str().ends_with("/chat/completions")).last().expect("the model was called");
    let mut v: Value = serde_json::from_slice(&call.body_bytes()).expect("a JSON payload");
    scrub(&mut v, ids);
    v.to_string()
}

/// The pin: the recorded answer, or -- while recording -- print it, so one
/// run of the module prints every pin (`every_pin_is_recorded` refuses an
/// empty one).
#[track_caller]
fn pin(name: &str, got: String, want: &str) {
    if want.is_empty() {
        eprintln!("PIN {name} {got}");
        return;
    }
    assert_eq!(got, want, "{name}");
}

/// A pin left empty is a test that passes without looking.
#[test]
fn every_pin_is_recorded() {
    let pins = [
        ("categories", PIN_CATEGORIES), ("i18n", PIN_I18N), ("integrations", PIN_INTEGRATIONS), ("privacy", PIN_PRIVACY),
        ("import-preview", PIN_IMPORT_PREVIEW), ("import-apply", PIN_IMPORT_APPLY), ("import-empty", PIN_IMPORT_EMPTY),
        ("owner-products", PIN_OWNER_PRODUCTS), ("owner-product", PIN_OWNER_PRODUCT),
        ("supplies-preview", PIN_SUPPLIES_PREVIEW), ("supplies-apply", PIN_SUPPLIES_APPLY), ("supplies-empty", PIN_SUPPLIES_EMPTY),
        ("recipes-preview", PIN_RECIPES_PREVIEW), ("recipes-apply", PIN_RECIPES_APPLY),
        ("image-ghost", PIN_IMAGE_GHOST), ("image", PIN_IMAGE), ("customers", PIN_CUSTOMERS), ("reveals", PIN_REVEALS),
        ("assist", PIN_ASSIST), ("assist-facts", PIN_ASSIST_FACTS), ("graph", PIN_GRAPH), ("graph-shape", PIN_GRAPH_SHAPE),
        ("kitchen-assist", PIN_KITCHEN_ASSIST), ("kitchen-facts", PIN_KITCHEN_FACTS),
        ("draft", PIN_DRAFT), ("draft-prompt", PIN_DRAFT_PROMPT), ("approve-nophoto", PIN_APPROVE_NOPHOTO), ("approve", PIN_APPROVE),
        ("approve-ig", PIN_APPROVE_IG),
        ("voice-owner-stock", PIN_VOICE_OWNER_STOCK), ("voice-cook-stock", PIN_VOICE_COOK_STOCK), ("voice-waiter-add", PIN_VOICE_WAITER_ADD),
        ("voice-owner-off", PIN_VOICE_OWNER_OFF), ("voice-cook-off", PIN_VOICE_COOK_OFF),
        ("waste", PIN_WASTE), ("branding", PIN_BRANDING), ("activation", PIN_ACTIVATION), ("reach", PIN_REACH), ("reach-zoned", PIN_REACH_ZONED),
        ("bootstrap", PIN_BOOTSTRAP),
    ];
    let empty: Vec<&str> = pins.iter().filter(|(_, p)| p.is_empty()).map(|(n, _)| *n).collect();
    assert!(empty.is_empty(), "unrecorded pins: {empty:?}");
}

// ── the pins ────────────────────────────────────────────────────────────────

#[test]
fn the_categories_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::catalog_edit::list_categories, get(&at("alpha", "/api/owner/categories")).bearer(&v.owner).on("alpha"), &[]);
    pin("categories", pinned(&r, &v.ids()), PIN_CATEGORIES);
}
const PIN_CATEGORIES: &str = r##"200 {"categories":[{"count":1,"id":"rolls","name":"Rolls","sortOrder":10}]}"##;

#[test]
fn translations_are_checked_against_the_same_ids() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(
        crate::owner::write_translations,
        post(
            &at("alpha", "/api/owner/i18n"),
            &json!({"location_id": "alpha", "entries": [
                {"entity": "product", "id": v.dish, "locale": "en", "field": "name", "value": "Futomaki roll"},
                {"entity": "category", "id": "rolls", "locale": "en", "field": "name", "value": "Rolls"},
                {"entity": "product", "id": "ghost", "locale": "sq", "field": "name", "value": "x"},
                {"entity": "planet", "id": "mars", "locale": "sq", "field": "name", "value": "x"},
            ]}),
        )
        .bearer(&v.owner)
        .on("alpha"),
        &[],
    );
    pin("i18n", pinned(&r, &v.ids()), PIN_I18N);
}
const PIN_I18N: &str = r##"200 {"ok":false,"refused":[{"id":"ghost","why":"unknown product"},{"id":"mars","why":"unknown planet"}],"written":2}"##;

#[test]
fn the_integrations_screen_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::integrations::status, get(&at("alpha", "/api/owner/integrations")).bearer(&v.owner).on("alpha"), &[]);
    pin("integrations", pinned(&r, &v.ids()), PIN_INTEGRATIONS);
}
const PIN_INTEGRATIONS: &str = r##"200 {"ai":{"enabled":false,"endpoint":"","usable":false},"cloud":{"bucket":"","configured":false},"crypto":{"wallets":0},"instagram":{"configured":false},"mcp":{"tools":18,"url":"https://alpha.dowiz.org/api/mcp"},"stripe":{"configured":false},"telegram":{"configured":false},"webhook":{"lastMs":null,"secretSet":false,"url":"https://alpha.dowiz.org/api/webhooks/meta","verifySet":false},"whatsapp":{"configured":false,"notifies":false}}"##;

#[test]
fn the_privacy_notice_answers_the_same_bytes() {
    let site = Site::new();
    let _v = fixture(&site);
    let r = site.run(crate::privacy::notice::serve, get(&at("alpha", "/privacy?lang=en")).on("alpha"), &[]);
    pin("privacy", pinned_digest(&r), PIN_PRIVACY);
}
const PIN_PRIVACY: &str = r##"200 sha256:0f7845e31973599d8b7e49b642224d19fc8185deea1be7d1a607d43c68569851"##;

fn csv_call(owner: &str, path: &str, body: &str) -> Call {
    Call::new(&at("alpha", path), worker::Method::Post).unwrap().with_body(body.as_bytes().to_vec()).bearer(owner).on("alpha")
}

#[test]
fn a_menu_csv_previews_and_applies_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    // The fixture dish by its own name (resolved to its id), a new one, and a line the parser warns about.
    let csv = "name,price,category\nFutomaki,950,Rolls\nMiso soup,400,Soups\nRamen,1200,Soups\n";
    let r = site.run(crate::services::catalogue::import::import_menu, csv_call(&v.owner, "/api/owner/menu/import", csv), &[]);
    pin("import-preview", pinned(&r, &v.ids()), PIN_IMPORT_PREVIEW);
    let r = site.run(crate::services::catalogue::import::import_menu, csv_call(&v.owner, "/api/owner/menu/import?apply=1&retire=1", csv), &[]);
    pin("import-apply", pinned(&r, &v.ids()), PIN_IMPORT_APPLY);
    let r = site.run(crate::services::catalogue::import::import_menu, csv_call(&v.owner, "/api/owner/menu/import?apply=1", "dish;cost\nx;1\n"), &[]);
    pin("import-empty", pinned(&r, &v.ids()), PIN_IMPORT_EMPTY);
    let r = site.run(crate::services::catalogue::import::bulk::owner_products, get(&at("alpha", "/api/owner/products")).bearer(&v.owner).on("alpha"), &[]);
    pin("owner-products", pinned(&r, &v.ids()), PIN_OWNER_PRODUCTS);
    let r = site.run(
        crate::services::catalogue::import::bulk::owner_products,
        get(&at("alpha", &format!("/api/owner/products?id={}", v.dish))).bearer(&v.owner).on("alpha"),
        &[],
    );
    pin("owner-product", pinned(&r, &v.ids()), PIN_OWNER_PRODUCT);
    assert_eq!(r.headers().get("cache-control").unwrap().as_deref(), Some("private, no-store"));
}
const PIN_IMPORT_PREVIEW: &str = r##"200 {"applied":false,"categories":2,"draft":{"categories":[{"id":"rolls","name":"Rolls","sortOrder":0},{"id":"soups","name":"Soups","sortOrder":1}],"products":[{"available":null,"categoryId":"rolls","description":null,"id":"DISH","name":"Futomaki","price":950,"sortOrder":0},{"available":null,"categoryId":"soups","description":null,"id":"soups-miso-soup","name":"Miso soup","price":400,"sortOrder":0},{"available":null,"categoryId":"soups","description":null,"id":"soups-ramen","name":"Ramen","price":1200,"sortOrder":1}],"warnings":[]},"notInFile":[],"products":3,"retired":0,"warnings":[]}"##;
const PIN_IMPORT_APPLY: &str = r##"200 {"applied":true,"categories":2,"heldForAllergens":["soups-miso-soup","soups-ramen"],"notInFile":[],"products":3,"retired":0,"warnings":[]}"##;
const PIN_IMPORT_EMPTY: &str = r##"400 nothing to import: the header must contain a name column and a price column; found: dish, cost"##;
const PIN_OWNER_PRODUCTS: &str = r##"200 {"products":[{"allergens":[],"available":true,"bom":[{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":180,"fat":null,"grossG":100,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Salmon","net":null,"netG":100,"out":null,"outG":100,"protein":null,"qty":100,"supply":"salmon","unit":"g","weightG":100.0}],"categoryId":"rolls","cookingMin":12,"cost":180,"description":"","id":"DISH","ingredients":["Salmon"],"ingredientsDerived":true,"name":"Futomaki","nutritionComplete":false,"price":950,"sortOrder":0,"unavailableNote":null,"weightDerived":true,"weightG":100},{"allergens":null,"available":false,"categoryId":"soups","description":"","id":"soups-miso-soup","imageUrl":null,"imageUrlSmall":null,"modifierGroups":null,"name":"Miso soup","price":400,"sizeCm":null,"sortOrder":0,"station":null,"unavailableNote":"declare this dish's allergens before it goes on sale"},{"allergens":null,"available":false,"categoryId":"soups","description":"","id":"soups-ramen","imageUrl":null,"imageUrlSmall":null,"modifierGroups":null,"name":"Ramen","price":1200,"sizeCm":null,"sortOrder":1,"station":null,"unavailableNote":"declare this dish's allergens before it goes on sale"}]}"##;
const PIN_OWNER_PRODUCT: &str = r##"200 {"products":[{"allergens":[],"available":true,"bom":[{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":180,"fat":null,"grossG":100,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Salmon","net":null,"netG":100,"out":null,"outG":100,"protein":null,"qty":100,"supply":"salmon","unit":"g","weightG":100.0}],"categoryId":"rolls","cookingMin":12,"cost":180,"description":"","id":"DISH","ingredients":["Salmon"],"ingredientsDerived":true,"name":"Futomaki","nutritionComplete":false,"price":950,"sortOrder":0,"unavailableNote":null,"weightDerived":true,"weightG":100}]}"##;

#[test]
fn a_supplies_csv_and_a_recipes_csv_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let supplies = "name,unit,cost,per\nSalmon,g,2400,1 kg\nRice,g,200,1 kg\nMayo,g,600,1 kg\n";
    let r = site.run(crate::services::catalogue::import::bulk::import_supplies, csv_call(&v.owner, "/api/owner/supplies/import", supplies), &[]);
    pin("supplies-preview", pinned(&r, &v.ids()), PIN_SUPPLIES_PREVIEW);
    let r = site.run(crate::services::catalogue::import::bulk::import_supplies, csv_call(&v.owner, "/api/owner/supplies/import?apply=1&cost=hundredths", supplies), &[]);
    pin("supplies-apply", pinned(&r, &v.ids()), PIN_SUPPLIES_APPLY);
    let r = site.run(crate::services::catalogue::import::bulk::import_supplies, csv_call(&v.owner, "/api/owner/supplies/import?apply=1", "x;y\n"), &[]);
    pin("supplies-empty", pinned(&r, &v.ids()), PIN_SUPPLIES_EMPTY);
    let recipes = "dish,ingredient,qty,unit\nFutomaki,Salmon,40,g\nFutomaki,Rice,100,g\nGhost roll,Rice,50,g\n";
    let r = site.run(crate::services::catalogue::import::bulk::import_recipes, csv_call(&v.owner, "/api/owner/recipes/import", recipes), &[]);
    pin("recipes-preview", pinned(&r, &v.ids()), PIN_RECIPES_PREVIEW);
    let r = site.run(crate::services::catalogue::import::bulk::import_recipes, csv_call(&v.owner, "/api/owner/recipes/import?apply=1", recipes), &[]);
    pin("recipes-apply", pinned(&r, &v.ids()), PIN_RECIPES_APPLY);
}
const PIN_SUPPLIES_PREVIEW: &str = r##"200 {"applied":false,"catalogue":{"fits":true,"nowPerMille":0,"perMille":0},"flattened":[],"notInFile":[],"preps":[],"recipes":0,"retired":0,"rows":[{"costPerBasis":240,"id":"salmon","kcalPer100":null,"kind":null,"lowAt":null,"name":"Salmon","new":false,"supplier":null,"unit":"g","weightPerUnit":null},{"costPerBasis":20,"id":"rice","kcalPer100":null,"kind":null,"lowAt":null,"name":"Rice","new":true,"supplier":null,"unit":"g","weightPerUnit":null},{"costPerBasis":60,"id":"mayo","kcalPer100":null,"kind":null,"lowAt":null,"name":"Mayo","new":true,"supplier":null,"unit":"g","weightPerUnit":null}],"supplies":3,"warnings":[],"withoutRecipe":[]}"##;
const PIN_SUPPLIES_APPLY: &str = r##"200 {"applied":true,"catalogue":{"fits":true,"nowPerMille":0,"perMille":0},"flattened":[],"notInFile":[],"preps":[],"recipes":0,"retired":0,"rows":[{"costPerBasis":null,"id":"salmon","kcalPer100":null,"kind":null,"lowAt":null,"name":"Salmon","new":false,"supplier":null,"unit":"g","weightPerUnit":null},{"costPerBasis":null,"id":"rice","kcalPer100":null,"kind":null,"lowAt":null,"name":"Rice","new":true,"supplier":null,"unit":"g","weightPerUnit":null},{"costPerBasis":null,"id":"mayo","kcalPer100":null,"kind":null,"lowAt":null,"name":"Mayo","new":true,"supplier":null,"unit":"g","weightPerUnit":null}],"supplies":3,"warnings":["ingredients row 2 (Salmon): cost \"2400\" comes to a fraction of the smallest ALL unit per 100 g; refused, not rounded; left out","ingredients row 3 (Rice): cost \"200\" comes to a fraction of the smallest ALL unit per 100 g; refused, not rounded; left out","ingredients row 4 (Mayo): cost \"600\" comes to a fraction of the smallest ALL unit per 100 g; refused, not rounded; left out"],"withoutRecipe":[],"written":3}"##;
const PIN_SUPPLIES_EMPTY: &str = r##"400 nothing to import: the ingredients file needs a name column and a unit column"##;
const PIN_RECIPES_PREVIEW: &str = r##"200 {"applied":false,"catalogue":{"fits":true,"nowPerMille":0,"perMille":0},"flattened":[],"notInFile":[],"preps":[],"recipes":1,"retired":0,"rows":[{"after":{"cost":null,"kcal":null,"lines":2,"weightG":140},"before":{"cost":180,"kcal":null,"lines":1,"weightG":100},"bom":[{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":72,"fat":null,"grossG":40,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Salmon","net":null,"netG":40,"out":null,"outG":40,"protein":null,"qty":40,"supply":"salmon","unit":"g","weightG":40.0},{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":null,"fat":null,"grossG":100,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Rice","net":null,"netG":100,"out":null,"outG":100,"protein":null,"qty":100,"supply":"rice","unit":"g","weightG":100.0}],"complete":false,"dish":"Futomaki","error":null,"productId":"DISH"}],"supplies":0,"warnings":["recipes row 4: dish \"Ghost roll\" is not on the menu; import the menu first"],"withoutRecipe":[]}"##;
const PIN_RECIPES_APPLY: &str = r##"200 {"applied":true,"catalogue":{"fits":true,"nowPerMille":0,"perMille":0},"flattened":[],"notInFile":[],"preps":[],"recipes":1,"retired":0,"rows":[{"after":{"cost":null,"kcal":null,"lines":2,"weightG":140},"before":{"cost":180,"kcal":null,"lines":1,"weightG":100},"bom":[{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":72,"fat":null,"grossG":40,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Salmon","net":null,"netG":40,"out":null,"outG":40,"protein":null,"qty":40,"supply":"salmon","unit":"g","weightG":40.0},{"carbs":null,"cleanPm":1000,"cookPm":1000,"cost":null,"fat":null,"grossG":100,"kcal":null,"kind":"food_ingredient","lossPm":0,"name":"Rice","net":null,"netG":100,"out":null,"outG":100,"protein":null,"qty":100,"supply":"rice","unit":"g","weightG":100.0}],"complete":false,"dish":"Futomaki","error":null,"productId":"DISH"}],"supplies":0,"warnings":["recipes row 4: dish \"Ghost roll\" is not on the menu; import the menu first"],"withoutRecipe":[],"written":1}"##;

/// A complete 1x1 PNG (magic and IEND both present).
fn png() -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==")
        .unwrap()
}

fn upload(site: &Site, owner: &str, id: &str) -> Reply {
    site.run(
        crate::services::catalogue::media::set_product_image,
        Call::new(&at("alpha", &format!("/api/owner/products/{id}/image")), worker::Method::Post).unwrap().with_body(png()).bearer(owner).on("alpha"),
        &[("id", id)],
    )
}

#[test]
fn a_dish_photo_checks_the_same_catalogue() {
    let site = Site::new();
    let v = fixture(&site);
    pin("image-ghost", pinned(&upload(&site, &v.owner, "ghost"), &v.ids()), PIN_IMAGE_GHOST);
    assert!(site.world.kv.borrow().is_empty(), "a refusal stored bytes");
    pin("image", pinned(&upload(&site, &v.owner, &v.dish), &v.ids()), PIN_IMAGE);
}
const PIN_IMAGE_GHOST: &str = r##"404 not found"##;
const PIN_IMAGE: &str = r##"200 {"bytes":70,"imageUrl":"/media/497790947d4666760ce38f3c00e852c71fdb66cae849bae8e9ede352719e1581.png","type":"image/png"}"##;

#[test]
fn the_customer_book_and_the_reveal_log_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::customers::handlers::customers, get(&at("alpha", "/api/owner/customers")).bearer(&v.owner).on("alpha"), &[]);
    pin("customers", pinned(&r, &v.ids()), PIN_CUSTOMERS);
    let key = r.body_value()["customers"][0]["key"].as_str().expect("one customer").to_string();
    let r = site.run(
        crate::services::customers::handlers::reveal_customer,
        post(&at("alpha", &format!("/api/owner/customers/{key}/reveal")), &json!({"reason": "allergy call"})).bearer(&v.owner).on("alpha"),
        &[("key", &key)],
    );
    ok(&r, "reveal");
    let r = site.run(crate::services::customers::handlers::reveals, get(&at("alpha", "/api/owner/customers/reveals")).bearer(&v.owner).on("alpha"), &[]);
    pin("reveals", pinned(&r, &v.ids()), PIN_REVEALS);
}
const PIN_CUSTOMERS: &str = r##"200 {"currency":"ALL","customers":[{"key":"6fd4bb81f5cdd9b7","lastAt":1700000000000,"name":"G.","offersWhatsapp":false,"orders":1,"phone":"+355•••••01","spent":1800}]}"##;
const PIN_REVEALS: &str = r##"200 {"reveals":[{"act":null,"at":1700000000000,"by":"OWNER","customer":"6fd4bb81f5cdd9b7","reason":"allergy call"}]}"##;

#[test]
fn the_owners_assistant_is_shown_the_same_facts() {
    let site = Site::new();
    let v = fixture(&site);
    ai_on(&site, &v.owner);
    model("Two Futomaki, waiting.");
    let r = site.run(
        crate::services::engagement::assist::owner_assist,
        post(&at("alpha", "/api/owner/assist"), &json!({"question": "which dishes need salmon?"})).bearer(&v.owner).on("alpha"),
        &[],
    );
    pin("assist", pinned(&r, &v.ids()), PIN_ASSIST);
    pin("assist-facts", model_saw(&v.ids()), PIN_ASSIST_FACTS);
    let r = site.run(crate::services::engagement::assist::graph, get(&at("alpha", "/api/owner/graph?q=salmon&limit=5")).bearer(&v.owner).on("alpha"), &[]);
    pin("graph", pinned(&r, &v.ids()), PIN_GRAPH);
    let r = site.run(crate::services::engagement::assist::graph, get(&at("alpha", "/api/owner/graph")).bearer(&v.owner).on("alpha"), &[]);
    pin("graph-shape", pinned(&r, &v.ids()), PIN_GRAPH_SHAPE);
}
const PIN_ASSIST: &str = r##"200 {"answer":"Two Futomaki, waiting.","redacted":true}"##;
const PIN_ASSIST_FACTS: &str = r##"{"max_tokens":400,"messages":[{"content":"You help the owner of one restaurant read their own live order data and stock. The FACTS block below is the truth; it was computed by the system, not by you. Never invent an order, a number, a name or a time that is not in it. If the answer is not in the FACTS, say you do not have it. Answer in the language the question was asked in. Be brief: two or three sentences, or a short list. Do not add pleasantries.","role":"system"},{"content":"FACTS:\n{\"hub_knows\":{\"found\":[{\"id\":\"ingredient:salmon\",\"kind\":\"ingredient\",\"label\":\"на складі 4900 зарезервовано 200\",\"related\":[],\"relevance\":32786}],\"nodes\":5,\"relations\":4},\"live_orders\":[{\"courier_id\":null,\"fulfilment\":\"pickup\",\"id\":\"ORDER\",\"items\":[{\"modifier_ids\":[],\"name\":\"[withheld]\",\"product_id\":\"DISH\",\"quantity\":2,\"unit_price\":900}],\"status\":\"PENDING\",\"total\":1800,\"waiting_minutes\":0}],\"now_ms\":1700000000000,\"off_the_menu\":[]}\n\nQUESTION:\nwhich dishes need salmon?","role":"user"}],"model":"tiny","stream":false,"temperature":0.2}"##;
const PIN_GRAPH: &str = r##"200 {"found":[{"id":"ingredient:salmon","kind":"ingredient","label":"на складі 4900 зарезервовано 200","related":[],"relevance":32786}],"nodes":5,"relations":4}"##;
const PIN_GRAPH_SHAPE: &str = r##"200 {"found":[],"nodes":5,"relations":4}"##;

#[test]
fn the_kitchens_assistant_is_shown_the_same_facts() {
    let site = Site::new();
    let v = fixture(&site);
    ai_on(&site, &v.owner);
    model("One ticket at the pass.");
    let r = site.run(
        crate::services::engagement::assist::kitchen::kitchen_assist,
        post(&at("alpha", "/api/staff/assist"), &json!({"question": "what is waiting?"})).bearer(&v.cook).on("alpha"),
        &[],
    );
    pin("kitchen-assist", pinned(&r, &v.ids()), PIN_KITCHEN_ASSIST);
    pin("kitchen-facts", model_saw(&v.ids()), PIN_KITCHEN_FACTS);
}
const PIN_KITCHEN_ASSIST: &str = r##"200 {"answer":"One ticket at the pass.","redacted":true}"##;
const PIN_KITCHEN_FACTS: &str = r##"{"max_tokens":400,"messages":[{"content":"You help the kitchen of one restaurant read its own open tickets, its menu and its stock. The FACTS block below is the truth; it was computed by the system, not by you. Never invent a dish, a quantity, a ticket or a time that is not in it. If the answer is not in the FACTS, say you do not have it. Answer in the language the question was asked in, in one to three short lines: it is read at the pass, with wet hands.","role":"system"},{"content":"FACTS:\n{\"now_ms\":1700000000000,\"off_the_menu\":[],\"open_tickets\":[{\"items\":[{\"dish\":\"Futomaki\",\"qty\":2,\"station\":\"kitchen\"}],\"kind\":\"pickup\",\"status\":\"PENDING\",\"table\":null,\"ticket\":\"ORDER\",\"waiting_minutes\":0}],\"shelf\":[{\"available\":4700,\"id\":\"salmon\",\"ingredient\":\"Salmon\",\"on_hand\":4900,\"reserved\":200}]}\n\nQUESTION:\nwhat is waiting?","role":"user"}],"model":"tiny","stream":false,"temperature":0.2}"##;

/// The ids a draft minted, so they scrub like the fixture's.
fn post_ids(list: &Value) -> Vec<(String, &'static str)> {
    list["posts"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| p["id"].as_str().map(|s| (s.to_string(), "POST"))).collect())
        .unwrap_or_default()
}

#[test]
fn a_social_draft_derives_the_same_subjects_and_an_approval_finds_the_same_photo() {
    let site = Site::new();
    let v = fixture(&site);
    ai_on(&site, &v.owner);
    feature(&site, &v.owner, "social.enabled");
    for (k, val) in [("social.instagram.token", "ig-token"), ("social.instagram.user_id", "1789")] {
        setting(&site, &v.owner, k, val);
    }
    model("Uramaki is here: rice outside, salmon inside. Come taste it.");
    let draft = || site.run(crate::services::engagement::posts::draft_post, post(&at("alpha", "/api/owner/posts/draft"), &json!({})).bearer(&v.owner).on("alpha"), &[]);
    // The first draft takes the catalogue snapshot ("open again"); a dish added
    // after it is what the second draft derives FROM THE CATALOGUE.
    ok(&draft(), "first draft");
    let new_dish = {
        let r = site.run(
            crate::catalog_edit::create_product,
            post(&at("alpha", "/api/owner/products"), &json!({"location_id": "alpha", "category_id": "rolls", "name": "Uramaki", "price": 1200})).bearer(&v.owner).on("alpha"),
            &[],
        );
        ok(&r, "uramaki");
        let id = r.body_value()["id"].as_str().unwrap().to_string();
        let r = site.run(
            crate::owner::update_product,
            post(&at("alpha", &format!("/api/owner/products/{id}")), &json!({"location_id": "alpha", "allergens": [], "available": true})).bearer(&v.owner).on("alpha"),
            &[("id", &id)],
        );
        ok(&r, "on sale");
        id
    };
    let r = draft();
    let mut ids = v.ids();
    ids.push((new_dish.clone(), "URAMAKI"));
    ids.extend(post_ids(&r.body_value()));
    pin("draft", pinned(&r, &ids), PIN_DRAFT);
    pin("draft-prompt", model_saw(&ids), PIN_DRAFT_PROMPT);
    let list = site.run(crate::services::engagement::posts::posts, get(&at("alpha", "/api/owner/posts")).bearer(&v.owner).on("alpha"), &[]).body_value();
    let id = list["posts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["state"] == "draft" && p["about"].as_str().unwrap_or("").contains("Uramaki"))
        .map(|p| p["id"].as_str().unwrap().to_string())
        .unwrap_or_else(|| panic!("no draft about the dish: {list}"));
    let approve = |id: &str| {
        site.run(crate::services::engagement::verdict::approve_post, post(&at("alpha", &format!("/api/owner/posts/{id}/approve")), &json!({})).bearer(&v.owner).on("alpha"), &[("id", id)])
    };
    // Instagram alone, and the dish has no photo yet: refused, said.
    pin("approve-nophoto", pinned(&approve(&id), &ids), PIN_APPROVE_NOPHOTO);
    ok(&upload(&site, &v.owner, &new_dish), "photo");
    pin("approve", pinned(&approve(&id), &ids), PIN_APPROVE);
    let media = sent().into_iter().filter(|c| c.url().unwrap().path().ends_with("/1789/media")).last().expect("a container");
    let mut body: Value = serde_json::from_slice(&media.body_bytes()).unwrap();
    scrub(&mut body, &ids);
    pin("approve-ig", body.to_string(), PIN_APPROVE_IG);
}
const PIN_DRAFT: &str = r##"200 {"drafted":1,"posts":[{"about":"Uramaki has been added to the menu","id":"POST","text":"Uramaki is here: rice outside, salmon inside. Come taste it."}]}"##;
const PIN_DRAFT_PROMPT: &str = r##"{"max_tokens":400,"messages":[{"content":"You write one short social post for a single restaurant. You are given ONE FACT that the restaurant's own system computed. Write about that fact and nothing else. NEVER invent a discount, a price, a deadline, an ingredient, an award, or a quantity that is not in the fact. NEVER write 'limited time', 'hurry', or 'don't miss out'. Two sentences at most. Warm, plain, the way a small restaurant actually speaks. Write in the language you are asked for. Output only the post text: no quotes, no hashtags unless they are the restaurant's own name, no emoji beyond one.","role":"system"},{"content":"FACTS:\n{}\n\nQUESTION:\nRESTAURANT: Venue alpha\nLANGUAGE: Albanian\nFACT: Uramaki has been added to the menu\n\nWrite the post.","role":"user"}],"model":"tiny","stream":false,"temperature":0.2}"##;
const PIN_APPROVE_NOPHOTO: &str = r##"502 no Telegram bot is configured · Instagram: the post's dish has no photo"##;
const PIN_APPROVE: &str = r##"200 {"ok":true,"state":"published"}"##;
const PIN_APPROVE_IG: &str = r##"{"caption":"Uramaki is here: rice outside, salmon inside. Come taste it.","image_url":"https://alpha.dowiz.org/media/497790947d4666760ce38f3c00e852c71fdb66cae849bae8e9ede352719e1581.png"}"##;

#[test]
fn voice_reads_the_same_shelf_and_the_same_menu() {
    let site = Site::new();
    let v = fixture(&site);
    let say = |tok: &str, transcript: &str| {
        site.run(
            crate::services::engagement::voice::voice,
            post(&at("alpha", "/api/voice"), &json!({"transcript": transcript, "confidence": 0.95, "is_final": true, "lang": "en"})).bearer(tok).on("alpha"),
            &[],
        )
    };
    pin("voice-owner-stock", pinned(&say(&v.owner, "received 4 kg salmon"), &v.ids()), PIN_VOICE_OWNER_STOCK);
    pin("voice-cook-stock", pinned(&say(&v.cook, "received 4 kg salmon"), &v.ids()), PIN_VOICE_COOK_STOCK);
    pin("voice-waiter-add", pinned(&say(&v.waiter, "add 2 futomaki to table 5"), &v.ids()), PIN_VOICE_WAITER_ADD);
    pin("voice-owner-off", pinned(&say(&v.owner, "take futomaki off the menu"), &v.ids()), PIN_VOICE_OWNER_OFF);
    pin("voice-cook-off", pinned(&say(&v.cook, "take futomaki off the menu"), &v.ids()), PIN_VOICE_COOK_OFF);
}
const PIN_VOICE_OWNER_STOCK: &str = r##"200 {"itemId":"salmon","needsConfirmation":true,"readback":"received onto the shelf: 4000 g Salmon","token":"TOKEN","understood":true,"verb":"receive"}"##;
const PIN_VOICE_COOK_STOCK: &str = r##"200 {"itemId":"salmon","needsConfirmation":true,"readback":"received onto the shelf: 4000 g Salmon","token":"TOKEN","understood":true,"verb":"receive"}"##;
const PIN_VOICE_WAITER_ADD: &str = r##"200 {"action":"draft_add","name":"Futomaki","needsConfirmation":false,"productId":"DISH","quantity":2,"table":"5","understood":true}"##;
const PIN_VOICE_OWNER_OFF: &str = r##"200 {"needsConfirmation":true,"productId":"DISH","readback":"take off sale: Futomaki","token":"TOKEN","understood":true,"verb":"dish_off"}"##;
const PIN_VOICE_COOK_OFF: &str = r##"200 {"needsConfirmation":true,"productId":"DISH","readback":"take off sale: Futomaki","token":"TOKEN","understood":true,"verb":"dish_off"}"##;

#[test]
fn the_waste_report_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::operations::waste::waste_report, get(&at("alpha", "/api/owner/stock/waste")).bearer(&v.owner).on("alpha"), &[]);
    pin("waste", pinned(&r, &v.ids()), PIN_WASTE);
}
const PIN_WASTE: &str = r##"200 {"rows":[{"at":1700000000000,"by":"OWNER","chosen_by":null,"item":"salmon","lot":null,"order":null,"qty":100,"reason":"spoiled","source":"stock","value":null}],"totals":{"byReason":{"stock":{"spoiled":100}},"bySigner":{"stock":{"OWNER":100}},"unvalued":1,"value":0,"valueByReason":{}},"venue":"alpha"}"##;

#[test]
fn branding_activation_and_reach_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::venue::brand::branding, get(&at("alpha", "/api/owner/branding")).bearer(&v.owner).on("alpha"), &[]);
    pin("branding", pinned(&r, &v.ids()), PIN_BRANDING);
    let r = site.run(crate::services::venue::activation::activation, get(&at("alpha", "/api/owner/activation")).bearer(&v.owner).on("alpha"), &[]);
    pin("activation", pinned(&r, &v.ids()), PIN_ACTIVATION);
    let r = site.run(crate::services::venue::zones::reach, get(&at("alpha", "/api/public/reach?lat_udeg=41310000&lon_udeg=19450000")).on("alpha"), &[]);
    pin("reach", pinned(&r, &v.ids()), PIN_REACH);
    let r = site.run(
        crate::services::venue::zones::set_zones,
        post(&at("alpha", "/api/owner/zones"), &json!({"zones": [{"kind": "circle", "lat": 41310000, "lon": 19450000, "radius_m": 1500}]})).bearer(&v.owner).on("alpha"),
        &[],
    );
    ok(&r, "zones");
    let r = site.run(crate::services::venue::zones::reach, get(&at("alpha", "/api/public/reach?lat_udeg=41400000&lon_udeg=19450000")).on("alpha"), &[]);
    pin("reach-zoned", pinned(&r, &v.ids()), PIN_REACH_ZONED);
}
const PIN_BRANDING: &str = r##"200 {"brand":{"ink":"#1a1a1a","paper":"#fbfaf8","primary":"#aa3311","radius":8,"typePair":"classic"},"presets":[{"id":"cosmo-noir","ink":"#1a1a1a","label":"Cosmo Noir","paper":"#fbfaf8","primary":"#d69a3d","radius":8,"typePair":"classic"},{"id":"adriatic","ink":"#10242b","label":"Adriatic","paper":"#f4f8f9","primary":"#1c6e8c","radius":12,"typePair":"modern"},{"id":"terracotta","ink":"#2b1a12","label":"Terracotta","paper":"#faf4ef","primary":"#b4552d","radius":16,"typePair":"warm"},{"id":"olive","ink":"#1e2416","label":"Olive","paper":"#f7f8f2","primary":"#5f7a3a","radius":10,"typePair":"warm"},{"id":"ink","ink":"#141416","label":"Ink","paper":"#ffffff","primary":"#2f2f33","radius":0,"typePair":"plain"},{"id":"rose","ink":"#2a121a","label":"Rose","paper":"#fdf5f7","primary":"#a83254","radius":14,"typePair":"modern"}],"radiusMax":20,"typePairs":[{"id":"classic","label":"Classic"},{"id":"modern","label":"Modern"},{"id":"warm","label":"Warm"},{"id":"plain","label":"Plain"}]}"##;
const PIN_ACTIVATION: &str = r##"200 {"canOpen":true,"facts":{"deliveryConfigured":true,"hasVenuePhone":true,"pickupEnabled":true,"sellableDishes":1,"telegramChats":0},"missing":[]}"##;
const PIN_REACH: &str = r##"200 {"metresAway":null,"reach":"unrestricted"}"##;
const PIN_REACH_ZONED: &str = r##"200 {"metresAway":8501,"reach":"outside"}"##;

#[test]
fn a_bootstrap_answers_the_same_root() {
    let site = Site::new();
    let location = json!({"id": "gamma", "slug": "gamma", "name": "Gamma", "phone": "+355690000003", "status": "open",
        "delivery_eta": "30-45", "delivery_fee": 0, "min_order": 0, "currency_code": "ALL", "menu_version": 1,
        "supported_locales": "sq,en", "default_locale": "sq", "delivery_paused": 0});
    let r = site.run(
        crate::bootstrap::seed,
        post(&apex("/api/bootstrap"), &json!({"location": location,
            "categories": [{"id": "c1", "name": "Mains"}], "products": [{"id": "p1", "name": "Soup", "price": 500, "category_id": "c1"}],
            "owner": {"email": "g@x.test", "password": "owner-password-1"}}))
            .with_header("x-dowiz-bootstrap", "bootstrap-secret-bootstrap-secret-0"),
        &[],
    );
    let owner_id = r.body_value()["ownerId"].as_str().map(|s| (s.to_string(), "OWNER")).into_iter().collect::<Vec<_>>();
    pin("bootstrap", pinned(&r, &owner_id), PIN_BOOTSTRAP);
}
const PIN_BOOTSTRAP: &str = r##"200 {"catalogRoot":"a81d28c2d14cf9d1","categories":1,"couriers":0,"ok":true,"ownerId":"OWNER","products":1,"secretHash":"7cc40e68"}"##;
