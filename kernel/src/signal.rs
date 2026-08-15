//! signal.rs — std host shim (pure `Signal`/`ReadSignal`/`WriteSignal`/`Computed`
//! live in `dowiz_core::signal`; the thread-local effect runtime stays here).
//!
//! The reactive *effect* runtime (`effect`/`track`/`CURRENT_EFFECT`) uses
//! `std::thread_local!` for the "current effect" slot — no `core::` equivalent
//! exists, so it is the std seam the no_std core cannot hold.

pub use dowiz_core::signal::*;

use core::cell::RefCell;
use alloc::rc::Rc;
use alloc::vec::Vec;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_runs_once() {
        use core::cell::Cell;
        let count = Rc::new(Cell::new(0));
        let c2 = Rc::clone(&count);
        effect(move || {
            c2.set(c2.get() + 1);
        });
        assert_eq!(count.get(), 1);
    }
}
