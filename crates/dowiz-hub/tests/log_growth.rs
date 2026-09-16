use dowiz_hub::{EventKind, Hub};

/// THE ORDER LOG MUST NOT REFUSE AN ORDER IT HAS ROOM FOR. It refused order
/// 2450 with fifty-six cells free: the record fitted and the tip update did
/// not, and only the record's failure grew the image. A venue stopped taking
/// orders mid-service with an arena message.
#[test]
fn the_order_log_grows_instead_of_refusing() {
    let mut h = Hub::create_sized(64 * 1024).expect("hub");
    for i in 0..3000 {
        let u = h.usage();
        h.append(EventKind::Placed, &format!("o{i}"), r#"{"status":"new"}"#, i as u64, [0u8; 32])
            .unwrap_or_else(|e| panic!(
                "order {i} refused with {} cells free of {}: {e:?}",
                u.capacity_cells - u.used_cells, u.capacity_cells));
    }
    assert_eq!(h.len(), 3000, "every order must be in the log");
    // And the history is intact rather than a heap of orphans.
    assert_eq!(h.events().len(), 3000, "the chain lost records while growing");
}
