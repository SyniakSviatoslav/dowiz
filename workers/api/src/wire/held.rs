//! A PLATFORM RESPONSE WHOSE BODY NEVER ENTERS WASM. Two cases share the slot:
//!
//! * THE UPGRADE. The object's `101 Switching Protocols`, whose WebSocket lives on the JS
//!   Response itself. Rebuilding it from status + headers + body dropped the socket, and the
//!   runtime answered 500 to every `/api/live` upgrade (live on b1f777b3, 2026-10-02, found by
//!   the flows gate). It is handed back exactly as it came.
//! * THE RELAY (W-OWNER-CPU, 2026-11-04). A Durable Object's answer the Worker only passes on.
//!   `Reply::from_worker` read every such body into a `Vec<u8>` (one copy JS -> wasm) and
//!   `into_response` copied it back (`fixed`); the owner's queue is 1.76 MB and the till tab
//!   851 KB on dubin-durres, against the Free plan's 10 ms of CPU per stateless request. A
//!   relayed `Reply` keeps the platform response here and `into_response` builds the Worker's
//!   answer from ITS OWN status and headers (so `harden`'s and the handler's headers still land,
//!   which a fetched response's immutable headers would refuse) over the object's body STREAM:
//!   the runtime pipes it, and the Worker touches none of its bytes.
//!
//! The body of a relayed reply is therefore NOT READABLE in the Worker: `bytes`/`text`/`json`
//! refuse by name, and `body()` is empty. A handler that must read the answer uses
//! `Stub::fetch_with_request`; one that only hands it on uses `Stub::relay`.

use super::Reply;
use worker::{Error, Result};

/// What is held: an upgrade, or a relayed answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Upgrade,
    Relay,
}

/// The untouched platform response (see the module).
#[derive(Clone, Default)]
pub struct Held(pub(super) std::rc::Rc<std::cell::RefCell<Option<(Mode, worker::Response)>>>);

impl Held {
    pub(super) fn of(mode: Mode, res: worker::Response) -> Self {
        Held(std::rc::Rc::new(std::cell::RefCell::new(Some((mode, res)))))
    }
    pub(super) fn mode(&self) -> Option<Mode> {
        self.0.borrow().as_ref().map(|(m, _)| *m)
    }
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.mode() {
            Some(Mode::Upgrade) => "Held(socket)",
            Some(Mode::Relay) => "Held(relayed body)",
            None => "Held(none)",
        })
    }
}

impl Reply {
    /// A platform response the Worker only passes on: its status and headers here, its body
    /// left where it is (see the module). A 101 is kept whole, as `from_worker` keeps it.
    pub fn relay_from_worker(res: worker::Response) -> Self {
        let mut headers = super::Fields::default();
        for (k, v) in res.headers().entries() {
            let _ = headers.append(&k, &v);
        }
        let status = res.status_code();
        let mode = if status == 101 { Mode::Upgrade } else { Mode::Relay };
        Reply { status, headers, body: None, held: Held::of(mode, res) }
    }

    /// Is this reply's body held outside wasm (a relay)? Such a body cannot be read here.
    pub fn is_relayed(&self) -> bool {
        self.held.mode() == Some(Mode::Relay)
    }

    /// The refusal every body reader gives on a relayed reply.
    pub(super) fn unreadable(&self) -> Result<()> {
        if self.is_relayed() {
            return Err(Error::RustError("relayed body: the bytes were not brought into the Worker (Stub::relay); read it with fetch_with_request".into()));
        }
        Ok(())
    }

    /// The Worker's response for what is held, if anything is: the upgrade as it came; a relay
    /// as THIS reply's status and headers over the held body stream.
    pub(super) fn held_response(&self) -> Result<Option<worker::Response>> {
        let Some((mode, res)) = self.held.0.borrow_mut().take() else { return Ok(None) };
        if mode == Mode::Upgrade {
            return Ok(Some(res));
        }
        let (_, body) = res.into_parts();
        let mut b = worker::Response::builder().with_status(self.status);
        for (k, v) in self.headers.entries() {
            b = b.with_header(k, v)?;
        }
        Ok(Some(b.body(body)))
    }
}
