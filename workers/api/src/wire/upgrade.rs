//! A platform response that cannot be read into bytes and rebuilt: the
//! object's `101 Switching Protocols`, whose WebSocket lives on the JS
//! Response itself. Rebuilding it from status + headers + body dropped the
//! socket, and the runtime answered 500 to every `/api/live` upgrade (live on
//! b1f777b3, 2026-10-02, found by the flows gate). `Reply::from_worker` keeps
//! a 101 here and `Reply::into_response` hands it back as it came.

/// The untouched platform response of an upgrade (see `Reply::upgrade`).
#[derive(Clone, Default)]
pub struct Upgrade(pub(super) std::rc::Rc<std::cell::RefCell<Option<worker::Response>>>);

impl std::fmt::Debug for Upgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.borrow().is_some() { "Upgrade(socket)" } else { "Upgrade(none)" })
    }
}
