//! B6: `services/analytics/kitchen/sales.rs:70 Window::bucket` -- `rposition` (linear from the
//! end) over the day starts, called once per order and per journal row. REPLACEMENT:
//! `partition_point` (binary search). Equivalence: the same bucket for every probe.
fn bucket_current(starts: &[i64], end: i64, at: i64) -> Option<usize> {
    if starts.is_empty() || at < starts[0] || at >= end {
        return None;
    }
    starts.iter().rposition(|s| *s <= at)
}

fn bucket_replacement(starts: &[i64], end: i64, at: i64) -> Option<usize> {
    if starts.is_empty() || at < starts[0] || at >= end {
        return None;
    }
    // first index with start > at, minus one
    Some(starts.partition_point(|s| *s <= at) - 1)
}

pub fn run(reps: usize) {
    let day = 86_400_000i64;
    let t0 = 1_782_900_000_000i64;
    for &days in &[7usize, 30, 62] {
        let starts: Vec<i64> = (0..days).map(|d| t0 + d as i64 * day).collect();
        let end = t0 + days as i64 * day;
        // probes: 1200 orders spread over the window plus a few outside
        let probes: Vec<i64> = (0..1200).map(|i| t0 - day + (i as i64 * 7_919_123) % (end - t0 + 2 * day)).collect();
        for &p in &probes {
            assert_eq!(bucket_current(&starts, end, p), bucket_replacement(&starts, end, p), "B6 probe {p}");
        }
        let hits = probes.iter().filter(|p| bucket_current(&starts, end, **p).is_some()).count();
        println!("B6 EQUIV ok: days={days}, {} probes agree ({hits} in window)", probes.len());
        let (_, cur, _) = super::median_ns(reps, || super::time_ns(|| { for p in &probes { std::hint::black_box(bucket_current(&starts, end, *p)); } }));
        let (_, rep, _) = super::median_ns(reps, || super::time_ns(|| { for p in &probes { std::hint::black_box(bucket_replacement(&starts, end, *p)); } }));
        println!("B6 days={days}: CURRENT {:.1} ns/call | REPLACE {:.1} ns/call | {:.1}x | per 1200 calls {:.1} us -> {:.1} us", cur as f64 / 1200.0, rep as f64 / 1200.0, cur as f64 / rep.max(1) as f64, cur as f64 / 1e3, rep as f64 / 1e3);
    }
}
