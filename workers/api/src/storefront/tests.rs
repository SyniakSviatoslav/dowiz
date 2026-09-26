use super::menu_cache_control;

/// The console's read-back of its own save may not be kept by the browser:
/// the owner saw the old price after "Saved" (QA walk Q1).
#[test]
fn a_fresh_menu_read_is_never_stored_by_the_browser() {
    assert_eq!(menu_cache_control(true), "no-store");
}

/// The positive twin: a customer's read keeps the thirty-second window the
/// edge cache is built around.
#[test]
fn a_customer_menu_read_keeps_the_thirty_second_window() {
    let cc = menu_cache_control(false);
    assert!(cc.starts_with("public"), "{cc}");
    assert!(cc.contains("max-age=30"), "{cc}");
}
