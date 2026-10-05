//! P16b: the lexicon in four languages, negation, the per-dish-per-week fold,
//! and the redaction of the example phrase.

use super::*;
use std::collections::HashMap;

fn t(text: &str) -> Vec<&'static str> {
    topics_of(text).into_iter().map(|(t, _)| t).collect()
}

#[test]
fn the_lexicon_in_four_languages() {
    for (text, want) in [
        ("the rice was cold", vec!["cold"]),
        ("Рис був холодний", vec!["cold"]),
        ("рис был холодный", vec!["cold"]),
        ("orizi ishte i ftohtë", vec!["cold"]),
        ("arrived very late", vec!["late"]),
        ("доставка запізнилась на годину", vec!["late"]),
        ("курьер опоздал", vec!["late"]),
        ("erdhi shumë vonë", vec!["late"]),
        ("way too salty", vec!["salty"]),
        ("суп пересолений", vec!["salty"]),
        ("очень солёный", vec!["salty"]),
        ("shumë i kripur", vec!["salty"]),
        ("the portion was small", vec!["small_portion"]),
        ("порція маленька", vec!["small_portion"]),
        ("порция маленькая", vec!["small_portion"]),
        ("porcioni i vogël", vec!["small_portion"]),
        ("delicious, and fresh fish", vec!["tasty", "fresh"]),
        ("дуже смачно", vec!["tasty"]),
        ("очень вкусно", vec!["tasty"]),
        ("shumë e shijshme", vec!["tasty"]),
        ("too spicy for me", vec!["spicy"]),
        ("дуже гостро", vec!["spicy"]),
        ("the sauce spilled, bad packaging", vec!["packaging"]),
        ("упаковка порвалась", vec!["packaging"]),
        ("ambalazhi ishte i prishur", vec!["packaging"]),
        ("the fish was stale", vec!["stale"]),
        ("перепрошую, все добре", vec![]),
    ] {
        assert_eq!(t(text), want, "{text:?}");
    }
}

/// "not tasty" is a complaint, not a compliment; "not cold" says nothing.
#[test]
fn a_negated_word_flips_or_drops() {
    assert_eq!(t("not tasty at all"), vec!["not_tasty"]);
    assert_eq!(t("не смачно"), vec!["not_tasty"]);
    assert_eq!(t("не свіжа риба"), vec!["stale"]);
    assert_eq!(t("nuk ishte e freskët"), vec!["stale"]);
    assert_eq!(t("the soup was not cold"), Vec::<&str>::new());
    assert_eq!(t("cold, cold, so cold"), vec!["cold"], "one note counts once");
}

fn dish(id: &str, name: &str) -> (String, Dish) {
    (id.into(), Dish { id: id.into(), name: name.into(), lines: Vec::new(), leaves: Vec::new() })
}
/// 2026-10-05 (a Monday) to 2026-10-14, days of 1000 ms for the test.
fn win() -> Window {
    let days: Vec<i64> = (5..=14).map(|d| 20261000 + d).collect();
    Window { starts: (0..10).map(|k| k * 1000).collect(), end: 10_000, days }
}
fn order(id: &str, items: &[&str], text: &str, at: i64) -> Value {
    json!({
        "id": id, "courier_id": "c-arben", "customer_phone": "+355691234567", "status": "DELIVERED",
        "items": items.iter().map(|p| json!({ "product_id": p, "quantity": 1 })).collect::<Vec<_>>(),
        "feedback": { "text": text, "at": at },
    })
}

#[test]
fn topics_are_counted_per_dish_per_week_and_never_per_person() {
    let dishes: HashMap<String, Dish> = [dish("roll", "Salmon roll"), dish("rice", "Rice bowl")].into_iter().collect();
    let orders = vec![
        order("o1", &["roll", "rice"], "the rice bowl was cold", 1500),        // names the bowl: the bowl only
        order("o2", &["roll", "rice"], "everything cold", 2500),               // names none: both dishes
        order("o3", &["roll"], "cold and salty", 7500),                        // 2026-10-12, the next week
        order("o4", &["roll"], "cold", 20_000),                                // outside the window
        order("o5", &["roll"], "thank you", 3000),                             // no topic
    ];
    let out = fold(&orders, &dishes, &win());
    let rows: Vec<(i64, &str, &str, u64)> = out.as_array().unwrap().iter()
        .map(|r| (r["week"].as_i64().unwrap(), r["dish"].as_str().unwrap(), r["topic"].as_str().unwrap(), r["count"].as_u64().unwrap()))
        .collect();
    assert_eq!(rows, vec![
        (20261005, "rice", "cold", 2),
        (20261005, "roll", "cold", 1),
        (20261012, "roll", "cold", 1),
        (20261012, "roll", "salty", 1),
    ]);
    // The key is (week, dish, topic): no order, guest, courier or phone reaches the answer.
    for r in out.as_array().unwrap() {
        let mut keys: Vec<&String> = r.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["count", "dish", "examples", "name", "topic", "week"]);
    }
    let all = out.to_string();
    for leak in ["o1", "o2", "c-arben", "+355", "691234567"] {
        assert!(!all.contains(leak), "{leak} reached the topics");
    }
    assert_eq!(week_of(20261011), 20261005, "Sunday belongs to Monday's week");
    assert_eq!(week_of(20261012), 20261012);
}

#[test]
fn an_example_phrase_never_shows_a_phone_an_address_or_a_name() {
    let dishes: HashMap<String, Dish> = [dish("rice", "Rice bowl")].into_iter().collect();
    let notes = [
        "The rice was cold, call me on +355 69 123 4567 please",
        "rice cold!! my number 0691234567",
        "Courier Arben was late and the rice was cold",
        "the courier arben was late, rice cold",
        "Рис холодний, телефонуйте 067-123-45-67, Олена",
        "rice cold, write to ana@example.com, Rruga Durrës 12",
    ];
    for (i, text) in notes.iter().enumerate() {
        let out = fold(&[order(&format!("o{i}"), &["rice"], text, 1000)], &dishes, &win());
        for row in out.as_array().unwrap() {
            let ex = row["examples"][0].as_str().unwrap().to_string();
            assert!(!ex.chars().any(|c| c.is_ascii_digit()), "{text:?} -> {ex:?}");
            for name in ["arben", "Arben", "Олена", "олена", "ana@", "example", "durrës", "Durrës", "rruga"] {
                assert!(!ex.contains(name), "{text:?} -> {ex:?}");
            }
        }
        let ex = out[0]["examples"][0].as_str().unwrap();
        assert!(ex.contains("cold") || ex.contains("холодний"), "the phrase still says what was wrong: {ex:?}");
    }
}
