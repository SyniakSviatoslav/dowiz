use super::*;

fn w(off: usize, v: i64) -> SimOp {
    SimOp::Write { off, cells: vec![v] }
}

fn count(ops: &[SimOp], model: Model) -> (usize, Vec<String>) {
    let mut names = Vec::new();
    let n = each_state(&[0; 256], ops, model, |p, _| names.push(p.to_string()));
    (n, names)
}

/// Two windows (sectors {0,1} then {2}) and the point after the last barrier:
/// 4 + 2 + 1 states with no tears; ordinals run 0.. in order.
#[test]
fn every_subset_of_every_window_is_a_state() {
    let ops = [w(0, 1), w(64, 2), SimOp::Barrier, w(128, 3), SimOp::Barrier];
    let (n, names) = count(&ops, Model::Power(Tear::None));
    assert_eq!(n, 7);
    assert_eq!(names[0], "#0 power loss after 0 barrier(s): new []");
    assert_eq!(names[3], "#3 power loss after 0 barrier(s): new [0, 1]");
    assert_eq!(names[6], "#6 after the last barrier after 2 barrier(s): new []");
}

/// Half tears: each not-new sector, both halves. Window 0: 1+4 + 3 + 3 + 1 = 12; window 1:
/// 3 + 1; and the final point: 17.
#[test]
fn half_tears_add_two_shapes_per_old_sector() {
    let ops = [w(0, 1), w(64, 2), SimOp::Barrier, w(128, 3), SimOp::Barrier];
    assert_eq!(count(&ops, Model::Power(Tear::Half)).0, 17);
    // Cells: 126 shapes per tear. Window 0: (1 + 2*126) + 2*(1 + 126) + 1 = 508; window 1: 128; final 1.
    assert_eq!(count(&ops, Model::Power(Tear::Cells)).0, 508 + 128 + 1);
}

/// The torn state really is part new, part old, at the named cells.
#[test]
fn a_torn_state_holds_the_named_half() {
    let ops = [SimOp::Write { off: 0, cells: vec![5; 64] }];
    let mut seen = 0;
    each_state(&[0; 64], &ops, Model::Power(Tear::Half), |p, img| {
        if let Some(t) = p.torn {
            let new = img.iter().filter(|&&c| c == 5).count();
            assert_eq!(new, 32);
            assert_eq!(img[0] == 5, t.front, "{p}");
            seen += 1;
        }
    });
    assert_eq!(seen, 2);
}

/// Above EXHAUSTIVE sectors: the eight named classes, each with first/last tears, no more.
#[test]
fn a_wide_window_is_the_eight_named_classes() {
    let ops: Vec<SimOp> = (0..8).map(|s| w(s * 64, 1)).collect();
    let (n, names) = count(&ops, Model::Power(Tear::None));
    assert_eq!(n, 8);
    assert!(names.iter().any(|s| s.contains("(all-but-last)")));
    // Torn sectors per class: all 0, empty 2, only-first 2, only-last 2, all-but-first 1,
    // all-but-last 1, even 2, odd 2 = 12, two half shapes each.
    assert_eq!(count(&ops, Model::Power(Tear::Half)).0, 8 + 12 * 2);
}

/// Process crash: the prefixes of the writes, barriers or not.
#[test]
fn a_process_crash_is_every_prefix() {
    let ops = [w(0, 1), SimOp::Barrier, w(1, 2), w(2, 3)];
    let mut imgs = Vec::new();
    let n = each_state(&[0; 4], &ops, Model::Process, |_, img| imgs.push(img.to_vec()));
    assert_eq!(n, 4);
    assert_eq!(imgs[2], vec![1, 2, 0, 0]);
}
