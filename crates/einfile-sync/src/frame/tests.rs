//! One decoder for a data channel and a stream: small frames, any cut, a break and a resume.

use super::*;
use crate::toys::ToyIds;

fn message(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i % 251) as u8).collect()
}

#[test]
fn a_message_in_small_frames_comes_back() {
    let msg = message(100_000);
    for max_frame in [FRAME_HEAD + 1, 1200, 16 << 10, 1 << 20] {
        if max_frame == FRAME_HEAD + 1 && msg.len() > 1000 {
            continue;
        }
        let fs = frames(&msg, max_frame, 0, &ToyIds).unwrap();
        assert!(fs.iter().all(|f| f.len() <= max_frame), "{max_frame}");
        let mut asm = Assembler::new(1 << 20);
        let mut out = Vec::new();
        for f in &fs {
            out.extend(asm.feed(f, &ToyIds).unwrap());
        }
        assert_eq!(out, vec![msg.clone()], "{max_frame}");
    }
}

/// A STREAM cuts where it likes: one byte at a time, or two messages in one read.
#[test]
fn a_stream_may_cut_anywhere() {
    let (m1, m2) = (message(3000), message(10));
    let mut wire: Vec<u8> = frames(&m1, 1200, 0, &ToyIds).unwrap().concat();
    wire.extend(frames(&m2, 1200, 0, &ToyIds).unwrap().concat());
    let mut asm = Assembler::new(1 << 20);
    let mut out = Vec::new();
    for b in &wire {
        out.extend(asm.feed(&[*b], &ToyIds).unwrap());
    }
    assert_eq!(out, vec![m1.clone(), m2.clone()]);
    let mut asm = Assembler::new(1 << 20);
    assert_eq!(asm.feed(&wire, &ToyIds).unwrap(), vec![m1, m2]);
}

/// A break: the receiver names the byte, the sender goes on from there, and nothing is sent twice.
#[test]
fn a_break_resumes_at_the_first_missing_byte() {
    let msg = message(5000);
    let fs = frames(&msg, 1000, 0, &ToyIds).unwrap();
    let mut asm = Assembler::new(1 << 20);
    for f in &fs[..3] {
        assert!(asm.feed(f, &ToyIds).unwrap().is_empty());
    }
    let (at, _) = asm.resume_at();
    assert_eq!(at, 3 * (1000 - FRAME_HEAD) as u64);
    let rest = frames(&msg, 700, at, &ToyIds).unwrap();
    let mut out = Vec::new();
    for f in &rest {
        out.extend(asm.feed(f, &ToyIds).unwrap());
    }
    assert_eq!(out, vec![msg]);
}

/// A frame past a hole is dropped and the hole stays named; a repeat is dropped without a trace.
#[test]
fn a_hole_stays_named_and_a_repeat_changes_nothing() {
    let msg = message(3000);
    let fs = frames(&msg, 1000, 0, &ToyIds).unwrap();
    let mut asm = Assembler::new(1 << 20);
    asm.feed(&fs[0], &ToyIds).unwrap();
    asm.feed(&fs[0], &ToyIds).unwrap();
    let held = asm.resume_at().0;
    assert_eq!(held, (1000 - FRAME_HEAD) as u64);
    asm.feed(&fs[2], &ToyIds).unwrap();
    assert_eq!(asm.resume_at().0, held, "a hole");
    let mut out = Vec::new();
    for f in &fs[1..] {
        out.extend(asm.feed(f, &ToyIds).unwrap());
    }
    assert_eq!(out, vec![msg]);
}

/// A frame of a newer message replaces a partial one, from its start only.
#[test]
fn a_new_message_replaces_a_partial_one_from_its_start() {
    let (old, new) = (message(3000), vec![7u8; 3000]);
    let (fo, fnew) = (frames(&old, 1000, 0, &ToyIds).unwrap(), frames(&new, 1000, 0, &ToyIds).unwrap());
    let mut asm = Assembler::new(1 << 20);
    asm.feed(&fo[0], &ToyIds).unwrap();
    asm.feed(&fnew[1], &ToyIds).unwrap();
    assert_eq!(asm.resume_at().1, frames(&old, 1000, 0, &ToyIds).unwrap()[0][20..52], "not from its start");
    let mut out = Vec::new();
    for f in &fnew {
        out.extend(asm.feed(f, &ToyIds).unwrap());
    }
    assert_eq!(out, vec![new]);
}

#[test]
fn a_damaged_payload_never_reaches_the_caller() {
    let msg = message(2000);
    let mut fs = frames(&msg, 1000, 0, &ToyIds).unwrap();
    fs[1][FRAME_HEAD + 3] ^= 1;
    let mut asm = Assembler::new(1 << 20);
    asm.feed(&fs[0], &ToyIds).unwrap();
    let last = fs.len() - 1;
    for f in &fs[1..last] {
        asm.feed(f, &ToyIds).unwrap();
    }
    assert_eq!(asm.feed(&fs[last], &ToyIds), Err(SyncError::BadSum));
    assert_eq!(asm.resume_at(), (0, [0u8; 32]), "nothing is kept of it");
}

#[test]
fn the_limits_are_named_before_a_byte_is_kept() {
    let msg = message(2000);
    let f = frames(&msg, 1000, 0, &ToyIds).unwrap().remove(0);
    let mut small = Assembler::new(1999);
    assert_eq!(small.feed(&f, &ToyIds), Err(SyncError::Over { what: "message", got: 2000, max: 1999 }));
    let mut exact = Assembler::new(2000);
    assert!(exact.feed(&f, &ToyIds).is_ok());
    let mut huge = f.clone();
    huge[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut asm = Assembler::new(1 << 20);
    assert!(matches!(asm.feed(&huge, &ToyIds), Err(SyncError::Over { what: "frame", .. })));
    let mut lying = f;
    lying[12..20].copy_from_slice(&1999u64.to_le_bytes());
    let mut asm = Assembler::new(1 << 20);
    assert_eq!(asm.feed(&lying, &ToyIds), Err(SyncError::Mark("frame")), "past its own total");
    assert!(matches!(frames(&msg, FRAME_HEAD, 0, &ToyIds), Err(SyncError::Over { what: "frame head", .. })));
    assert!(frames(&[], 1000, 0, &ToyIds).is_err());
}
