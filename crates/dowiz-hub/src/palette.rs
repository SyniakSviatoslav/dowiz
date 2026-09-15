//! A venue's colours, taken from a picture it already has.
//!
//! WHAT THIS IS FOR. A restaurant has a logo, or a photographed menu, or a
//! signboard. It does not have a design system. This turns the first into the
//! second: dominant colours out of the image, then a complete token set derived
//! from them and CHECKED against WCAG before it is allowed to exist.
//!
//! THERE IS NO IMAGE DECODER HERE, and that is deliberate rather than a
//! limitation. PNG and JPEG decoders are outside native-spa-server's
//! zero-dep allowlist, and the browser already contains excellent ones. So the
//! admin pane draws the image to a canvas, downsamples it, and posts the pixels;
//! this does the colour mathematics. The split also puts the part that MUST be
//! authoritative — contrast — on the server, where a client cannot skip it.
//!
//! CONTRAST IS ENFORCED, NOT REPORTED. A palette that fails WCAG AA is not
//! returned with a warning attached; the derivation walks the colour until it
//! passes, and says how far it had to walk. A theme nobody can read is not a
//! theme, and "the owner chose it" is not a defence to a customer who cannot
//! see the price.
//!
//! FLOATS LIVE HERE AND NOWHERE NEAR MONEY. Relative luminance needs a 2.4
//! power; MANIFESTO C2 bans floats on the kernel's decision path, which this is
//! not. Nothing computed here is ever compared for equality or stored as a
//! quantity — it becomes a hex string.

/// A colour, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn from_hex(s: &str) -> Option<Rgb> {
        let h = s.trim().trim_start_matches('#');
        let h = match h.len() {
            3 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
            6 => h.to_string(),
            _ => return None,
        };
        let v = u32::from_str_radix(&h, 16).ok()?;
        Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    /// sRGB relative luminance (WCAG 2.x §relative luminance).
    pub fn luminance(self) -> f64 {
        fn lin(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// HSL, with hue in degrees and saturation/lightness in 0..1.
    pub fn hsl(self) -> (f64, f64, f64) {
        let (r, g, b) = (self.r as f64 / 255.0, self.g as f64 / 255.0, self.b as f64 / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        if (max - min).abs() < f64::EPSILON {
            return (0.0, 0.0, l);
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        ((h + 360.0) % 360.0, s, l)
    }

    pub fn from_hsl(h: f64, s: f64, l: f64) -> Rgb {
        let h = ((h % 360.0) + 360.0) % 360.0;
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match h as u32 / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let q = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Rgb::new(q(r), q(g), q(b))
    }
}

/// WCAG 2.x contrast ratio, 1.0 to 21.0.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// WCAG AA for normal text.
pub const AA_TEXT: f64 = 4.5;
/// WCAG AA for large text and for UI component boundaries.
pub const AA_LARGE: f64 = 3.0;

/// Move a colour's lightness until it clears `target` against `against`.
///
/// Hue and saturation are PRESERVED: the venue's colour stays their colour, it
/// just becomes one that can be read on. Darkening and lightening are both
/// tried and the nearer result wins, so a mid-tone brand does not always end up
/// black.
///
/// Returns the colour and how far it moved, so the caller can tell the owner
/// their colour was adjusted rather than quietly substituting one.
pub fn ensure_contrast(colour: Rgb, against: Rgb, target: f64) -> (Rgb, f64) {
    if contrast(colour, against) >= target {
        return (colour, 0.0);
    }
    let (h, s, l) = colour.hsl();
    let mut best: Option<(Rgb, f64)> = None;
    // One step per percent of lightness: fine enough that the shift is not
    // visible as a jump, coarse enough to terminate.
    for step in 1..=100 {
        let d = step as f64 / 100.0;
        for cand_l in [l - d, l + d] {
            if !(0.0..=1.0).contains(&cand_l) {
                continue;
            }
            let cand = Rgb::from_hsl(h, s, cand_l);
            if contrast(cand, against) >= target {
                let moved = (cand_l - l).abs();
                if best.as_ref().is_none_or(|(_, m)| moved < *m) {
                    best = Some((cand, moved));
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    // Black or white always clears any achievable target against the other, so
    // this only fires when `against` is itself a mid-tone -- and then the honest
    // answer is the extreme rather than a colour that fails.
    best.unwrap_or_else(|| {
        let fallback = if against.luminance() > 0.5 {
            Rgb::new(0, 0, 0)
        } else {
            Rgb::new(255, 255, 255)
        };
        (fallback, 1.0)
    })
}

/// A colour found in the image, and how much of it there was.
#[derive(Debug, Clone, Copy)]
pub struct Swatch {
    pub colour: Rgb,
    /// Share of sampled pixels, in ten-thousandths. Integer so it can be
    /// reported without a float crossing the wire.
    pub share_bp: u32,
}

/// Dominant colours, most common first.
///
/// Buckets at 5 bits per channel (32 levels) rather than clustering: k-means on
/// a phone photo gives different answers on different runs unless it is seeded,
/// and a venue's colours changing between two uploads of the SAME image would
/// be indefensible. Bucketing is deterministic by construction.
///
/// Near-black, near-white and near-grey pixels are excluded from the ranking:
/// a photograph of a menu is mostly paper and ink, and "your brand colour is
/// white" is not a useful answer. They are still counted in the total, so the
/// reported shares stay honest about how much of the image was chromatic.
pub fn dominant(pixels: &[Rgb], want: usize) -> Vec<Swatch> {
    if pixels.is_empty() {
        return Vec::new();
    }
    const LEVELS: u32 = 32;
    let mut counts: Vec<(u32, u32)> = Vec::new(); // (bucket key, count)
    for p in pixels {
        let (_, s, l) = p.hsl();
        // Chromatic enough to be a brand colour, and not so dark or light that
        // its hue is meaningless.
        if s < 0.18 || !(0.10..=0.92).contains(&l) {
            continue;
        }
        let q = |c: u8| (c as u32 * LEVELS / 256).min(LEVELS - 1);
        let key = q(p.r) << 10 | q(p.g) << 5 | q(p.b);
        match counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, n)) => *n += 1,
            None => counts.push((key, 1)),
        }
    }
    // Sort by count, then by key: ties must break the same way every time, or
    // the same image yields two different palettes.
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let total = pixels.len() as u64;
    counts
        .into_iter()
        .take(want)
        .map(|(key, n)| {
            // Back to the middle of the bucket, so the colour returned is
            // representative rather than the bucket's floor.
            let unq = |v: u32| ((v * 256 / LEVELS) + (128 / LEVELS)).min(255) as u8;
            Swatch {
                colour: Rgb::new(unq(key >> 10 & 31), unq(key >> 5 & 31), unq(key & 31)),
                share_bp: ((n as u64 * 10_000) / total.max(1)) as u32,
            }
        })
        .collect()
}

/// A complete, checked token set for one venue.
#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Rgb,
    pub primary_hover: Rgb,
    /// Text ON the primary colour.
    pub on_primary: Rgb,
    pub bg: Rgb,
    pub surface: Rgb,
    pub surface_raised: Rgb,
    pub text: Rgb,
    pub text_muted: Rgb,
    pub border: Rgb,
    /// The dark-mode half. A venue theme that only exists in light mode is half
    /// a theme, and the design rules here forbid a screen without dark mode.
    pub dark_bg: Rgb,
    pub dark_surface: Rgb,
    pub dark_surface_raised: Rgb,
    pub dark_text: Rgb,
    pub dark_text_muted: Rgb,
    pub dark_border: Rgb,
    pub dark_primary: Rgb,
    /// How far the primary had to move to become legible, in lightness percent.
    /// Zero means the venue's colour was used exactly as found.
    pub primary_adjusted_pct: u32,
}

impl Theme {
    /// Derive a theme from one seed colour.
    pub fn from_seed(seed: Rgb) -> Theme {
        let (h, s, _) = seed.hsl();
        let white = Rgb::new(255, 255, 255);

        // The primary is a BUTTON FILL with white text on it, so it must clear
        // AA against white -- not merely be pretty.
        let (primary, moved) = ensure_contrast(seed, white, AA_TEXT);
        let (ph, ps, pl) = primary.hsl();
        let primary_hover = Rgb::from_hsl(ph, ps, (pl - 0.06).max(0.0));

        // Neutrals carry a trace of the brand hue rather than being pure grey:
        // a pure mid-grey reads as unconsidered, a hue-biased one reads as
        // chosen. The bias is small enough not to look tinted.
        let bg = Rgb::from_hsl(h, (s * 0.12).min(0.06), 0.985);
        let surface = white;
        let surface_raised = Rgb::from_hsl(h, (s * 0.10).min(0.05), 0.965);
        let text = Rgb::from_hsl(h, (s * 0.15).min(0.10), 0.12);
        let border = Rgb::from_hsl(h, (s * 0.15).min(0.08), 0.89);

        // Muted text must still clear AA against the surface it sits on. This is
        // the token that fails most often in hand-made themes, because "muted"
        // is chosen by eye and eyes adapt.
        let (text_muted, _) =
            ensure_contrast(Rgb::from_hsl(h, (s * 0.20).min(0.12), 0.45), surface, AA_TEXT);

        // Dark mode is DESIGNED, not inverted. Inverting gives a washed accent
        // on a muddy ground; the accent is lightened so it still reads on dark.
        let dark_bg = Rgb::from_hsl(h, (s * 0.18).min(0.10), 0.07);
        let dark_surface = Rgb::from_hsl(h, (s * 0.16).min(0.09), 0.11);
        let dark_surface_raised = Rgb::from_hsl(h, (s * 0.16).min(0.09), 0.15);
        let dark_text = Rgb::from_hsl(h, (s * 0.10).min(0.05), 0.95);
        let (dark_text_muted, _) =
            ensure_contrast(Rgb::from_hsl(h, (s * 0.18).min(0.12), 0.62), dark_surface, AA_TEXT);
        let dark_border = Rgb::from_hsl(h, (s * 0.18).min(0.10), 0.22);
        let (dark_primary, _) = ensure_contrast(seed, dark_surface, AA_LARGE);

        Theme {
            primary,
            primary_hover,
            on_primary: white,
            bg,
            surface,
            surface_raised,
            text,
            text_muted,
            border,
            dark_bg,
            dark_surface,
            dark_surface_raised,
            dark_text,
            dark_text_muted,
            dark_border,
            dark_primary,
            primary_adjusted_pct: (moved * 100.0).round() as u32,
        }
    }

    /// Derive a theme from the three COLOUR tokens a venue owns.
    ///
    /// `from_seed` is this with the ink and the paper left at their derived
    /// defaults, and it stays because a colour picked out of a photograph is
    /// one colour and should not have to invent two more.
    ///
    /// EVERY ONE IS CORRECTED, NOT OBEYED. The accent moves until white text on
    /// it clears AA; the ink moves until it clears AA on the paper the venue
    /// actually chose, which is the pair that fails in hand-made themes because
    /// "dark enough" is judged against whatever ground the designer happened to
    /// be looking at. The owner's colour is used exactly when it works, and the
    /// nearest one that does when it does not.
    ///
    /// The paper decides the surfaces: a venue that picks a warm cream gets
    /// cards slightly lighter than it and a raised layer slightly darker, in
    /// its own hue, rather than white cards floating on cream.
    pub fn from_brand(accent: Rgb, ink: Rgb, paper: Rgb) -> Theme {
        let mut t = Theme::from_seed(accent);
        let (ph, ps, pl) = paper.hsl();

        // A dark paper is a deliberate choice and the surfaces have to go the
        // other way: lighter than the ground rather than darker, or every card
        // disappears into it.
        let dark_paper = pl < 0.5;
        t.bg = paper;
        t.surface = Rgb::from_hsl(ph, ps, if dark_paper { (pl + 0.05).min(1.0) } else { (pl + 0.015).min(1.0) });
        t.surface_raised =
            Rgb::from_hsl(ph, ps, if dark_paper { (pl + 0.10).min(1.0) } else { (pl - 0.02).max(0.0) });
        t.border = Rgb::from_hsl(ph, ps, if dark_paper { (pl + 0.16).min(1.0) } else { (pl - 0.10).max(0.0) });

        let (corrected_ink, _) = ensure_contrast(ink, t.surface, AA_TEXT);
        t.text = corrected_ink;
        // Muted is derived FROM the corrected ink rather than from the hue, so
        // it stays a quieter version of the text the venue chose instead of a
        // grey that happens to pass.
        let (ih, is_, il) = corrected_ink.hsl();
        let muted_l = if dark_paper { (il - 0.30).max(0.0) } else { (il + 0.32).min(1.0) };
        let (muted, _) = ensure_contrast(Rgb::from_hsl(ih, is_, muted_l), t.surface, AA_TEXT);
        t.text_muted = muted;

        // The accent has to clear AA-large on the venue's own surface too, not
        // only on white: a pale gold on a cream page is invisible even though
        // it passed against a ground this venue does not have.
        let (on_surface, _) = ensure_contrast(t.primary, t.surface, AA_LARGE);
        t.primary = on_surface;
        let (ah, as_, al) = t.primary.hsl();
        t.primary_hover = Rgb::from_hsl(ah, as_, (al - 0.06).max(0.0));
        t
    }

    /// Every contrast pair that must hold, with its measured ratio.
    ///
    /// Returned rather than merely checked, so the owner sees the numbers and so
    /// a regression here is visible in a test's output instead of being a
    /// boolean that flipped.
    pub fn contrast_report(&self) -> Vec<(&'static str, f64, f64)> {
        vec![
            ("text on surface", contrast(self.text, self.surface), AA_TEXT),
            ("text on bg", contrast(self.text, self.bg), AA_TEXT),
            ("muted on surface", contrast(self.text_muted, self.surface), AA_TEXT),
            ("on-primary on primary", contrast(self.on_primary, self.primary), AA_TEXT),
            ("border on surface", contrast(self.border, self.surface), 1.2),
            ("dark text on dark surface", contrast(self.dark_text, self.dark_surface), AA_TEXT),
            ("dark muted on dark surface", contrast(self.dark_text_muted, self.dark_surface), AA_TEXT),
            ("dark primary on dark surface", contrast(self.dark_primary, self.dark_surface), AA_LARGE),
        ]
    }

    /// The CSS custom properties, named exactly as the three surfaces already
    /// spell them. A theme that does not use the existing token names would
    /// require every stylesheet to change, which is how a design system stops
    /// being one.
    pub fn as_css_tokens(&self) -> String {
        format!(
            "--brand-primary:{};--brand-primary-hover:{};--brand-on-primary:{};\
             --brand-bg:{};--brand-surface:{};--brand-surface-raised:{};\
             --brand-text:{};--brand-text-muted:{};--brand-border:{}",
            self.primary.hex(),
            self.primary_hover.hex(),
            self.on_primary.hex(),
            self.bg.hex(),
            self.surface.hex(),
            self.surface_raised.hex(),
            self.text.hex(),
            self.text_muted.hex(),
            self.border.hex()
        )
    }

    pub fn as_dark_css_tokens(&self) -> String {
        format!(
            "--brand-primary:{};--brand-bg:{};--brand-surface:{};--brand-surface-raised:{};\
             --brand-text:{};--brand-text-muted:{};--brand-border:{}",
            self.dark_primary.hex(),
            self.dark_bg.hex(),
            self.dark_surface.hex(),
            self.dark_surface_raised.hex(),
            self.dark_text.hex(),
            self.dark_text_muted.hex(),
            self.dark_border.hex()
        )
    }
}

#[cfg(test)]
mod tests {
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
}
