//! A TEXT FIELD's state. Rust keeps the value and draws it; the HOST owns the keyboard.
//!
//! Where the browser has EditContext (Chromium 121+, Chrome on Android) the canvas itself receives
//! text and IME composition -- zero elements. Elsewhere (Safari, Firefox) the loader adds ONE
//! transient `<input>` while a field is focused and removes it on blur (operator 2026-10-06; the
//! dom-count gate counts it, it is never hidden from the count). Either way the host hands the
//! whole current value back through `set`, so this is the one copy the board reads.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Email,
    Password,
    Code,
    /// Free text (a reason).
    Text,
}

impl Kind {
    /// What the host tells the keyboard/autofill (`type`, `autocomplete` on the transient input).
    pub const fn host_code(self) -> i32 {
        match self {
            Kind::Email => 1,
            Kind::Password => 2,
            Kind::Code => 3,
            Kind::Text => 4,
        }
    }
}

pub const MAX: usize = 120;

#[derive(Clone, Copy)]
pub struct Field {
    pub kind: Kind,
    bytes: [u8; MAX],
    len: usize,
}

impl Field {
    pub const fn new(kind: Kind) -> Field {
        Field { kind, bytes: [0; MAX], len: 0 }
    }

    pub fn value(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Replace the value. Too long is cut on a character boundary; bytes that are not UTF-8 are
    /// refused (the value is left as it was) -- the host always sends what TextEncoder made.
    pub fn set(&mut self, b: &[u8]) -> bool {
        let Ok(s) = core::str::from_utf8(b) else { return false };
        let mut n = s.len().min(MAX);
        while n > 0 && !s.is_char_boundary(n) {
            n -= 1;
        }
        self.bytes[..n].copy_from_slice(&s.as_bytes()[..n]);
        self.len = n;
        true
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// What is DRAWN: a password as one bullet per character, written into `buf`.
    pub fn shown<'a>(&'a self, buf: &'a mut [u8; MAX * 3]) -> &'a str {
        if self.kind != Kind::Password {
            return self.value();
        }
        let mut n = 0;
        for _ in self.value().chars() {
            if n + 3 > buf.len() {
                break;
            }
            buf[n..n + 3].copy_from_slice("•".as_bytes());
            n += 3;
        }
        core::str::from_utf8(&buf[..n]).unwrap_or("")
    }
}
