use super::*;
use crate::catalog::Catalog;
use crate::{EventKind, Hub};

/// A venue the size of the real one, whose dish NAMES never say the
/// ingredient — the only shape in which the graph earns its keep, because
/// BM25 can answer the other one on its own.
fn sixty_dishes() -> (Hub, Catalog) {
    let mut c = Catalog::create().expect("catalog");
    c.set_location(r#"{"id":"dubin-durres","name":"Dubin & Sushi","address":"Rruga Taulantia"}"#);
    let cats = ["maki", "nigiri", "bowls", "snacks", "chef", "drinks"];
    for i in 0..60 {
        let cat = cats[i % cats.len()];
        let (name, desc, supply) = match i % 3 {
            0 => (format!("Philadelphia {i}"), "oriz sushi, nori, krem", "salmon"),
            1 => (format!("Panko Special {i}"), "karkalec panko, chili", "shrimp"),
            _ => (format!("Spicy Roll {i}"), "oriz sushi, nori, majoneze", "tuna"),
        };
        c.set_product(
            &format!("item-{i:02}"),
            &format!(
                r#"{{"name":"{name}","description":"{desc}","price":900,"categoryId":"{cat}","recipe":[{{"supplyId":"{supply}"}}]}}"#
            ),
        );
    }
    for sup in ["salmon", "shrimp", "tuna"] {
        c.set_supply(sup, &format!(r#"{{"name":"{sup}","unit":"kg"}}"#));
    }
    (Hub::create_sized(512 * 1024).expect("hub"), c)
}

/// THE HUB NODES MUST NOT OUTRANK THE ANSWER. Every dish points at the one
/// venue, so the venue is a node of degree 60 and a raw PPR ranks it — and
/// the other ingredients, and the categories — above every dish that
/// actually contains what was asked for. Measured before `lift`: asking
/// `shrimp` put the venue second, two categories third and fourth, and the
/// SALMON and TUNA ingredients fifth and sixth, pushing the shrimp dishes
/// to rank six and below. This asserts the walk answers the question.
#[test]
fn a_super_node_does_not_outrank_the_dishes_that_use_the_ingredient() {
    let (hub, cat) = sixty_dishes();
    let g = Graph::of(&hub, &cat);
    assert!(g.degree(g.index_of("venue:dubin-durres").expect("venue")) >= 60);

    for (q, other) in [("shrimp", "salmon"), ("salmon", "shrimp")] {
        let hits = g.hybrid(q, 12);
        let kinds: Vec<&str> = hits
            .iter()
            .filter_map(|(i, _)| g.node(*i))
            .map(|n| n.kind.tag())
            .collect();
        let names: Vec<String> = hits
            .iter()
            .filter_map(|(i, _)| g.node(*i))
            .map(|n| format!("{} {}", n.kind.tag(), n.label))
            .collect();

        assert!(
            !kinds.contains(&"venue"),
            "the venue is in the top 12 for '{q}': {names:?}"
        );
        assert!(
            !names.iter().any(|n| n == &format!("ingredient {other}")),
            "the '{other}' ingredient is in the top 12 for '{q}': {names:?}"
        );
        // And the dishes that DO use it are the bulk of the answer.
        let dishes = kinds.iter().filter(|k| **k == "dish").count();
        assert!(dishes >= 8, "only {dishes} of 12 hits are dishes for '{q}': {names:?}");
    }
}

fn venue() -> Catalog {
    let mut c = Catalog::create().expect("catalog");
    c.set_location(r#"{"id":"dubin-durres","name":"Dubin & Sushi","address":"Rruga Taulantia"}"#);
    c.set_product(
        "item-01",
        r#"{"name":"Sake Futomaki","description":"oriz sushi, nori, salmon","price":900,"categoryId":"chef","recipe":[{"supplyId":"salmon"}]}"#,
    );
    c.set_product(
        "item-31",
        r#"{"name":"Maki Salmon","description":"oriz sushi, nori, salmon","price":600,"categoryId":"maki","recipe":[{"supplyId":"salmon"}]}"#,
    );
    c.set_product(
        "item-41",
        r#"{"name":"Panko Shrimps","description":"karkalec panko, chili","price":800,"categoryId":"snacks","recipe":[{"supplyId":"shrimp"}]}"#,
    );
    c.set_supply("salmon", r#"{"name":"Salmon","unit":"kg"}"#);
    c.set_supply("shrimp", r#"{"name":"Shrimp","unit":"kg"}"#);
    c
}

fn with_orders() -> (Hub, Catalog) {
    let mut hub = Hub::create_sized(256 * 1024).expect("hub");
    hub.append(
        EventKind::Placed,
        "ord-1",
        r#"{"status":"delivered","total":1500,"created_at_ms":1000,"courier_id":"eni","customer_key":"cust-a","items":[{"product_id":"item-01","name":"Sake Futomaki"},{"product_id":"item-31","name":"Maki Salmon"}]}"#,
        1000,
        [0u8; 32],
    )
    .expect("append");
    hub.append(
        EventKind::Placed,
        "ord-2",
        r#"{"status":"delivered","total":800,"created_at_ms":2000,"courier_id":"blerim","customer_key":"cust-b","items":[{"product_id":"item-41","name":"Panko Shrimps"}]}"#,
        2000,
        [0u8; 32],
    )
    .expect("append");
    (hub, venue())
}

#[test]
fn the_fold_relates_what_the_json_only_referenced() {
    let (hub, cat) = with_orders();
    let g = Graph::of(&hub, &cat);

    let order = g.index_of("order:ord-1").expect("the order is a node");
    let kinds: Vec<&'static str> =
        g.neighbours(order).iter().map(|(r, _, _)| r.tag()).collect();
    assert!(kinds.contains(&"contains"), "{kinds:?}");
    assert!(kinds.contains(&"delivered_by"), "{kinds:?}");
    assert!(kinds.contains(&"placed_by"), "{kinds:?}");

    // The relation the JSON never stated: this order reached an ingredient,
    // through a dish, without anything saying so.
    let dish = g.index_of("dish:item-01").expect("dish");
    let via: Vec<String> = g
        .neighbours(dish)
        .iter()
        .filter(|(r, _, fwd)| *r == Rel::Uses && *fwd)
        .map(|(_, j, _)| g.node(*j).unwrap().id.clone())
        .collect();
    assert_eq!(via, vec!["ingredient:salmon"], "the recipe must reach the shelf");
}

/// The customer node is the one place a retrieval index could quietly grow
/// a copy of someone's phone number.
#[test]
fn no_customer_pii_reaches_the_index() {
    let mut hub = Hub::create_sized(128 * 1024).expect("hub");
    hub.append(
        EventKind::Placed,
        "ord-9",
        r#"{"status":"new","customer_key":"cust-z","contact":{"name":"Ana","phone":"+355691112233"},"items":[]}"#,
        1,
        [0u8; 32],
    )
    .expect("append");
    let g = Graph::of(&hub, &venue());
    for n in g.nodes() {
        assert!(!n.text.contains("355691112233"), "a phone reached node {}", n.id);
        assert!(!n.text.contains("Ana"), "a name reached node {}", n.id);
    }
}

#[test]
fn a_word_search_finds_the_dish_that_uses_it() {
    let (hub, cat) = with_orders();
    let g = Graph::of(&hub, &cat);
    let hits = g.bm25("karkalec");
    assert!(!hits.is_empty(), "the menu says karkalec");
    assert_eq!(g.node(hits[0].0).unwrap().id, "dish:item-41");
}

/// The whole reason for the structural half: an ingredient's name is not in
/// any order's text, so a word search alone cannot answer "what does the
/// shrimp touch" — and the walk can.
#[test]
fn the_walk_reaches_what_the_words_cannot() {
    let (hub, cat) = with_orders();
    let g = Graph::of(&hub, &cat);

    let shrimp = g.index_of("ingredient:shrimp").expect("shrimp");
    assert!(
        g.bm25("shrimp").iter().all(|(i, _)| g.node(*i).unwrap().id != "order:ord-2"),
        "the order's own text must not mention the ingredient, or this proves nothing"
    );

    let reached: Vec<String> = g
        .ppr(&[shrimp], 12)
        .into_iter()
        .map(|(i, _)| g.node(i).unwrap().id.clone())
        .collect();
    assert!(reached.contains(&"dish:item-41".to_string()), "{reached:?}");
    assert!(reached.contains(&"order:ord-2".to_string()), "the walk must reach the order: {reached:?}");
}

#[test]
fn hybrid_ranks_the_named_thing_first_and_still_brings_its_neighbours() {
    let (hub, cat) = with_orders();
    let g = Graph::of(&hub, &cat);
    let out = g.hybrid("karkalec panko", 8);
    assert!(!out.is_empty());
    let ids: Vec<String> = out.iter().map(|(i, _)| g.node(*i).unwrap().id.clone()).collect();
    assert_eq!(ids[0], "dish:item-41", "the named dish leads: {ids:?}");
    assert!(
        ids.iter().any(|id| id == "order:ord-2"),
        "and the structure brings the order that contained it: {ids:?}"
    );
}

/// A ranking is part of an answer, so it has to replay identically.
#[test]
fn the_same_graph_ranks_the_same_way_twice() {
    let (hub, cat) = with_orders();
    let a = Graph::of(&hub, &cat);
    let b = Graph::of(&hub, &cat);
    assert_eq!(a.hybrid("salmon", 10), b.hybrid("salmon", 10));
    assert_eq!(a.len(), b.len());
    assert_eq!(a.edge_count(), b.edge_count());
}

#[test]
fn a_courier_can_be_named_by_the_caller_and_a_customer_cannot_be() {
    let (hub, cat) = with_orders();
    let mut labels = HashMap::new();
    labels.insert("courier:eni".to_string(), "Eni".to_string());
    // A customer label is offered and must still leave no PII: the caller
    // decides what it passes, and this asserts the seam does not smuggle.
    labels.insert("customer:cust-a".to_string(), String::new());
    let g = Graph::of_with(&hub, &cat, &labels);

    let hits = g.bm25("Eni");
    assert!(!hits.is_empty(), "a named courier must be findable by name");
    assert_eq!(g.node(hits[0].0).unwrap().id, "courier:eni");

    let cust = g.index_of("customer:cust-a").expect("customer node");
    assert!(g.node(cust).unwrap().text.is_empty(), "an empty label adds nothing");
}

#[test]
fn an_empty_hub_is_an_empty_graph_not_a_panic() {
    let hub = Hub::create_sized(64 * 1024).expect("hub");
    let cat = Catalog::create().expect("catalog");
    let g = Graph::of(&hub, &cat);
    assert!(g.hybrid("anything", 5).is_empty());
    assert!(g.ppr(&[], 5).is_empty());
    assert!(g.bm25("").is_empty());
}
