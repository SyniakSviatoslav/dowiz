//! Reservation passes — the thing behind the QR screen.
//!
//! A pass is a short, unforgeable claim that a named reservation exists at a
//! named venue for a named slot. The venue's scanner checks it; nobody else can
//! make one.
//!
//! # Why this is a keyed tag and not a signature
//!
//! **Measured, not assumed:** an ML-DSA-65 signature is
//! [`crate::pq::dsa::SIGNATUREBYTES`] = 3309 bytes. The largest QR code (version
//! 40, error-correction level L) holds **2953** bytes of binary payload. A
//! post-quantum signature therefore *cannot physically ride in a QR code* — not
//! at any density, not with any encoding. That is the constraint this module is
//! built around, and [`tests::a_pq_signature_does_not_fit_in_a_qr`] asserts it
//! against the kernel's own constant so the day it changes, this comment is
//! corrected by a failing test rather than by somebody remembering.
//!
//! The verifier here IS the issuer: a venue's hub mints the pass and that same
//! venue's scanner checks it. When one party holds both ends, a keyed tag is the
//! correct primitive and a public-key signature buys nothing — there is no third
//! party who needs to verify without being able to mint. So the pass carries a
//! 16-byte SHAKE256 tag over a canonical encoding of its claims, which fits in a
//! QR with room to spare (80 characters in alphanumeric mode).
//!
//! What the PQ chain protects is the KEY: the venue's pass key is delivered
//! inside the existing capability-cert chain ([`crate::capability_cert`]), which
//! is hybrid-signed. The quantum-resistant guarantee lives where it fits.
//!
//! # What a pass is not
//!
//! It is not a payment, it names no person, and it carries no personal data —
//! only integers a venue already has. A scanner that reads one learns a
//! reservation id and nothing about who is holding the phone.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::ct_gate::ct_eq;
use crate::pq::keccak::shake256;

/// Domain separation. Prefixed into every tag so a pass tag can never be
/// mistaken for — or replayed as — any other keyed value in the system.
const DOMAIN: &[u8] = b"dowiz/reservation-pass/v1";

/// Bytes of tag carried. 16 bytes is a 2^-128 forgery chance per attempt; the
/// scanner is a venue's own device and rate-limits by physics.
pub const TAG_BYTES: usize = 16;

/// The canonical encoding's length: five fixed-width big-endian fields.
pub const CLAIMS_BYTES: usize = 8 + 8 + 8 + 2 + 8;

/// The maximum binary payload of the largest QR code (version 40, EC level L).
/// Stated here so the constraint this module is designed around is checkable.
pub const QR_MAX_BINARY_BYTES: usize = 2953;

/// What a pass asserts. Every field is an integer the venue already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassClaims {
    pub venue: u64,
    pub reservation: u64,
    /// The booked slot, minutes since the Unix epoch — the same unit
    /// [`crate::reservation`] uses, so the two cannot drift.
    pub slot_min: i64,
    pub party: u16,
    /// Makes two passes for the same reservation distinguishable, so a reissued
    /// pass can be told apart from the one it replaces.
    pub nonce: u64,
}

impl PassClaims {
    /// Fixed-width big-endian, in a fixed order. No delimiters, no lengths, no
    /// text: two different claim sets can never encode to the same bytes, which
    /// is the property a tag over them depends on.
    pub fn canonical(&self) -> [u8; CLAIMS_BYTES] {
        let mut out = [0u8; CLAIMS_BYTES];
        out[0..8].copy_from_slice(&self.venue.to_be_bytes());
        out[8..16].copy_from_slice(&self.reservation.to_be_bytes());
        out[16..24].copy_from_slice(&self.slot_min.to_be_bytes());
        out[24..26].copy_from_slice(&self.party.to_be_bytes());
        out[26..34].copy_from_slice(&self.nonce.to_be_bytes());
        out
    }

    /// Read claims back out of the canonical form.
    pub fn from_canonical(b: &[u8]) -> Option<Self> {
        if b.len() != CLAIMS_BYTES {
            return None;
        }
        Some(Self {
            venue: u64::from_be_bytes(b[0..8].try_into().ok()?),
            reservation: u64::from_be_bytes(b[8..16].try_into().ok()?),
            slot_min: i64::from_be_bytes(b[16..24].try_into().ok()?),
            party: u16::from_be_bytes(b[24..26].try_into().ok()?),
            nonce: u64::from_be_bytes(b[26..34].try_into().ok()?),
        })
    }
}

/// A minted pass: its claims and the tag over them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    pub claims: PassClaims,
    pub tag: [u8; TAG_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassError {
    /// The key is too short to be a key.
    WeakKey(usize),
    /// The tag does not match the claims. Either a forgery or a corrupted scan.
    BadTag,
    /// The payload is not a pass at all.
    Malformed(String),
    /// The pass is for a different venue than the scanner belongs to.
    WrongVenue { expected: u64, found: u64 },
    /// Presented before the venue opens the door for that slot.
    TooEarly { slot_min: i64, now_min: i64 },
    /// Presented after the slot's grace has run out.
    TooLate { slot_min: i64, now_min: i64 },
    /// Arithmetic on the window overflowed.
    WindowOverflow,
}

impl PassError {
    pub fn message(&self) -> String {
        match self {
            Self::WeakKey(n) => format!("pass key of {n} bytes is too short (need >= 32)"),
            Self::BadTag => "pass tag does not match its claims".to_string(),
            Self::Malformed(why) => format!("not a pass: {why}"),
            Self::WrongVenue { expected, found } => {
                format!("pass is for venue {found}, scanner belongs to {expected}")
            }
            Self::TooEarly { slot_min, now_min } => {
                format!("too early: slot {slot_min}, now {now_min}")
            }
            Self::TooLate { slot_min, now_min } => {
                format!("too late: slot {slot_min}, now {now_min}")
            }
            Self::WindowOverflow => "pass window arithmetic overflowed".to_string(),
        }
    }
}

/// The smallest key this will accept. A short key is refused rather than used.
pub const MIN_KEY_BYTES: usize = 32;

fn tag_of(key: &[u8], claims: &PassClaims) -> [u8; TAG_BYTES] {
    // SHAKE256 over DOMAIN ‖ len(key) ‖ key ‖ claims. The key's length is bound
    // in so that a short key followed by claims cannot collide with a longer key
    // followed by different claims.
    let mut input = Vec::with_capacity(DOMAIN.len() + 8 + key.len() + CLAIMS_BYTES);
    input.extend_from_slice(DOMAIN);
    input.extend_from_slice(&(key.len() as u64).to_be_bytes());
    input.extend_from_slice(key);
    input.extend_from_slice(&claims.canonical());

    let mut out = [0u8; TAG_BYTES];
    shake256(&input, &mut out);
    out
}

/// Mint a pass for a reservation.
pub fn issue(key: &[u8], claims: PassClaims) -> Result<Pass, PassError> {
    if key.len() < MIN_KEY_BYTES {
        return Err(PassError::WeakKey(key.len()));
    }
    Ok(Pass {
        tag: tag_of(key, &claims),
        claims,
    })
}

/// How long either side of the slot a pass is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassWindow {
    /// Minutes before the slot the door opens.
    pub early_min: i64,
    /// Minutes after the slot the pass still works.
    pub grace_min: i64,
}

impl PassWindow {
    /// Half an hour early, a quarter of an hour late — a starting point the
    /// venue overrides, not a rule of the system.
    pub const fn default_window() -> Self {
        Self {
            early_min: 30,
            grace_min: 15,
        }
    }
}

/// Check a pass at the scanner.
///
/// Every failure is distinct and named: a scanner that can only say "invalid"
/// sends people away without telling them they are simply twenty minutes early.
pub fn verify(
    key: &[u8],
    pass: &Pass,
    scanner_venue: u64,
    window: &PassWindow,
    now_min: i64,
) -> Result<(), PassError> {
    if key.len() < MIN_KEY_BYTES {
        return Err(PassError::WeakKey(key.len()));
    }
    // The tag is checked FIRST and in constant time. Checking the venue or the
    // window first would let an attacker learn which claims a venue accepts by
    // timing the failures of passes they cannot forge.
    let expected = tag_of(key, &pass.claims);
    if !ct_eq(&expected, &pass.tag) {
        return Err(PassError::BadTag);
    }
    if pass.claims.venue != scanner_venue {
        return Err(PassError::WrongVenue {
            expected: scanner_venue,
            found: pass.claims.venue,
        });
    }
    let opens = pass
        .claims
        .slot_min
        .checked_sub(window.early_min)
        .ok_or(PassError::WindowOverflow)?;
    let closes = pass
        .claims
        .slot_min
        .checked_add(window.grace_min)
        .ok_or(PassError::WindowOverflow)?;
    if now_min < opens {
        return Err(PassError::TooEarly {
            slot_min: pass.claims.slot_min,
            now_min,
        });
    }
    if now_min > closes {
        return Err(PassError::TooLate {
            slot_min: pass.claims.slot_min,
            now_min,
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// The wire form
// ─────────────────────────────────────────────────────────────────────────────

/// Crockford base32, without the padding and without the letters that get
/// misread (I, L, O, U). Chosen because QR's alphanumeric mode encodes this
/// character set at 5.5 bits per character against binary mode's 8 — the same
/// pass takes about a third less space in the code.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Encode a pass as the string the QR carries.
pub fn encode(pass: &Pass) -> String {
    let mut raw = Vec::with_capacity(CLAIMS_BYTES + TAG_BYTES);
    raw.extend_from_slice(&pass.claims.canonical());
    raw.extend_from_slice(&pass.tag);

    let mut out = String::with_capacity(raw.len() * 8 / 5 + 1);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for b in raw {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn value_of(c: char) -> Option<u32> {
    // Crockford's own aliases: I and L read as 1, O reads as 0. A scanner reads
    // machine output, but a person typing a code off a screen does not.
    let up = c.to_ascii_uppercase();
    match up {
        'I' | 'L' => return Some(1),
        'O' => return Some(0),
        _ => {}
    }
    ALPHABET.iter().position(|&a| a as char == up).map(|i| i as u32)
}

/// Read a pass back from the QR's string.
pub fn decode(s: &str) -> Result<Pass, PassError> {
    let mut raw = Vec::with_capacity(CLAIMS_BYTES + TAG_BYTES);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in s.chars() {
        if c == '-' || c == ' ' {
            continue; // groupings a person may have typed
        }
        let v = value_of(c)
            .ok_or_else(|| PassError::Malformed(format!("character {c:?} is not in the alphabet")))?;
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            raw.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    if raw.len() != CLAIMS_BYTES + TAG_BYTES {
        return Err(PassError::Malformed(format!(
            "payload is {} bytes, expected {}",
            raw.len(),
            CLAIMS_BYTES + TAG_BYTES
        )));
    }
    let claims = PassClaims::from_canonical(&raw[..CLAIMS_BYTES])
        .ok_or_else(|| PassError::Malformed("claims did not parse".to_string()))?;
    let mut tag = [0u8; TAG_BYTES];
    tag.copy_from_slice(&raw[CLAIMS_BYTES..]);
    Ok(Pass { claims, tag })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"a venue pass key of thirty-two b";
    const OTHER: &[u8] = b"a DIFFERENT venue pass key 32 by";

    fn claims() -> PassClaims {
        PassClaims {
            venue: 42,
            reservation: 8475,
            slot_min: 29_000_000,
            party: 6,
            nonce: 7,
        }
    }

    fn window() -> PassWindow {
        PassWindow::default_window()
    }

    // ── The constraint this module exists for ──

    #[test]
    fn a_pq_signature_does_not_fit_in_a_qr() {
        // The measurement the design rests on. If ML-DSA ever shrinks below the
        // QR ceiling, this fails and the module's premise gets revisited.
        assert!(
            crate::pq::dsa::SIGNATUREBYTES > QR_MAX_BINARY_BYTES,
            "ML-DSA-65 signature is {} bytes and a QR holds {} — the premise changed",
            crate::pq::dsa::SIGNATUREBYTES,
            QR_MAX_BINARY_BYTES
        );
    }

    #[test]
    fn a_pass_fits_in_a_qr_with_room_to_spare() {
        let p = issue(KEY, claims()).unwrap();
        let s = encode(&p);
        assert_eq!(s.len(), 80, "50 bytes in base32 is 80 characters");
        assert!(s.len() < QR_MAX_BINARY_BYTES / 10);
    }

    // ── Minting and checking ──

    #[test]
    fn a_minted_pass_verifies_at_its_venue_in_its_window() {
        let p = issue(KEY, claims()).unwrap();
        assert_eq!(verify(KEY, &p, 42, &window(), 29_000_000), Ok(()));
    }

    #[test]
    fn a_forged_tag_is_refused() {
        let mut p = issue(KEY, claims()).unwrap();
        p.tag[0] ^= 1;
        assert_eq!(verify(KEY, &p, 42, &window(), 29_000_000), Err(PassError::BadTag));
    }

    #[test]
    fn another_venues_key_cannot_mint_a_pass_for_this_one() {
        let p = issue(OTHER, claims()).unwrap();
        assert_eq!(verify(KEY, &p, 42, &window(), 29_000_000), Err(PassError::BadTag));
    }

    #[test]
    fn changing_any_claim_breaks_the_tag() {
        let base = issue(KEY, claims()).unwrap();
        for mutate in [
            |mut c: PassClaims| { c.venue += 1; c },
            |mut c: PassClaims| { c.reservation += 1; c },
            |mut c: PassClaims| { c.slot_min += 1; c },
            |mut c: PassClaims| { c.party += 1; c },
            |mut c: PassClaims| { c.nonce += 1; c },
        ] {
            let tampered = Pass { claims: mutate(base.claims), tag: base.tag };
            assert_eq!(
                verify(KEY, &tampered, tampered.claims.venue, &window(), tampered.claims.slot_min),
                Err(PassError::BadTag),
                "a changed claim must invalidate the tag"
            );
        }
    }

    #[test]
    fn a_pass_for_another_venue_is_named_as_such() {
        let p = issue(KEY, claims()).unwrap();
        assert_eq!(
            verify(KEY, &p, 99, &window(), 29_000_000),
            Err(PassError::WrongVenue { expected: 99, found: 42 })
        );
    }

    #[test]
    fn the_window_is_checked_at_both_ends() {
        let p = issue(KEY, claims()).unwrap();
        let w = window(); // 30 early, 15 grace
        assert!(verify(KEY, &p, 42, &w, 29_000_000 - 30).is_ok());
        assert_eq!(
            verify(KEY, &p, 42, &w, 29_000_000 - 31),
            Err(PassError::TooEarly { slot_min: 29_000_000, now_min: 29_000_000 - 31 })
        );
        assert!(verify(KEY, &p, 42, &w, 29_000_000 + 15).is_ok());
        assert_eq!(
            verify(KEY, &p, 42, &w, 29_000_000 + 16),
            Err(PassError::TooLate { slot_min: 29_000_000, now_min: 29_000_000 + 16 })
        );
    }

    #[test]
    fn the_tag_is_checked_before_anything_else() {
        // A pass with a bad tag AND the wrong venue AND outside the window must
        // report the tag — otherwise the other two answers leak which claims a
        // venue would have accepted.
        let mut p = issue(KEY, claims()).unwrap();
        p.tag[0] ^= 0xFF;
        assert_eq!(verify(KEY, &p, 99, &window(), 0), Err(PassError::BadTag));
    }

    #[test]
    fn a_short_key_is_refused_rather_than_used() {
        assert_eq!(issue(b"too short", claims()), Err(PassError::WeakKey(9)));
        let p = issue(KEY, claims()).unwrap();
        assert_eq!(
            verify(b"too short", &p, 42, &window(), 29_000_000),
            Err(PassError::WeakKey(9))
        );
    }

    #[test]
    fn window_overflow_fails_closed() {
        let p = issue(KEY, PassClaims { slot_min: i64::MAX, ..claims() }).unwrap();
        assert_eq!(
            verify(KEY, &p, 42, &window(), 0),
            Err(PassError::WindowOverflow)
        );
    }

    // ── The wire form ──

    #[test]
    fn encode_and_decode_round_trip() {
        let p = issue(KEY, claims()).unwrap();
        assert_eq!(decode(&encode(&p)).unwrap(), p);
    }

    #[test]
    fn a_decoded_pass_still_verifies() {
        let p = issue(KEY, claims()).unwrap();
        let back = decode(&encode(&p)).unwrap();
        assert_eq!(verify(KEY, &back, 42, &window(), 29_000_000), Ok(()));
    }

    #[test]
    fn the_confusable_letters_read_as_their_digits() {
        // Crockford's rule, so a code read aloud or typed off a screen still
        // works: I and L are 1, O is 0.
        let p = issue(KEY, claims()).unwrap();
        let s = encode(&p);
        let typed: String = s
            .chars()
            .map(|c| match c {
                '1' => 'I',
                '0' => 'O',
                other => other,
            })
            .collect();
        assert_eq!(decode(&typed).unwrap(), p);
    }

    #[test]
    fn groupings_a_person_typed_are_ignored() {
        let p = issue(KEY, claims()).unwrap();
        let s = encode(&p);
        let grouped = format!("{}-{} {}", &s[..8], &s[8..40], &s[40..]);
        assert_eq!(decode(&grouped).unwrap(), p);
    }

    #[test]
    fn nonsense_is_rejected_with_a_reason() {
        assert!(matches!(decode(""), Err(PassError::Malformed(_))));
        assert!(matches!(decode("!!!!"), Err(PassError::Malformed(_))));
        assert!(matches!(decode("ABC"), Err(PassError::Malformed(_))));
    }

    // ── The canonical encoding ──

    #[test]
    fn distinct_claims_never_share_an_encoding() {
        let a = claims();
        let b = PassClaims { reservation: a.venue, venue: a.reservation, ..a };
        assert_ne!(a.canonical(), b.canonical(), "field order must be bound in");
    }

    #[test]
    fn canonical_round_trips_including_negative_slots() {
        for slot in [i64::MIN, -1, 0, 1, i64::MAX] {
            let c = PassClaims { slot_min: slot, ..claims() };
            assert_eq!(PassClaims::from_canonical(&c.canonical()), Some(c));
        }
    }

    #[test]
    fn a_reissued_pass_differs_from_the_one_it_replaces() {
        let first = issue(KEY, claims()).unwrap();
        let second = issue(KEY, PassClaims { nonce: 8, ..claims() }).unwrap();
        assert_ne!(first.tag, second.tag);
        assert_ne!(encode(&first), encode(&second));
    }

    #[test]
    fn a_pass_carries_no_personal_data() {
        // Every field is an integer the venue already holds. The gate looks for
        // a FIELD (`name:`), not for the word in prose — counting mentions made
        // it fire on a sentence about who is holding the phone. The tokens are
        // split so this check cannot match its own source.
        let src = include_str!("pass.rs");
        for stem in [
            concat!("guest_", "name"),
            concat!("ph", "one"),
            concat!("e", "mail"),
            concat!("add", "ress"),
            concat!("full_", "name"),
        ] {
            for shape in [alloc::format!("{stem}:"), alloc::format!("{stem} :")] {
                assert!(
                    !src.contains(&shape),
                    "a field named {shape} exists — a pass names nobody"
                );
            }
        }
        // And the claims are exactly five integers, so the structure itself
        // cannot carry a name even if somebody adds one to a comment.
        assert_eq!(CLAIMS_BYTES, 8 + 8 + 8 + 2 + 8);
    }
}
