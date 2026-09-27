use super::menu_cache_control;

/// The console's read-back of its own save may not be kept by the browser:
/// the owner saw the old price after "Saved" (QA walk Q1).
#[test]
fn a_fresh_menu_read_is_never_stored_by_the_browser() {
    assert_eq!(menu_cache_control(true), "no-store");
}

/// The positive twin: a customer's read keeps the thirty-second window the
/// edge cache is built around.
#[test]
fn a_customer_menu_read_keeps_the_thirty_second_window() {
    let cc = menu_cache_control(false);
    assert!(cc.starts_with("public"), "{cc}");
    assert!(cc.contains("max-age=30"), "{cc}");
}

/// A dish's Russian name wins; a dish with only an English one is read in
/// English, whichever order the table walks the keys; one with neither keeps
/// the venue's own (lane W-RU, `ru -> en -> venue`).
#[test]
fn a_missing_translation_is_read_in_english_before_the_venues_words() {
    use super::i18n_keep;
    let mut m = std::collections::HashMap::new();
    for (k, v) in [
        ("en/product/p1/name", "Salmon roll"),
        ("ru/product/p1/name", "Ролл с лососем"),
        ("en/product/p2/name", "Miso soup"),
        ("ru/product/p3/name", "Чай"),
        ("en/product/p3/name", "Tea"),
        ("uk/product/p4/name", "Чай"),
        ("ru/product/p5/colour", "x"),
        ("ru/product/p6", "x"),
    ] {
        i18n_keep(&mut m, k, v.to_string(), "ru", Some("en"));
    }
    let got = |id: &str| m.get(&(id.to_string(), "name".to_string())).map(|(_, v)| v.as_str());
    assert_eq!(got("p1"), Some("Ролл с лососем"));
    assert_eq!(got("p2"), Some("Miso soup"));
    assert_eq!(got("p3"), Some("Чай"));
    assert_eq!(got("p4"), None);
    assert_eq!(m.len(), 3);
    // Without a second language only the asked one is read.
    let mut only = std::collections::HashMap::new();
    i18n_keep(&mut only, "en/product/p2/name", "Miso soup".to_string(), "ru", None);
    assert!(only.is_empty());
}
