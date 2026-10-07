//! WIDGETS: the few drawing shapes the board is made of, each drawing into the command buffer AND
//! registering its node in the scene in one call, so a thing that is drawn is always a thing that
//! can be found and tapped (and one that is not drawn cannot be tapped).
//!
//! Colours are the design system's tokens (lib/tokens.css `--ui-*`, light and dark), as RGBA.

use crate::cmd::Cmd;
use crate::geom::Rect;
use crate::lang::Lang;
use crate::scene::{Act, Scene};
use crate::text::{fit, Measure, Widths};

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: u32,
    pub surface: u32,
    pub surface2: u32,
    pub fg: u32,
    pub muted: u32,
    pub line: u32,
    pub accent: u32,
    pub on_accent: u32,
    pub success: u32,
    pub warning: u32,
    pub danger: u32,
    pub on_tone: u32,
    pub on_danger: u32,
}

pub const LIGHT: Palette = Palette {
    bg: 0xf5efe5ff, surface: 0xffffffff, surface2: 0xf5efe5ff, fg: 0x061b1aff, muted: 0x4a5a58ff,
    line: 0xd9d2c5ff, accent: 0xd69a3dff, on_accent: 0x061b1aff, success: 0x059669ff,
    warning: 0xd97706ff, danger: 0xdc2626ff, on_tone: 0x061b1aff, on_danger: 0xffffffff,
};
pub const DARK: Palette = Palette {
    bg: 0x131517ff, surface: 0x1b1e20ff, surface2: 0x25282bff, fg: 0xf3f1ecff, muted: 0xa8aaa6ff,
    line: 0x33373aff, accent: 0xd69a3dff, on_accent: 0x061b1aff, success: 0x059669ff,
    warning: 0xd97706ff, danger: 0xdc2626ff, on_tone: 0x061b1aff, on_danger: 0xffffffff,
};

/// The smallest tappable side, CSS px (tap-size gate; WCAG 2.5.5).
pub const TAP: i32 = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// The one main action on a card: accent fill.
    Primary,
    /// A tone-filled action (Ready = success).
    Tone(u32),
    /// A quiet button: surface2 fill.
    Plain,
    /// A filter chip; `true` = selected.
    Chip(bool),
}

pub struct Ui<'a> {
    pub cmd: &'a mut Cmd,
    pub scene: &'a mut Scene,
    pub widths: &'a mut Widths,
    pub host: &'a mut dyn Measure,
    pub pal: Palette,
    pub lang: Lang,
}

impl Ui<'_> {
    pub fn width(&mut self, s: &str, px: i32, weight: i32) -> i32 {
        self.widths.get(self.host, s, px, weight)
    }

    /// Text at (x, y) (top of the line box); returns its width.
    pub fn text(&mut self, s: &str, x: i32, y: i32, px: i32, weight: i32, rgba: u32) -> i32 {
        let w = self.width(s, px, weight);
        self.cmd.text(s, x, y, px, weight, rgba);
        w
    }

    /// One line inside `r`, vertically centred, cut with an ellipsis when too wide.
    /// `align`: 0 left, 1 right, 2 centre.
    pub fn line(&mut self, s: &str, r: Rect, px: i32, weight: i32, rgba: u32, align: i32) {
        let (head, cut) = fit(self.widths, self.host, s, px, weight, r.w);
        let mut w = self.width(head, px, weight);
        let ell = if cut { self.width("…", px, weight) } else { 0 };
        w += ell;
        let x = match align {
            1 => r.right() - w,
            2 => r.x + (r.w - w) / 2,
            _ => r.x,
        };
        let y = r.y + (r.h - line_h(px)) / 2;
        let hw = self.text(head, x, y, px, weight, rgba);
        if cut {
            self.cmd.text("…", x + hw, y, px, weight, rgba);
        }
    }

    pub fn fill(&mut self, r: Rect, radius: i32, rgba: u32) {
        self.cmd.rect(r.x, r.y, r.w, r.h, radius, rgba, 0);
    }

    /// A button: drawn, and registered as a node. Returns the node id.
    pub fn button(&mut self, parent: u16, r: Rect, label: &str, tour: &'static str, act: Act, look: Look) -> u16 {
        let (bg, fg, weight) = match look {
            Look::Primary => (self.pal.accent, self.pal.on_accent, 700),
            Look::Tone(c) => (c, if c == self.pal.danger { self.pal.on_danger } else { self.pal.on_tone }, 700),
            Look::Plain => (self.pal.surface2, self.pal.fg, 600),
            Look::Chip(true) => (self.pal.fg, self.pal.bg, 700),
            Look::Chip(false) => (self.pal.surface2, self.pal.fg, 500),
        };
        let radius = if matches!(look, Look::Chip(_)) { r.h / 2 } else { 12 };
        self.fill(r, radius, bg);
        if matches!(look, Look::Plain | Look::Chip(false)) {
            self.cmd.stroke(r.x, r.y, r.w, r.h, radius, self.pal.line, 1);
        }
        let px = if r.h >= 52 { 18 } else { 15 };
        self.line(label, r.inset(8), px, weight, fg, 2);
        self.scene.push(parent, tour, r, act)
    }
}

/// The height Canvas2D's 'top' baseline line box takes for a font size (integer, 1.25 x).
pub const fn line_h(px: i32) -> i32 {
    px + px / 4
}

/// A label composed from pieces without an allocator ("#AB12 · Table 4 · 12 min"). Too long is cut
/// on a character boundary; the command buffer copies the result, so a stack Buf is safe to draw.
pub struct Buf<const N: usize> {
    b: [u8; N],
    n: usize,
}

impl<const N: usize> Buf<N> {
    pub const fn new() -> Self {
        Buf { b: [0; N], n: 0 }
    }
    pub fn push(&mut self, s: &str) -> &mut Self {
        let mut k = s.len().min(N - self.n);
        while k > 0 && !s.is_char_boundary(k) {
            k -= 1;
        }
        self.b[self.n..self.n + k].copy_from_slice(&s.as_bytes()[..k]);
        self.n += k;
        self
    }
    pub fn num(&mut self, v: u64) -> &mut Self {
        let mut d = [0u8; 20];
        let s = crate::itoa(v, &mut d);
        self.push(s)
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.b[..self.n]).unwrap_or("")
    }
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

/// Greedy word wrap of `s` inside `r`, each line centred when `center`; returns the height used.
pub fn para(ui: &mut Ui, s: &str, r: Rect, px: i32, weight: i32, rgba: u32, center: bool) -> i32 {
    let lh = line_h(px) + 4;
    let space = ui.width(" ", px, weight);
    let mut y = r.y;
    let mut line: Buf<256> = Buf::new();
    let mut lw = 0;
    for word in s.split(' ').filter(|w| !w.is_empty()) {
        let ww = ui.width(word, px, weight);
        if !line.is_empty() && lw + space + ww > r.w {
            ui.line(line.as_str(), Rect::new(r.x, y, r.w, lh), px, weight, rgba, if center { 2 } else { 0 });
            y += lh;
            line = Buf::new();
            lw = 0;
        }
        if !line.is_empty() {
            line.push(" ");
            lw += space;
        }
        line.push(word);
        lw += ww;
    }
    if !line.is_empty() {
        ui.line(line.as_str(), Rect::new(r.x, y, r.w, lh), px, weight, rgba, if center { 2 } else { 0 });
        y += lh;
    }
    y - r.y
}
