//! The rules of the payroll tab, as functions of their arguments: what an entry may be, and
//! how minutes and a rate become money (exact integers, one rounding, the table adds up).

use super::*;

fn e(staff: &str, date: &str, minutes: i64, rate: i64) -> Entry {
    Entry { staff_id: staff.into(), date: date.into(), minutes, rate_minor: rate, note: String::new() }
}

#[test]
fn a_month_is_yyyy_mm_and_nothing_else() {
    assert_eq!(check_month("2026-11"), Ok(()));
    for bad in ["2026-13", "2026-00", "2026-1", "26-11", "2026/11", "2026-11-01", ""] {
        assert!(check_month(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_date_is_a_real_calendar_day() {
    for ok in ["2026-11-08", "2024-02-29", "2026-12-31"] {
        assert_eq!(check_date(ok), Ok(()), "{ok}");
    }
    for bad in ["2026-02-29", "2026-02-30", "2026-04-31", "2026-13-01", "2026-11-8", "2026-11-00", "nope"] {
        assert!(check_date(bad).is_err(), "{bad}");
    }
}

#[test]
fn hours_are_whole_quarters_of_a_day_at_most() {
    for ok in [15, 30, 45, 60, 450, 1440] {
        assert_eq!(check_minutes(ok), Ok(()), "{ok}");
    }
    for bad in [0, -15, 7, 20, 1455, 1441] {
        assert!(check_minutes(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_rate_is_positive_and_bounded() {
    assert_eq!(check_rate(1), Ok(()));
    assert_eq!(check_rate(MAX_RATE_MINOR), Ok(()));
    for bad in [0, -1, MAX_RATE_MINOR + 1] {
        assert!(check_rate(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_note_is_trimmed_and_bounded() {
    assert_eq!(clean_note(None), Ok(String::new()));
    assert_eq!(clean_note(Some("  shift swap ")), Ok("shift swap".to_string()));
    assert!(clean_note(Some(&"x".repeat(MAX_NOTE))).is_ok());
    assert!(clean_note(Some(&"x".repeat(MAX_NOTE + 1))).is_err());
}

#[test]
fn the_venue_local_day_and_its_month() {
    let z = dowiz_hub::tz::zone("Europe/Tirane").expect("zone");
    // 23:30 UTC on the 7th is already the 8th in Tirana.
    let utc = 1_762_558_200_000; // 2025-11-07T23:30:00Z
    assert_eq!(date_of(dowiz_hub::tz::local_ms(z, utc)), "2025-11-08");
    assert_eq!(month_of("2025-11-08"), "2025-11");
}

#[test]
fn pay_is_rate_times_minutes_over_sixty_rounded_half_up() {
    assert_eq!(pay_minor(250 * 60), Some(250)); // one hour
    assert_eq!(pay_minor(250 * 450), Some(1875)); // 7.5 hours at 250
    assert_eq!(pay_minor(250 * 15), Some(63)); // 62.5 rounds up, once
    assert_eq!(pay_minor(100 * 15), Some(25));
    assert_eq!(pay_minor(0), Some(0));
    assert_eq!(pay_minor(i128::MAX - 40), None, "too big is a refusal, not a wrap");
}

#[test]
fn the_row_is_rounded_once_so_the_table_adds_up() {
    let entries = [e("a", "2026-11-01", 15, 250), e("a", "2026-11-02", 15, 250), e("b", "2026-11-02", 15, 250)];
    // a: 30 min at 250 = 125 (not 63 + 63 = 126); b: 62.5 -> 63. Grand total 188.
    assert_eq!(month_total(&entries, "2026-11"), Some(188));
    let v = month_view("2026-11", &[], &Default::default(), &entries).expect("view");
    let rows = v["rows"].as_array().expect("rows");
    let row = |id: &str| rows.iter().find(|r| r["staffId"] == id).expect("row");
    assert_eq!(row("a")["payMinor"], 125);
    assert_eq!(row("b")["payMinor"], 63);
    assert_eq!(v["totalMinor"], 188);
    assert_eq!(v["totalMinutes"], 45);
    let sum: i64 = rows.iter().map(|r| r["payMinor"].as_i64().unwrap()).sum();
    assert_eq!(sum, v["totalMinor"].as_i64().unwrap(), "grand total is the sum of the rows");
}

#[test]
fn a_month_counts_only_its_own_days_and_keeps_each_entrys_rate() {
    let entries = [e("a", "2026-10-31", 60, 100), e("a", "2026-11-01", 60, 300), e("a", "2026-12-01", 60, 999)];
    assert_eq!(month_total(&entries, "2026-11"), Some(300));
    assert_eq!(month_total(&entries, "2026-10"), Some(100));
    assert_eq!(month_total(&entries, "2026-01"), Some(0));
}

#[test]
fn rows_are_employees_and_columns_are_the_entered_dates() {
    let people = vec![
        Person { id: "b".into(), name: "Bora".into(), role: "kitchen".into(), active: true },
        Person { id: "a".into(), name: "Ana".into(), role: "waiter".into(), active: true },
        Person { id: "c".into(), name: "Cila".into(), role: "kitchen".into(), active: false },
    ];
    let mut rates = std::collections::BTreeMap::new();
    rates.insert("a".to_string(), 200);
    let entries = [e("a", "2026-11-05", 60, 200), e("a", "2026-11-02", 120, 200), e("gone", "2026-11-03", 60, 150)];
    let v = month_view("2026-11", &people, &rates, &entries).expect("view");
    let dates: Vec<_> = v["dates"].as_array().unwrap().iter().map(|d| d.as_str().unwrap()).collect();
    assert_eq!(dates, ["2026-11-02", "2026-11-03", "2026-11-05"]);
    let ids: Vec<_> = v["rows"].as_array().unwrap().iter().map(|r| r["staffId"].as_str().unwrap().to_string()).collect();
    assert_eq!(ids, ["a", "b", "gone"], "by name; suspended with no hours is not a row; a person whose record is gone keeps their hours");
    let a = &v["rows"][0];
    assert_eq!(a["minutes"], 180);
    assert_eq!(a["payMinor"], 600);
    assert_eq!(a["days"]["2026-11-02"]["minutes"], 120);
    assert_eq!(a["rateMinor"], 200);
    assert!(v["rows"][1]["rateMinor"].is_null(), "no rate yet");
    let staff = v["staff"].as_array().unwrap();
    assert_eq!(staff.len(), 3, "the picker offers every employee: {staff:?}");
}

#[test]
fn one_employees_history_is_pay_by_month_newest_first() {
    let entries = [
        e("a", "2026-09-01", 60, 100),
        e("a", "2026-11-01", 60, 300),
        e("a", "2026-11-02", 30, 300),
        e("b", "2026-11-02", 60, 999),
    ];
    let v = history(&entries, "a").expect("history");
    let months: Vec<_> = v["months"].as_array().unwrap().iter().map(|m| (m["month"].as_str().unwrap(), m["minutes"].as_i64().unwrap(), m["payMinor"].as_i64().unwrap())).collect();
    assert_eq!(months, [("2026-11", 90, 450), ("2026-09", 60, 100)]);
}

#[test]
fn a_corrupt_record_is_loud_not_skipped() {
    let rows = vec![("x".to_string(), "{not json".to_string())];
    let err = parse_all(rows).expect_err("a broken record refuses the read");
    assert!(err.contains('x'), "{err}");
    let good = serde_json::to_string(&e("a", "2026-11-01", 60, 100)).unwrap();
    assert_eq!(parse_all(vec![("id".to_string(), good)]).unwrap().len(), 1);
}
