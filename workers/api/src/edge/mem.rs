//! A world of Durable Objects in memory (tests only): one real `HubImages` per name, each over
//! its own `MemHost`, plus the secrets and vars a handler reads. `Env::Mem(world)` is what a
//! route test hands a handler; everything it asks the HUB namespace for lands here.

use crate::hubdo::host::mem::MemHost;
use crate::hubdo::HubImages;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct World {
    pub secrets: HashMap<String, String>,
    pub vars: HashMap<String, String>,
    pub clock: Cell<i64>,
    /// The KV namespace's bytes (`edge::Kv::Mem`; MEDIA is the only binding).
    pub kv: RefCell<HashMap<String, Vec<u8>>>,
    objects: RefCell<HashMap<String, (Rc<MemHost>, Rc<HubImages>)>>,
}

#[allow(dead_code)]
impl World {
    pub fn new(clock: i64) -> Self {
        World { clock: Cell::new(clock), ..Default::default() }
    }
    /// The object addressed by `name`, made on first use (as `id_from_name` does).
    pub fn object(&self, name: &str) -> Rc<HubImages> {
        self.entry(name).1
    }
    /// That object's storage, for assertions.
    pub fn host(&self, name: &str) -> Rc<MemHost> {
        self.entry(name).0
    }
    /// The names of every object a test caused to exist.
    pub fn names(&self) -> Vec<String> {
        let mut n: Vec<String> = self.objects.borrow().keys().cloned().collect();
        n.sort();
        n
    }
    fn entry(&self, name: &str) -> (Rc<MemHost>, Rc<HubImages>) {
        if let Some(e) = self.objects.borrow().get(name) {
            return e.clone();
        }
        let host = MemHost::named(name, self.clock.get());
        let obj = Rc::new(HubImages::in_memory(host.clone()));
        self.objects.borrow_mut().insert(name.to_string(), (host.clone(), obj.clone()));
        (host, obj)
    }
}

/// Run a future to completion on this thread. For tests of code whose only awaits are on
/// in-memory stores, which are always ready: a future that returns `Pending` here would hang,
/// so it panics instead and names the defect.

pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn raw() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            raw()
        }
        fn noop(_: *const ()) {}
        static VT: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(std::ptr::null(), &VT)
    }
    // SAFETY: the vtable's functions ignore the data pointer.
    let waker = unsafe { Waker::from_raw(raw()) };
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    for _ in 0..1_000 {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
    panic!("block_on: the future is still pending after 1000 polls — something awaited real I/O");
}

thread_local! {
    /// This test thread's answer to outbound calls (`edge::fetch`), and every call it saw.
    static OUTBOUND: RefCell<Option<Box<dyn Fn(&crate::wire::Call) -> worker::Result<crate::wire::Reply>>>> = RefCell::new(None);
    static SENT: RefCell<Vec<crate::wire::Call>> = const { RefCell::new(Vec::new()) };
}

/// Answer this thread's outbound calls with `f` from now on.
pub fn answer_outbound(f: impl Fn(&crate::wire::Call) -> worker::Result<crate::wire::Reply> + 'static) {
    OUTBOUND.with(|o| *o.borrow_mut() = Some(Box::new(f)));
}

/// Every outbound call this thread made, in order (the recording is kept with or without a hook).
pub fn sent() -> Vec<crate::wire::Call> {
    SENT.with(|s| s.borrow().clone())
}

pub(crate) fn outbound(req: &crate::wire::Call) -> Option<worker::Result<crate::wire::Reply>> {
    SENT.with(|s| s.borrow_mut().push(req.clone()));
    OUTBOUND.with(|o| o.borrow().as_ref().map(|f| f(req)))
}
