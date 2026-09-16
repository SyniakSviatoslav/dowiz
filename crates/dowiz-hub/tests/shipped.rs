/// The tokens a surface should ship with before any venue sets a theme.
///
/// Printed from the real derivation rather than typed by hand: values typed by
/// hand are how a storefront ends up shipping a palette nobody chose, which is
/// exactly what happened -- its default was a pink that appears nowhere else in
/// the product.
#[test]
fn print_shipped_tokens() {
    let b = dowiz_hub::brand::Brand::shipped();
    let t = b.theme();
    println!("LIGHT {}", t.as_css_tokens());
    println!("DARK  {}", t.as_dark_css_tokens());
    for (pair, got, want) in t.contrast_report() {
        assert!(got >= want, "{pair} {got:.2} < {want}");
    }
}
