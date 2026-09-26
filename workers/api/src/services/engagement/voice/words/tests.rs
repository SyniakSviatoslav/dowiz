use super::*;

#[test]
fn apostrophes_vanish_and_punctuation_splits() {
    assert_eq!(norm("П'ять, будь ласка!"), "пять будь ласка");
    assert_eq!(norm("п’ять"), "пять");
    assert_eq!(norm("  Table   5 "), "table 5");
    assert_eq!(norm("..."), "");
}

#[test]
fn has_is_whole_words_only() {
    let t = norm("one margherita");
    let ws = words(&t);
    assert!(has(&ws, &["one"]));
    // "on" is not inside "one".
    assert!(!has(&ws, &["on"]));
}

#[test]
fn numbers_in_digits_words_and_marks() {
    for (w, n) in [("5", 5), ("x2", 2), ("2x", 2), ("х3", 3), ("four", 4), ("дві", 2), ("пять", 5), ("katër", 4), ("dy", 2)] {
        assert_eq!(number(w), Some(n), "{w}");
    }
    // Not numbers: a word, a bare mark, zero, and more than anybody says aloud.
    for w in ["pizza", "x", "0", "100", "x0"] {
        assert_eq!(number(w), None, "{w}");
    }
    assert_eq!(number("99"), Some(MAX_SAID));
}
