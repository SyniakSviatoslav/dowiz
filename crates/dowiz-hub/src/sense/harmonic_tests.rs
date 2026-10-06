//! S7b pinned: the recipe graph drafts a dish's taste from the declared dishes it shares supplies
//! with, in integers, deterministically, within the sweep bound -- and says so instead of inventing
//! anything when there are no recipes, no recipe for the dish, or nobody declared to learn from.

use super::*;
use std::collections::BTreeMap;

fn v(xs: &[(&str, i64)]) -> Option<BTreeMap<String, i64>> {
    Some(xs.iter().map(|(k, w)| (k.to_string(), *w)).collect())
}
fn bom(xs: &[(&str, i64)]) -> Vec<(String, i64)> {
    xs.iter().map(|(s, q)| (s.to_string(), *q)).collect()
}

/// Two declared dishes: one smoky (eel + rice), one sweet (mango + rice). A new dish of eel and
/// rice only, undeclared.
fn venue() -> Vec<DishIn> {
    vec![
        ("unagi-don".into(), bom(&[("eel", 100), ("rice", 100)]), v(&[("a:smoky", 1000), ("t:salty", 600)])),
        ("mango-roll".into(), bom(&[("mango", 100), ("rice", 100)]), v(&[("t:sweet", 1000), ("x:soft", 666)])),
        ("eel-nigiri".into(), bom(&[("eel", 60), ("rice", 40)]), None),
        ("miso".into(), Vec::new(), v(&[("t:umami", 800)])),
    ]
}

#[test]
fn no_recipes_no_recipe_or_no_declared_neighbour_is_said_and_nothing_is_invented() {
    let none: Vec<DishIn> = venue().into_iter().map(|(id, _, s)| (id, Vec::new(), s)).collect();
    assert_eq!(extend(&none, "eel-nigiri", PASSES), Draft::NoRecipes, "0 recipes, the live venues today");
    assert_eq!(extend(&venue(), "miso", PASSES), Draft::NoRecipe);
    assert_eq!(extend(&venue(), "ghost", PASSES), Draft::NoRecipe);
    let lonely = vec![("a".into(), bom(&[("x", 1)]), None), ("b".into(), bom(&[("y", 1)]), v(&[("t:sour", 1000)]))];
    assert_eq!(extend(&lonely, "a", PASSES), Draft::NoNeighbours, "shares no supply with a declared dish");
    for d in [Draft::NoRecipes, Draft::NoRecipe, Draft::NoNeighbours] {
        assert_ne!(d.word(), "drafted");
    }
}

#[test]
fn a_dish_leans_toward_the_declared_dishes_it_shares_more_of() {
    let Draft::From { vector, neighbours, passes } = extend(&venue(), "eel-nigiri", PASSES) else { panic!("drafted") };
    assert_eq!(neighbours, 2, "both rice dishes are boundary");
    assert!(passes <= PASSES);
    // Mostly eel (only unagi-don), some rice (both): smoky over sweet.
    assert!(vector["a:smoky"] > vector.get("t:sweet").copied().unwrap_or(0), "{vector:?}");
    assert!(vector["a:smoky"] < 1000 && vector["a:smoky"] > 500, "between its neighbours: {vector:?}");
    assert!(!vector.contains_key("t:umami"), "miso shares nothing and lends nothing: {vector:?}");
    // In the stored shape: smoky as a tag, never a 0 claim.
    let s = to_sense(&vector);
    assert!(s.aroma.get("smoky").copied().unwrap_or(0) >= 2, "{s:?}");
    assert!(s.taste.values().all(|l| *l >= 1), "no taste axis at 0: {s:?}");
    assert!(crate::sense::validate(&s.json()).is_ok(), "a valid edit for the owner to review");
}

#[test]
fn the_draft_is_deterministic_and_the_fixed_point_does_not_move_with_more_sweeps() {
    let a = extend(&venue(), "eel-nigiri", PASSES);
    assert_eq!(a, extend(&venue(), "eel-nigiri", PASSES), "same input, same bytes");
    let Draft::From { vector: long, .. } = extend(&venue(), "eel-nigiri", 400) else { panic!() };
    let Draft::From { vector: short, passes, .. } = a else { panic!() };
    assert!(passes < PASSES, "converged inside the bound ({passes})");
    for (k, x) in &long {
        assert!((x - short.get(k).copied().unwrap_or(0)).abs() <= 1, "{k}: {x} vs {short:?}");
    }
    // The dish's own declaration is never its own evidence.
    let mut declared = venue();
    declared[2].2 = v(&[("t:bitter", 1000)]);
    let Draft::From { vector, .. } = extend(&declared, "eel-nigiri", PASSES) else { panic!() };
    assert!(!vector.contains_key("t:bitter"), "{vector:?}");
}

/// The harmonic value at a free node equals the share-weighted mean of its neighbours (the
/// discrete Laplace equation), checked directly on the converged answer.
#[test]
fn the_answer_satisfies_the_laplace_equation_at_the_dish() {
    // One dish, two supplies each used by one declared dish: the value at the dish is the share mean.
    let d: Vec<DishIn> = vec![
        ("x".into(), bom(&[("s1", 3), ("s2", 1)]), None),
        ("p".into(), bom(&[("s1", 1)]), v(&[("t:sour", 1000)])),
        ("q".into(), bom(&[("s2", 1)]), v(&[("t:sour", 200)])),
    ];
    let Draft::From { vector, .. } = extend(&d, "x", PASSES) else { panic!() };
    // s1 = (x*3/4 + p) / (3/4 + 1), s2 = (x/4 + q) / (1/4 + 1), x = 3/4 s1 + 1/4 s2.
    // Solved: x (1 - 9/28 - 1/20) = 3000/7 + 40, x = 745.45.
    assert!((vector["t:sour"] - 745).abs() <= 1, "{vector:?}");
}
