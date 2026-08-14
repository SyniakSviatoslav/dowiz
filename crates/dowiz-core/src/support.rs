//! support.rs — customer-support domain: ticket lifecycle FSM + conversation
//! thread (item #13, Chatwoot), zero-dep.
//!
//! Chatwoot itself is a web SaaS (out of scope), but its *domain core* — a
//! ticket state machine, a multi-channel conversation, and canned responses —
//! is pure state-transition logic, the same discipline as `order_machine`.
//! Closed enum + single transition table + wire codes, no floats, no I/O.

use alloc::vec::Vec;
use alloc::string::String;
/// Support ticket lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TicketState {
    Open = 0,
    Pending = 1,
    Resolved = 2,
    Escalated = 3,
    Closed = 4,
}

impl TicketState {
    /// Wire code for storage (closed enum discipline).
    pub const fn wire(self) -> u8 {
        self as u8
    }

    /// Parse a wire code; `None` for unknown (never fabricate a state).
    pub const fn from_wire(code: u8) -> Option<Self> {
        match code {
            0 => Some(TicketState::Open),
            1 => Some(TicketState::Pending),
            2 => Some(TicketState::Resolved),
            3 => Some(TicketState::Escalated),
            4 => Some(TicketState::Closed),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            TicketState::Open => "open",
            TicketState::Pending => "pending",
            TicketState::Resolved => "resolved",
            TicketState::Escalated => "escalated",
            TicketState::Closed => "closed",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, TicketState::Closed)
    }
}

/// Ticket actions that drive transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketAction {
    Assign,
    Reply,
    Hold,
    Escalate,
    Resolve,
    Reopen,
    Close,
}

/// The single transition table (single source of truth). Returns the next
/// state or `None` if the transition is illegal.
pub fn transition(state: TicketState, action: TicketAction) -> Option<TicketState> {
    use TicketAction::*;
    use TicketState::*;
    match (state, action) {
        (Open, Assign) | (Open, Reply) => Some(Open),
        (Open, Hold) => Some(Pending),
        (Open, Escalate) => Some(Escalated),
        (Open, Resolve) => Some(Resolved),
        (Pending, Assign) | (Pending, Reply) | (Pending, Hold) => Some(Pending),
        (Pending, Resolve) => Some(Resolved),
        (Pending, Escalate) => Some(Escalated),
        (Escalated, Reply) | (Escalated, Assign) => Some(Escalated),
        (Escalated, Resolve) => Some(Resolved),
        (Resolved, Reopen) => Some(Open),
        (Resolved, Close) => Some(Closed),
        (Closed, _) => None, // terminal: no transition out
        _ => None,
    }
}

/// A conversation message channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Channel {
    Email = 0,
    Chat = 1,
    Sms = 2,
    Social = 3,
}

/// A conversation message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub channel: Channel,
    pub author: String,
    pub body: String,
    pub ts: u64,
}

/// A conversation thread (ordered messages) attached to a ticket.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new() -> Self {
        Self { messages: Vec::new() }
    }

    pub fn push(&mut self, m: Message) {
        self.messages.push(m);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    /// Messages from a specific channel, in order.
    pub fn by_channel(&self, ch: Channel) -> Vec<&Message> {
        self.messages.iter().filter(|m| m.channel == ch).collect()
    }
}

/// A support ticket: state + conversation + resolution flag.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub id: u64,
    pub state: TicketState,
    pub conversation: Conversation,
}

impl Ticket {
    pub fn new(id: u64) -> Self {
        Self { id, state: TicketState::Open, conversation: Conversation::new() }
    }

    /// Apply an action; updates state if the transition is legal. Returns the
    /// new state (unchanged on illegal transition).
    pub fn apply(&mut self, action: TicketAction) -> TicketState {
        if let Some(next) = transition(self.state, action) {
            self.state = next;
        }
        self.state
    }

    /// Append a message and (for reply actions) keep the ticket alive.
    pub fn reply(&mut self, m: Message) {
        self.conversation.push(m);
    }
}

/// Canned-response store: a const LUT of (key, template). Lookup is O(1) via
/// a small match (the kernel's closed-enum / const-table discipline).
pub fn canned_response(key: &str) -> Option<&'static str> {
    Some(match key {
        "greeting" => "Hi! How can we help you today?",
        "thanks" => "You're welcome — anything else we can do?",
        "closing" => "Thanks for reaching out. We're closing this ticket; reply any time to reopen.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_to_closed_path() {
        let mut t = Ticket::new(0);
        assert_eq!(t.state, TicketState::Open);
        t.apply(TicketAction::Assign);
        assert_eq!(t.state, TicketState::Open);
        t.apply(TicketAction::Resolve);
        assert_eq!(t.state, TicketState::Resolved);
        t.apply(TicketAction::Close);
        assert_eq!(t.state, TicketState::Closed);
        assert!(t.state.is_terminal());
        // No transition out of Closed.
        assert_eq!(t.apply(TicketAction::Reopen), TicketState::Closed);
    }

    #[test]
    fn escalate_and_resolve() {
        let mut t = Ticket::new(1);
        t.apply(TicketAction::Escalate);
        assert_eq!(t.state, TicketState::Escalated);
        t.apply(TicketAction::Resolve);
        assert_eq!(t.state, TicketState::Resolved);
    }

    #[test]
    fn reopen_resolved() {
        let mut t = Ticket::new(2);
        t.apply(TicketAction::Resolve);
        assert_eq!(t.state, TicketState::Resolved);
        t.apply(TicketAction::Reopen);
        assert_eq!(t.state, TicketState::Open);
    }

    #[test]
    fn wire_roundtrip() {
        for code in 0..=4u8 {
            let st = TicketState::from_wire(code).unwrap();
            assert_eq!(st.wire(), code);
        }
        assert_eq!(TicketState::from_wire(9), None);
    }

    #[test]
    fn conversation_filters_by_channel() {
        let mut c = Conversation::new();
        c.push(Message { channel: Channel::Email, author: "a".into(), body: "1".into(), ts: 0 });
        c.push(Message { channel: Channel::Chat, author: "b".into(), body: "2".into(), ts: 1 });
        c.push(Message { channel: Channel::Email, author: "c".into(), body: "3".into(), ts: 2 });
        assert_eq!(c.by_channel(Channel::Email).len(), 2);
        assert_eq!(c.by_channel(Channel::Chat).len(), 1);
        assert_eq!(c.by_channel(Channel::Sms).len(), 0);
    }

    #[test]
    fn canned_responses() {
        assert_eq!(canned_response("greeting"), Some("Hi! How can we help you today?"));
        assert_eq!(canned_response("nope"), None);
    }
}
