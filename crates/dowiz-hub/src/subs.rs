//! Who to tell, in a bebop KV store.
//!
//! WHY THIS EXISTS AS STATE AT ALL. A Telegram chat id cannot be derived from
//! anything the customer types on the website. It is minted by Telegram the
//! first time a person messages the bot, and there is no lookup from a phone
//! number to a chat. So the binding — this order belongs to that chat — has to
//! be RECORDED when the person starts the chat, and cannot be inferred later.
//! Any design that skips this step is quietly assuming a customer who already
//! messaged the bot.
//!
//! WHAT IS DELIBERATELY NOT STORED. No phone numbers, no names, no message
//! bodies. A chat id is an opaque handle scoped to one bot: it identifies a
//! conversation, not a person, and it is useless to anyone who does not hold
//! the bot token. That is the whole reason the binding is stored this way round.
//!
//! A THIRD image beside the log and the catalogue, for the same reason the
//! catalogue is a second one: a bebop store has one root, and a third layout
//! cannot share it. The lifecycle matches the catalogue's — small, rewritten
//! whole — so it borrows that shape rather than the log's.

use bebop_store::kv::Kv;
use bebop_store::Store;

use crate::HubError;

/// Subscriptions are a handful of short strings; a restaurant with a thousand
/// live orders is still well inside this.
pub const DEFAULT_SUBS_BYTES: usize = 512 * 1024;

const P_ORDER: &str = "order:";
const P_STAFF: &str = "staff:";

pub struct Subs {
    store: Store,
    kv: Kv,
}

impl Subs {
    pub fn create() -> Result<Self, HubError> {
        let mut store = Store::create_bytes(DEFAULT_SUBS_BYTES);
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Subs { store, kv })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Subs { store, kv })
    }

    /// THE IMAGE IS REWRITTEN WHOLE, not appended to.
    ///
    /// The store is append-only: every commit allocates a new generation and
    /// the old one is never reclaimed. For a KV that rewrites the same small
    /// map over and over, the arena is spent by the NUMBER OF WRITES rather
    /// than by the data -- measured at 313 empty commits before a fresh roster
    /// refused, while five hundred sessions in ONE commit fitted easily. A hub
    /// would therefore stop accepting logins after a few hundred of them.
    ///
    /// `compacted_bytes` commits the entries into a fresh image, so the file is
    /// as large as its content rather than as large as its history. See
    /// `Kv::compacted_bytes` for what that gives up (nothing anything here
    /// reads).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        Ok(self.kv.compacted_bytes(DEFAULT_SUBS_BYTES)?)
    }

    /// Bind one order to the chat that asked about it.
    ///
    /// Last writer wins, on purpose: a customer who opens the tracking link on
    /// their phone after starting on a laptop has moved, and the newer chat is
    /// the one they are looking at.
    pub fn bind_order(&mut self, order_id: &str, chat_id: &str) {
        self.kv.put(&format!("{P_ORDER}{order_id}"), chat_id.as_bytes());
    }

    pub fn chat_for_order(&self, order_id: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_ORDER}{order_id}"))
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    /// Add a staff chat. Keyed BY the chat id rather than holding a list, so
    /// enrolling the same chat twice is idempotent and cannot produce a venue
    /// that pings one phone twice for every order.
    pub fn bind_staff(&mut self, chat_id: &str) {
        self.kv.put(&format!("{P_STAFF}{chat_id}"), b"1");
    }

    /// Remove a staff chat. Stores a tombstone rather than deleting, because the
    /// KV layout has no delete: it is rewritten whole from `entries`, and a
    /// removed key would need a rewrite path that does not exist yet. A
    /// tombstone costs two bytes and keeps the single write path.
    pub fn unbind_staff(&mut self, chat_id: &str) {
        self.kv.put(&format!("{P_STAFF}{chat_id}"), b"0");
    }

    pub fn staff(&self) -> Vec<String> {
        self.kv
            .entries
            .iter()
            .filter(|(k, v)| k.starts_with(P_STAFF) && v.as_slice() == b"1")
            .map(|(k, _)| k[P_STAFF.len()..].to_string())
            .collect()
    }

    /// A fingerprint, the same property the catalogue has: two hubs holding the
    /// same subscriptions produce the same root.
    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bindings must survive the byte image, or every hub restart silently
    /// unsubscribes every waiting customer.
    #[test]
    fn bindings_survive_a_reload() {
        let mut s = Subs::create().expect("create");
        s.bind_order("ord_0001", "55501");
        s.bind_staff("99001");
        s.bind_staff("99002");
        let bytes = s.to_bytes().expect("to_bytes");

        let s = Subs::load(&bytes).expect("load");
        assert_eq!(s.chat_for_order("ord_0001").as_deref(), Some("55501"));
        assert_eq!(s.chat_for_order("ord_0002"), None, "an unbound order has no chat");
        let mut staff = s.staff();
        staff.sort();
        assert_eq!(staff, vec!["99001", "99002"]);
    }

    /// Enrolling the same phone twice must not double every alert.
    #[test]
    fn staff_enrolment_is_idempotent() {
        let mut s = Subs::create().expect("create");
        s.bind_staff("99001");
        s.bind_staff("99001");
        assert_eq!(s.staff().len(), 1);
    }

    /// Leaving must actually stop the messages -- a tombstone that still reads
    /// as subscribed is worse than no unsubscribe at all.
    #[test]
    fn unbinding_removes_from_the_roster() {
        let mut s = Subs::create().expect("create");
        s.bind_staff("99001");
        s.bind_staff("99002");
        s.unbind_staff("99001");
        assert_eq!(s.staff(), vec!["99002"]);

        // And it survives the image: the tombstone is real state, not a
        // filter applied in memory.
        let bytes = s.to_bytes().expect("to_bytes");
        assert_eq!(Subs::load(&bytes).expect("load").staff(), vec!["99002"]);

        // Re-joining after leaving must work.
        let mut s = Subs::load(&bytes).expect("load");
        s.bind_staff("99001");
        let mut staff = s.staff();
        staff.sort();
        assert_eq!(staff, vec!["99001", "99002"]);
    }

    /// A customer who reopens the link on another device moves the binding.
    #[test]
    fn rebinding_an_order_moves_it() {
        let mut s = Subs::create().expect("create");
        s.bind_order("ord_0001", "55501");
        s.bind_order("ord_0001", "55502");
        assert_eq!(s.chat_for_order("ord_0001").as_deref(), Some("55502"));
    }

    /// The staff prefix and the order prefix must not read each other's keys.
    #[test]
    fn prefixes_do_not_collide() {
        let mut s = Subs::create().expect("create");
        s.bind_order("77", "chat_a");
        s.bind_staff("77");
        assert_eq!(s.chat_for_order("77").as_deref(), Some("chat_a"));
        assert_eq!(s.staff(), vec!["77"]);
    }
}
