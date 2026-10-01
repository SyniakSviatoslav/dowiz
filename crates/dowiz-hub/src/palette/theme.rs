//! THE TOKEN SET: one venue's complete, contrast-checked theme, light and dark,
//! derived from a seed colour or from a brand's accent, ink and paper.

use super::{contrast, ensure_contrast, Rgb, AA_LARGE, AA_TEXT};

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
        //
        // FROM THE SEED, NOT FROM `t.primary`. `from_seed` has already walked
        // the accent darker to clear a WHITE ground, and `ensure_contrast` only
        // ever fixes a FAILURE -- so on a DARK paper the darkened accent passed
        // and stayed darkened, and there was nothing to walk it back. A venue
        // whose gold is #d4af37 was served #8c721e on its own near-black page,
        // where the real gold clears AA-large several times over. Starting from
        // the accent the venue actually chose makes the correction depend on
        // the ground it is correcting for, which is what the comment above
        // always claimed.
        let (on_surface, _) = ensure_contrast(accent, t.surface, AA_LARGE);
        t.primary = on_surface;
        // `on_primary` was chosen against the OLD primary, so it is re-derived:
        // a brighter accent can need black where the darker one needed white.
        // White or near-black, whichever a label can actually be read in. The
        // dark set already derives its accent from the seed this way (see
        // `dark_primary`); this is the light set catching up.
        let white = Rgb { r: 255, g: 255, b: 255 };
        let near_black = Rgb { r: 17, g: 17, b: 17 };
        t.on_primary = if contrast(white, t.primary) >= contrast(near_black, t.primary) {
            white
        } else {
            near_black
        };
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
