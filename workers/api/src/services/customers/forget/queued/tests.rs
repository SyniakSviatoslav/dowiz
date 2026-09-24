//! G8: what is waiting to be sent about a forgotten person is dropped; what
//! is about anybody else, and what the law keeps, stays.

use super::*;
use crate::services::campaigns::send::entry_id;

fn entry(id: &str, kind: &str, to: &str) -> Entry {
    Entry::new(id.to_string(), kind, to.to_string(), "Arben +355691111111".to_string(), 1)
}

fn outbox() -> Table {
    let mut t = Table::create(1 << 18).unwrap();
    for e in [
        entry("o1/telegram", "telegram", "chat"),
        entry("o1/telegram/bar", "telegram", "chat-bar"),
        entry("o1/whatsapp", "whatsapp", "355690000000"),
        entry(&entry_id("c1", "k_national"), OUTBOX_KIND, "355691111111"),
        entry(&entry_id("c1", "k_stranger"), OUTBOX_KIND, "355692222222"),
        entry("o2/telegram", "telegram", "chat"),
        entry("0f3a-uuid-of-a-fiscal-doc", "fiscal", "o1"),
    ] {
        t.put(KIND, &e.id, &serde_json::to_string(&e).unwrap(), &[], &[]).unwrap();
    }
    t
}

fn ids(t: &Table) -> Vec<String> {
    t.all(KIND).into_iter().map(|(id, _)| id).collect()
}

/// Forget via ONE spelling: the linked spelling's queued campaign entry goes
/// too (the circle is the person), and so does every ticket of their order.
#[test]
fn the_persons_tickets_and_their_linked_spellings_campaign_entry_go() {
    let mut t = outbox();
    let orders = BTreeSet::from(["o1".to_string()]);
    let keys = BTreeSet::from(["k_e164".to_string(), "k_national".to_string()]);
    assert_eq!(drop_queued(&mut t, &orders, &keys), 4);
    let left = ids(&t);
    assert!(left.contains(&"o2/telegram".to_string()), "a stranger's ticket went: {left:?}");
    assert!(left.contains(&entry_id("c1", "k_stranger")), "a stranger's campaign entry went: {left:?}");
    assert!(left.contains(&"0f3a-uuid-of-a-fiscal-doc".to_string()), "a fiscal document went: {left:?}");
    assert_eq!(left.len(), 3);
    assert_eq!(drop_queued(&mut t, &orders, &keys), 0, "idempotent");
}

/// The twin: with only the OTHER spelling in scope, the linked spelling's
/// entry stays -- which is exactly why the circle, not the key, is passed.
#[test]
fn one_key_alone_misses_the_linked_spelling() {
    let mut t = outbox();
    let keys = BTreeSet::from(["k_e164".to_string()]);
    assert_eq!(drop_queued(&mut t, &BTreeSet::new(), &keys), 0);
    assert!(ids(&t).contains(&entry_id("c1", "k_national")));
}
