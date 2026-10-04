//! A venue's object running its own timed work: see [`Own`].

use super::Env;

/// THE OBJECT'S OWN WORK, IN ITS OWN TURN (W-LOOP, docs/research/2026-10-03-hub-cost-recomputed.md
/// §1.3). The venue's alarm used to ask a second object (`cron~<venue>`) to run its jobs, and
/// that runner read the venue back through its stub about ten times a run: 11.9 billed object
/// requests per firing, 95 % of the account's day. The jobs talk to the venue only through
/// `Place`/`Stub`, so they run unchanged inside the venue's own `alarm()` when the `Env` they
/// are handed answers the venue's OWN name with the object itself: a function call, not a
/// request. Any other name (the registry, another venue) is still a real stub of `base`.
pub struct Own {
    /// The venue object running the alarm. `'static` for the reason worker-macros gives for
    /// every `alarm()` and `fetch()` it dispatches (`durable_object.rs`: "Durable Object will
    /// never be destroyed while there is still a running promise inside of it"); an `Own` is
    /// built by `hubdo/timer.rs` inside that promise and dropped before it settles.
    pub(crate) me: &'static crate::hubdo::HubImages,
    /// The venue's name: the one `id_from_name` that is answered in process.
    pub(crate) venue: String,
    /// Everything else.
    pub(crate) base: Env,
}
