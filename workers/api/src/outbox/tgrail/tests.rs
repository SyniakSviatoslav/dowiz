//! The drain's Telegram bookkeeping, against a real outbox table.
use super::*;
use crate::notify::route::HEALTH_KIND;
use crate::notify::route::{groups::Pii, Mode};
use dowiz_hub::table::Table;
use dowiz_hub::tz::{Dst, Zone};
use serde_json::json;

const UTC: Zone = Zone { standard_minutes: 0, dst: Dst::None };
const NOW: i64 = 1_768_435_200_000 + 12 * 3_600_000;

fn tg(id: &str, to: &str) -> Entry {
    Entry::new(id.into(), "telegram", to.into(), "x".into(), NOW)
}

fn table(es: &[Entry]) -> Table {
    let mut t = Table::create(crate::outbox::OUTBOX_BYTES).unwrap();
    for e in es {
        t.put(KIND, &e.id, &serde_json::to_string(e).unwrap(), &[], &[]).unwrap();
    }
    t
}

fn ids(t: &Table) -> Vec<String> {
    t.all(KIND).into_iter().map(|(k, _)| k).collect()
}

fn group(id: &str, chat: &str, evs: &[(&str, Mode)]) -> Group {
    let mut g = Group::fresh(id.into(), chat.into(), None, format!("T-{id}"), "group".into(), "en");
    g.pii = Pii::None;
    g.subs = evs.iter().map(|(e, m)| (e.to_string(), *m)).collect();
    g
}

#[test]
fn a_gone_chat_takes_every_entry_to_it_with_it_and_nothing_else() {
    let mut t = table(&[tg("a", "-1:5"), tg("b", "-1"), tg("c", "-2"), Entry::new("w".into(), "whatsapp", "-1".into(), "x".into(), NOW)]);
    let mut ops = Ops::default();
    ops.gone.push(("-1".into(), "kicked".into()));
    ops.put(tg("fresh", "-1:9"));
    let mut dropped = ops.apply(&mut t).unwrap();
    dropped.sort();
    assert_eq!(dropped, vec!["a", "b", "fresh"]);
    assert_eq!(ids(&t), vec!["c", "w"], "another chat, and another channel, are untouched");
}

#[test]
fn a_moved_chat_re_aims_every_entry_and_keeps_its_topic() {
    let mut t = table(&[tg("a", "-55:3"), tg("b", "-2")]);
    let mut ops = Ops::default();
    ops.moved.push(("-55".into(), "-100999".into()));
    ops.put(tg("fresh", "-55"));
    assert!(ops.apply(&mut t).unwrap().is_empty());
    let to = |id: &str| serde_json::from_str::<Entry>(&t.get(KIND, id).unwrap()).unwrap().to;
    assert_eq!((to("a"), to("b"), to("fresh")), ("-100999:3".into(), "-2".into(), "-100999".into()));
}

#[test]
fn puts_removes_and_records_land_in_one_apply() {
    let mut t = table(&[tg("a", "-1"), tg("b", "-1")]);
    let mut ops = Ops::default();
    ops.remove("a");
    ops.put(Entry { tries: 2, ..tg("b", "-1") });
    ops.records.push((HEALTH_KIND, "-1".into(), Some("{}".into())));
    ops.records.push((DIGEST_KIND, "g/x".into(), None));
    ops.apply(&mut t).unwrap();
    assert_eq!(ids(&t), vec!["b"]);
    assert_eq!(serde_json::from_str::<Entry>(&t.get(KIND, "b").unwrap()).unwrap().tries, 2);
    assert_eq!(t.get(HEALTH_KIND, "-1").as_deref(), Some("{}"));
    // A put after a remove of the same id wins, and the other way round.
    let mut ops = Ops::default();
    ops.remove("b");
    ops.put(tg("b", "-1"));
    assert!(ops.removes.is_empty() && ops.puts.contains_key("b"));
    ops.remove("b");
    assert!(ops.puts.is_empty());
}

#[test]
fn routing_replaces_a_routed_entry_by_its_groups_and_files_digest_lines() {
    let groups = [
        group("k", "-1", &[("stock.low", Mode::Now)]),
        group("o", "-2", &[("stock.low", Mode::Digest)]),
    ];
    let low = route::routed("s1".into(), "stock.low", &json!({ "data": { "items": [{ "name": "Rice", "on_hand": 1, "low_at": 2, "unit": "kg" }] } }), NOW);
    let other = tg("keep", "-9");
    let mut ops = Ops::default();
    let work = route_all(vec![low, other], &groups, UTC, NOW, &mut ops);
    let got: Vec<&str> = work.iter().map(|e| e.id.as_str()).collect();
    assert!(got.contains(&"s1/g/k") && got.contains(&"keep") && got.contains(&"digest/o/daily"));
    assert!(ops.removes.contains(&"s1".to_string()));
    assert_eq!(ops.records.len(), 1);
    assert_eq!(ops.records[0].1, "o/s1");
    assert!(ops.puts.contains_key("digest/o/daily"), "a group that chose the summary is owed one");
}

#[test]
fn routing_removes_a_summary_nobody_wants_any_more() {
    let stale = Entry::new("digest/o/daily".into(), digest::KIND, "o".into(), "daily@540".into(), NOW);
    let mut ops = Ops::default();
    let work = route_all(vec![stale], &[group("o", "-2", &[])], UTC, NOW, &mut ops);
    assert!(work.is_empty());
    assert_eq!(ops.removes, vec!["digest/o/daily".to_string()]);
}

#[test]
fn pacing_admits_fifteen_per_chat_and_a_stopped_chat_none() {
    let mut p = Pace::default();
    let admitted = (0..20).filter(|_| p.admit("-1:4")).count();
    assert_eq!(admitted, route::PER_CHAT);
    assert!(p.admit("-2"));
    p.stop("-2");
    assert!(!p.admit("-2:7"), "a stopped chat is stopped in every topic");
}

#[test]
fn settle_maps_each_outcome_to_the_image() {
    let (mut ops, mut pace) = (Ops::default(), Pace::default());
    assert_eq!(settle(&tg("a", "-1"), &Ok(()), NOW, &mut ops, &mut pace), (true, false, None));
    assert!(ops.removes.contains(&"a".to_string()));

    let e = Entry { tries: 3, ..tg("b", "-1") };
    assert_eq!(settle(&e, &Err(Fail::RetryAfter(4)), NOW, &mut ops, &mut pace), (false, true, None));
    assert_eq!((ops.puts["b"].tries, ops.puts["b"].next_at_ms), (3, NOW + 4000), "a 429 spends no try");
    assert!(!pace.admit("-1"), "and the chat waits for the rest of this drain");

    assert_eq!(settle(&tg("c", "-55:2"), &Err(Fail::Migrated(-100)), NOW, &mut ops, &mut pace), (false, true, None));
    assert_eq!(ops.puts["c"].to, "-100:2");
    assert_eq!(ops.moved, vec![("-55".to_string(), "-100".to_string())]);

    let (s, k, line) = settle(&tg("d", "-7"), &Err(Fail::Gone("Forbidden: bot was kicked".into())), NOW, &mut ops, &mut pace);
    assert!(!s && !k && line.unwrap().contains("gone from chat -7"));
    assert_eq!(ops.gone[0].0, "-7");

    let e = Entry { tries: 5, ..tg("e", "-8") };
    let (_, _, line) = settle(&e, &Err(Fail::Other("Bad Gateway".into())), NOW, &mut ops, &mut pace);
    assert_eq!(line.as_deref(), Some("e after 6 attempts"));
    let (_, k, _) = settle(&tg("f", "-8"), &Err(Fail::Other("Bad Gateway".into())), NOW, &mut ops, &mut pace);
    assert!(k);
    assert_eq!(ops.puts["f"].tries, 1);
}

#[test]
fn health_keeps_the_last_success_and_the_last_refusal_per_target() {
    let mut p = Pace::default();
    p.note(None, "-1", &Err(Fail::Gone("kicked".into())), 10);
    p.note(None, "-1", &Ok(()), 20);
    let h = &p.health["-1"];
    assert_eq!((h.ok_ms, h.err_ms, h.err.as_str()), (20, 10, "kicked"));
}

#[test]
fn the_owners_are_told_in_every_language_by_the_groups_title() {
    let n = gone_notice("-1", &[group("k", "-1", &[])], NOW);
    assert_eq!((n.kind.as_str(), n.to.as_str()), (route::ROUTE_KIND, "system.alert"));
    let owners = group("o", "-2", &[("system.alert", Mode::Now)]);
    let mut uk = owners.clone();
    uk.lang = "uk".into();
    let sent = route::fan_out(&n, &[owners, uk], UTC, NOW).send;
    assert!(sent[0].text.starts_with("⚠ The bot was removed") && sent[0].text.ends_with("T-k"));
    assert!(sent[1].text.starts_with("⚠ Бота видалили"));
}
