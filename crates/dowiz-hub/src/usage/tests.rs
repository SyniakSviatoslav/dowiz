use crate::*;

/// The number that matters is how close the arena is to refusing a write,
/// and a reading that does not move as the log grows is a gauge that is not
/// connected to anything.
#[test]
fn usage_climbs_as_the_log_is_written() {
    let mut hub = Hub::create_sized(256 * 1024).expect("hub");
    let empty = hub.usage();
    assert!(empty.capacity_cells > 0, "capacity must be readable: {empty:?}");
    assert_eq!(empty.used_per_mille(), 0, "a fresh image has spent nothing");

    for i in 0..20 {
        hub.append(
            EventKind::Placed,
            &format!("ord-{i}"),
            r#"{"status":"new","items":[]}"#,
            i as u64,
            [0u8; 32],
        )
        .expect("append");
    }
    let after = hub.usage();
    assert!(after.used_cells > empty.used_cells, "{empty:?} -> {after:?}");
    assert!(after.generation > empty.generation);
    assert!(after.used_per_mille() > 0, "the gauge must move: {after:?}");
    assert!(
        after.used_per_mille() < 1000,
        "and must not read full when it is not: {after:?}"
    );
}

/// THE KV IMAGES ARE THE ONES THIS GETS WRONG IF IT USES CAPACITY. A
/// compacted image is re-sized on every save, so measured against its own
/// capacity the reading sawtooths — it climbed to 942 per mille and fell
/// back to 517 on the next save, which would send an owner compacting
/// something with nothing to reclaim. Against the ceiling it only climbs.
#[test]
fn a_compacted_image_gauge_never_falls_as_it_grows() {
    let mut worst_drop = 0;
    let mut prev = 0;
    let mut last = 0;
    for n in [1usize, 20, 60, 100, 140, 180, 220, 400] {
        let mut s = settings::Settings::create().expect("settings");
        for i in 0..n {
            s.set(&format!("ai.k{i}"), &"x".repeat(60));
        }
        let bytes = s.to_bytes().expect("to_bytes");
        let u = settings::Settings::load(&bytes).expect("reload").usage();
        let now = u.used_per_mille();
        worst_drop = worst_drop.max(prev - now);
        prev = now;
        last = now;
    }
    assert_eq!(
        worst_drop, 0,
        "the reading fell by {worst_drop} per mille as the image GREW; \
         it is being measured against a capacity that is re-chosen on save"
    );
    assert!(last > 0, "and it has to move at all: {last}");
}

/// The ceiling is the point a write is actually refused, so a reading near
/// full must mean the next save is near failing — not that a doubling is due.
#[test]
fn the_compacted_gauge_predicts_the_real_refusal() {
    let mut last_ok = 0;
    for n in 1.. {
        let mut s = settings::Settings::create().expect("settings");
        // 1500 entries a step: about 6% of the 10 MiB ceiling, fine enough
        // for the last reading before the refusal to land above 700.
        for i in 0..n * 1500 {
            s.set(&format!("ai.k{i}"), &"x".repeat(60));
        }
        match s.to_bytes() {
            Ok(b) => {
                last_ok = settings::Settings::load(&b)
                    .expect("reload")
                    .usage()
                    .used_per_mille()
            }
            Err(_) => break,
        }
        assert!(n < 100, "settings must refuse eventually");
    }
    assert!(
        last_ok > 700,
        "the last reading before settings refused a save was only {last_ok} per mille"
    );
}

/// AN APPEND LOG DOES NOT REFUSE, SO THE GAUGE CANNOT WARN OF A REFUSAL.
///
/// This test used to assert that a 16 KiB hub filled and that the reading
/// just before the refusal was above 800 per mille. It passed for the wrong
/// reason: the hub DID refuse, but only because `append` grew the image for
/// a full record and not for the tip update that follows it -- fifty-six
/// cells free and an order turned away. With that fixed the log grows
/// instead, so what this must assert is the opposite: the reading moves,
/// and filling is not a failure.
#[test]
fn an_append_log_doubles_instead_of_refusing() {
    let mut hub = Hub::create_sized(16 * 1024).expect("hub");
    let first = hub.usage();
    assert!(first.grows, "the order log grows; the gauge must say so");
    let mut doubled = false;
    for i in 0..2000 {
        hub.append(EventKind::Placed, &format!("o{i}"), "{}", i as u64, [0u8; 32])
            .unwrap_or_else(|e| panic!("the log refused order {i} rather than growing: {e:?}"));
        if hub.usage().capacity_cells > first.capacity_cells {
            doubled = true;
        }
    }
    assert!(doubled, "2000 orders did not outgrow a 16 KiB image: {:?}", hub.usage());
    assert_eq!(hub.len(), 2000, "growing lost records");
}

/// A COMPACTED IMAGE DOES refuse, and there the warning is the whole point.
#[test]
fn a_compacted_image_still_warns_before_it_refuses() {
    let mut last = 0;
    for n in 1.. {
        let mut s = settings::Settings::create().expect("settings");
        // 1500 entries a step: about 6% of the 10 MiB ceiling, fine enough
        // for the last reading before the refusal to land above 700.
        for i in 0..n * 1500 {
            s.set(&format!("k{i}"), &"x".repeat(60));
        }
        match s.to_bytes() {
            Ok(b) => {
                let u = settings::Settings::load(&b).expect("reload").usage();
                assert!(!u.grows, "a compacted image must not claim to grow");
                last = u.used_per_mille();
            }
            Err(_) => break,
        }
        assert!(n < 100, "settings must refuse eventually");
    }
    assert!(last > 700, "the last reading before the refusal was only {last}");
}
