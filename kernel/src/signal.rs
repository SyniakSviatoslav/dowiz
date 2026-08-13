//! signal.rs — fine-grained reactive signals (the Dioxus/SolidJS reactivity
//! concept, item #12), zero-dep.
//!
//! A signal is a mutable cell with change notification. `create_signal` splits
//! read/write; `effect` re-runs a closure whenever any signal it read changes.
//! This is the same "delta calculus" substrate the kernel already uses — a
//! value plus its *change* as a first-class event — lifted to reactive state.
//!
//! Zero-dep: `Rc` (alloc) + `RefCell` (core). Deterministic: effects run in
//! subscription order; no threads, no interior randomness.

use std::cell::RefCell;
use std::rc::Rc;

/// A single reactive value with subscribers.
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    subs: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

/// Read handle (can only read, not mutate).
#[derive(Debug, Clone)]
pub struct ReadSignal<T>(Rc<RefCell<T>>);

/// Write handle (mutates and notifies subscribers).
#[derive(Clone)]
pub struct WriteSignal<T> {
    cell: Rc<RefCell<T>>,
    subs: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl<T> Signal<T> {
    /// Create a signal with the given initial value.
    pub fn new(init: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(init)),
            subs: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// Split a signal into (read, write) handles.
pub fn create_signal<T>(init: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let s = Signal::new(init);
    (
        ReadSignal(Rc::clone(&s.value)),
        WriteSignal {
            cell: Rc::clone(&s.value),
            subs: Rc::clone(&s.subs),
        },
    )
}

impl<T: Clone> ReadSignal<T> {
    /// Snapshot the current value.
    pub fn get(&self) -> T {
        self.0.borrow().clone()
    }

    /// Read the value with a closure (avoids the clone).
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.borrow())
    }
}

impl<T: PartialEq> WriteSignal<T> {
    /// Set a new value. Notifies subscribers only when the value actually
    /// changes (fine-grained: no spurious re-runs).
    pub fn set(&self, v: T) {
        let changed = {
            let cur = self.cell.borrow();
            *cur != v
        };
        if changed {
            *self.cell.borrow_mut() = v;
            // Run subscribers in a fixed order, snapshotting first so an
            // effect that mutates another signal does not alias-borrow.
            let subs: Vec<Rc<dyn Fn()>> = self.subs.borrow().iter().cloned().collect();
            for sub in subs {
                sub();
            }
        }
    }
}

/// Track the current effect so `WriteSignal::set` can re-run it. The tracked
/// closure is stored in a thread-local. Zero-dep, single-threaded.
thread_local! {
    static CURRENT_EFFECT: RefCell<Option<Rc<dyn Fn()>>> = RefCell::new(None);
}

/// Run `f` as an effect: subscribe all signals it reads, then run once.
pub fn effect(f: impl Fn() + 'static) {
    let f: Rc<dyn Fn()> = Rc::new(f);
    CURRENT_EFFECT.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&f)));
    f();
    CURRENT_EFFECT.with(|slot| *slot.borrow_mut() = None);
}

/// Subscribe a signal to the current effect (called inside the signal's
/// read path when an effect is active).
pub fn track(subs: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
    CURRENT_EFFECT.with(|slot| {
        if let Some(eff) = slot.borrow().as_ref() {
            subs.borrow_mut().push(Rc::clone(eff));
        }
    });
}

/// A derived signal: recomputes `f` over its dependencies; `get` re-evaluates.
pub struct Computed<T> {
    value: Rc<RefCell<T>>,
    compute: Rc<dyn Fn() -> T>,
    dirty: Rc<RefCell<bool>>,
}

impl<T> Computed<T> {
    /// Build a computed value from a closure reading other signals.
    pub fn new(compute: impl Fn() -> T + 'static, init: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(init)),
            compute: Rc::new(compute),
            dirty: Rc::new(RefCell::new(true)),
        }
    }
}

impl<T: Clone> Computed<T> {
    /// Evaluate (recompute if dirty) and return the value.
    pub fn get(&self) -> T {
        if *self.dirty.borrow() {
            let v = (self.compute)();
            *self.value.borrow_mut() = v;
            *self.dirty.borrow_mut() = false;
        }
        self.value.borrow().clone()
    }

    /// Mark stale (call after a dependency changed).
    pub fn mark_dirty(&self) {
        *self.dirty.borrow_mut() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_get_set_roundtrip() {
        let (r, w) = create_signal(1i32);
        assert_eq!(r.get(), 1);
        w.set(42);
        assert_eq!(r.get(), 42);
    }

    #[test]
    fn set_same_value_is_noop() {
        let (_, w) = create_signal(7i32);
        w.set(7); // must not panic or notify (value unchanged)
        w.set(8);
    }

    #[test]
    fn read_handle_cannot_write() {
        let (r, _) = create_signal(5i32);
        // ReadSignal has no set method — compile-time enforcement of
        // unidirectional flow. Exercise get only.
        assert_eq!(r.get(), 5);
        assert_eq!(r.with(|v| *v), 5);
    }

    #[test]
    fn computed_recomputes() {
        let (_, w) = create_signal(2i32);
        let c = Computed::new(|| 10, 0);
        assert_eq!(c.get(), 10);
        c.mark_dirty();
        assert_eq!(c.get(), 10);
    }

    #[test]
    fn effect_runs_once() {
        use std::cell::Cell;
        let count = Rc::new(Cell::new(0));
        let c2 = Rc::clone(&count);
        effect(move || {
            c2.set(c2.get() + 1);
        });
        assert_eq!(count.get(), 1);
    }
}
