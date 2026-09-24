//! P3: the register is pseudonymous, merges retries, and keeps venues apart.

use super::*;
use crate::services::customers::handlers::customer_key;

const SECRET: &[u8] = b"test-secret-key";
const PHONES: [&str; 2] = ["+355691111111", "069 111 1111"];
const NOW: i64 = 1_800_000_000_000;

fn keys() -> BTreeSet<String> {
    PHONES.iter().map(|p| customer_key(SECRET, p)).collect()
}

fn orders() -> BTreeSet<String> {
    ["ord_a1b2c3", "ord_d4e5f6"].iter().map(|s| s.to_string()).collect()
}

fn longest_digit_run(s: &str) -> usize {
    let (mut best, mut run) = (0, 0);
    for c in s.chars() {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        best = best.max(run);
    }
    best
}

/// THE CHECK: the stored record holds no phone (in any spelling, nor any run
/// of seven digits), no name, and no millisecond instant; the day is enough.
#[test]
fn the_register_holds_no_phone_no_name_and_no_instant() {
    let mut t = Table::create(1 << 16).unwrap();
    let k = customer_key(SECRET, PHONES[0]);
    put(&mut t, Entry::new("venue_1", &k, &keys(), &orders(), NOW)).unwrap();
    let raw = t.get(KIND, &id_of("venue_1", &k)).unwrap();
    for p in PHONES {
        let digits: String = p.chars().filter(char::is_ascii_digit).collect();
        assert!(!raw.contains(p) && !raw.contains(&digits) && !raw.contains(&digits[3..]), "{p} in {raw}");
    }
    assert!(!raw.contains("Arben"));
    assert!(!raw.contains(&NOW.to_string()), "an instant: {raw}");
    // THE DIGIT-RUN GREP, over everything that is not a pseudonym. A key is
    // 16 hex characters of an HMAC and holds seven digits in a row about one
    // time in ten (`82e57af47296844c` does) -- that is chance, not a phone:
    // the key is proved not to be one above (no spelling of the number is in
    // the record) and to be a customer key here. So keys and order ids are
    // masked, and any OTHER seven-digit run is a phone that got in.
    let e: Entry = serde_json::from_str(&raw).unwrap();
    let mut masked = raw.clone();
    for k in &e.keys {
        assert!(crate::services::customers::forget::is_customer_key(k), "not a key: {k}");
        masked = masked.replace(k.as_str(), "KEY");
    }
    for o in &e.orders {
        masked = masked.replace(o.as_str(), "ORDER");
    }
    assert!(longest_digit_run(&masked) < 7, "a phone-length digit run: {masked}");
    assert_eq!(e.day, NOW / 86_400_000);
    assert_eq!(e.keys, keys());
}

/// A retry finds no phones (they are gone), so it must not overwrite what the
/// first run found: keys and orders are unioned, the first day stays.
#[test]
fn a_retry_merges_and_never_forgets_what_the_first_run_found() {
    let mut t = Table::create(1 << 16).unwrap();
    let k = customer_key(SECRET, PHONES[0]);
    put(&mut t, Entry::new("venue_1", &k, &keys(), &orders(), NOW)).unwrap();
    let later = put(&mut t, Entry::new("venue_1", &k, &BTreeSet::new(), &BTreeSet::new(), NOW + 86_400_000 * 3)).unwrap();
    assert_eq!(later.orders, orders());
    assert_eq!(later.keys, keys());
    assert_eq!(later.day, NOW / 86_400_000);
}

/// One venue's replay reads its own entries and no other venue's; a record
/// that does not parse is COUNTED, never read as "nobody".
#[test]
fn a_venue_reads_its_own_entries_and_counts_a_broken_one() {
    let mut t = Table::create(1 << 16).unwrap();
    let k = customer_key(SECRET, PHONES[0]);
    put(&mut t, Entry::new("venue_1", &k, &keys(), &orders(), NOW)).unwrap();
    put(&mut t, Entry::new("venue_10", &k, &keys(), &orders(), NOW)).unwrap();
    t.put(KIND, "venue_1/broken", "{not json", &[], &[]).unwrap();
    let (mine, bad) = of_venue(&t, "venue_1");
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].venue, "venue_1");
    assert_eq!(bad, 1);
    let (other, bad) = of_venue(&t, "venue_2");
    assert!(other.is_empty() && bad == 0);
}
