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

mod theme;
pub use theme::Theme;

#[cfg(test)]
mod tests;
