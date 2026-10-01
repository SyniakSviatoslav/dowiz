use super::*;

/// Against the published WCAG figures, not against my own output.
#[test]
fn contrast_matches_the_known_numbers() {
    let white = Rgb::new(255, 255, 255);
    let black = Rgb::new(0, 0, 0);
    assert!((contrast(white, black) - 21.0).abs() < 0.01, "black on white is 21:1");
    assert!((contrast(white, white) - 1.0).abs() < 0.001);
    // #767676 on white is the canonical "exactly AA" grey.
    let aa_grey = Rgb::from_hex("#767676").unwrap();
    let r = contrast(aa_grey, white);
    assert!((4.5..4.6).contains(&r), "#767676 on white should be ~4.54, got {r}");
    // #777777 is the one shade that fails, which is what makes this a real
    // boundary test rather than a round number.
    assert!(contrast(Rgb::from_hex("#777").unwrap(), white) < 4.5);
}

#[test]
fn hex_round_trips() {
    for h in ["#000000", "#ffffff", "#e11d48", "#061b1a", "#d69a3d"] {
        assert_eq!(Rgb::from_hex(h).unwrap().hex(), h);
    }
    assert_eq!(Rgb::from_hex("#fff").unwrap(), Rgb::new(255, 255, 255));
    assert!(Rgb::from_hex("nope").is_none());
    assert!(Rgb::from_hex("#12345").is_none());
}

#[test]
fn hsl_round_trips_closely() {
    for c in [
        Rgb::new(225, 29, 72),
        Rgb::new(214, 154, 61),
        Rgb::new(6, 27, 26),
        Rgb::new(128, 128, 128),
        Rgb::new(0, 0, 0),
    ] {
        let (h, s, l) = c.hsl();
        let back = Rgb::from_hsl(h, s, l);
        let d = |a: u8, b: u8| (a as i32 - b as i32).abs();
        assert!(
            d(back.r, c.r) <= 2 && d(back.g, c.g) <= 2 && d(back.b, c.b) <= 2,
            "{c:?} -> {back:?}"
        );
    }
}

/// The whole point of `ensure_contrast`: a colour that fails must come back
/// passing, and must still be recognisably the same hue.
#[test]
fn a_pale_brand_colour_is_darkened_until_it_is_legible() {
    let white = Rgb::new(255, 255, 255);
    let pale = Rgb::from_hex("#ffd9e3").unwrap(); // a pale pink, ~1.2:1 on white
    assert!(contrast(pale, white) < AA_TEXT);
    let (fixed, moved) = ensure_contrast(pale, white, AA_TEXT);
    assert!(contrast(fixed, white) >= AA_TEXT, "{} is still {}", fixed.hex(), contrast(fixed, white));
    assert!(moved > 0.0);
    // Same hue: it is still their pink.
    let (h0, _, _) = pale.hsl();
    let (h1, _, _) = fixed.hsl();
    assert!((h0 - h1).abs() < 2.0, "hue moved from {h0} to {h1}");
}

#[test]
fn a_colour_that_already_passes_is_left_alone() {
    let white = Rgb::new(255, 255, 255);
    let strong = Rgb::from_hex("#b91c1c").unwrap();
    let (fixed, moved) = ensure_contrast(strong, white, AA_TEXT);
    assert_eq!(fixed, strong);
    assert_eq!(moved, 0.0);
}

/// EVERY derived theme must pass every pair, for any seed. This is the test
/// that makes the feature safe to hand to a restaurant owner with a pastel
/// logo.
#[test]
fn every_theme_passes_every_contrast_pair() {
    let seeds = [
        "#e11d48", "#ffd9e3", "#0d9488", "#fde047", "#1e1b4b", "#000000", "#ffffff",
        "#808080", "#d69a3d", "#7c3aed", "#84cc16", "#0891b2",
    ];
    for s in seeds {
        let seed = Rgb::from_hex(s).unwrap();
        let t = Theme::from_seed(seed);
        for (what, got, want) in t.contrast_report() {
            assert!(
                got >= want,
                "seed {s}: {what} is {got:.2}:1, needs {want}:1 (primary {})",
                t.primary.hex()
            );
        }
    }
}

/// A venue's own colour must be used unchanged when it already works --
/// a theme generator that always "improves" the brand is one nobody trusts.
#[test]
fn a_usable_brand_colour_is_reported_as_unadjusted() {
    let t = Theme::from_seed(Rgb::from_hex("#e11d48").unwrap());
    assert_eq!(t.primary_adjusted_pct, 0);
    assert_eq!(t.primary.hex(), "#e11d48");
    let pale = Theme::from_seed(Rgb::from_hex("#ffd9e3").unwrap());
    assert!(pale.primary_adjusted_pct > 0, "a pastel must be reported as moved");
}

/// The same pixels must always give the same palette. If this ever fails,
/// a venue's colours change between two uploads of one image.
#[test]
fn the_palette_is_deterministic() {
    let px: Vec<Rgb> = (0..500)
        .map(|i| match i % 5 {
            0 | 1 => Rgb::new(225, 29, 72),
            2 => Rgb::new(13, 148, 136),
            3 => Rgb::new(255, 255, 255),
            _ => Rgb::new(20, 20, 20),
        })
        .collect();
    let a = dominant(&px, 4);
    let b = dominant(&px, 4);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.colour, y.colour);
        assert_eq!(x.share_bp, y.share_bp);
    }
}

/// Paper and ink must not win. A photographed menu is mostly white and
/// black, and "your brand colour is white" helps nobody.
#[test]
fn greys_do_not_beat_the_brand_colour() {
    let mut px = vec![Rgb::new(252, 251, 248); 900]; // paper
    px.extend(vec![Rgb::new(18, 18, 20); 80]); // ink
    px.extend(vec![Rgb::new(225, 29, 72); 20]); // the logo
    let top = dominant(&px, 3);
    assert!(!top.is_empty(), "the chromatic colour must be found at all");
    let (_, s, _) = top[0].colour.hsl();
    assert!(s >= 0.18, "the winner must be chromatic, got {:?}", top[0].colour);
    let d = |a: u8, b: u8| (a as i32 - b as i32).abs();
    assert!(
        d(top[0].colour.r, 225) < 20 && d(top[0].colour.g, 29) < 20,
        "expected the logo red, got {:?}",
        top[0].colour
    );
    // The share is measured against ALL pixels, so it honestly reports that
    // only ~2% of the image was this colour.
    assert!(top[0].share_bp < 400, "share should reflect the whole image: {}", top[0].share_bp);
}

#[test]
fn an_empty_image_yields_nothing_rather_than_a_default() {
    assert!(dominant(&[], 4).is_empty());
    // An image with no chromatic pixels at all also yields nothing, so the
    // caller can say "no colours found" rather than invent one.
    assert!(dominant(&vec![Rgb::new(255, 255, 255); 100], 4).is_empty());
}

/// THE PAPER DECIDES THE CORRECTION. sushi-durres chose gold #d4af37 on a
/// near-black #07141c, and for a week its light set carried #8c721e: the
/// accent had been darkened to clear WHITE and nothing walked it back once
/// the venue's own dark ground was known. Gold on near-black clears AA-large
/// several times over, so the venue's colour must be served untouched.
#[test]
fn a_dark_paper_keeps_the_venues_own_accent() {
    let gold = Rgb::from_hex("#d4af37").unwrap();
    let t = Theme::from_brand(
        gold,
        Rgb::from_hex("#f8f5ee").unwrap(),
        Rgb::from_hex("#07141c").unwrap(),
    );
    assert_eq!(t.primary, gold, "served {} on a dark paper", t.primary.hex());
    // Gold needs dark lettering, and the label colour follows the accent
    // that is actually served rather than the one that was replaced.
    assert!(contrast(t.on_primary, t.primary) >= AA_TEXT, "on-primary {} on {}", t.on_primary.hex(), t.primary.hex());
    for (what, got, want) in t.contrast_report() {
        assert!(got >= want, "{what} is {got:.2}:1, needs {want}:1");
    }
}

/// The control: on a WHITE paper the same pale accent still has to move,
/// so the fix above is a change of ground and not a change of policy.
#[test]
fn a_light_paper_still_corrects_a_pale_accent() {
    let pale = Rgb::from_hex("#ffd9e3").unwrap();
    let t = Theme::from_brand(
        pale,
        Rgb::from_hex("#111111").unwrap(),
        Rgb::from_hex("#ffffff").unwrap(),
    );
    assert_ne!(t.primary, pale);
    assert!(contrast(t.primary, t.surface) >= AA_LARGE);
    assert!(contrast(t.on_primary, t.primary) >= AA_TEXT);
}

#[test]
fn the_token_names_match_the_ones_the_surfaces_use() {
    let css = Theme::from_seed(Rgb::from_hex("#e11d48").unwrap()).as_css_tokens();
    for tok in [
        "--brand-primary:",
        "--brand-primary-hover:",
        "--brand-bg:",
        "--brand-surface:",
        "--brand-surface-raised:",
        "--brand-text:",
        "--brand-text-muted:",
        "--brand-border:",
    ] {
        assert!(css.contains(tok), "missing {tok} in {css}");
    }
    let dark = Theme::from_seed(Rgb::from_hex("#e11d48").unwrap()).as_dark_css_tokens();
    assert!(dark.contains("--brand-bg:"));
}
