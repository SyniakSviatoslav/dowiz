use super::*;

fn rows() -> (Vec<(String, String)>, Vec<(String, String)>) {
    let products = vec![
        ("marg".to_string(), r#"{"name":"Pica Margarita","available":true}"#.to_string()),
        ("tira".to_string(), r#"{"name":"Tiramisu","available":false}"#.to_string()),
        ("nameless".to_string(), r#"{"price":100}"#.to_string()),
        ("broken".to_string(), "not json".to_string()),
    ];
    let i18n = vec![
        ("uk/product/marg/name".to_string(), "Піца Маргарита".to_string()),
        ("en/product/marg/name".to_string(), "Pizza Margherita".to_string()),
        ("en/product/marg/description".to_string(), "Tomato".to_string()),
        ("uk/product/tira/name".to_string(), "  ".to_string()),
        ("uk/product/other/name".to_string(), "Інше".to_string()),
        ("bad-key".to_string(), "x".to_string()),
    ];
    (products, i18n)
}

#[test]
fn the_speakers_name_comes_first_then_the_venues_then_the_rest() {
    let (p, t) = rows();
    let uk = dishes(&p, &t, "uk-UA");
    assert_eq!(uk[0].names, vec!["Піца Маргарита", "Pica Margarita", "Pizza Margherita"]);
    let sq = dishes(&p, &t, "sq");
    assert_eq!(sq[0].names[0], "Pica Margarita");
    assert_eq!(sq[0].names.len(), 3);
}

#[test]
fn availability_is_carried_and_nameless_or_broken_rows_are_left_out() {
    let (p, t) = rows();
    let d = dishes(&p, &t, "en");
    assert_eq!(d.len(), 2);
    assert!(d[0].available);
    // A blank translation is not a name; the venue's own stands.
    assert_eq!(d[1].names, vec!["Tiramisu"]);
    assert!(!d[1].available);
}
