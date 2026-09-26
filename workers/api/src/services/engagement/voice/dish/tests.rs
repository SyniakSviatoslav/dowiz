use super::*;

fn d(id: &str, names: &[&str]) -> Dish {
    Dish { id: id.into(), names: names.iter().map(|s| s.to_string()).collect(), available: true }
}

fn menu() -> Vec<Dish> {
    vec![
        d("marg", &["Pizza Margherita", "Піца Маргарита", "Pica Margarita"]),
        d("pep", &["Pizza Pepperoni", "Піца Пепероні"]),
        d("cola", &["Cola"]),
        d("colaz", &["Cola Zero"]),
        d("tira", &["Tiramisu", "Тірамісу"]),
    ]
}

#[test]
fn a_dish_is_found_by_any_of_its_names_and_their_endings() {
    let m = menu();
    for said in ["margherita", "pizza margherita", "маргариту", "маргарити", "margaritën", "тірамісу", "Tiramisu"] {
        assert!(find(&m, said).is_ok(), "{said}");
    }
    assert_eq!(find(&m, "маргариту").unwrap().id, "marg");
    assert_eq!(find(&m, "pepperoni").unwrap().id, "pep");
}

/// The exact name wins: "cola" is Cola even with Cola Zero on the menu.
#[test]
fn an_exact_name_beats_a_partial_one() {
    let m = menu();
    assert_eq!(find(&m, "cola").unwrap().id, "cola");
    assert_eq!(find(&m, "cola zero").unwrap().id, "colaz");
}

/// Two dishes fit: refused, and the candidates are named.
#[test]
fn a_name_that_fits_two_dishes_is_refused() {
    let m = menu();
    assert_eq!(find(&m, "pizza"), Err(Miss::Many(vec!["Pizza Margherita".into(), "Pizza Pepperoni".into()])));
    assert_eq!(find(&m, "піца"), Err(Miss::Many(vec!["Pizza Margherita".into(), "Pizza Pepperoni".into()])));
}

#[test]
fn a_name_that_fits_nothing_is_refused() {
    let m = menu();
    assert_eq!(find(&m, "sushi"), Err(Miss::None));
    assert_eq!(find(&m, ""), Err(Miss::None));
    // A short word is not a stem: "tir" does not reach "tiramisu".
    assert_eq!(find(&m, "tir"), Err(Miss::None));
}

#[test]
fn ambiguity_reads_back_at_most_three() {
    let many: Vec<Dish> = (0..5).map(|i| d(&format!("p{i}"), &[&format!("Pizza {i}x")])).collect();
    match find(&many, "pizza") {
        Err(Miss::Many(v)) => assert_eq!(v.len(), SHOWN),
        other => panic!("expected Many, got {other:?}"),
    }
}

#[test]
fn stems_match_endings_but_not_short_words() {
    assert!(same_word("маргариту", "маргарита"));
    assert!(same_word("margaritën", "margarita"));
    assert!(same_word("cola", "cola"));
    assert!(!same_word("tea", "team"));
    assert!(!same_word("pizza", "pasta"));
}
