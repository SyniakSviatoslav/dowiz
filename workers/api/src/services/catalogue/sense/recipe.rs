//! PURE. "Suggest" from the dish's RECIPE (W-TASTE2 S7b): the harmonic extension of the declared
//! dishes' taste over the dish--supply graph (`dowiz_hub::sense::harmonic`), read from the venue's
//! own blocks (`taste`, and the catalogue projection's `menu_prices`, `bom`, `names`). A SECOND
//! source beside the lexicon: never merged into the main draft, never written, each value marked
//! `from = recipe`. With no recipes it answers the state and nothing else -- no fabricated values.

use dowiz_hub::block::view::{Catalogue, View};
use dowiz_hub::sense::harmonic::{self, Draft};
use serde_json::{json, Value};

/// The recipe draft as the Suggest answer carries it.
pub fn json_of(d: &Draft) -> Value {
    let Draft::From { vector, neighbours, passes } = d else { return json!({ "state": d.word() }) };
    let s = harmonic::to_sense(vector);
    if s.is_empty() {
        return json!({ "state": "faint", "neighbours": neighbours, "passes": passes });
    }
    let why: Vec<Value> = dowiz_hub::sense::vector(&s).keys().map(|k| json!({ "key": k, "from": "recipe" })).collect();
    json!({ "state": d.word(), "draft": s.json(), "why": why, "neighbours": neighbours, "passes": passes })
}

/// The draft for `id` from the four blocks; `None` for a block the object did not have.
pub fn of_blocks(taste: Option<&[u8]>, menu: Option<&[u8]>, bom: Option<&[u8]>, names: Option<&[u8]>, id: &str) -> Value {
    let (Some(t), Some(m), Some(b), Some(n)) = (taste, menu, bom, names) else { return json!({ "state": "no-block" }) };
    let read = View::new(t).map_err(|e| format!("{e:?}")).and_then(|tv| Catalogue::new(m, b, n).map(|c| (tv, c)).map_err(|e| format!("{e:?}")));
    match read {
        Ok((tv, cat)) => json_of(&harmonic::extend(&harmonic::dishes_of(&tv, &cat), id, harmonic::PASSES)),
        Err(why) => json!({ "state": "unreadable", "error": why }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_hub::block::encode::{encode, project};

    /// The four blocks of a small catalogue, as the object folds them.
    fn blocks(products: &[(String, String)]) -> [Vec<u8>; 4] {
        let p = project(products).unwrap();
        let (t, _) = dowiz_hub::block::taste::project(products).unwrap();
        [encode(&t).unwrap(), encode(&p.menu_prices).unwrap(), encode(&p.bom).unwrap(), encode(&p.names).unwrap()]
    }
    fn dish(id: &str, bom: serde_json::Value, sense: serde_json::Value) -> (String, String) {
        (id.into(), json!({ "id": id, "name": id, "price": 900, "bom": bom, "sense": sense }).to_string())
    }
    fn ask(products: &[(String, String)], id: &str) -> Value {
        let b = blocks(products);
        of_blocks(Some(&b[0]), Some(&b[1]), Some(&b[2]), Some(&b[3]), id)
    }

    #[test]
    fn with_no_recipes_it_says_so_and_drafts_nothing() {
        let menu = vec![dish("a", json!([]), json!({"aroma": {"smoky": 3}})), dish("b", json!([]), json!(null))];
        let v = ask(&menu, "b");
        assert_eq!(v, json!({ "state": "no-recipes" }), "no draft, no number: {v}");
        assert_eq!(of_blocks(None, None, None, None, "b"), json!({ "state": "no-block" }));
    }

    #[test]
    fn a_recipe_sharing_supplies_with_declared_dishes_gets_a_draft_marked_from_recipe() {
        let menu = vec![
            dish("unagi-don", json!([{"supply": "eel", "qty": 100}, {"supply": "rice", "qty": 100}]), json!({"taste": {"salty": 3}, "aroma": {"smoky": 3}})),
            dish("mango-roll", json!([{"supply": "mango", "qty": 100}, {"supply": "rice", "qty": 100}]), json!({"taste": {"sweet": 5}})),
            dish("eel-nigiri", json!([{"supply": "eel", "qty": 60}, {"supply": "rice", "qty": 40}]), json!(null)),
        ];
        let v = ask(&menu, "eel-nigiri");
        assert_eq!(v["state"], "drafted", "{v}");
        assert!(v["draft"]["aroma"]["smoky"].as_i64().unwrap() >= 2, "{v}");
        assert!(dowiz_hub::sense::validate(&v["draft"]).is_ok(), "a valid edit: {v}");
        assert!(v["why"].as_array().unwrap().iter().all(|w| w["from"] == "recipe"), "{v}");
        assert_eq!(v["neighbours"], 2);
        // Positive twin of "no recipe": the declared dish with no recipe line of its own.
        let mut lone = menu.clone();
        lone.push(dish("miso", json!([]), json!({"taste": {"umami": 4}})));
        assert_eq!(ask(&lone, "miso")["state"], "no-recipe");
    }

    #[test]
    fn a_corrupt_block_is_said_not_drafted() {
        let menu = vec![dish("a", json!([{"supply": "x", "qty": 1}]), json!(null))];
        let mut b = blocks(&menu);
        let n = b[2].len();
        b[2][n - 1] ^= 0xff;
        assert_eq!(of_blocks(Some(&b[0]), Some(&b[1]), Some(&b[2]), Some(&b[3]), "a")["state"], "unreadable");
    }
}
