use super::*;
use crate::services::orders::status::took_money;
use dowiz_hub::tables::{Shape, Table, Zone};

fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 10, order_json: o.to_string() }
}

/// One round as the log folds it: `paid` is ONE payment at `paid_at`, the
/// shape `command::pay` writes.
fn round(id: &str, sitting: &str, table: &str, status: &str, total: i64, paid: i64, at: i64) -> Value {
    let payments: Vec<Value> =
        if paid > 0 { vec![json!({"amount": paid, "method": "card", "at": at + 50})] } else { vec![] };
    json!({"id": id, "location_id": "v1", "sitting_id": sitting, "status": status, "total": total,
           "payments": payments, "created_at_ms": at, "fulfilment": {"kind": "dine_in", "table": table}})
}

fn t(n: i64) -> Table {
    Table { n, x: 40 * n, y: 50, w: 30, h: 30, seats: 4, shape: Shape::Rect }
}

fn plan() -> Plan {
    Plan {
        zones: vec![
            Zone { id: "salla".into(), name: "Salla".into(), tables: (1..=7).map(t).collect() },
            Zone { id: "terasa".into(), name: "Terasa".into(), tables: vec![t(7), t(9)] },
        ],
    }
}

fn state_at(fl: &Value, zone: &str, n: i64) -> String {
    let z = fl["zones"].as_array().unwrap().iter().find(|z| z["id"] == zone).unwrap();
    let t = z["tables"].as_array().unwrap().iter().find(|t| t["n"] == n).unwrap();
    t["state"].as_str().unwrap().to_string()
}

const NOW: i64 = 1_000 * 60_000;

/// THE SIX STATES on one synthetic floor, through `floor` — the function the
/// route answers with.
#[test]
fn six_states_on_one_floor() {
    let listed = vec![
        view("a", round("a", "s1", "1", "CONFIRMED", 900, 0, 1)),
        view("b", round("b", "s2", "salla:2", "PREPARING", 900, 0, 1)),
        view("c", round("c", "s3", "3", "READY", 900, 400, 1)),
        view("d", round("d", "s4", "4", "PICKED_UP", 900, 900, 1)),
    ];
    let held = vec![Held { zone: "salla".into(), n: 5, slot_min: NOW / 60_000 + 30, reservation: "r1".into() }];
    let fl = floor(&plan(), &held, &listed, NOW);
    assert_eq!(state_at(&fl, "salla", 1), "ordering");
    assert_eq!(state_at(&fl, "salla", 2), "waiting");
    assert_eq!(state_at(&fl, "salla", 3), "paying");
    assert_eq!(state_at(&fl, "salla", 4), "dirty");
    assert_eq!(state_at(&fl, "salla", 5), "booked");
    assert_eq!(state_at(&fl, "salla", 6), "free");
    let t4 = &fl["zones"][0]["tables"][3];
    assert_eq!(t4["sitting_id"], "s4", "a dirty table names the sitting to clear");
}

/// TOTALITY AND DISJOINTNESS over the full grid: booking {no, yes} × newest
/// status {every kernel status, an unknown one, no sitting} of a one-round sitting × Σ paid {<, ==,
/// >} bill × mark {none, before the last payment, after it}. Each point goes
/// through the real `seated` + `booked_at` + `state_of`, and exactly ONE of
/// §2.7's six sentences — written below as the blueprint says them — is true
/// of it, and it is the state returned. Every state is reached.
#[test]
fn the_six_states_are_a_total_disjoint_function_of_the_grid() {
    let statuses = [
        "PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED", "REJECTED", "CANCELLED",
        "SCHEDULED", "PICKED_UP", "REFUNDING", "COMPENSATED_REFUND", "NOT_A_STATUS",
    ];
    let mut seen = std::collections::HashSet::new();
    let mut points = 0;
    for booked in [false, true] {
        let held: Vec<Held> = if booked {
            vec![Held { zone: "salla".into(), n: 1, slot_min: NOW / 60_000, reservation: "r".into() }]
        } else {
            vec![]
        };
        for newest in statuses.iter().copied().map(Some).chain([None]) {
            for paid in [999, 1000, 1001] {
                for mark_at in [None, Some(0), Some(10_000)] {
                    let mut listed = vec![];
                    if let Some(st) = newest {
                        let mut o = round("new", "s", "1", st, 1000, paid, 2);
                        if let Some(at) = mark_at {
                            o["cleared"] = json!({"by": "w", "at": at});
                        }
                        listed.push(view("new", o));
                    }
                    let rs = sitting::rounds(&listed, "s");
                    let seat = seated("s", &rs);
                    let got = state_of(booked_at(&held, "salla", 1, NOW), seat.as_ref());

                    // §2.7, sentence by sentence.
                    let live = newest.is_some_and(took_money);
                    let st = newest.unwrap_or("");
                    let past = !matches!(st, "PENDING" | "CONFIRMED" | "SCHEDULED" | "PREPARING");
                    let settled = paid >= 1000;
                    let cleared = mark_at.is_some_and(|m| m >= 2 + 50);
                    let ordering = live && matches!(st, "PENDING" | "CONFIRMED" | "SCHEDULED");
                    let waiting = live && st == "PREPARING";
                    let paying = live && past && !settled;
                    let dirty = live && past && settled && !cleared;
                    let sat = ordering || waiting || paying || dirty;
                    let truths = [
                        (FloorState::Free, !sat && !booked),
                        (FloorState::Booked, !sat && booked),
                        (FloorState::Ordering, ordering),
                        (FloorState::Waiting, waiting),
                        (FloorState::Paying, paying),
                        (FloorState::Dirty, dirty),
                    ];
                    let true_ones: Vec<FloorState> = truths.iter().filter(|x| x.1).map(|x| x.0).collect();
                    assert_eq!(true_ones.len(), 1, "not disjoint/total at {booked} {newest:?} {paid} {mark_at:?}");
                    assert_eq!(got, true_ones[0], "at {booked} {newest:?} {paid} {mark_at:?}");
                    seen.insert(got);
                    points += 1;
                }
            }
        }
    }
    assert_eq!(points, 2 * 14 * 3 * 3);
    assert_eq!(seen.len(), FloorState::ALL.len(), "every state is reached: {seen:?}");
}

/// §2.7's prove case: a paid sitting with no `cleared` is Dirty, and Free ONE
/// EVENT LATER — the mark, applied as the `Noted` delta the route writes.
#[test]
fn a_paid_sitting_is_dirty_until_the_mark_and_free_after_it() {
    let old = round("a", "s1", "4", "PICKED_UP", 900, 900, 1);
    let before = vec![view("a", old.clone())];
    assert_eq!(state_at(&floor(&plan(), &[], &before, NOW), "salla", 4), "dirty");
    assert_eq!(clear_target(&before, "s1"), Ok(("a".to_string(), true)));

    let new = mark(&old, "waiter-1", NOW).expect("marked");
    let delta = crate::fold::delta(&old, &new).to_string();
    let folded = crate::fold::fold_one(old.clone(), &delta);
    assert_eq!(folded["cleared"], json!({"by": "waiter-1", "at": NOW}));
    let after = vec![view("a", folded)];
    assert_eq!(state_at(&floor(&plan(), &[], &after, NOW), "salla", 4), "free");
    // A second tap writes nothing.
    assert_eq!(clear_target(&after, "s1"), Ok(("a".to_string(), false)));
}

/// A payment AFTER the mark makes the table dirty again: the mark only
/// counts from the last payment on.
#[test]
fn a_payment_after_the_mark_makes_it_dirty_again() {
    let mut o = round("a", "s1", "4", "PICKED_UP", 900, 900, 1);
    o["cleared"] = json!({"by": "w", "at": 10});
    o["payments"] = json!([{"amount": 900, "at": 5}, {"amount": 0, "tip": 100, "at": 20}]);
    assert_eq!(state_at(&floor(&plan(), &[], &[view("a", o)], NOW), "salla", 4), "dirty");
}

/// REFUSED: a table still eating or still owing is not cleared; a sitting
/// that is not in the log is not found. POSITIVE TWIN: a dirty one is.
#[test]
fn only_a_dirty_table_can_be_cleared() {
    let paying = vec![view("a", round("a", "s1", "4", "READY", 900, 400, 1))];
    assert_eq!(clear_target(&paying, "s1").map_err(|e| e.status()), Err(409));
    let cooking = vec![view("a", round("a", "s1", "4", "PREPARING", 900, 900, 1))];
    assert_eq!(clear_target(&cooking, "s1").map_err(|e| e.status()), Err(409));
    assert_eq!(clear_target(&paying, "nope"), Err(Refused::NotFound));
    let dirty = vec![view("a", round("a", "s1", "4", "READY", 900, 900, 1))];
    assert_eq!(clear_target(&dirty, "s1"), Ok(("a".to_string(), true)));
}

/// The mark goes on the LAST round, even when an earlier one carried the money.
#[test]
fn the_mark_goes_on_the_last_round() {
    let listed = vec![
        view("a", round("a", "s1", "4", "PICKED_UP", 900, 900, 1)),
        view("b", round("b", "s1", "4", "REJECTED", 300, 0, 2)),
    ];
    assert_eq!(clear_target(&listed, "s1"), Ok(("b".to_string(), true)));
}

/// A mark must name who cleared; the twin with a name is written.
#[test]
fn a_mark_names_its_signer() {
    let o = round("a", "s1", "4", "PICKED_UP", 900, 900, 1);
    assert_eq!(mark(&o, " ", 1).map_err(|e| e.status()), Err(400));
    assert_eq!(mark(&o, "w", 1).unwrap()["cleared"]["by"], "w");
}

/// The NEWEST sitting at a table is its state; an older one is history.
#[test]
fn the_newest_sitting_at_a_table_wins() {
    let listed = vec![
        view("b", round("b", "s2", "4", "PENDING", 900, 0, 50)),
        view("a", round("a", "s1", "4", "PICKED_UP", 900, 900, 1)),
    ];
    assert_eq!(state_at(&floor(&plan(), &[], &listed, NOW), "salla", 4), "ordering");
}

/// Which table the free text names; a bare number two zones share names none,
/// and such a sitting is listed as `unplaced`, never dropped.
#[test]
fn table_text_resolves_to_one_plan_table_or_is_listed_unplaced() {
    let p = plan();
    assert_eq!(table_of(&p, "terasa:9"), Some(("terasa".into(), 9)));
    assert_eq!(table_of(&p, " 9 "), Some(("terasa".into(), 9)));
    assert_eq!(table_of(&p, "salla/3"), Some(("salla".into(), 3)));
    assert_eq!(table_of(&p, "7"), None, "two zones have a table 7");
    assert_eq!(table_of(&p, "bar"), None);
    let listed = vec![view("a", round("a", "s1", "7", "PENDING", 900, 0, 1))];
    let fl = floor(&p, &[], &listed, NOW);
    assert_eq!(fl["unplaced"], json!([{"table": "7", "sitting_id": "s1", "state": "ordering"}]));
    assert_eq!(state_at(&fl, "salla", 7), "free");
}

/// A booking holds its table for `DWELL_MIN` either side of its slot, as the
/// booking write's own collision rule says; outside that it does not.
#[test]
fn a_booking_holds_within_the_dwell_only() {
    let slot = NOW / 60_000;
    let held = vec![Held { zone: "salla".into(), n: 1, slot_min: slot, reservation: "r".into() }];
    assert!(booked_at(&held, "salla", 1, NOW + (plan::DWELL_MIN - 1) * 60_000));
    assert!(!booked_at(&held, "salla", 1, NOW + plan::DWELL_MIN * 60_000));
    assert!(!booked_at(&held, "salla", 2, NOW));
}

/// `held_of` reads exactly what `booking::table_key` writes.
#[test]
fn held_of_reads_the_booking_index_key() {
    let k = crate::booking::table_key("terasa", 12, 1_234_567);
    assert_eq!(
        held_of(&k, "r9".into()),
        Some(Held { zone: "terasa".into(), n: 12, slot_min: 1_234_567, reservation: "r9".into() })
    );
    assert_eq!(held_of("rsv.user/u/1/x", "r".into()), None);
}
