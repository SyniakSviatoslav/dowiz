//! THE DIRTY FLAG (R-GRAPH D11 part 2): a frame whose inputs did not change is not built or drawn
//! again. The host asks for frames it does not need -- board.js's poll tick asks every POLL_MS, a
//! socket state change asks, a toast timer asks -- and each asked frame laid the whole board out and
//! replayed every command on Canvas2D for a picture identical to the one already on the canvas.
//!
//! A frame's inputs are the board's state and the clock, and nothing else:
//!   * STATE changes only through the host ABI (ffi.rs). There `bd()` MARKS on every mutable access,
//!     so an export added later is dirty by default; only the read-only exports opt out (`quiet()`).
//!     A wiped backing store (resize, a restored context) always arrives through `resize`, which marks.
//!   * THE CLOCK reaches the picture only as ticket ages in whole minutes (cards.rs `age_min`, and
//!     `heat_of` from the same minutes), so `clock_key` folds those ages: a frame one second later
//!     with the same minutes is the same picture, and the frame on which any age turns is drawn.
//! A skipped frame leaves the command buffer and the scene as the last drawn frame left them, so
//! `frame_hash`, the hit-test and the tour read the picture that is on the canvas.
use super::model::age_min;
use super::Board;
use crate::ui::Ui;

pub struct Dirty {
    on: bool,
    clock: u64,
    /// Frames skipped since start (tests and the host's debugging read it).
    pub skipped: u32,
}

impl Dirty {
    pub const fn new() -> Dirty {
        Dirty { on: true, clock: 0, skipped: 0 }
    }
    /// The state changed: the next frame is drawn.
    pub fn mark(&mut self) {
        self.on = true;
    }
    /// Must the frame at `b.now_ms` be drawn? Consumes the mark.
    pub fn due(&mut self, b: &Board) -> bool {
        let k = b.clock_key();
        let due = self.on || k != self.clock;
        self.on = false;
        self.clock = k;
        due
    }
}

impl Board {
    /// FNV-1a over every ticket's age in whole minutes at `now_ms` (the clock's only way into a frame).
    pub fn clock_key(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for t in crate::head(&self.data.tickets, self.data.nt) {
            h ^= age_min(t.start_ms, self.now_ms) as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }
}

/// What `ffi::frame` does: build the frame only when it is due. Some(command words) when drawn,
/// None when skipped (the host replays nothing and the canvas keeps the last frame).
pub fn frame(d: &mut Dirty, b: &mut Board, ui: &mut Ui) -> Option<u32> {
    if !d.due(b) {
        d.skipped += 1;
        return None;
    }
    b.draw(ui);
    Some(ui.cmd.len as u32)
}

#[cfg(test)]
mod tests {
    use super::Dirty;
    use crate::board::testrig::Rig;

    #[test]
    fn an_unchanged_board_is_drawn_once() {
        let mut r = Rig::new(390, 844).signed(true, true);
        let mut d = Dirty::new();
        let first = r.frame_gated(&mut d).expect("the first frame is drawn");
        assert!(first > 0 && r.scene.len() > 0, "first frame drew {first} words");
        let h = r.cmd.hash();
        assert_eq!(r.frame_gated(&mut d), None, "the same state drew a second frame");
        r.b.now_ms += 1_000; // the same minute for every ticket
        assert_eq!(r.frame_gated(&mut d), None, "a second later, no age moved, yet it drew");
        assert_eq!(d.skipped, 2);
        assert_eq!(r.cmd.hash(), h, "a skip must leave the last frame's commands in place");
    }

    #[test]
    fn a_state_change_or_a_turned_minute_is_drawn() {
        let mut r = Rig::new(390, 844).signed(true, true);
        let mut d = Dirty::new();
        r.frame_gated(&mut d).unwrap();
        d.mark(); // what every mutating export does (ffi.rs `bd()`)
        r.b.wheel(40);
        assert!(r.frame_gated(&mut d).is_some(), "a marked change was skipped");
        assert_eq!(r.frame_gated(&mut d), None);
        // The fixture's tickets started on whole minutes before NOW: +60 s turns every age.
        r.b.now_ms += 60_000;
        assert!(r.frame_gated(&mut d).is_some(), "an age turned a minute and the frame was skipped");
        assert_eq!(r.frame_gated(&mut d), None);
    }
}
