use super::{classify, Command, Speaker};

#[test]
fn a_nan_confidence_is_not_confidence() {
    assert_eq!(classify("cancel 4821", f64::NAN, true, Speaker::Owner), Command::Unclear("not sure I heard that"));
    assert_ne!(classify("cancel 4821", 0.9, true, Speaker::Owner), Command::Unclear("not sure I heard that"));
}
