//! ITEM 1 — Speculative/optimistic execution + rollback vs verify-before-persist.
//!
//! Operator claim: "it will be faster." Prior rejection: verify ≈0.1-1ms ≪ mesh RTT
//! 10-100ms makes speculation pointless. This bench gets REAL numbers on THIS host for a
//! realistic small, LOCAL (same-process) order transition — no network involved.
//!
//! We measure three things on identical input:
//!   A) verify-before-persist  : decide(transition+money) -> content-address(sha3) -> append
//!   B) speculative-then-maybe-rollback : snapshot -> apply mutation -> decide -> commit|rollback
//!   V) the verify-only slice   : just decide(), the thing speculation tries to "hide"
//! Then we sweep an injected verify-latency knob (a spin loop standing in for a signature
//! verify / network floor) to find where — if ever — speculation could pay for itself.

use std::time::Instant;

// ---- real content-addressing cost: SHA3-256, copied verbatim from kernel/src/event_log.rs ----
fn sha3_256(input: &[u8]) -> [u8; 32] {
    const RC: [u64; 24] = [
        0x0000000000000001,0x0000000000008082,0x800000000000808a,0x8000000080008000,
        0x000000000000808b,0x0000000080000001,0x8000000080008081,0x8000000000008009,
        0x000000000000008a,0x0000000000000088,0x0000000080008009,0x000000008000000a,
        0x000000008000808b,0x800000000000008b,0x8000000000008089,0x8000000000008003,
        0x8000000000008002,0x8000000000000080,0x000000000000800a,0x800000008000000a,
        0x8000000080008081,0x8000000000008080,0x0000000080000001,0x8000000080008008,
    ];
    const R: [[u32; 5]; 5] = [
        [0,36,3,41,18],[1,44,10,45,2],[62,6,43,15,61],[28,55,25,21,56],[27,20,39,8,14],
    ];
    fn keccak_f(s: &mut [u64; 25]) {
        for r in 0..24 {
            let mut c = [0u64; 5];
            for x in 0..5 { c[x] = s[x]^s[x+5]^s[x+10]^s[x+15]^s[x+20]; }
            let mut d = [0u64; 5];
            for x in 0..5 { d[x] = c[(x+4)%5] ^ c[(x+1)%5].rotate_left(1); }
            for x in 0..5 { for y in 0..5 { s[x+5*y] ^= d[x]; } }
            let mut b = [0u64; 25];
            for x in 0..5 { for y in 0..5 {
                b[y+5*((2*x+3*y)%5)] = s[x+5*y].rotate_left(R[x][y]);
            }}
            for x in 0..5 { for y in 0..5 {
                let idx = x+5*y;
                s[idx] = b[idx] ^ ((!b[((x+1)%5)+5*y]) & b[((x+2)%5)+5*y]);
            }}
            s[0] ^= RC[r];
        }
    }
    const RATE: usize = 136;
    let mut msg = input.to_vec();
    msg.push(0x06);
    while msg.len() % RATE != 0 { msg.push(0); }
    *msg.last_mut().unwrap() |= 0x80;
    let mut state = [0u64; 25];
    for block in msg.chunks_exact(RATE) {
        for j in 0..(RATE/8) {
            state[j] ^= u64::from_le_bytes(block[j*8..j*8+8].try_into().unwrap());
        }
        keccak_f(&mut state);
    }
    let mut out = [0u8; 32];
    for i in 0..4 { out[i*8..i*8+8].copy_from_slice(&state[i].to_le_bytes()); }
    out
}

// ---- realistic small, local state: one order ----
#[derive(Clone, Copy)]
struct OrderState { status: u8, subtotal_cents: i64, tax_bps: i64, total_cents: i64, seq: u64 }

// order_machine::allowed_next, condensed (Pending=0..PickedUp=9)
#[inline]
fn legal(from: u8, to: u8) -> bool {
    match from {
        0 => to == 1 || to == 6 || to == 7,      // Pending -> Confirmed|Rejected|Cancelled
        1 => to == 2 || to == 4,                  // Confirmed -> Preparing|InDelivery
        2 => to == 3,                             // Preparing -> Ready
        3 => to == 4 || to == 9,                  // Ready -> InDelivery|PickedUp
        4 => to == 5,                             // InDelivery -> Delivered
        _ => false,                               // terminal
    }
}

// money::apply_tax, integer basis-points (the eqc-rs money law shape)
#[inline]
fn money(subtotal: i64, tax_bps: i64) -> i64 { subtotal + subtotal * tax_bps / 10_000 }

#[inline]
fn payload_bytes(st: &OrderState, to: u8) -> [u8; 40] {
    let mut b = [0u8; 40];
    b[0] = st.status; b[1] = to;
    b[2..10].copy_from_slice(&st.subtotal_cents.to_le_bytes());
    b[10..18].copy_from_slice(&st.tax_bps.to_le_bytes());
    b[18..26].copy_from_slice(&st.total_cents.to_le_bytes());
    b[26..34].copy_from_slice(&st.seq.to_le_bytes());
    b
}

// Optional injected verify latency (spin), standing in for a signature/network cost.
#[inline]
fn extra_verify_work(spin: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..spin { acc = acc.wrapping_add(i ^ (acc >> 1)); }
    acc
}

fn main() {
    // Build a workload: N attempts, mix of legal and illegal transitions (roughly 70% legal).
    const N: usize = 2_000_000;
    let mut attempts: Vec<(u8, u8)> = Vec::with_capacity(N);
    let mut s: u64 = 0x1234_5678_9abc_def0;
    for _ in 0..N {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17; // xorshift
        let from = (s % 5) as u8;                 // 0..4 (non-terminal origins)
        let to = ((s >> 8) % 10) as u8;           // 0..9 (some illegal)
        attempts.push((from, to));
    }
    let base = OrderState { status: 0, subtotal_cents: 2599, tax_bps: 875, total_cents: 0, seq: 0 };

    for &spin in &[0u64, 50, 200, 1000] {
        // ---------- Path A: verify-before-persist ----------
        let mut log: Vec<[u8; 32]> = Vec::with_capacity(N);
        let mut committed_a: u64 = 0;
        let t0 = Instant::now();
        let mut sink = 0u64;
        for &(from, to) in &attempts {
            let st = OrderState { status: from, ..base };
            // VERIFY (decide) first — nothing mutated yet.
            sink = sink.wrapping_add(extra_verify_work(spin));
            if legal(from, to) {
                // only now do we mutate + persist
                let mut nst = st;
                nst.status = to;
                nst.total_cents = money(nst.subtotal_cents, nst.tax_bps);
                nst.seq += 1;
                let h = sha3_256(&payload_bytes(&st, to));
                log.push(h);
                committed_a += 1;
                sink ^= nst.total_cents as u64 ^ h[0] as u64;
            }
        }
        let dur_a = t0.elapsed();

        // ---------- Path B: speculative-apply-then-maybe-rollback ----------
        let mut log_b: Vec<[u8; 32]> = Vec::with_capacity(N);
        let mut committed_b: u64 = 0;
        let t1 = Instant::now();
        for &(from, to) in &attempts {
            let mut st = OrderState { status: from, ..base };
            let snapshot = st;                     // rollback checkpoint (small local copy)
            // SPECULATE: apply the mutation optimistically, BEFORE verifying.
            st.status = to;
            st.total_cents = money(st.subtotal_cents, st.tax_bps);
            st.seq += 1;
            // Now verify.
            sink = sink.wrapping_add(extra_verify_work(spin));
            if legal(from, to) {
                let h = sha3_256(&payload_bytes(&snapshot, to));
                log_b.push(h);
                committed_b += 1;
                sink ^= st.total_cents as u64 ^ h[0] as u64;
            } else {
                st = snapshot;                     // ROLLBACK
                sink ^= st.status as u64;
            }
        }
        let dur_b = t1.elapsed();

        // ---------- V: verify-only slice (what speculation tries to hide) ----------
        let t2 = Instant::now();
        let mut vsink = 0u64;
        for &(from, to) in &attempts {
            vsink = vsink.wrapping_add(extra_verify_work(spin));
            if legal(from, to) { vsink += 1; }
        }
        let dur_v = t2.elapsed();

        assert_eq!(committed_a, committed_b);
        assert_eq!(log, log_b);
        let per_a = dur_a.as_nanos() as f64 / N as f64;
        let per_b = dur_b.as_nanos() as f64 / N as f64;
        let per_v = dur_v.as_nanos() as f64 / N as f64;
        println!("spin={:>4}  verify-only V={:7.2} ns  A(verify-first)={:7.2} ns  B(speculate+rollback)={:7.2} ns  | B-A overhead = {:+6.2} ns/op  ({} committed)  [sink={}]",
            spin, per_v, per_a, per_b, per_b - per_a, committed_a, sink ^ vsink);
    }
    println!("\nInterpretation:");
    println!("  * spin=0 is the pure local path (no network/sig cost). B does strictly MORE work");
    println!("    (snapshot + conditional rollback) than A for identical output.");
    println!("  * The verify slice V is the ONLY thing speculation could hide. Speculation needs an");
    println!("    in-flight window (concurrency) to hide it; a synchronous local verify has none, so");
    println!("    on THIS host, same-process, speculation is pure overhead regardless of spin.");
}
