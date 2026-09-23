use super::*;
use serde_json::json;

fn asg(order: &str, courier: &str, delivered: Option<i64>) -> (String, String) {
    let rec = json!({
        "order_id": order, "courier_id": courier, "assigned_at_ms": 1,
        "delivered_at_ms": delivered, "cash_collected": null,
    });
    (order.to_string(), rec.to_string())
}

fn order(status: &str) -> String {
    json!({ "id": "x", "location_id": "v", "status": status, "total": 1500 }).to_string()
}

#[test]
fn a_refunded_order_is_over_and_a_live_one_is_not() {
    assert!(run_over("REFUNDING"));
    assert!(run_over("COMPENSATED_REFUND"));
    assert!(run_over("DELIVERED"));
    // Positive twins: every status a courier can still carry to the door.
    for live in ["CONFIRMED", "PREPARING", "READY", "IN_DELIVERY"] {
        assert!(!run_over(live), "{live} is still a run");
    }
    // Unreadable is held, never released.
    assert!(!run_over("NONSENSE"));
    assert!(!run_over(""));
}

#[test]
fn the_ended_set_is_exactly_where_delivered_is_unreachable() {
    // Derived against the kernel, not listed twice: a status is over iff no
    // path of kernel edges leads from it to DELIVERED.
    use dowiz_kernel::order_machine::assert_transition;
    let all = [
        "PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED",
        "REJECTED", "CANCELLED", "SCHEDULED", "PICKED_UP", "REFUNDING", "COMPENSATED_REFUND",
    ];
    let st: Vec<OrderStatus> = all.iter().map(|s| OrderStatus::from_str(s).expect(s)).collect();
    let reaches = |from: OrderStatus| {
        let mut seen = vec![from];
        let mut i = 0;
        while i < seen.len() {
            let f = seen[i];
            for t in &st {
                if assert_transition(f, *t).is_ok() && !seen.contains(t) {
                    seen.push(*t);
                }
            }
            i += 1;
        }
        seen.contains(&OrderStatus::Delivered) && from != OrderStatus::Delivered
    };
    for (name, s) in all.iter().zip(&st) {
        // SCHEDULED is a scaffold terminal the kernel does not call terminal;
        // PENDING is not yet a run. Neither is ever assigned (accept refuses).
        if matches!(*name, "SCHEDULED" | "PENDING") {
            continue;
        }
        assert_eq!(run_over(name), !reaches(*s), "{name}");
    }
}

#[test]
fn a_courier_holding_only_a_refunded_order_holds_nothing() {
    // The shift handler's own path: open runs, their orders as the log
    // answers them, `ended`, then `in_hand`.
    let rows = vec![asg("o1", "c1", None)];
    for st in ["REFUNDING", "COMPENSATED_REFUND"] {
        let read = vec![("o1".to_string(), Some(order(st)))];
        assert!(in_hand(&rows, "c1", &ended(&read)).is_empty(), "{st}");
    }
}

#[test]
fn a_courier_holding_a_live_order_still_holds_it() {
    let rows = vec![asg("o1", "c1", None), asg("o2", "c1", None)];
    // o1 refunded, o2 still on the road: o2 is the run in hand.
    let read = vec![
        ("o1".to_string(), Some(order("REFUNDING"))),
        ("o2".to_string(), Some(order("IN_DELIVERY"))),
    ];
    assert_eq!(in_hand(&rows, "c1", &ended(&read)), vec!["o2".to_string()]);
    // Nothing ended: both held.
    assert_eq!(in_hand(&rows, "c1", &[]).len(), 2);
}

#[test]
fn a_missing_or_unreadable_order_keeps_its_run_held() {
    let rows = vec![asg("o1", "c1", None), asg("o2", "c1", None)];
    let read = vec![("o1".to_string(), None), ("o2".to_string(), Some("{bad".to_string()))];
    assert!(ended(&read).is_empty());
    assert_eq!(in_hand(&rows, "c1", &ended(&read)).len(), 2);
}

#[test]
fn delivered_and_other_couriers_runs_are_not_candidates() {
    let rows = vec![asg("o1", "c1", Some(5)), asg("o2", "c2", None), asg("o3", "c1", None)];
    assert_eq!(open_runs(&rows, "c1"), vec!["o3".to_string()]);
    assert_eq!(open_runs(&rows, "c2"), vec!["o2".to_string()]);
}

#[test]
fn an_unreadable_row_is_skipped_as_it_always_was() {
    let rows = vec![("bad".to_string(), "{not json".to_string()), asg("o1", "c1", None)];
    assert_eq!(in_hand(&rows, "c1", &[]), vec!["o1".to_string()]);
}

#[test]
fn refunding_an_order_nobody_carries_moves_no_courier() {
    // A table or collection order refunded: it has no assignment, so no
    // courier's run changes -- and the live run beside it is still held.
    let rows = vec![asg("o1", "c1", None)];
    let over = vec!["table-7".to_string()];
    assert_eq!(in_hand(&rows, "c1", &over), vec!["o1".to_string()]);
    assert!(in_hand(&[], "c1", &over).is_empty());
}

// ── the owner's console: `still_carried` then the real `record::tally` ──

fn in_flight(rows: &[(String, String)], read: &[(String, Option<String>)]) -> i64 {
    let kept = still_carried(rows, &ended(read));
    crate::services::courier::record::tally(&kept, "c1", 0, 10).in_flight
}

#[test]
fn the_console_does_not_count_a_refunded_run_in_flight() {
    let rows = vec![asg("o1", "c1", None)];
    for st in ["REFUNDING", "COMPENSATED_REFUND"] {
        assert_eq!(in_flight(&rows, &[("o1".into(), Some(order(st)))]), 0, "{st}");
    }
}

#[test]
fn the_console_counts_a_live_run_in_flight() {
    let rows = vec![asg("o1", "c1", None), asg("o2", "c1", None)];
    let read = vec![("o1".into(), Some(order("REFUNDING"))), ("o2".into(), Some(order("IN_DELIVERY")))];
    assert_eq!(in_flight(&rows, &read), 1);
}

#[test]
fn the_console_keeps_an_unreadable_run_in_flight() {
    let rows = vec![asg("o1", "c1", None), asg("o2", "c1", None)];
    let read = vec![("o1".into(), None), ("o2".into(), Some("{bad".into()))];
    assert_eq!(in_flight(&rows, &read), 2);
}

#[test]
fn the_console_keeps_delivered_runs_counted() {
    let rows = vec![asg("o1", "c1", Some(5)), asg("o2", "c1", None)];
    let kept = still_carried(&rows, &ended(&[("o2".into(), Some(order("REFUNDING")))]));
    let t = crate::services::courier::record::tally(&kept, "c1", 0, 10);
    assert_eq!((t.today_deliveries, t.in_flight), (1, 0));
}

#[test]
fn only_the_assigned_courier_may_tap_refused() {
    let rows = vec![asg("o1", "c1", None), asg("o2", "c1", Some(5))];
    assert!(may_refuse(&rows, "c1", "o1"));
    assert!(!may_refuse(&rows, "c2", "o1"), "another courier");
    assert!(!may_refuse(&rows, "c1", "o2"), "already delivered");
    assert!(!may_refuse(&rows, "c1", "o9"), "no assignment");
}
