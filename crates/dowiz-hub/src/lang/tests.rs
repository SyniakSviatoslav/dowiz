use super::*;

#[test]
fn four_languages_albanian_first_ru_included() {
    assert_eq!(LANGS[0], "sq");
    assert!(LANGS.contains(&"ru"));
    for (i, l) in LANGS.iter().enumerate() {
        assert!(!LANGS[i + 1..].contains(l), "{l} twice");
    }
}

#[test]
fn is_lang_takes_codes_only() {
    assert!(is_lang("ru"));
    assert!(!is_lang("ru-RU"));
    assert!(!is_lang("de"));
}

#[test]
fn norm_reads_the_tag_head() {
    assert_eq!(norm("ru-RU"), Some("ru"));
    assert_eq!(norm(" UK "), Some("uk"));
    assert_eq!(norm("de-DE"), None);
    assert_eq!(norm("r"), None);
    assert_eq!(norm(""), None);
}

#[test]
fn or_falls_back_only_when_not_ours() {
    assert_eq!(or("ru", "sq"), "ru");
    assert_eq!(or("xx", "sq"), "sq");
}

#[test]
fn accept_language_honours_order_and_q() {
    assert_eq!(from_accept("ru-RU,ru;q=0.9,en-US;q=0.8"), Some("ru"));
    assert_eq!(from_accept("de-DE, en;q=0.5, uk;q=0.7"), Some("uk"));
    assert_eq!(from_accept("en;q=0.5, ru;q=0.5"), Some("en"));
    assert_eq!(from_accept("ru;q=0, sq"), Some("sq"));
    assert_eq!(from_accept("ru;q=junk"), None);
    assert_eq!(from_accept("ru;q=1.0"), Some("ru"));
    assert_eq!(from_accept("de, fr"), None);
    assert_eq!(from_accept(""), None);
}

#[test]
fn q_permille_is_integer_and_capped() {
    assert_eq!(q_permille("0.8"), 800);
    assert_eq!(q_permille("0.123"), 123);
    assert_eq!(q_permille("1"), 1000);
    assert_eq!(q_permille("2"), 1000);
    assert_eq!(q_permille("0.x"), 0);
    assert_eq!(q_permille("x"), 0);
}

#[test]
fn english_names_for_the_model_prompt() {
    assert_eq!(english_name("ru-RU"), "Russian");
    assert_eq!(english_name("uk"), "Ukrainian");
    assert_eq!(english_name("sq"), "Albanian");
    assert_eq!(english_name("en"), "English");
    assert_eq!(english_name("de"), "English");
}

#[test]
fn slavic_plural_three_forms() {
    let f = ["заказ", "заказа", "заказов"];
    let got: Vec<&str> = [1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 111].iter().map(|n| slavic_plural(*n, f)).collect();
    assert_eq!(got, ["заказ", "заказа", "заказа", "заказов", "заказов", "заказов", "заказов", "заказ", "заказа", "заказов", "заказов"]);
}

#[test]
fn media_languages_are_the_ui_ones_but_russian() {
    assert!(is_media_lang("uk"));
    assert!(is_media_lang("sq"));
    assert!(!is_media_lang("ru"));
    assert!(!is_media_lang("de"));
}

#[test]
fn choose_takes_the_query_then_the_browser_then_the_fallback() {
    assert_eq!(choose(Some("ru"), Some("uk"), "sq"), "ru");
    assert_eq!(choose(Some("de"), Some("ru-RU,en;q=0.5"), "sq"), "ru");
    assert_eq!(choose(None, Some("ru-RU,en;q=0.5"), "sq"), "ru");
    assert_eq!(choose(Some(""), None, "uk"), "uk");
    assert_eq!(choose(None, Some("de"), "sq"), "sq");
}

#[test]
fn the_json_literal_is_the_set() {
    let want = format!("[{}]", LANGS.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join(","));
    assert_eq!(crate::langs_json!(), want);
}

/// Dish content reads `ru -> en -> venue` (research 2026-09-26 B2.4): English
/// is the second step unless it is the one asked for or the venue's own.
#[test]
fn content_falls_back_to_english_before_the_venue() {
    assert_eq!(content_fallback("ru", "sq"), Some("en"));
    assert_eq!(content_fallback("uk", "sq"), Some("en"));
    assert_eq!(content_fallback("en", "sq"), None);
    assert_eq!(content_fallback("ru", "en"), None);
}
