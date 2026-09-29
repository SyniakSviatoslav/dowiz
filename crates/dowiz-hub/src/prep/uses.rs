//! WHERE AN ITEM IS USED, transitively: the ПФ whose trees reach it and the
//! dishes whose trees reach it -- what the owner sees before deleting one
//! (SPEC §e), and the chain above a card (SPEC §b's depth cap).

use super::{card_of, Card, DEPTH_MAX};

/// Where an item is used, TRANSITIVELY: every ПФ whose tree reaches it and
/// every dish whose tree reaches it. `(id, name)` each, sorted by id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Uses {
    pub preps: Vec<(String, String)>,
    pub dishes: Vec<(String, String)>,
}

pub fn uses_of(item: &str, supplies: &[(String, String)], products: &[(String, String)]) -> Uses {
    let name_of = |j: &str, id: &str| crate::minijson::str_field(j, "name").unwrap_or_else(|| id.to_string());
    let cards: Vec<(String, Card)> = supplies.iter().filter_map(|(id, j)| Some((id.clone(), card_of(j)?))).collect();
    let mut memo: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    fn reaches(id: &str, target: &str, cards: &[(String, Card)], memo: &mut std::collections::HashMap<String, bool>, depth: usize) -> bool {
        if let Some(b) = memo.get(id) {
            return *b;
        }
        if depth > DEPTH_MAX + 1 {
            return false;
        }
        let hit = cards.iter().find(|(c, _)| c == id).is_some_and(|(_, card)| {
            card.lines.iter().any(|l| l.item == target || reaches(&l.item, target, cards, memo, depth + 1))
        });
        memo.insert(id.to_string(), hit);
        hit
    }
    let mut out = Uses::default();
    for (id, j) in supplies {
        if id != item && reaches(id, item, &cards, &mut memo, 0) {
            out.preps.push((id.clone(), name_of(j, id)));
        }
    }
    for (id, j) in products {
        let hit = crate::stock::bom_of(j).iter().any(|l| l.supply == item || reaches(&l.supply, item, &cards, &mut memo, 0));
        if hit {
            out.dishes.push((id.clone(), name_of(j, id)));
        }
    }
    out.preps.sort();
    out.dishes.sort();
    out
}


/// The longest chain of cards that name `id`, transitively, in cards.
pub fn height_above(id: &str, supplies: &[(String, String)]) -> usize {
    let cards: Vec<(String, Card)> = supplies.iter().filter_map(|(i, j)| Some((i.clone(), card_of(j)?))).collect();
    fn climb(id: &str, cards: &[(String, Card)], depth: usize) -> usize {
        if depth > DEPTH_MAX {
            return depth;
        }
        cards.iter().filter(|(_, c)| c.lines.iter().any(|l| l.item == id)).map(|(u, _)| 1 + climb(u, cards, depth + 1)).max().unwrap_or(0)
    }
    climb(id, &cards, 0)
}
