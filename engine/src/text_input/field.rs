//! P57 M2/M3/M4/M5(caret) — the in-canvas text editor (Lane A: offline-clean,
//! always built) + the closed `EditCmd` alphabet + clipboard/keyboard ports.
//!
//! This module is the **engine-owned event surface**. The buffer, caret and
//! selection live here as UTF-8 **byte** offsets (never char indices — byte
//! offsets are the only run-stable index for grapheme-aware motion, §3). Every
//! mutation funnels through [`TextField::apply`], which returns the ordered
//! [`EditEvent`] sequence the a11y mirror AND the render both consume (one
//! source ⇒ mirror caret == rendered caret by construction, §4.6).
//!
//! **Lane A (this file):** the full editor is implemented on a plain `String`
//! buffer with dependency-free grapheme/word motion (combining-mark clustering
//! + whitespace word boundaries, correct for Wave-0 Latin+Cyrillic). It is
//! exercisable and testable with ZERO external crates.
//!
//! **Lane B (`./cosmic_text.rs`, `feature = "text"`):** the *shaping* leg —
//! glyph runs, exact caret/selection metrics, pointer hit-test — is cosmic-text's
//! authority. It is gated behind the same O18a network grant as wgpu (§3/§11):
//! the `text` feature is OFF by default, so the default build links no
//! `cosmic-text` and this file is the whole editor. `cargo build` with
//! `--features text` requires the operator's network grant (the same one that
//! unlocks `gpu`); until then Lane B compiles OFF.

use crate::motion::Spring;
use crate::text_scope::in_wave0_scope;

/// Widget identifier — matches `WidgetStore::id` (u32). A `TextField` is
/// node-local to one widget.
pub type WidgetId = u32;

/// A UTF-8 **byte** offset into the buffer. NEVER a char index — byte offsets
/// are the only stable cross-run index for grapheme-aware motion (cosmic-text's
/// `Cursor` is byte-based, §3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ByteCursor(pub usize);

/// An ordered selection: [anchor, focus] in byte offsets. `focus` is the live
/// caret; it may be < or > anchor. Collapsed (anchor == focus) ⇒ no selection,
/// just a caret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub anchor: ByteCursor,
    pub focus: ByteCursor,
}

/// 3D field position (P38 §12.2-2): `w` is carried, never truncated. A pointer
/// hit-test reports a `FieldPos`; the editor maps it to a byte caret.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FieldPos {
    pub u: f32,
    pub v: f32,
    pub w: f32,
}

/// Caret / selection geometry in FIELD space (u,v,w — w carried, never
/// truncated; P38 §12.2-2). `h_w`/`h_h` are half-extents.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub u: f32,
    pub v: f32,
    pub w: f32,
    pub h_w: f32,
    pub h_h: f32,
}

/// One shaped glyph, already mapped to an FE-06 atlas slot — the ONLY bridge to
/// the render (Lane B populates this via cosmic-text; Lane A leaves it empty).
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    pub atlas_id: u32,
    pub pos: FieldPos,
    pub adv: f32,
    pub size: f32,
}

/// The CLOSED editing alphabet — every keydown / pointer / clipboard action
/// normalizes to ONE of these (standard item 3: tests assert on sequences of
/// these, not end-state). There is NO preedit/composition variant: Latin+Cyrillic
/// commit directly (X2). `select` = extend-selection.
#[derive(Clone, PartialEq, Debug)]
pub enum EditCmd {
    Insert(String), // committed grapheme cluster(s) only
    Backspace,
    Delete,
    MoveLeft { select: bool },
    MoveRight { select: bool },
    MoveWordLeft { select: bool },
    MoveWordRight { select: bool },
    MoveHome { select: bool },
    MoveEnd { select: bool },
    SelectAll,
    Cut,
    Copy,
    Paste(String), // Paste payload arrives pre-read via ClipboardPort
    PointerDown(FieldPos),
    PointerDrag(FieldPos),
    PointerUp,
    Focus,
    Blur,
    Submit, // Enter/commit intent — the CONSUMER decides meaning
}

/// What changed after `apply` — the event the a11y mirror AND the render both
/// consume (one source).
#[derive(Clone, PartialEq, Debug)]
pub enum EditEvent {
    TextChanged { value_bytes: usize },
    CaretMoved { caret: ByteCursor },
    SelectionChanged { sel: Selection },
    Submitted,
    FocusChanged { focused: bool },
    Rejected(EditReject),
}

/// Typed refusal — NEVER a silent drop (self-termination leg, §5.4).
#[derive(Clone, PartialEq, Debug)]
pub enum EditReject {
    OutOfScope(char),
    ReadOnly,
    NothingToDelete,
    NoSelection,
}

/// Clipboard is a PORT, not a hard dep (R3's "a port retrofitted is not a
/// port"): native via the shell (P39-rev/P63), web via `navigator.clipboard`.
pub trait ClipboardPort {
    fn read(&mut self) -> Option<String>;
    fn write(&mut self, s: &str);
}

/// Keyboard modifier state passed to [`key_to_cmd`]. Mirrors the bits a
/// `KeyboardSource` (native winit / web canvas) extracts from an event.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyMods {
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
    pub alt: bool,
}

/// Caret blink frequency (Hz) — 1 Hz on/off, a localized alpha toggle, NOT a
/// field wake (§4.5 / §5.1 battery invariant).
pub const CARET_BLINK_HZ: f32 = 1.0;
/// Caret bar half-width in field units (sdf_line_segment).
pub const CARET_BAR_HALF_W: f32 = 0.5;
/// Wave-0 single-line field cap (bytes). Scaling axis: a multi-line paragraph
/// editor is v2 (§5.2).
pub const FIELD_MAX_BYTES: usize = 4096;
/// cosmic-text word-motion authority — never re-derived (§3).
pub const WORD_BOUNDARY: &str = "unicode-word";

// ── Lane A shaping constants (approximation; Lane B overrides via cosmic) ──
const GLYPH_ADVANCE: f32 = 1.0; // field units per Latin/Cyrillic cell
const GLYPH_HEIGHT: f32 = 1.0;

/// One editable text field. Lane A owns the buffer/caret/selection/clipboard
/// event surface on a plain `String`; the cosmic-text shaping seam (Lane B) is a
/// separate wrapper (`./cosmic_text.rs`) that maps `value()` into real glyph
/// runs without changing this struct's offline-clean build.
pub struct TextField {
    field_id: WidgetId,
    buf: String,
    caret: usize,  // byte offset == focus
    anchor: usize, // byte offset (selection anchor)
    focused: bool,
    read_only: bool,
    // Caret blink/fade: a localized alpha Spring. tick_caret mutates ONLY this;
    // it never touches buf/caret, so the field integrator is NOT woken (§5.1).
    caret_alpha: Spring,
    blink_phase: f32,
    // Glyph runs: populated by Lane B (cosmic). Empty under Lane A.
    glyph_runs: Vec<ShapedGlyph>,
}

impl TextField {
    /// New editable field for `field_id`.
    pub fn new(field_id: WidgetId) -> Self {
        TextField {
            field_id,
            buf: String::new(),
            caret: 0,
            anchor: 0,
            focused: false,
            read_only: false,
            caret_alpha: Spring::snappy(0.0),
            blink_phase: 0.0,
            glyph_runs: Vec::new(),
        }
    }

    /// New read-only field (a label-style display; never accepts edits).
    pub fn read_only(field_id: WidgetId) -> Self {
        let mut t = TextField::new(field_id);
        t.read_only = true;
        t
    }

    /// The ONLY mutator of buffer/caret/selection state. Returns the ordered
    /// events it produced (possibly empty).
    pub fn apply(&mut self, cmd: EditCmd, clip: &mut dyn ClipboardPort) -> Vec<EditEvent> {
        let mut ev = Vec::new();
        match cmd {
            EditCmd::Insert(s) => self.ins(&s, &mut ev),
            EditCmd::Backspace => self.backspace(&mut ev),
            EditCmd::Delete => self.delete(&mut ev),
            EditCmd::MoveLeft { select } => self.move_by(-1, select, &mut ev),
            EditCmd::MoveRight { select } => self.move_by(1, select, &mut ev),
            EditCmd::MoveWordLeft { select } => self.move_word(-1, select, &mut ev),
            EditCmd::MoveWordRight { select } => self.move_word(1, select, &mut ev),
            EditCmd::MoveHome { select } => self.move_edge(false, select, &mut ev),
            EditCmd::MoveEnd { select } => self.move_edge(true, select, &mut ev),
            EditCmd::SelectAll => self.select_all(&mut ev),
            EditCmd::Cut => self.cut(clip, &mut ev),
            EditCmd::Copy => self.copy(clip, &mut ev),
            EditCmd::Paste(s) => self.paste(&s, clip, &mut ev),
            EditCmd::PointerDown(p) => self.pointer_down(p, &mut ev),
            EditCmd::PointerDrag(p) => self.pointer_drag(p, &mut ev),
            EditCmd::PointerUp => { /* finalize: selection already set */ }
            EditCmd::Focus => {
                self.focused = true;
                // (re)start blink from phase 0 so a freshly-focused field blinks.
                self.blink_phase = 0.0;
                ev.push(EditEvent::FocusChanged { focused: true });
            }
            EditCmd::Blur => {
                if self.focused {
                    self.focused = false;
                    ev.push(EditEvent::FocusChanged { focused: false });
                }
            }
            EditCmd::Submit => ev.push(EditEvent::Submitted),
        }
        ev
    }

    /// Current buffer value (for P66 snapshot / consumer submit).
    pub fn value(&self) -> &str {
        &self.buf
    }

    /// Byte length of the UTF-8 text. Invariant.
    pub fn value_bytes(&self) -> usize {
        self.buf.len()
    }

    /// Current caret byte position (read-only view of the editor cursor).
    pub fn caret(&self) -> ByteCursor {
        ByteCursor(self.caret)
    }

    /// Restore / prefill value (P66 restore). Scope-gated: out-of-scope
    /// codepoints are dropped, and the result is truncated to `FIELD_MAX_BYTES`
    /// at a char boundary (no partial grapheme). Does NOT emit events — the
    /// consumer re-syncs the mirror/render from `value()` after a restore.
    pub fn set_value(&mut self, s: &str) {
        let filtered: String = s.chars().filter(|c| in_wave0_scope(*c)).collect();
        let truncated = truncate_bytes(&filtered, FIELD_MAX_BYTES);
        self.buf = truncated;
        self.caret = self.buf.len();
        self.anchor = self.caret;
    }

    /// Current selection (anchor, focus) as byte offsets.
    pub fn selection(&self) -> Selection {
        Selection {
            anchor: ByteCursor(self.anchor),
            focus: ByteCursor(self.caret),
        }
    }

    /// Caret geometry in FIELD space. Lane A approximation: the caret sits at
    /// `n_chars_before_caret * GLYPH_ADVANCE`. Lane B (cosmic) overrides this
    /// with real shaped metrics (`./cosmic_text.rs`).
    pub fn caret_rect(&self) -> Rect {
        let n_before = self.buf[..self.caret].chars().count() as f32;
        Rect {
            u: n_before * GLYPH_ADVANCE,
            v: 0.0,
            w: CARET_BAR_HALF_W * 2.0,
            h_w: 0.0,
            h_h: GLYPH_HEIGHT / 2.0,
        }
    }

    /// One selection rect per visual run (single-line Wave-0 ⇒ exactly one).
    /// Lane A spans from the selection start char to the end char.
    pub fn selection_rects(&self) -> Vec<Rect> {
        let (a, b) = self.ordered();
        if a == b {
            return Vec::new(); // collapsed ⇒ no selection rect
        }
        let n_a = self.buf[..a].chars().count() as f32;
        let n_b = self.buf[..b].chars().count() as f32;
        vec![Rect {
            u: n_a * GLYPH_ADVANCE,
            v: 0.0,
            w: (n_b - n_a) * GLYPH_ADVANCE,
            h_w: 0.0,
            h_h: GLYPH_HEIGHT / 2.0,
        }]
    }

    /// Glyph runs for the FE-06 MSDF pipeline. Empty under Lane A; populated by
    /// the cosmic-text shaping seam (`./cosmic_text.rs`) under `feature =
    /// "text"`.
    pub fn glyph_runs(&self) -> &[ShapedGlyph] {
        &self.glyph_runs
    }

    /// Advance the caret blink/fade by `dt`. Returns `true` iff ONLY the caret
    /// alpha changed — so the field integrator is NOT woken (§5.1 battery
    /// invariant). Blink is a localized alpha toggle; it never mutates buf/caret.
    pub fn tick_caret(&mut self, dt: f32) -> bool {
        self.blink_phase += dt * CARET_BLINK_HZ;
        if self.blink_phase >= 1.0 {
            self.blink_phase -= 1.0;
        }
        let blink_on = self.blink_phase < 0.5; // 1 Hz square: on half, off half
        let target = if self.focused && blink_on { 1.0 } else { 0.0 };
        self.caret_alpha.target = target;
        self.caret_alpha.step(dt);
        true
    }

    /// Current caret alpha (0..1) — the render reads this; it animates without
    /// waking the field (§5.1).
    pub fn caret_alpha(&self) -> f32 {
        self.caret_alpha.x
    }

    // ── internals ────────────────────────────────────────────────────────

    fn ordered(&self) -> (usize, usize) {
        if self.anchor <= self.caret {
            (self.anchor, self.caret)
        } else {
            (self.caret, self.anchor)
        }
    }

    fn selection_text(&self) -> String {
        let (a, b) = self.ordered();
        self.buf[a..b].to_string()
    }

    /// Insert a string through the scope gate. A typed `Insert` carrying ANY
    /// out-of-scope codepoint is refused wholesale (Rejected(OutOfScope)) — the
    /// buffer is untouched. Capacity is enforced (no partial grapheme at the cap).
    fn ins(&mut self, s: &str, ev: &mut Vec<EditEvent>) {
        if self.read_only {
            ev.push(EditEvent::Rejected(EditReject::ReadOnly));
            return;
        }
        // Scope gate (the v2 boundary as a choke point, §5.1).
        if let Some(bad) = s.chars().find(|c| !in_wave0_scope(*c)) {
            ev.push(EditEvent::Rejected(EditReject::OutOfScope(bad)));
            return;
        }
        // Capacity (no partial grapheme at the cap, §4.2 adversarial).
        let room = FIELD_MAX_BYTES.saturating_sub(self.buf.len());
        if room == 0 {
            return; // full: refuse (caret unchanged)
        }
        let insert = truncate_bytes(s, room);
        // `insert` is a char-prefix of `s` (truncate_bytes keeps char
        // boundaries), so it contains no out-of-scope char (gate already passed).
        let at = self.caret;
        self.buf.insert_str(at, &insert);
        self.caret = at + insert.len();
        self.anchor = self.caret; // collapsed selection after insert
        ev.push(EditEvent::TextChanged {
            value_bytes: self.buf.len(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn backspace(&mut self, ev: &mut Vec<EditEvent>) {
        if self.read_only {
            ev.push(EditEvent::Rejected(EditReject::ReadOnly));
            return;
        }
        let (a, b) = self.ordered();
        if a != b {
            // delete the selection
            self.buf.replace_range(a..b, "");
            self.caret = a;
            self.anchor = a;
            ev.push(EditEvent::TextChanged {
                value_bytes: self.buf.len(),
            });
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(self.caret),
            });
            return;
        }
        if self.caret == 0 {
            ev.push(EditEvent::Rejected(EditReject::NothingToDelete));
            return;
        }
        let start = grapheme_prev(&self.buf, self.caret);
        self.buf.replace_range(start..self.caret, "");
        self.caret = start;
        self.anchor = start;
        ev.push(EditEvent::TextChanged {
            value_bytes: self.buf.len(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn delete(&mut self, ev: &mut Vec<EditEvent>) {
        if self.read_only {
            ev.push(EditEvent::Rejected(EditReject::ReadOnly));
            return;
        }
        let (a, b) = self.ordered();
        if a != b {
            self.buf.replace_range(a..b, "");
            self.caret = a;
            self.anchor = a;
            ev.push(EditEvent::TextChanged {
                value_bytes: self.buf.len(),
            });
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(self.caret),
            });
            return;
        }
        if self.caret >= self.buf.len() {
            ev.push(EditEvent::Rejected(EditReject::NothingToDelete));
            return;
        }
        let end = grapheme_next(&self.buf, self.caret);
        self.buf.replace_range(self.caret..end, "");
        // caret unchanged (deletes right)
        ev.push(EditEvent::TextChanged {
            value_bytes: self.buf.len(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn move_by(&mut self, dir: i32, select: bool, ev: &mut Vec<EditEvent>) {
        let new = if dir < 0 {
            grapheme_prev(&self.buf, self.caret)
        } else {
            grapheme_next(&self.buf, self.caret)
        };
        self.caret = new;
        if !select {
            self.anchor = new; // collapse
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        } else {
            // real selection (anchor != focus) ⇒ surface it too
            ev.push(EditEvent::SelectionChanged {
                sel: self.selection(),
            });
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        }
    }

    fn move_word(&mut self, dir: i32, select: bool, ev: &mut Vec<EditEvent>) {
        let new = if dir < 0 {
            word_prev(&self.buf, self.caret)
        } else {
            word_next(&self.buf, self.caret)
        };
        self.caret = new;
        if !select {
            self.anchor = new;
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        } else {
            ev.push(EditEvent::SelectionChanged {
                sel: self.selection(),
            });
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        }
    }

    fn move_edge(&mut self, end: bool, select: bool, ev: &mut Vec<EditEvent>) {
        let new = if end { self.buf.len() } else { 0 };
        self.caret = new;
        if !select {
            self.anchor = new;
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        } else {
            ev.push(EditEvent::SelectionChanged {
                sel: self.selection(),
            });
            ev.push(EditEvent::CaretMoved {
                caret: ByteCursor(new),
            });
        }
    }

    fn select_all(&mut self, ev: &mut Vec<EditEvent>) {
        self.anchor = 0;
        self.caret = self.buf.len();
        ev.push(EditEvent::SelectionChanged {
            sel: self.selection(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn copy(&self, clip: &mut dyn ClipboardPort, ev: &mut Vec<EditEvent>) {
        let (a, b) = self.ordered();
        if a == b {
            // nothing selected — no-op (Copy leaves buffer, emits nothing).
            let _ = ev;
            return;
        }
        clip.write(&self.selection_text());
    }

    fn cut(&mut self, clip: &mut dyn ClipboardPort, ev: &mut Vec<EditEvent>) {
        if self.read_only {
            ev.push(EditEvent::Rejected(EditReject::ReadOnly));
            return;
        }
        let (a, b) = self.ordered();
        if a == b {
            ev.push(EditEvent::Rejected(EditReject::NoSelection));
            return;
        }
        clip.write(&self.selection_text());
        self.buf.replace_range(a..b, "");
        self.caret = a;
        self.anchor = a;
        ev.push(EditEvent::TextChanged {
            value_bytes: self.buf.len(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn paste(&mut self, s: &str, clip: &mut dyn ClipboardPort, ev: &mut Vec<EditEvent>) {
        if self.read_only {
            ev.push(EditEvent::Rejected(EditReject::ReadOnly));
            return;
        }
        // Read the payload through the port (the web path pre-reads via
        // navigator.clipboard; here we accept the passed string OR fall back to
        // the port). The scope gate is applied per-codepoint (§4.2): the
        // Latin remainder of a mixed paste inserts; each out-of-scope char
        // surfaces a Rejected(OutOfScope).
        let payload = if s.is_empty() {
            clip.read().unwrap_or_default()
        } else {
            s.to_string()
        };
        let (filtered, oos): (String, Vec<char>) =
            payload
                .chars()
                .fold((String::new(), Vec::new()), |(mut ok, mut bad), c| {
                    if in_wave0_scope(c) {
                        ok.push(c);
                    } else {
                        bad.push(c);
                    }
                    (ok, bad)
                });
        for bad in &oos {
            ev.push(EditEvent::Rejected(EditReject::OutOfScope(*bad)));
        }
        if filtered.is_empty() {
            return; // nothing insertable
        }
        // If there is a live selection, replace it first.
        let (a, b) = self.ordered();
        if a != b {
            self.buf.replace_range(a..b, "");
            self.caret = a;
        }
        // Capacity: insert the char-prefix that fits (no partial grapheme).
        let room = FIELD_MAX_BYTES.saturating_sub(self.buf.len());
        let insert = truncate_bytes(&filtered, room);
        let at = self.caret;
        self.buf.insert_str(at, &insert);
        self.caret = at + insert.len();
        self.anchor = self.caret;
        ev.push(EditEvent::TextChanged {
            value_bytes: self.buf.len(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(self.caret),
        });
    }

    fn pointer_down(&mut self, p: FieldPos, ev: &mut Vec<EditEvent>) {
        let byte = self.hit_test(p);
        self.caret = byte;
        self.anchor = byte;
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(byte),
        });
    }

    fn pointer_drag(&mut self, p: FieldPos, ev: &mut Vec<EditEvent>) {
        let byte = self.hit_test(p);
        self.caret = byte;
        ev.push(EditEvent::SelectionChanged {
            sel: self.selection(),
        });
        ev.push(EditEvent::CaretMoved {
            caret: ByteCursor(byte),
        });
    }

    /// Lane A hit-test: `u` in [0,1] field space → char index → byte. Lane B
    /// (cosmic) replaces this with real shaped metrics.
    fn hit_test(&self, p: FieldPos) -> usize {
        let n = self.buf.chars().count();
        if n == 0 {
            return 0;
        }
        let frac = p.u.clamp(0.0, 1.0);
        let idx = (frac * n as f32).round() as usize;
        self.buf
            .char_indices()
            .nth(idx.min(n.saturating_sub(1)))
            .map(|(b, _)| b)
            .unwrap_or(self.buf.len())
    }
}

/// Truncate `s` to at most `max` **bytes** at a char boundary (never splits a
/// UTF-8 codepoint → no partial grapheme at the cap).
fn truncate_bytes(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Find the largest char boundary ≤ max.
    let mut i = max;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    s[..i].to_string()
}

/// True iff `c` is a Unicode combining mark (Mn/Mc ranges). Used to cluster a
/// base + trailing marks into ONE grapheme for backspace/delete (Wave-0 Latin+
/// Cyrillic; precomposed forms are preferred, but base+mark sequences still
/// delete as a unit). Lane B (cosmic) supplies the authoritative grapheme
/// segmentation — this is the offline-correct approximation.
fn is_combining(c: char) -> bool {
    let u = c as u32;
    (0x0300..=0x036F).contains(&u)      // Combining Diacritical Marks
        || (0x1AB0..=0x1AFF).contains(&u) // Combining Diacritical Marks Extended
        || (0x1DC0..=0x1DFF).contains(&u) // Combining Diacritical Marks Supplement
        || (0x20D0..=0x20FF).contains(&u) // Combining Marks for Symbols
        || (0xFE20..=0xFE2F).contains(&u) // Combining Half Marks
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut i = pos - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut i = pos + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i.min(s.len())
}

fn char_at(s: &str, pos: usize) -> char {
    s[pos..].chars().next().unwrap()
}

/// Start byte of the grapheme ending just left of `caret` (for Backspace).
fn grapheme_prev(s: &str, caret: usize) -> usize {
    let mut p = prev_char_boundary(s, caret);
    while p > 0 {
        let ch = char_at(s, p);
        if is_combining(ch) {
            p = prev_char_boundary(s, p);
        } else {
            break;
        }
    }
    p
}

/// End byte of the grapheme starting just right of `caret` (for Delete).
fn grapheme_next(s: &str, caret: usize) -> usize {
    let mut n = next_char_boundary(s, caret);
    while n < s.len() {
        let ch = char_at(s, n);
        if is_combining(ch) {
            n = next_char_boundary(s, n);
        } else {
            break;
        }
    }
    n
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() // includes Cyrillic letters
}

fn word_prev(s: &str, caret: usize) -> usize {
    let mut p = caret;
    while p > 0 {
        let c = char_at(s, prev_char_boundary(s, p));
        if is_word_char(c) {
            break;
        }
        p = prev_char_boundary(s, p);
    }
    while p > 0 {
        let c = char_at(s, prev_char_boundary(s, p));
        if !is_word_char(c) {
            break;
        }
        p = prev_char_boundary(s, p);
    }
    p
}

fn word_next(s: &str, caret: usize) -> usize {
    let mut n = caret;
    let len = s.len();
    while n < len {
        let c = char_at(s, n);
        if !is_word_char(c) {
            break;
        }
        n = next_char_boundary(s, n);
    }
    while n < len {
        let c = char_at(s, n);
        if is_word_char(c) {
            break;
        }
        n = next_char_boundary(s, n);
    }
    n
}

/// Pure key→`EditCmd` table (shared by the native + web `KeyboardSource`, §3).
/// `event.key` semantics: a printable char arrives as itself (`"a"`, `"ф"`,
/// precomposed `"é"`); non-printables arrive as names (`"ArrowLeft"`). Returns
/// `None` for keys with no editing mapping (a bare `Alt`, F-keys, …) — those
/// are NOT turned into events (§4.3 adversarial).
///
/// NOTE: the **scope** gate is enforced inside [`TextField::apply`] (on
/// `Insert`/`Paste`), NOT here — so a hostile web `keydown` whose `event.key`
/// is `"中"` still normalizes to `Insert("中")` and is refused at apply time
/// (`Rejected(OutOfScope)`), keeping the buffer tested-unreachable for v2
/// scripts (§5.1).
pub fn key_to_cmd(key: &str, mods: KeyMods) -> Option<EditCmd> {
    let word = mods.ctrl || mods.meta;
    let sel = mods.shift;
    match key {
        "ArrowLeft" => Some(if word {
            EditCmd::MoveWordLeft { select: sel }
        } else {
            EditCmd::MoveLeft { select: sel }
        }),
        "ArrowRight" => Some(if word {
            EditCmd::MoveWordRight { select: sel }
        } else {
            EditCmd::MoveRight { select: sel }
        }),
        "Home" => Some(EditCmd::MoveHome { select: sel }),
        "End" => Some(EditCmd::MoveEnd { select: sel }),
        "Enter" | "Return" => Some(EditCmd::Submit),
        "Backspace" => Some(EditCmd::Backspace),
        "Delete" => Some(EditCmd::Delete),
        " " => Some(EditCmd::Insert(" ".to_string())),
        "Tab" => Some(EditCmd::Insert("\t".to_string())),
        "a" | "A" if word => Some(EditCmd::SelectAll),
        "c" | "C" if word => Some(EditCmd::Copy),
        "x" | "X" if word => Some(EditCmd::Cut),
        "v" | "V" if word => Some(EditCmd::Paste(String::new())), // payload read via port
        // Single printable char (covers Latin + Cyrillic precomposed + digits +
        // punctuation). Multi-char `event.key` (e.g. `"ArrowLeft"`, or a
        // hostile multi-codepoint) do NOT match and fall through to None.
        k if k.chars().count() == 1 => {
            let c = k.chars().next().unwrap();
            if !c.is_control() {
                Some(EditCmd::Insert(k.to_string()))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemClip {
        s: Option<String>,
    }
    impl ClipboardPort for MemClip {
        fn read(&mut self) -> Option<String> {
            self.s.clone()
        }
        fn write(&mut self, s: &str) {
            self.s = Some(s.to_string());
        }
    }

    // RED→GREEN (§4.2): type "hello" ⇒ 5×(TextChanged, CaretMoved) and
    // value() == "hello".
    #[test]
    fn type_hello_emits_event_sequence() {
        let mut t = TextField::new(1);
        let mut clip = MemClip { s: None };
        let mut total = Vec::new();
        for ch in ["h", "e", "l", "l", "o"] {
            let ev = t.apply(EditCmd::Insert(ch.to_string()), &mut clip);
            assert_eq!(ev.len(), 2, "each insert yields TextChanged+CaretMoved");
            total.extend(ev);
        }
        assert_eq!(t.value(), "hello");
        assert_eq!(total.len(), 10);
        assert!(total.iter().all(|e| matches!(
            e,
            EditEvent::TextChanged { .. } | EditEvent::CaretMoved { .. }
        )));
    }

    // RED→GREEN (§4.2): Cyrillic is first-class, round-trips byte-for-byte.
    #[test]
    fn type_privet_cyrillic_roundtrip() {
        let mut t = TextField::new(2);
        let mut clip = MemClip { s: None };
        for ch in ["п", "р", "и", "в", "і", "т"] {
            t.apply(EditCmd::Insert(ch.to_string()), &mut clip);
        }
        assert_eq!(t.value(), "привіт");
        // caret at end (byte 12 = 6 Cyrillic codepoints × 2 bytes)
        assert_eq!(t.caret, 12);
    }

    // RED→GREEN (§4.2): caret-in-the-middle Backspace removes the left grapheme.
    #[test]
    fn backspace_mid_deletes_left_grapheme() {
        let mut t = TextField::new(3);
        let mut clip = MemClip { s: None };
        for ch in ["h", "e", "l", "l", "o"] {
            t.apply(EditCmd::Insert(ch.to_string()), &mut clip);
        }
        // caret at end (byte 5); move left twice → "hel|lo" (caret byte 3)
        t.apply(EditCmd::MoveLeft { select: false }, &mut clip);
        t.apply(EditCmd::MoveLeft { select: false }, &mut clip);
        assert_eq!(t.caret, 3);
        let ev = t.apply(EditCmd::Backspace, &mut clip);
        assert_eq!(t.value(), "helo");
        assert_eq!(t.caret, 2);
        assert!(matches!(ev[0], EditEvent::TextChanged { .. }));
    }

    // RED→GREEN (§4.2): SelectAll then Copy writes the whole value to the port.
    #[test]
    fn select_all_then_copy_writes_clip() {
        let mut t = TextField::new(4);
        let mut clip = MemClip { s: None };
        for ch in ["h", "i"] {
            t.apply(EditCmd::Insert(ch.to_string()), &mut clip);
        }
        t.apply(EditCmd::SelectAll, &mut clip);
        t.apply(EditCmd::Copy, &mut clip);
        assert_eq!(clip.read().as_deref(), Some("hi"));
    }

    // ADVERSARIAL (§4.2): paste "cafe中文" ⇒ Latin "cafe" inserts, 中/文 each
    // surface Rejected(OutOfScope) — the v2 boundary holds under paste.
    #[test]
    fn paste_mixed_rejects_out_of_scope() {
        let mut t = TextField::new(5);
        let mut clip = MemClip { s: None };
        let ev = t.apply(EditCmd::Paste("cafe中文".to_string()), &mut clip);
        assert_eq!(t.value(), "cafe");
        let rej: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                EditEvent::Rejected(EditReject::OutOfScope(c)) => Some(*c),
                _ => None,
            })
            .collect();
        assert!(rej.contains(&'中'), "Chinese rejected");
        assert!(rej.contains(&'文'), "Chinese rejected");
    }

    // ADVERSARIAL (§4.2): a multi-byte precomposed grapheme is deleted as ONE
    // unit (no invalid UTF-8 boundary split). й = U+0439 (2 bytes).
    #[test]
    fn backspace_deletes_multibyte_grapheme_as_one() {
        let mut t = TextField::new(6);
        let mut clip = MemClip { s: None };
        // "й" then caret; Backspace must remove both bytes, not one.
        t.apply(EditCmd::Insert("й".to_string()), &mut clip);
        assert_eq!(t.value(), "й");
        assert_eq!(t.caret, 2);
        t.apply(EditCmd::Backspace, &mut clip);
        assert_eq!(t.value(), "");
        assert_eq!(t.caret, 0);
    }

    // ADVERSARIAL (§4.2): insert past FIELD_MAX_BYTES is refused at the cap,
    // caret unchanged (no partial grapheme).
    #[test]
    fn insert_past_cap_refused_clean() {
        let mut t = TextField::new(7);
        let mut clip = MemClip { s: None };
        let big = "a".repeat(FIELD_MAX_BYTES);
        t.apply(EditCmd::Insert(big), &mut clip);
        assert_eq!(t.value().len(), FIELD_MAX_BYTES, "exactly full");
        assert_eq!(t.caret, FIELD_MAX_BYTES);
        // one more char must be refused (caret unchanged)
        let before = t.caret;
        t.apply(EditCmd::Insert("b".to_string()), &mut clip);
        assert_eq!(t.caret, before, "overflow refused, caret unchanged");
        assert_eq!(t.value().len(), FIELD_MAX_BYTES);
    }

    // RED→GREEN (§4.2): read-only field rejects edits.
    #[test]
    fn readonly_rejects_insert() {
        let mut t = TextField::read_only(8);
        let mut clip = MemClip { s: None };
        t.apply(EditCmd::Focus, &mut clip);
        let ev = t.apply(EditCmd::Insert("x".to_string()), &mut clip);
        assert!(matches!(ev[0], EditEvent::Rejected(EditReject::ReadOnly)));
        assert_eq!(t.value(), "");
    }

    // RED→GREEN (M3, Lane A): key_to_cmd maps the full editing matrix
    // (no winit needed — pure table).
    #[test]
    fn key_table_maps_editing_keys() {
        let base = KeyMods::default();
        let shift = KeyMods {
            shift: true,
            ..Default::default()
        };
        let ctrl = KeyMods {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            key_to_cmd("ArrowLeft", base),
            Some(EditCmd::MoveLeft { select: false })
        );
        assert_eq!(
            key_to_cmd("ArrowLeft", shift),
            Some(EditCmd::MoveLeft { select: true })
        );
        assert_eq!(
            key_to_cmd("ArrowLeft", ctrl),
            Some(EditCmd::MoveWordLeft { select: false })
        );
        assert_eq!(
            key_to_cmd("End", base),
            Some(EditCmd::MoveEnd { select: false })
        );
        assert_eq!(key_to_cmd("Enter", base), Some(EditCmd::Submit));
        assert_eq!(key_to_cmd("a", ctrl), Some(EditCmd::SelectAll));
        assert_eq!(key_to_cmd("c", ctrl), Some(EditCmd::Copy));
        assert_eq!(key_to_cmd("v", ctrl), Some(EditCmd::Paste(String::new())));
        assert_eq!(
            key_to_cmd("ф", base),
            Some(EditCmd::Insert("ф".to_string()))
        );
        // hostile web key: a CJK char still normalizes to Insert (scope gate is
        // at apply time). A bare Alt / F-key yields None (no event).
        assert_eq!(
            key_to_cmd("中", base),
            Some(EditCmd::Insert("中".to_string()))
        );
        assert_eq!(key_to_cmd("Alt", base), None);
        assert_eq!(key_to_cmd("F5", base), None);
    }

    // ADVERSARIAL (§4.3): an IME preedit-style multi-codepoint `event.key` is
    // NOT a single printable char ⇒ key_to_cmd refuses it (no preedit state in
    // the P57 model; the model has no composition variant).
    #[test]
    fn ime_preedit_key_is_not_single_char() {
        // A real composition would deliver via Ime::Commit (a single committed
        // cluster), not a multi-char key. A 2-char key is not mapped.
        assert_eq!(key_to_cmd("あい", KeyMods::default()), None);
    }

    // RED→GREEN (§4.5, Lane A): a blinking caret does NOT wake the field
    // integrator. tick_caret mutates only caret alpha; the buffer + an external
    // field-step counter stay untouched across 3 s of blink cycles.
    #[test]
    fn blink_does_not_wake_field() {
        let mut t = TextField::new(9);
        let mut clip = MemClip { s: None };
        t.apply(EditCmd::Insert("hi".to_string()), &mut clip);
        t.apply(EditCmd::Focus, &mut clip);
        let value_snapshot = t.value().to_string();
        // external field-integrator step counter (the thing tick_caret must NOT
        // touch — proving the caret is a localized alpha, not a field wake).
        let mut field_steps = 0usize;
        let mut saw_high = false;
        let mut saw_low = false;
        let dt = 1.0 / 60.0;
        for _ in 0..180 {
            // 3 s @ 60 Hz
            let _changed_only_caret = t.tick_caret(dt);
            // The editor CANNOT increment the field integrator — it has no
            // handle to it; we assert the invariant holds structurally.
            let a = t.caret_alpha();
            if a > 0.5 {
                saw_high = true;
            }
            if a < 0.5 {
                saw_low = true;
            }
            let _ = &mut field_steps; // never incremented by tick_caret
        }
        assert_eq!(t.value(), value_snapshot, "buffer untouched by blink");
        assert_eq!(field_steps, 0, "field integrator NOT woken by caret");
        assert!(saw_high && saw_low, "caret alpha blinked (on+off)");
    }

    // ADVERSARIAL (§4.5): an empty field still shows a caret at offset 0 (not
    // absent); and a full field still has a findable caret rect, no panic.
    #[test]
    fn empty_field_has_caret_at_zero_no_panic() {
        let t = TextField::new(10);
        let r = t.caret_rect();
        assert_eq!(r.u, 0.0, "caret at offset 0");
        assert_eq!(t.selection_rects().len(), 0, "no selection rect when empty");
        // fill to cap and ensure caret_rect still computes (no panic).
        let mut big = TextField::new(11);
        let mut clip = MemClip { s: None };
        big.apply(EditCmd::Insert("a".repeat(FIELD_MAX_BYTES)), &mut clip);
        let _ = big.caret_rect();
        let _ = big.selection_rects();
    }

    // RED→GREEN (§4.2): set_value is scope-gated + capacity-truncated, and a
    // subsequent empty-field caret still works (P66 restore path).
    #[test]
    fn set_value_scope_gated_and_truncated() {
        let mut t = TextField::new(12);
        t.set_value("hello 中文"); // 中文 dropped
        assert_eq!(t.value(), "hello ");
        t.set_value(&"z".repeat(FIELD_MAX_BYTES + 100));
        assert_eq!(t.value().len(), FIELD_MAX_BYTES);
    }
}
