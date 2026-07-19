//! money_ledger — GROWTH TRIPWIRE bench (P80 / S1 §3.3-C1 / §2 binding verdict).
//!
//! HARD RULE (S1 §2, BINDING): DO NOT change `money.rs::ledger_sum`. It is O(n²)
//! (a full `ledger.iter().any(|r| r.reverses == Some(e.id))` scan per non-reversal
//! Earn), but by construction every real per-order ledger has n ≤ 2 (one Earn at
//! confirm + one Reversal at compensate; NO multi-leg order ledgers exist today).
//! The linear scans ARE the fail-closed conservation / idempotency probes — correctness
//! first on money-authority code. R3's "no code change" verdict STANDS.
//!
//! This bench therefore exists ONLY as a GROWTH TRIPWIRE: it sweeps n ∈ {2, 8, 64, 256}
//! to keep the quadratic curve on the record (n=2 is the real-shape anchor; the larger
//! sizes exist to catch a future structural change that would push real ledgers past the
//! bound). It is NOT a regression gate for a "fix" — there is nothing to fix today.
//!
//! REVISIT THRESHOLD (written deliberately, not implied): revisit `ledger_sum`
//! representation only if a future change introduces multi-leg order ledgers (per-item
//! earns, tips, fee splits, settlement legs) or any real ledger exceeds ~8 entries.
//! The correct fix, if that ever fires, is an O(n) reversed-id `HashSet` pre-pass INSIDE
//! `ledger_sum` (semantics-identical) — not a restructuring of the probes, and never
//! before the trigger fires.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::money::{
    ledger_append, ledger_sum, reverse_transfer, Currency, EntryKind, LedgerEntry, Money,
};

/// Build a ledger of `n` Earn legs, reversing every even one so the reversed-id
/// filter path is exercised (realistic earn/reversal mix).
fn build_ledger(n: usize) -> Vec<LedgerEntry> {
    let mut ledger: Vec<LedgerEntry> = Vec::new();
    for i in 0..n {
        let earn = LedgerEntry {
            id: i as u64,
            kind: EntryKind::Earn,
            amount: Money::new(100, Currency::All),
            reverses: None,
        };
        ledger = ledger_append(ledger, earn).unwrap();
    }
    for i in 0..(n / 2) {
        let earn_id = (2 * i) as u64;
        ledger = reverse_transfer(ledger, earn_id, 1000 + earn_id).unwrap();
    }
    ledger
}

fn money_ledger(c: &mut Criterion) {
    let mut group = c.benchmark_group("money_ledger");
    for &n in &[2usize, 8, 64, 256] {
        let ledger = build_ledger(n);
        group.bench_function(format!("{n}"), |b| {
            b.iter(|| black_box(ledger_sum(&ledger)))
        });
    }
    group.finish();
}

criterion_group!(benches, money_ledger);
criterion_main!(benches);
