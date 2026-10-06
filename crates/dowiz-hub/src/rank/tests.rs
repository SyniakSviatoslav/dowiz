//! W-TASTE row 1: the hub's twin of the phone's strip gives the shared fixture's answer, 1000/1000.
//! The same file is read by `workers/api/public/store/taste-int.test.mjs` with the phone's taste.js,
//! so the two rankers agree with each other through it (`fixtures/rank/gen.mjs` wrote it once).

use super::strip::{self, Opts};
use super::*;
use serde_json::Value;

fn fixture() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/rank/strip.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).expect("strip.json is JSON")
}

fn map(v: &Value) -> BTreeMap<String, i64> {
    v.as_object().into_iter().flatten().filter_map(|(k, x)| Some((k.clone(), x.as_i64()?))).collect()
}

fn opts(o: &Value) -> Opts {
    let strs = |v: Option<&Value>| v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect::<Vec<_>>());
    Opts {
        avoid_guess: strs(o.get("avoidGuess")).unwrap_or_default(),
        prior: o.get("prior").filter(|p| !p.is_null()).cloned(),
        ctx: strs(o.get("ctx")),
        mood: o.get("mood").and_then(Value::as_str).map(str::to_string),
    }
}

#[test]
fn the_half_life_table_is_the_phones_and_sixty_days_is_exactly_half() {
    let f = fixture();
    let half: Vec<i64> = f["half"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    assert_eq!(half, HALF.to_vec(), "the Q32 chain gives the same table in both languages");
    assert_eq!(HALF[0], 1 << 16);
    assert_eq!(fade(1000, 60), 500);
    assert_eq!(fade(-1000, 60), -500);
    assert_eq!(fade(1000, -3), 1000, "a day after today does not grow a weight");
    assert_eq!(fade(1000, 60 * 37), 0);
    let rows = f["fade"].as_array().unwrap();
    let bad: Vec<&Value> = rows.iter().filter(|r| fade(r[0].as_i64().unwrap(), r[1].as_i64().unwrap()) != r[2].as_i64().unwrap()).collect();
    assert!(bad.is_empty(), "{} of {} fade spot checks differ, first {:?}", bad.len(), rows.len(), bad.first());
}

#[test]
fn per_mille_cosine_is_exact_and_truncates_toward_zero() {
    assert_eq!((isqrt(0), isqrt(15), isqrt(16), isqrt((1 << 52) + 1)), (0, 3, 4, 1 << 26));
    let v: BTreeMap<String, i64> = [("t:spicy".to_string(), 800), ("x:crispy".to_string(), 1000)].into_iter().collect();
    assert_eq!(cos_pm(&v, &v), 1000);
    assert_eq!(cos_pm(&v, &BTreeMap::new()), 0);
    let rows = fixture()["cos"].as_array().unwrap().clone();
    let bad = rows.iter().filter(|r| cos_pm(&map(&r[0]), &map(&r[1])) != r[2].as_i64().unwrap() || per_mille(&map(&r[0])) != map(&r[3])).count();
    assert_eq!(bad, 0, "{bad} of {} cosine spot checks differ", rows.len());
}

#[test]
fn the_hub_ranks_every_fixture_guest_exactly_as_the_phone_1000_of_1000() {
    let f = fixture();
    let menus: Vec<Vec<Value>> = f["menus"].as_array().unwrap().iter().map(|m| m.as_array().unwrap().clone()).collect();
    let cases = f["cases"].as_array().unwrap();
    let mut same = 0;
    let mut first = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let menu = &menus[c["menu"].as_u64().unwrap() as usize];
        let got: Vec<Value> = strip::strip(menu, &c["profile"], c["day"].as_i64().unwrap(), &opts(&c["opts"]))
            .into_iter()
            .map(|x| serde_json::json!([x.id, x.why, i64::from(x.guessed), x.s]))
            .collect();
        if Value::Array(got.clone()) == c["want"] {
            same += 1;
        } else if first.len() < 3 {
            first.push(format!("case {i}: got {} want {}", Value::Array(got), c["want"]));
        }
    }
    println!("MEASURED hub strip vs shared fixture: {same}/{} equal rankings (ids, reasons and integer scores)", cases.len());
    assert_eq!(same, cases.len(), "{}", first.join("\n"));
}

#[test]
fn an_inferred_allergy_moves_a_dish_down_and_never_out() {
    let menu: Vec<Value> = serde_json::from_str(
        r#"[{"id":"maki","categoryId":"rolls","tags":["salmon"]},{"id":"ramen","categoryId":"soups","tags":["hot"],"allergens":["gluten"]},
            {"id":"udon","categoryId":"soups","tags":["hot"],"allergens":["gluten"]}]"#,
    )
    .unwrap();
    let p: Value = serde_json::from_str(r#"{"v":1,"dishes":{"ramen":{"open":[[100,2]]},"maki":{"open":[[100,1]]}},"cats":{}}"#).unwrap();
    let plain: Vec<String> = strip::strip(&menu, &p, 100, &Opts::default()).into_iter().map(|x| x.id).collect();
    let o = Opts { avoid_guess: vec!["gluten".into()], ..Opts::default() };
    let moved: Vec<String> = strip::strip(&menu, &p, 100, &o).into_iter().map(|x| x.id).collect();
    let (mut a, mut b) = (plain.clone(), moved.clone());
    a.sort();
    b.sort();
    assert_eq!(a, b, "the same dishes");
    assert_eq!(moved[0], "maki", "{moved:?}");
}
