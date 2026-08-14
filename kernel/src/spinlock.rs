//! `kernel::spinlock` — zero-dependency spinlock with `std::sync::Mutex`-compatible
//! poisoning semantics.
//!
//! Replaces `std::sync::Mutex` for the four non-thread modules
//! (`breaker::audit`, `breaker`, `ports::agent::admission`,
//! `retrieval::memory_store`) so their shared-state serialization no longer
//! depends on the OS threading primitive. The acquire/release core is pure
//! `core::sync::atomic::AtomicBool` (test-and-set, `Acquire`/`Release`) — the
//! only `std` touch-point is [`SpinLockGuard`]'s `Drop`-time panic detection,
//! isolated below so a future no_std extraction can `#[cfg]`-gate that one line
//! (a no_std spinlock has no unwind to detect).

use core::cell::UnsafeCell;
use core::fmt;
use core::hint;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// The lock was poisoned: a previous holder panicked while holding it, so the
/// guarded data may be in an inconsistent state. Fail-closed callers map this to
/// their own named-absence error (e.g. `AuditError::RingPoisoned`), mirroring
/// `std::sync::Mutex`'s `PoisonError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poisoned;

impl fmt::Display for Poisoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("poisoned lock: a previous holder panicked while holding it")
    }
}

impl std::error::Error for Poisoned {}

/// A spinlock: test-and-set on an `AtomicBool`. `lock()` returns
/// `Result<SpinLockGuard<T>, Poisoned>` so every `std::sync::Mutex` call site of
/// the form `.lock().map_err(|_| E::…)?` / `.lock().ok()?` / `match … .lock()`
/// compiles unchanged after the type swap.
pub struct SpinLock<T> {
    locked: AtomicBool,
    poisoned: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: SpinLock provides interior mutability guarded by an atomic flag, like
// std::sync::Mutex; Send/Sync hold exactly when T: Send (the guarded data can
// only be reached through a lock that serializes access).
unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Create an unlocked, unpoisoned lock around `value`.
    pub const fn new(value: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Acquire the lock, spinning until it is free. Returns `Err(Poisoned)` if a
    /// previous holder panicked while holding it (fail-closed — the caller
    /// decides whether to trust the possibly-inconsistent data).
    pub fn lock(&self) -> Result<SpinLockGuard<'_, T>, Poisoned> {
        // Test-and-set: Acquire pairs with the guard's Release on unlock.
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        // We now hold the lock; only the (already-finished) panicking holder could
        // have set `poisoned`, and it has already released, so this read is stable.
        if self.poisoned.load(Ordering::Acquire) {
            self.locked.store(false, Ordering::Release);
            return Err(Poisoned);
        }
        Ok(SpinLockGuard { lock: self })
    }

    /// Whether the lock is currently poisoned (a past holder panicked).
    pub fn is_poisoned(&self) -> bool {
        self.poisoned.load(Ordering::Acquire)
    }
}

impl<T> fmt::Debug for SpinLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Do NOT lock here (Debugging a held lock must not deadlock); report only
        // the poison flag, like std::sync::Mutex's guarded field.
        f.debug_struct("SpinLock")
            .field("poisoned", &self.poisoned.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

/// A held spinlock guard, released (with `Release` ordering) on drop.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: the guard holds the lock; no other guard can exist concurrently.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: same as Deref — exclusive access while the lock is held.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        // Poison on unwind (a panic while the lock was held), mirroring
        // std::sync::Mutex. std-only detection — cfg-gate this line on no_std
        // extraction (a no_std spinlock cannot observe unwind and would skip it).
        if std::thread::panicking() {
            self.lock.poisoned.store(true, Ordering::Release);
        }
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_returns_guard_and_unlocks() {
        let l = SpinLock::new(0u32);
        {
            let mut g = l.lock().expect("unpoisoned lock must lock");
            *g += 1;
        }
        assert_eq!(*l.lock().unwrap(), 1);
    }

    #[test]
    fn fresh_lock_is_not_poisoned() {
        let l = SpinLock::new(());
        assert!(!l.is_poisoned());
        assert!(l.lock().is_ok());
    }

    #[test]
    fn poison_flags_after_panic_while_held() {
        let l = SpinLock::new(());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = l.lock().unwrap();
            panic!("boom while holding the lock");
        }));
        assert!(result.is_err());
        assert!(l.is_poisoned(), "a panic while held must poison the lock");
        assert!(
            matches!(l.lock(), Err(Poisoned)),
            "poisoned lock must return Err(Poisoned)"
        );
    }

    #[test]
    fn debug_does_not_deadlock_when_held() {
        let l = SpinLock::new(42u32);
        let _g = l.lock().unwrap();
        // Debug must not try to re-lock (it would spin forever) — just format it.
        let _ = format!("{:?}", l);
    }
}
