use super::*;

#[test]
fn a_cyrillic_id_reaches_the_handler_as_typed() {
    // dubin-sushi 2026-10-17: GET /api/owner/supplies/%D0%BD%D0%BE%D1%80%D0%B8/uses answered
    // 404 "unknown supply" for "нори" -- 45 of its 63 supplies have ids like that.
    assert_eq!(decoded("%D0%BD%D0%BE%D1%80%D0%B8"), "нори");
    assert_eq!(decoded("%D0%BA%D1%80%D0%B5%D0%BC-%D1%81%D1%8B%D1%80"), "крем-сыр");
}

#[test]
fn an_ascii_id_and_a_broken_escape_are_kept_as_they_came() {
    assert_eq!(decoded("salmon-01"), "salmon-01");
    assert_eq!(decoded("100%"), "100%");
    assert_eq!(decoded("%zz"), "%zz");
    // Bytes that are not UTF-8 are not guessed at.
    assert_eq!(decoded("%FF%FE"), "%FF%FE");
    assert_eq!(decoded("a%20b"), "a b");
}

#[test]
fn every_path_parameter_the_router_names_is_decoded() {
    // A route added with a new `:name` would otherwise reach its handler still encoded.
    let src = include_str!("../../lib.rs");
    let mut missing = Vec::new();
    for route in src.split('"').filter(|s| s.starts_with('/')) {
        for seg in route.split('/').filter_map(|s| s.strip_prefix(':')) {
            if !NAMES.contains(&seg) && !missing.contains(&seg) {
                missing.push(seg);
            }
        }
    }
    assert!(missing.is_empty(), "path parameters not in edge::params::NAMES: {missing:?}");
}
