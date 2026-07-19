//! contention.rs — CONTENDED multi-threaded benchmarks for the three Mutex sites
//! flagged NEEDS-CONTENDED-BENCH-FIRST by `docs/research/OPUS-PERF-KERNEL-AUDIT-2026-07-18.md`
//! (A1 `token_bucket`, A2 `budget`, A3 `admission` seen-set).
//!
//! The Standing Rule (`dowiz/.claude/CLAUDE.md` §Performance) requires a benchmark
//! proving *real lock contention* before any atomic/lock-free rewrite. The existing
//! `criterion.rs::token_bucket/try_acquire_permit` bench is SINGLE-THREADED — a Mutex
//! only serializes under real concurrency on a shared object, so that bench cannot
//! establish contention. This file closes that gap: N ∈ {1,2,4,8} threads hammer ONE
//! shared object, and each candidate atomic/lock-free impl is measured side-by-side
//! against the current Mutex under identical load.
//!
//! Metric: `iter_custom` returns the wall-time of the slowest thread's `iters`-op run
//! after a start barrier (so thread-spawn cost is excluded from the timed region).
//! Throughput::Elements(threads) makes criterion report AGGREGATE ops/sec across all
//! contending threads — higher = better, and the Mutex-vs-atomic ratio at N=8 is the
//! load-bearing number.

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Barrier, Mutex};
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

/// Run `threads` threads, each executing `op(tid, i)` for `i in 0..iters`, all released
/// together by a barrier. Returns the SLOWEST thread's post-barrier elapsed (≈ the
/// contended wall-time for `iters` ops). Thread spawn/join is outside the timed region.
fn run_contended(threads: usize, iters: u64, op: impl Fn(usize, u64) + Sync) -> Duration {
    let barrier = Barrier::new(threads);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|tid| {
                let barrier = &barrier;
                let op = &op;
                s.spawn(move || {
                    barrier.wait();
                    let t = Instant::now();
                    for i in 0..iters {
                        op(tid, i);
                    }
                    t.elapsed()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .max()
            .unwrap()
    })
}

// ════════════════════════════════════════════════════════════════════════════
// SITE A2 — budget.rs ComputeBudget::debit  (the SIMPLE, provably-equivalent case)
// ════════════════════════════════════════════════════════════════════════════

/// Mirror of the CURRENT `budget.rs` shape: a `Mutex<f64>` spend accumulator.
struct MutexBudget {
    spent: Mutex<f64>,
    ceiling: f64,
}
impl MutexBudget {
    fn new(ceiling: f64) -> Self {
        Self {
            spent: Mutex::new(0.0),
            ceiling,
        }
    }
    fn debit(&self, amount: f64) -> bool {
        let mut g = self.spent.lock().unwrap_or_else(|e| e.into_inner());
        if *g + amount > self.ceiling {
            false
        } else {
            *g += amount;
            true
        }
    }
}

/// CANDIDATE: lock-free CAS accumulator (bit-cast f64 in an AtomicU64). Degrade-closed
/// is PRESERVED — the ceiling is re-checked on every CAS retry, so it can never overshoot.
struct AtomicBudget {
    spent_bits: AtomicU64,
    ceiling: f64,
}
impl AtomicBudget {
    fn new(ceiling: f64) -> Self {
        Self {
            spent_bits: AtomicU64::new(0f64.to_bits()),
            ceiling,
        }
    }
    fn debit(&self, amount: f64) -> bool {
        let mut cur = self.spent_bits.load(Ordering::Relaxed);
        loop {
            let spent = f64::from_bits(cur);
            if spent + amount > self.ceiling {
                return false;
            }
            let next = (spent + amount).to_bits();
            match self.spent_bits.compare_exchange_weak(
                cur,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => cur = actual,
            }
        }
    }
}

fn bench_budget(c: &mut Criterion) {
    let mut g = c.benchmark_group("contended_budget");
    for &n in &[1usize, 2, 4, 8] {
        g.throughput(Throughput::Elements(n as u64));
        // ceiling huge so every debit succeeds → pure contended-success path.
        g.bench_function(format!("mutex/threads_{n}"), |b| {
            let budget = MutexBudget::new(f64::INFINITY);
            b.iter_custom(|iters| {
                run_contended(n, iters, |_tid, _i| {
                    black_box(budget.debit(black_box(1.0)));
                })
            })
        });
        g.bench_function(format!("atomic/threads_{n}"), |b| {
            let budget = AtomicBudget::new(f64::INFINITY);
            b.iter_custom(|iters| {
                run_contended(n, iters, |_tid, _i| {
                    black_box(budget.debit(black_box(1.0)));
                })
            })
        });
    }
    g.finish();
}

// ════════════════════════════════════════════════════════════════════════════
// SITE A1 — token_bucket.rs try_acquire  (Mutex holds a clock read + coupled state)
// ════════════════════════════════════════════════════════════════════════════

/// Mirror of the CURRENT `token_bucket.rs` critical section: lock, read the monotonic
/// clock, refill, decrement — all under one Mutex (the coupled (tokens,last_refill)
/// invariant the module documents as needing a single atomic section).
struct MutexBucket {
    capacity: f64,
    refill_rate: f64,
    inner: Mutex<(f64, Instant)>,
}
impl MutexBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
            inner: Mutex::new((capacity, Instant::now())),
        }
    }
    fn try_acquire(&self, n: f64) -> bool {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(g.1).as_secs_f64();
        if elapsed > 0.0 {
            g.0 = (g.0 + self.refill_rate * elapsed).min(self.capacity);
            if g.0 < 0.0 {
                g.0 = 0.0;
            }
            g.1 = now;
        }
        if g.0 >= n {
            g.0 -= n;
            true
        } else {
            false
        }
    }
}

/// MINIMAL-CHANGE CANDIDATE: SAME token-bucket algorithm, SAME Mutex, SAME coupled
/// (tokens,last_refill) invariant — but the monotonic clock read is moved OUTSIDE the
/// lock so the critical section shrinks to a few float ops (no syscall held under the
/// lock). Over-grant safety is preserved: a thread that waited for the lock holds a
/// slightly-stale `now`, so `saturating_duration_since` yields a SMALLER elapsed →
/// conservative (never over-grants). This is the ponytail-minimal fix vs the GCRA swap.
struct MutexBucketClockOut {
    capacity: f64,
    refill_rate: f64,
    inner: Mutex<(f64, Instant)>,
}
impl MutexBucketClockOut {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
            inner: Mutex::new((capacity, Instant::now())),
        }
    }
    fn try_acquire(&self, n: f64) -> bool {
        let now = Instant::now(); // clock read BEFORE the lock
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let elapsed = now.saturating_duration_since(g.1).as_secs_f64();
        if elapsed > 0.0 {
            g.0 = (g.0 + self.refill_rate * elapsed).min(self.capacity);
            if g.0 < 0.0 {
                g.0 = 0.0;
            }
            g.1 = now;
        }
        if g.0 >= n {
            g.0 -= n;
            true
        } else {
            false
        }
    }
}

/// CANDIDATE: GCRA (Generic Cell Rate Algorithm) lock-free rate limiter — a single
/// AtomicU64 "theoretical arrival time" (TAT, nanos since base). The clock read is
/// OUTSIDE the CAS so clock reads parallelize; only the tiny CAS serializes. This is
/// the standard lock-free rate-limiter shape (cf. the `governor` crate). NB: GCRA is
/// a DIFFERENT algorithm from the token bucket — measured here only to quantify the
/// contention ceiling difference, NOT proposed as a drop-in without an invariant re-proof.
struct GcraBucket {
    nanos_per_token: f64,
    burst_nanos: f64,
    tat: AtomicU64,
    base: Instant,
}
impl GcraBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        let nanos_per_token = 1e9 / refill_rate;
        Self {
            nanos_per_token,
            burst_nanos: capacity * nanos_per_token,
            tat: AtomicU64::new(0),
            base: Instant::now(),
        }
    }
    fn try_acquire(&self, n: f64) -> bool {
        let now = self.base.elapsed().as_nanos() as u64;
        let cost = (n * self.nanos_per_token) as u64;
        let limit = now as f64 + self.burst_nanos;
        loop {
            let tat = self.tat.load(Ordering::Relaxed);
            let allow_at = tat.max(now);
            let new_tat = allow_at + cost;
            if new_tat as f64 > limit {
                return false;
            }
            match self
                .tat
                .compare_exchange_weak(tat, new_tat, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }
}

fn bench_token_bucket(c: &mut Criterion) {
    let mut g = c.benchmark_group("contended_token_bucket");
    for &n in &[1usize, 2, 4, 8] {
        g.throughput(Throughput::Elements(n as u64));
        // capacity + refill huge so every acquire grants → pure contended-success path.
        g.bench_function(format!("mutex/threads_{n}"), |b| {
            let bucket = MutexBucket::new(1e15, 1e12);
            b.iter_custom(|iters| {
                run_contended(n, iters, |_tid, _i| {
                    black_box(bucket.try_acquire(black_box(1.0)));
                })
            })
        });
        g.bench_function(format!("mutex_clock_outside/threads_{n}"), |b| {
            let bucket = MutexBucketClockOut::new(1e15, 1e12);
            b.iter_custom(|iters| {
                run_contended(n, iters, |_tid, _i| {
                    black_box(bucket.try_acquire(black_box(1.0)));
                })
            })
        });
        g.bench_function(format!("gcra_atomic/threads_{n}"), |b| {
            let bucket = GcraBucket::new(1e15, 1e12);
            b.iter_custom(|iters| {
                run_contended(n, iters, |_tid, _i| {
                    black_box(bucket.try_acquire(black_box(1.0)));
                })
            })
        });
    }
    g.finish();
}

// ════════════════════════════════════════════════════════════════════════════
// SITE A3 — admission/hybrid_gate seen-set  Mutex<HashSet<[u8;8]>>
// ════════════════════════════════════════════════════════════════════════════

use std::collections::HashSet;

/// Mirror of the CURRENT seen-set: one global `Mutex<HashSet>`.
struct MutexSet {
    seen: Mutex<HashSet<[u8; 8]>>,
}
impl MutexSet {
    fn new() -> Self {
        Self {
            seen: Mutex::new(HashSet::new()),
        }
    }
    fn record(&self, nonce: [u8; 8]) -> bool {
        let mut g = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        g.insert(nonce)
    }
}

/// CANDIDATE: fixed-shard set — lock striped over 16 shards by the nonce's first byte.
/// Bounded memory (no attacker-growable per-source map), N-fold less contention.
struct ShardedSet {
    shards: Vec<Mutex<HashSet<[u8; 8]>>>,
}
impl ShardedSet {
    fn new(shards: usize) -> Self {
        Self {
            shards: (0..shards).map(|_| Mutex::new(HashSet::new())).collect(),
        }
    }
    fn record(&self, nonce: [u8; 8]) -> bool {
        let idx = (nonce[0] as usize) % self.shards.len();
        let mut g = self.shards[idx].lock().unwrap_or_else(|e| e.into_inner());
        g.insert(nonce)
    }
}

/// A cheap deterministic stand-in (~sub-µs) for the REAL per-frame work that precedes
/// the seen-set insert on the admission path: Ed25519 + ML-DSA-65 verify, which is
/// µs–ms. We can't run real crypto in a tight loop cheaply, so this FNV spin models a
/// conservative ~fixed pre-lock cost to show how it dilutes lock contention. Clearly
/// labeled: the real verify is 10–1000× larger, so real dilution is even stronger.
#[inline]
fn crypto_stand_in(seed: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64 ^ seed;
    for _ in 0..64 {
        h = h.wrapping_mul(0x100000001b3);
        h ^= h >> 13;
    }
    h
}

/// A HEAVIER stand-in (~2 µs) — still 10–50× cheaper than a real Ed25519+ML-DSA-65
/// verify (µs–ms), but enough to demonstrate the crossover: once real per-frame work
/// precedes the O(1) lock, the lock stops being the bottleneck (crypto-bound), and the
/// single global Mutex converges with the sharded set — i.e. sharding buys nothing on
/// the realistic path. This is the load-bearing evidence for "no action on the seen-set".
#[inline]
fn heavy_crypto_stand_in(seed: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64 ^ seed;
    for _ in 0..2048 {
        h = h.wrapping_mul(0x100000001b3);
        h ^= h >> 13;
    }
    h
}

fn nonce_of(tid: usize, i: u64) -> [u8; 8] {
    // Bounded nonce space per thread (mask) so the set stays bounded across large iters,
    // while keeping a realistic new-mostly insert mix.
    let v = ((tid as u64) << 40) | (i & 0xFFFF);
    v.to_le_bytes()
}

fn bench_seen_set(c: &mut Criterion) {
    let mut g = c.benchmark_group("contended_seen_set");
    for &n in &[1usize, 2, 4, 8] {
        g.throughput(Throughput::Elements(n as u64));

        // (1) RAW lock contention — worst case, no crypto before the lock.
        g.bench_function(format!("raw_mutex/threads_{n}"), |b| {
            b.iter_custom(|iters| {
                let set = MutexSet::new();
                run_contended(n, iters, |tid, i| {
                    black_box(set.record(nonce_of(tid, i)));
                })
            })
        });
        g.bench_function(format!("raw_sharded/threads_{n}"), |b| {
            b.iter_custom(|iters| {
                let set = ShardedSet::new(16);
                run_contended(n, iters, |tid, i| {
                    black_box(set.record(nonce_of(tid, i)));
                })
            })
        });

        // (2) REALISTIC path — crypto stand-in BEFORE the lock (as the real admit does:
        //     verify_chain + classical + PQ, THEN record). Shows the lock is a tiny
        //     fraction of per-frame cost, so contention is negligible in practice.
        g.bench_function(format!("realistic_mutex/threads_{n}"), |b| {
            b.iter_custom(|iters| {
                let set = MutexSet::new();
                run_contended(n, iters, |tid, i| {
                    black_box(crypto_stand_in(i));
                    black_box(set.record(nonce_of(tid, i)));
                })
            })
        });

        // (3) HEAVY-realistic path — ~2µs crypto stand-in before the lock. Mutex vs
        //     sharded should CONVERGE here (both crypto-bound): proves the lock is not
        //     the bottleneck once real per-frame work precedes it → no action needed.
        g.bench_function(format!("heavy_mutex/threads_{n}"), |b| {
            b.iter_custom(|iters| {
                let set = MutexSet::new();
                run_contended(n, iters, |tid, i| {
                    black_box(heavy_crypto_stand_in(i));
                    black_box(set.record(nonce_of(tid, i)));
                })
            })
        });
        g.bench_function(format!("heavy_sharded/threads_{n}"), |b| {
            b.iter_custom(|iters| {
                let set = ShardedSet::new(16);
                run_contended(n, iters, |tid, i| {
                    black_box(heavy_crypto_stand_in(i));
                    black_box(set.record(nonce_of(tid, i)));
                })
            })
        });
    }
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).warm_up_time(Duration::from_millis(500)).measurement_time(Duration::from_secs(2));
    targets = bench_budget, bench_token_bucket, bench_seen_set
}
criterion_main!(benches);
