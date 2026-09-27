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
