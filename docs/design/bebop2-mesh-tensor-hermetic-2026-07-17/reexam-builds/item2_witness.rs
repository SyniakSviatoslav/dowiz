//! ITEM 2 — "Self-auditing inline witnessing": naive (rejected) vs steelmanned (commitment).
//!
//! Rejection: self-certification = RC-2, "the check restates the claim." True for the NAIVE
//! form. This file builds BOTH forms against the same forged artifact and shows an independent
//! verifier catches the forgery under the steelman but NOT under the naive form.
//!
//! Distinction being demonstrated:
//!   NAIVE  = "I checked my own work, it's valid"  -> a boolean/attestation the author controls.
//!   STEELMAN = "here is a commitment to (inputs, pinned-law-id, output); recompute it yourself" ->
//!            the VALIDITY judgment is made LATER by an independent party running an EXTERNAL law,
//!            not asserted by the author. That is a witness/commitment, not a self-certification.
//!
//! Production hash = SHA3-256 (kernel/src/event_log.rs). Here we use a compact FNV-1a-64 purely
//! to keep the file self-contained; the argument is hash-agnostic (needs only 2nd-preimage
//! resistance, which SHA3 provides in the real path).

// compact stand-in hash (SHA3-256 in production)
fn h(bytes: &[u8]) -> u64 {
    let mut x: u64 = 0xcbf29ce484222325;
    for &b in bytes { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
    x
}
fn h_i64(v: i64) -> u64 { h(&v.to_le_bytes()) }
fn h_inputs(subtotal: i64, tax_bps: i64) -> u64 {
    let mut buf = [0u8; 16];
    buf[0..8].copy_from_slice(&subtotal.to_le_bytes());
    buf[8..16].copy_from_slice(&tax_bps.to_le_bytes());
    h(&buf)
}

// ---- THE PINNED, DETERMINISTIC LAW (money.v1). The verifier holds this; NOT the author. ----
fn money_v1(subtotal: i64, tax_bps: i64) -> i64 { subtotal + subtotal * tax_bps / 10_000 }
const LAW_ID: &str = "money.v1";

// =====================================================================================
// NAIVE self-certifying witness: author reruns the law itself and attests a boolean.
// =====================================================================================
struct SelfCert { output: i64, self_valid: bool /* author's own verdict */ }

fn author_selfcert(subtotal: i64, tax_bps: i64, claimed_output: i64) -> SelfCert {
    // A malicious author simply SETS self_valid = true regardless of truth.
    // (An honest author would compute money_v1 and compare; but the verifier can't tell which.)
    SelfCert { output: claimed_output, self_valid: true }
}

// Verifier that TRUSTS the attestation (the only thing the naive form offers cheaply).
fn verify_selfcert_trusting(w: &SelfCert) -> bool { w.self_valid }

// Verifier that DISTRUSTS and recomputes — at which point the attestation added nothing (RC-2).
fn verify_selfcert_recompute(w: &SelfCert, subtotal: i64, tax_bps: i64) -> bool {
    w.output == money_v1(subtotal, tax_bps) // the boolean w.self_valid is irrelevant / ignored
}

// =====================================================================================
// STEELMAN commitment witness: author commits (inputs, law_id, output). No validity claim.
// =====================================================================================
struct WorkReceipt { input_commit: u64, law_id: &'static str, output_commit: u64, output: i64 }

fn author_receipt(subtotal: i64, tax_bps: i64, claimed_output: i64) -> WorkReceipt {
    // The author commits to WHAT they claim they computed. They do NOT assert it is correct.
    // A malicious author can only commit to the (wrong) output they actually put on the wire.
    WorkReceipt {
        input_commit: h_inputs(subtotal, tax_bps),
        law_id: LAW_ID,
        output_commit: h_i64(claimed_output),
        output: claimed_output,
    }
}

// INDEPENDENT verifier: receives the inputs out-of-band, re-runs the EXTERNAL pinned law,
// and checks the commitments. Trusts the law + the hash, NEVER the author.
fn verify_receipt_independent(w: &WorkReceipt, subtotal: i64, tax_bps: i64) -> Result<(), &'static str> {
    if w.law_id != LAW_ID { return Err("unknown/again law version"); }
    if w.input_commit != h_inputs(subtotal, tax_bps) { return Err("input commitment mismatch (equivocation)"); }
    if w.output_commit != h_i64(w.output) { return Err("output commitment mismatch (tamper)"); }
    let recomputed = money_v1(subtotal, tax_bps);           // <-- the independent re-execution
    if h_i64(recomputed) != w.output_commit { return Err("REPLAY MISMATCH: committed output != law(inputs)"); }
    Ok(())
}

fn main() {
    let (subtotal, tax_bps) = (2599i64, 875i64);
    let honest = money_v1(subtotal, tax_bps);   // 2826
    let forged = subtotal;                       // 2599 — a skim-the-tax forgery

    println!("law money.v1({}, {} bps) = {}   (forged claim = {})\n", subtotal, tax_bps, honest, forged);

    for (label, claimed) in [("HONEST author", honest), ("MALICIOUS author (skims tax)", forged)] {
        println!("--- {} ---", label);

        // NAIVE self-cert
        let sc = author_selfcert(subtotal, tax_bps, claimed);
        let trust = verify_selfcert_trusting(&sc);
        let recompute = verify_selfcert_recompute(&sc, subtotal, tax_bps);
        println!("  NAIVE self-cert:  trusting-verifier accepts = {:5}   (self_valid flag = {})", trust, sc.self_valid);
        println!("                    recompute-verifier accepts = {:5}   <- but this IS just re-doing the work (RC-2)", recompute);

        // STEELMAN commitment
        let wr = author_receipt(subtotal, tax_bps, claimed);
        let indep = verify_receipt_independent(&wr, subtotal, tax_bps);
        println!("  STEELMAN receipt: independent-verifier result = {:?}", indep);
        println!();
    }

    println!("MEASURED DIFFERENCE:");
    println!("  * NAIVE trusting-verifier ACCEPTS the malicious output (self_valid=true is author-controlled).");
    println!("  * NAIVE recompute-verifier catches it, but only by redoing the law -> the witness added nothing (RC-2).");
    println!("  * STEELMAN independent-verifier REJECTS the malicious output via replay, WITHOUT trusting the author,");
    println!("    and the receipt still bought: input non-equivocation, tamper-evidence, and offline/deferred/3rd-party");
    println!("    verifiability that travels with the artifact. That is a commitment, not a self-certification.");
}
