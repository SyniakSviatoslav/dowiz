//! THE COMMAND BUFFER: what the loader replays on Canvas2D, one i32 word stream per frame.
//!
//! Layout (words): RECT  [1, x, y, w, h, radius, rgba, grad_rgba]      (grad 0 = flat)
//!                 TEXT  [2, ptr, len, x, y, px, weight, rgba]          (ptr/len: UTF-8 in wasm memory)
//!                 CLIP  [3, x, y, w, h]   UNCLIP [4]
//!                 STROKE[5, x, y, w, h, radius, rgba, line_width]
//! Text is drawn with textBaseline 'top' at (x, y). EVERY string is COPIED into the buffer's own
//! byte arena (`strs`), so a run composed in a stack buffer is still there when the loader replays
//! the frame after `frame()` returned -- a pointer into a dead stack frame would draw garbage.
//! The loader's `replay` in loader.js is the one reader; a change here is a change there in the
//! same commit.
//!
//! THE HASH is FNV-1a over the CONTENT: every word, except that a TEXT's pointer and length are
//! replaced by its bytes. Two frames that draw the same thing hash the same even when a string
//! moved in the arena -- which is what the context-loss gate compares after a restore.

pub const OP_RECT: i32 = 1;
pub const OP_TEXT: i32 = 2;
pub const OP_CLIP: i32 = 3;
pub const OP_UNCLIP: i32 = 4;
pub const OP_STROKE: i32 = 5;

/// Words per frame. The busiest board measured in tests (64 tickets, all on screen at 1920 px)
/// uses well under half; past this a frame is cut and `overflow` counts the words lost.
pub const CAP: usize = 24_576;
/// Bytes of text per frame.
pub const STRS: usize = 32_768;

const FNV_OFF: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

pub struct Cmd {
    pub words: [i32; CAP],
    pub len: usize,
    pub overflow: u32,
    strs: [u8; STRS],
    slen: usize,
    hash: u32,
    #[cfg(test)]
    pub texts: Vec<String>,
}

impl Cmd {
    pub const fn new() -> Cmd {
        Cmd {
            words: [0; CAP],
            len: 0,
            overflow: 0,
            strs: [0; STRS],
            slen: 0,
            // Zero, not FNV_OFF: an all-zero `static` costs the module nothing (`clear` sets it).
            hash: 0,
            #[cfg(test)]
            texts: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.overflow = 0;
        self.slen = 0;
        self.hash = FNV_OFF;
        #[cfg(test)]
        self.texts.clear();
    }

    /// The content hash of everything pushed since `clear`.
    pub fn hash(&self) -> u32 {
        self.hash
    }

    fn mix_bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.hash ^= x as u32;
            self.hash = self.hash.wrapping_mul(FNV_PRIME);
        }
    }
    fn mix(&mut self, w: i32) {
        self.mix_bytes(&w.to_le_bytes());
    }

    fn push(&mut self, ws: &[i32]) -> bool {
        if self.len + ws.len() > CAP {
            self.overflow += ws.len() as u32;
            return false;
        }
        self.words[self.len..self.len + ws.len()].copy_from_slice(ws);
        self.len += ws.len();
        true
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, radius: i32, rgba: u32, grad: u32) {
        let ws = [OP_RECT, x, y, w, h, radius, rgba as i32, grad as i32];
        if self.push(&ws) {
            for w in ws {
                self.mix(w);
            }
        }
    }

    pub fn stroke(&mut self, x: i32, y: i32, w: i32, h: i32, radius: i32, rgba: u32, lw: i32) {
        let ws = [OP_STROKE, x, y, w, h, radius, rgba as i32, lw];
        if self.push(&ws) {
            for w in ws {
                self.mix(w);
            }
        }
    }

    pub fn text(&mut self, s: &str, x: i32, y: i32, px: i32, weight: i32, rgba: u32) {
        if s.is_empty() {
            return;
        }
        if self.slen + s.len() > STRS || self.len + 8 > CAP {
            self.overflow += 8;
            return;
        }
        let at = self.slen;
        self.strs[at..at + s.len()].copy_from_slice(s.as_bytes());
        self.slen += s.len();
        let ptr = self.strs[at..].as_ptr() as usize as i32;
        let ws = [OP_TEXT, ptr, s.len() as i32, x, y, px, weight, rgba as i32];
        if self.push(&ws) {
            self.mix(OP_TEXT);
            self.mix_bytes(s.as_bytes());
            for w in &ws[3..] {
                self.mix(*w);
            }
            #[cfg(test)]
            self.texts.push(s.to_string());
        }
    }

    pub fn clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let ws = [OP_CLIP, x, y, w, h];
        if self.push(&ws) {
            for w in ws {
                self.mix(w);
            }
        }
    }

    pub fn unclip(&mut self) {
        if self.push(&[OP_UNCLIP]) {
            self.mix(OP_UNCLIP);
        }
    }
}
