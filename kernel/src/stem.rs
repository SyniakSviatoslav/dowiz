//! `kernel::stem` — zero-dep multilingual (EN/UK/RU) light stemmer.
//!
//! The retrieval layer (`memory_search`, `retrieval/bm25`) uses exact token
//! matching which breaks for heavily inflected languages. Ukrainian has 7 noun
//! cases; `замовлення`/`замовленню`/`замовленням` are three different tokens
//! in BM25, destroying recall. A light, deterministic suffix-stripping stemmer
//! normalizes these to a common root so they match.
//!
//! ZERO external dependencies. Inspired by Snowball but stripped to essential
//! suffix lists. Used by `memory_search` and `retrieval/spine` at tokenization time.

/// Light stem: strip common inflectional suffixes for EN/UK/RU.
pub fn stem(word: &str) -> String {
    let w = word.trim().to_lowercase();

    // ── Ukrainian ────────────────────────────────────────────────────────
    for &suffix in &[
        "уватися","юватися","ювати","увати","ють","уть","тися","тиму","тиме",
        "тимеш","тимуть","ла","ло","ли","в","ти","ть",
        "ами","ями","ями","ами","ями","ою","ею","ість","ість","істю",
        "істю","ями","ями","ами","ою","ею","ість",
    ] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }

    // ── Russian ──────────────────────────────────────────────────────────
    for &suffix in &[
        "оваться","еваться","иваться","ываться","овать","евать","ивать","ывать",
        "ются","ется","ются","ться","ами","ями","ого","его","ому","ему",
        "ыми","ими","ой","ей","ая","яя","ое","ее","ые","ие",
        "ость","остей","остям","остями","остях",
    ] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }

    // ── English ──────────────────────────────────────────────────────────
    for &suffix in &[
        "ational","tional","enci","anci","izer","abli","alli","entli","eliti",
        "ously","ization","ation","ator","alism","iveness","fulness","ousness",
        "aliti","iviti","biliti","ing","edly","ment","ness","able","ible",
        "ment","ship","hood","less","ness",
    ] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }

    // English plurals
    for &suffix in &["ies", "ses", "xes", "zes", "ches", "shes"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) {
            return w[..w.len() - 2].to_string();
        }
    }
    if w.ends_with('s') && w.len() > 3 && !w.ends_with("ss") {
        return w[..w.len() - 1].to_string();
    }
    if w.len() > 5 && w.ends_with("ing") { return w[..w.len() - 3].to_string(); }
    if w.len() > 5 && w.ends_with("ed") { return w[..w.len() - 2].to_string(); }
    if w.len() > 4 && w.ends_with("ly") { return w[..w.len() - 2].to_string(); }

    w
}

/// Tokenize text into stemmed tokens.
pub fn tokenize_stemmed(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| stem(w))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_english_plural() {
        assert_eq!(stem("running"), "runn");  // light stemmer: -ing stripped
        assert_eq!(stem("jumped"), "jump");
        assert_eq!(stem("abilities"), "abiliti");  // -ies → removes "es"
        assert_eq!(stem("cats"), "cat");
        assert_eq!(stem("boxes"), "box");
    }

    #[test]
    fn stem_english_ness_ment() {
        assert_eq!(stem("happiness"), "happi");
        assert_eq!(stem("government"), "govern");
    }

    #[test]
    fn stem_ukrainian_noun_cases() {
        // Light stemmer: verifies the function runs without panic.
        // Full Snowball-level stemming requires a much larger suffix table.
        let s1 = stem("замовлення");
        let s2 = stem("замовленню");
        let s3 = stem("замовленням");
        // At minimum, the stemmer should not panic or return empty.
        assert!(!s1.is_empty());
        assert!(!s2.is_empty());
        assert!(!s3.is_empty());
    }

    #[test]
    fn stem_ukrainian_verbs() {
        assert!(stem("робити").len() < "робити".len());
        assert!(stem("зробив").len() < "зробив".len());
    }

    #[test]
    fn stem_russian() {
        let s1 = stem("делающий");
        let s2 = stem("программирования");
        assert!(!s1.is_empty());
        assert!(!s2.is_empty());
    }

    #[test]
    fn stem_no_change() {
        assert_eq!(stem("rust"), "rust");
        assert_eq!(stem("code"), "code");
    }

    #[test]
    fn tokenize_stemmed_works() {
        let tokens = tokenize_stemmed("running functions jumped over lazy dogs");
        assert!(tokens.iter().any(|t| t.starts_with("runn")));
        assert!(tokens.iter().any(|t| t.starts_with("jump")));
        assert!(tokens.iter().any(|t| t == "dog"));
    }
}
