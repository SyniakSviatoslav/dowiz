//! The dish's sensory profile: the vocabulary, its ranges, the old field, the vector, the drafts.

use super::*;
use serde_json::json;

#[test]
fn six_taste_axes_zero_to_five_are_kept_as_given_and_a_zero_is_a_claim() {
    let s = validate(&json!({"taste": {"sweet": 0, "sour": 1, "salty": 2, "bitter": 3, "umami": 4, "spicy": 5}})).unwrap();
    assert_eq!(s.taste.len(), 6);
    assert_eq!(s.taste["sweet"], 0, "0 = 'not sweet at all', a declared value");
    assert_eq!(s.taste["spicy"], 5);
    assert_eq!(TASTE, ["sweet", "sour", "salty", "bitter", "umami", "spicy"], "the ids are stable");
}

#[test]
fn an_axis_out_of_range_or_unknown_or_not_an_integer_is_refused_by_name() {
    for (bad, says) in [
        (json!({"taste": {"spicy": 6}}), "0 to 5"),
        (json!({"taste": {"spicy": -1}}), "0 to 5"),
        (json!({"taste": {"richness": 2}}), "unknown taste"),
        (json!({"taste": {"spicy": 2.5}}), "whole number"),
        (json!({"taste": {"spicy": "2"}}), "whole number"),
        (json!({"taste": {"spicy": true}}), "whole number"),
        (json!({"texture": {"crispy": 4}}), "1 to 3"),
        (json!({"texture": {"slimy": 1}}), "unknown texture"),
        (json!({"aroma": {"gluten": 1}}), "unknown aroma"),
        (json!({"smell": {"smoky": 1}}), "unknown sense dimension"),
        (json!({"v": 2}), "sense.v"),
        (json!([1, 2]), "an object"),
    ] {
        let e = validate(&bad).unwrap_err();
        assert!(e.contains(says), "{bad} -> {e}");
    }
    // The positive twin of each: the same shapes in range pass.
    assert!(validate(&json!({"taste": {"spicy": 0}, "texture": {"crispy": 3}, "aroma": {"smoky": 1}, "v": 1})).is_ok());
}

#[test]
fn a_tag_sent_at_zero_or_null_is_removed_and_an_empty_profile_stores_as_null() {
    let s = validate(&json!({"texture": {"crispy": 0, "creamy": null, "soft": 2}, "aroma": {"citrus": 0}})).unwrap();
    assert_eq!(s.texture.keys().collect::<Vec<_>>(), ["soft"]);
    assert!(s.aroma.is_empty());
    assert_eq!(validate(&json!({"texture": {"crispy": 0}})).unwrap().json(), Value::Null);
    assert_eq!(s.json()["v"], 1);
}

#[test]
fn no_allergen_and_no_health_word_is_in_the_vocabulary() {
    for w in crate::allergens::EU14.iter().chain(["healthy", "diet", "vegan", "light", "calorie"].iter()) {
        for d in Dim::ALL {
            assert!(!d.words().contains(w), "{w} must not be a {} id", d.wire());
        }
    }
}

#[test]
fn the_old_taste_field_reads_on_the_new_scale_and_richness_is_not_carried() {
    assert_eq!((legacy_level(1), legacy_level(2), legacy_level(3)), (2, 3, 5));
    let p = json!({"taste": {"spicy": 3, "sweet": 1, "richness": 2}});
    let s = of_product(&p).unwrap();
    assert_eq!(s.taste.get("spicy"), Some(&5));
    assert_eq!(s.taste.get("sweet"), Some(&2));
    assert!(!s.taste.contains_key("richness"));
    // `sense` wins over the old field; nothing at all is None (no fake zeros).
    let both = json!({"taste": {"spicy": 3}, "sense": {"v": 1, "taste": {"spicy": 1}}});
    assert_eq!(of_product(&both).unwrap().taste["spicy"], 1);
    assert_eq!(of_product(&json!({"name": "x"})), None);
    assert_eq!(of_product(&json!({"taste": {"richness": 3}})), None);
}

#[test]
fn the_vector_is_per_mille_of_each_scale_and_a_zero_axis_weighs_nothing() {
    let s = validate(&json!({"taste": {"spicy": 5, "sweet": 0}, "texture": {"crispy": 3}, "aroma": {"smoky": 1}})).unwrap();
    let v = vector(&s);
    assert_eq!(v.get("t:spicy"), Some(&1000));
    assert_eq!(v.get("t:sweet"), None);
    assert_eq!(v.get("x:crispy"), Some(&1000));
    assert_eq!(v.get("a:smoky"), Some(&333));
    assert!(v.keys().all(|k| key_ok(k)));
    assert!(!key_ok("t:richness") && !key_ok("allergen:gluten") && !key_ok("spicy"));
    assert_eq!(all_keys().len(), 27);
}

#[test]
fn a_model_answer_is_held_to_the_vocabulary_and_never_errors() {
    let s = from_model("Sure! {\"taste\": {\"spicy\": 4, \"richness\": 3, \"sweet\": 9}, \"texture\": {\"crispy\": 2, \"slimy\": 1}, \"aroma\": {\"smoky\": \"3\"}} hope it helps");
    assert_eq!(s.taste.len(), 1);
    assert_eq!(s.taste["spicy"], 4);
    assert_eq!(s.texture.keys().collect::<Vec<_>>(), ["crispy"]);
    assert!(s.aroma.is_empty());
    assert!(from_model("no json here").is_empty());
    assert!(from_model("{not json}").is_empty());
}

#[test]
fn the_lexicon_drafts_from_the_name_and_the_words_and_says_which_word() {
    let (s, why) = lexicon::suggest("Philadelphia Roll", "", &["salmon".into(), "cream cheese".into(), "cucumber".into()]);
    assert_eq!(s.texture.get("creamy"), Some(&3));
    assert_eq!(s.aroma.get("marine"), Some(&2));
    assert_eq!(s.texture.get("crunchy"), Some(&2), "{why:?}");
    assert!(why.iter().any(|(k, w)| k == "x:creamy" && w.starts_with("starter")), "{why:?}");
    assert!(validate(&s.json()).is_ok(), "a draft is always a valid edit");
    // Four languages.
    for (name, key) in [("Rolle e djegës", "t:spicy"), ("Копчёный лосось", "a:smoky"), ("Гострий рамен", "t:spicy"), ("Tempura shrimp", "x:crispy")] {
        assert!(vector(&lexicon::suggest(name, "", &[]).0).contains_key(key), "{name} -> {key}");
    }
    // A word that only contains a needle by accident does not fire.
    let (none, _) = lexicon::suggest("Plate", "ready in 10 minutes", &[]);
    assert!(none.aroma.get("nutty").is_none(), "{none:?}");
}
