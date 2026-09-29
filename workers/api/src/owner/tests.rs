//! The owner routes' pure decisions.

use super::*;

/// A translation is taken in every storefront language, Russian included,
/// and refused in one no surface reads (lane W-RU, research 2026-09-26 B2).
#[test]
fn a_translation_is_taken_in_every_storefront_language_and_no_other() {
    for l in dowiz_hub::lang::LANGS {
        assert_eq!(i18n_check("product", l, "name", "x").as_deref(), Ok(l), "{l}");
    }
    assert_eq!(i18n_check("category", " RU ", "name", "Роллы").as_deref(), Ok("ru"));
    let e = i18n_check("product", "de", "name", "x").unwrap_err();
    assert!(e.contains("\"de\"") && e.contains("ru"), "{e}");
    assert!(i18n_check("product", "rus", "name", "x").unwrap_err().contains("not a locale"));
    assert!(i18n_check("order", "ru", "name", "x").unwrap_err().contains("not translatable"));
}

/// W-CRUD: the dish sheet's body refuses a field it has no name for, and
/// takes the four it gained (name, description, category_id, move). RED on
/// 2026-09-29 through `req.json()`: an extra field passed and was dropped.
#[test]
fn the_dish_body_refuses_an_unknown_field_and_takes_the_new_ones() {
    let bad = serde_json::from_str::<ProductEdit>(r#"{"location_id":"v","price":100,"zzz":1}"#).err().expect("an unknown field is refused");
    assert!(bad.to_string().contains("zzz"), "{bad}");
    let good: ProductEdit = serde_json::from_str(r#"{"location_id":"v","name":"Sake","description":"two","category_id":"rolls","move":"up","price":100}"#).unwrap();
    assert_eq!((good.name.as_deref(), good.description.as_deref(), good.category_id.as_deref(), good.nudge.as_deref(), good.price),
        (Some("Sake"), Some("two"), Some("rolls"), Some("up"), Some(100)));
    assert!(serde_json::from_str::<ProductEdit>(r#"{"price":1}"#).is_err(), "the venue is not optional");
}
