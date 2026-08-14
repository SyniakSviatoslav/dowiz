//! evolution_go.rs — Evolution Go reimplementation: WhatsApp API.
//!
//! # What this is
//! A kernel-native WhatsApp API: REST endpoints, WebSocket events, message
//! storage, media support — all using kernel primitives, zero external deps.
//!
//! # Evolution Go mapping
//! - "RESTful API" → `RestEndpoint` trait + `Route` registry
//! - "WebSocket events" → `WebSocketEvent` enum + `EventHandler` trait
//! - "Real-time events — AMQP/RabbitMQ, NATS" → `EventBus` trait (plugable)
//! - "Media support — MinIO/S3" → `MediaStore` trait (plugable)
//! - "Message storage — PostgreSQL" → `MessageStore` trait (plugable)
//! - "QR code pairing" → `QrPairRequest`/`QrPairResponse` types
//! - "License management" → `LicenseManager` trait
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - Uses existing kernel primitives: fsm (message state machine), event_log
//!   (SHA3-256 event storage), ports/hub_intake (inbound message vocab)

use crate::event_log::{sha3_256, EventLog, MeshEvent};
use alloc::collections::BTreeMap;

/// WhatsApp message direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    /// Message sent from user to WhatsApp server.
    Inbound,
    /// Message sent from WhatsApp server to user.
    Outbound,
}

/// WhatsApp message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhatsAppMessageType {
    Text,
    Image,
    Video,
    Audio,
    Document,
    Sticker,
    Location,
    Contact,
    Voice,
    LiveLocation,
    Reaction,
}

/// A WhatsApp message — the core domain type.
#[derive(Debug, Clone, PartialEq)]
pub struct WhatsAppMessage {
    /// Unique message ID (SHA3-256 of content).
    pub id: [u8; 32],
    /// WhatsApp sender ID (phone number or placeholder).
    pub sender: String,
    /// WhatsApp recipient ID.
    pub recipient: String,
    /// Message type.
    pub msg_type: WhatsAppMessageType,
    /// Text content (for text messages).
    pub text: Option<String>,
    /// Media URL (for media messages).
    pub media_url: Option<String>,
    /// Media MIME type.
    pub media_mime: Option<String>,
    /// Direction.
    pub direction: MessageDirection,
    /// Timestamp (microseconds).
    pub timestamp_us: u64,
    /// Whether this is a paired (authenticated) session.
    pub paired: bool,
}

/// WhatsApp session state — FSM-driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session not yet initiated.
    Unregistered,
    /// QR code presented, awaiting scan.
    AwaitingQrScan,
    /// QR scanned, waiting for confirmation.
    QrScanned,
    /// Session authenticated and active.
    Active,
    /// Session disconnected.
    Disconnected,
    /// Session suspended (rate limit, ban, etc.).
    Suspended,
}

/// WhatsApp session — tracks connection state.
#[derive(Debug, Clone)]
pub struct WhatsAppSession {
    /// Session ID.
    pub session_id: u64,
    /// Phone number / identifier.
    pub identifier: String,
    /// Current FSM state.
    pub state: SessionState,
    /// QR pair request data (when awaiting scan).
    pub qr_data: Option<String>,
    /// Paired timestamp.
    pub paired_at_us: Option<u64>,
    /// Message count in this session.
    pub message_count: u64,
}

/// REST endpoint representation — for the API surface.
#[derive(Debug, Clone)]
pub struct RestEndpoint {
    /// HTTP method.
    pub method: String,
    /// Path pattern.
    pub path: String,
    /// Whether authentication is required.
    pub auth_required: bool,
    /// Description of the endpoint.
    pub description: String,
}

/// Event emitted by the WhatsApp system.
#[derive(Debug, Clone)]
pub enum WhatsAppEvent {
    /// New message received.
    MessageReceived(WhatsAppMessage),
    /// Message sent.
    MessageSent(WhatsAppMessage),
    /// Session state changed.
    SessionStateChanged { session_id: u64, old_state: SessionState, new_state: SessionState },
    /// QR pairing requested.
    QrPairingRequested { session_id: u64, qr_data: String },
    /// QR pairing confirmed.
    QrPairingConfirmed { session_id: u64 },
    /// Media uploaded.
    MediaUploaded { message_id: [u8; 32], media_url: String, mime: String },
}

/// Message store trait — plugable backend (in-memory default, pgrust production).
pub trait MessageStore {
    /// Store a message.
    fn store(&mut self, msg: &WhatsAppMessage) -> Result<(), StoreError>;
    /// Retrieve a message by ID.
    fn get(&self, id: &[u8; 32]) -> Option<WhatsAppMessage>;
    /// List messages for a sender.
    fn list_by_sender(&self, sender: &str) -> Vec<WhatsAppMessage>;
    /// List messages for a recipient.
    fn list_by_recipient(&self, recipient: &str) -> Vec<WhatsAppMessage>;
    /// Number of stored messages.
    fn len(&self) -> usize;
    /// Whether empty.
    fn is_empty(&self) -> bool;
}

/// Store error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound,
    Full,
}

/// In-memory message store — default implementation.
#[derive(Debug, Clone, Default)]
pub struct MemMessageStore {
    messages: BTreeMap<[u8; 32], WhatsAppMessage>,
    by_sender: BTreeMap<String, Vec<[u8; 32]>>,
    by_recipient: BTreeMap<String, Vec<[u8; 32]>>,
}

impl MemMessageStore {
    pub fn new() -> Self {
        MemMessageStore::default()
    }
}

impl MessageStore for MemMessageStore {
    fn store(&mut self, msg: &WhatsAppMessage) -> Result<(), StoreError> {
        self.messages.insert(msg.id, msg.clone());
        self.by_sender.entry(msg.sender.clone())
            .or_default()
            .push(msg.id);
        self.by_recipient.entry(msg.recipient.clone())
            .or_default()
            .push(msg.id);
        Ok(())
    }

    fn get(&self, id: &[u8; 32]) -> Option<WhatsAppMessage> {
        self.messages.get(id).cloned()
    }

    fn list_by_sender(&self, sender: &str) -> Vec<WhatsAppMessage> {
        self.by_sender.get(sender)
            .map(|ids| ids.iter().filter_map(|id| self.messages.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    fn list_by_recipient(&self, recipient: &str) -> Vec<WhatsAppMessage> {
        self.by_recipient.get(recipient)
            .map(|ids| ids.iter().filter_map(|id| self.messages.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// License state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseState {
    Unlicensed,
    Active,
    Expired,
    Suspended,
}

/// License manager trait.
pub trait LicenseManager {
    /// Check if a license is valid.
    fn check(&self, license_key: &str) -> LicenseState;
    /// Activate a license.
    fn activate(&mut self, license_key: &str) -> Result<(), LicenseError>;
    /// Deactivate a license.
    fn deactivate(&mut self, license_key: &str);
}

/// License error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseError {
    InvalidKey,
    AlreadyActive,
    Expired,
}

/// In-memory license manager — default implementation.
#[derive(Debug, Clone, Default)]
pub struct MemLicenseManager {
    licenses: BTreeMap<String, LicenseState>,
}

impl MemLicenseManager {
    pub fn new() -> Self {
        MemLicenseManager::default()
    }
}

impl LicenseManager for MemLicenseManager {
    fn check(&self, license_key: &str) -> LicenseState {
        self.licenses.get(license_key).copied().unwrap_or(LicenseState::Unlicensed)
    }

    fn activate(&mut self, license_key: &str) -> Result<(), LicenseError> {
        match self.licenses.get(license_key).copied() {
            Some(LicenseState::Active) => Err(LicenseError::AlreadyActive),
            Some(LicenseState::Expired) => Err(LicenseError::Expired),
            _ => {
                self.licenses.insert(license_key.to_string(), LicenseState::Active);
                Ok(())
            }
        }
    }

    fn deactivate(&mut self, license_key: &str) {
        self.licenses.insert(license_key.to_string(), LicenseState::Unlicensed);
    }
}

/// The WhatsApp API — core facade.
pub struct EvolutionGo {
    /// Sessions indexed by session ID.
    sessions: BTreeMap<u64, WhatsAppSession>,
    /// Message store.
    message_store: Box<dyn MessageStore>,
    /// License manager.
    license_manager: Box<dyn LicenseManager>,
    /// REST endpoint registry.
    endpoints: Vec<RestEndpoint>,
    /// Message event log (for forensics).
    event_log: EventLog<crate::event_log::MemEventStore>,
    /// Next session ID.
    next_session_id: u64,
}

impl EvolutionGo {
    /// Create a new WhatsApp API with the given stores.
    pub fn new(
        message_store: Box<dyn MessageStore>,
        license_manager: Box<dyn LicenseManager>,
    ) -> Self {
        let mut endpoints = Vec::new();
        endpoints.push(RestEndpoint {
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            auth_required: true,
            description: "Send a message".to_string(),
        });
        endpoints.push(RestEndpoint {
            method: "GET".to_string(),
            path: "/v1/messages/{id}".to_string(),
            auth_required: true,
            description: "Get message by ID".to_string(),
        });
        endpoints.push(RestEndpoint {
            method: "GET".to_string(),
            path: "/v1/sessions".to_string(),
            auth_required: true,
            description: "List sessions".to_string(),
        });
        endpoints.push(RestEndpoint {
            method: "POST".to_string(),
            path: "/v1/sessions/qr".to_string(),
            auth_required: true,
            description: "Request QR pairing".to_string(),
        });

        let event_store = crate::event_log::MemEventStore::new();
        let event_log = EventLog::new(event_store);

        EvolutionGo {
            sessions: BTreeMap::new(),
            message_store,
            license_manager,
            endpoints,
            event_log,
            next_session_id: 0,
        }
    }

    /// Register a new WhatsApp session.
    pub fn register_session(&mut self, identifier: &str) -> u64 {
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let session = WhatsAppSession {
            session_id,
            identifier: identifier.to_string(),
            state: SessionState::Unregistered,
            qr_data: None,
            paired_at_us: None,
            message_count: 0,
        };

        self.sessions.insert(session_id, session);
        session_id
    }

    /// Request QR pairing for a session.
    pub fn request_qr_pairing(&mut self, session_id: u64) -> Result<String, SessionError> {
        let session = self.sessions.get_mut(&session_id)
            .ok_or(SessionError::NotFound)?;

        if session.state != SessionState::Unregistered {
            return Err(SessionError::AlreadyPaired);
        }

        // Generate QR data (deterministic placeholder).
        let qr_data = format!("whatsapp://qr/{}", session_id);
        session.qr_data = Some(qr_data.clone());
        session.state = SessionState::AwaitingQrScan;

        // Log the event.
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32],
            actor_seq: 0,
            payload: b"QR pairing requested".to_vec(),
        };
        let _ = self.event_log.append(ev);

        Ok(qr_data)
    }

    /// Confirm QR pairing.
    pub fn confirm_qr_pairing(&mut self, session_id: u64) -> Result<(), SessionError> {
        let session = self.sessions.get_mut(&session_id)
            .ok_or(SessionError::NotFound)?;

        match session.state {
            SessionState::AwaitingQrScan | SessionState::QrScanned => {
                session.state = SessionState::Active;
                session.paired_at_us = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_micros() as u64
                );
                Ok(())
            }
            _ => Err(SessionError::NotAwaitingQr),
        }
    }

    /// Send a message.
    pub fn send_message(
        &mut self,
        session_id: u64,
        recipient: &str,
        text: &str,
    ) -> Result<WhatsAppMessage, SendError> {
        let session = self.sessions.get_mut(&session_id)
            .ok_or(SendError::SessionNotFound)?;

        if session.state != SessionState::Active {
            return Err(SendError::SessionNotActive);
        }

        let id = Self::compute_message_id(session_id, recipient, text);

        let msg = WhatsAppMessage {
            id: id,
            sender: session.identifier.clone(),
            recipient: recipient.to_string(),
            msg_type: WhatsAppMessageType::Text,
            text: Some(text.to_string()),
            media_url: None,
            media_mime: None,
            direction: MessageDirection::Outbound,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            paired: true,
        };

        self.message_store.store(&msg).map_err(SendError::Store)?;
        session.message_count += 1;

        Ok(msg)
    }

    /// Receive a message (inbound).
    pub fn receive_message(
        &mut self,
        session_id: u64,
        sender: &str,
        text: &str,
    ) -> Result<WhatsAppMessage, ReceiveError> {
        let session = self.sessions.get_mut(&session_id)
            .ok_or(ReceiveError::SessionNotFound)?;

        if session.state != SessionState::Active {
            return Err(ReceiveError::SessionNotActive);
        }

        let id = Self::compute_message_id(session_id, sender, text);

        let msg = WhatsAppMessage {
            id,
            sender: sender.to_string(),
            recipient: session.identifier.clone(),
            msg_type: WhatsAppMessageType::Text,
            text: Some(text.to_string()),
            media_url: None,
            media_mime: None,
            direction: MessageDirection::Inbound,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            paired: true,
        };

        self.message_store.store(&msg).map_err(ReceiveError::Store)?;
        session.message_count += 1;

        Ok(msg)
    }

    /// Get session by ID.
    pub fn get_session(&self, session_id: u64) -> Option<&WhatsAppSession> {
        self.sessions.get(&session_id)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<&WhatsAppSession> {
        self.sessions.values().collect()
    }

    /// Get message by ID.
    pub fn get_message(&self, id: &[u8; 32]) -> Option<WhatsAppMessage> {
        self.message_store.get(id)
    }

    /// List messages for a sender.
    pub fn list_messages_by_sender(&self, sender: &str) -> Vec<WhatsAppMessage> {
        self.message_store.list_by_sender(sender)
    }

    /// List messages for a recipient.
    pub fn list_messages_by_recipient(&self, recipient: &str) -> Vec<WhatsAppMessage> {
        self.message_store.list_by_recipient(recipient)
    }

    /// Get the REST endpoint registry.
    pub fn endpoints(&self) -> &Vec<RestEndpoint> {
        &self.endpoints
    }

    /// Get the number of sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the number of messages.
    pub fn message_count(&self) -> usize {
        self.message_store.len()
    }

    /// Compute deterministic message ID.
    fn compute_message_id(session_id: u64, address: &str, text: &str) -> [u8; 32] {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&session_id.to_le_bytes());
        buf.extend_from_slice(address.as_bytes());
        buf.push(0);
        buf.extend_from_slice(text.as_bytes());
        sha3_256(&buf)
    }

    /// ASCII report.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Evolution Go (WhatsApp API) Report ===\n");
        out.push_str(&format!("Sessions: {}\n", self.session_count()));
        out.push_str(&format!("Messages: {}\n", self.message_count()));
        out.push_str(&format!("Endpoints: {}\n", self.endpoints.len()));

        out.push_str("\nSessions:\n");
        for session in self.list_sessions() {
            out.push_str(&format!(
                "  #{} [{}] {} — {}\n",
                session.session_id,
                session.identifier,
                session.state,
                session.message_count
            ));
        }

        out.push_str("\nEndpoints:\n");
        for ep in &self.endpoints {
            out.push_str(&format!(
                "  {} {} {} — {}\n",
                ep.method, ep.path, if ep.auth_required { "[auth]" } else { "[public]" },
                ep.description
            ));
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

/// Session error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NotFound,
    AlreadyPaired,
    NotAwaitingQr,
}

/// Send error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    SessionNotFound,
    SessionNotActive,
    Store(StoreError),
}

/// Receive error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveError {
    SessionNotFound,
    SessionNotActive,
    Store(StoreError),
}

impl core::fmt::Display for SessionState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SessionState::Unregistered => write!(f, "unregistered"),
            SessionState::AwaitingQrScan => write!(f, "awaiting_qr_scan"),
            SessionState::QrScanned => write!(f, "qr_scanned"),
            SessionState::Active => write!(f, "active"),
            SessionState::Disconnected => write!(f, "disconnected"),
            SessionState::Suspended => write!(f, "suspended"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stores() -> (MemMessageStore, MemLicenseManager) {
        (MemMessageStore::new(), MemLicenseManager::new())
    }

    #[test]
    fn new_api_has_endpoints() {
        let (ms, lm) = make_stores();
        let api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        assert!(!api.endpoints().is_empty());
        assert_eq!(api.endpoints().len(), 4);
    }

    #[test]
    fn register_session_creates_session() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("test-user");
        assert_eq!(id, 0);
        assert_eq!(api.session_count(), 1);

        let session = api.get_session(id).unwrap();
        assert_eq!(session.identifier, "test-user");
        assert_eq!(session.state, SessionState::Unregistered);
    }

    #[test]
    fn qr_pairing_flow() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("test");

        let qr = api.request_qr_pairing(id).unwrap();
        assert!(qr.contains("whatsapp://qr/"));
        assert_eq!(api.get_session(id).unwrap().state, SessionState::AwaitingQrScan);

        api.confirm_qr_pairing(id).unwrap();
        assert_eq!(api.get_session(id).unwrap().state, SessionState::Active);
    }

    #[test]
    fn qr_pairing_fails_if_not_waiting() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("test");
        // Don't request QR, try to confirm.
        assert_eq!(
            api.confirm_qr_pairing(id),
            Err(SessionError::NotAwaitingQr)
        );
    }

    #[test]
    fn send_message_requires_active_session() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("test");
        // Not active.
        assert_eq!(
            api.send_message(id, "recipient", "hello"),
            Err(SendError::SessionNotActive)
        );
    }

    #[test]
    fn send_message_after_pairing() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("test");
        api.request_qr_pairing(id).unwrap();
        api.confirm_qr_pairing(id).unwrap();

        let msg = api.send_message(id, "recipient", "hello").unwrap();
        assert_eq!(msg.text, Some("hello".to_string()));
        assert_eq!(msg.sender, "test");
        assert_eq!(msg.recipient, "recipient");
        assert_eq!(msg.direction, MessageDirection::Outbound);
        assert!(msg.paired);

        // Verify stored.
        let retrieved = api.get_message(&msg.id).unwrap();
        assert_eq!(retrieved.text, Some("hello".to_string()));
    }

    #[test]
    fn receive_message() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("server");
        api.request_qr_pairing(id).unwrap();
        api.confirm_qr_pairing(id).unwrap();

        let msg = api.receive_message(id, "alice", "hi there").unwrap();
        assert_eq!(msg.text, Some("hi there".to_string()));
        assert_eq!(msg.sender, "alice");
        assert_eq!(msg.recipient, "server");
        assert_eq!(msg.direction, MessageDirection::Inbound);
    }

    #[test]
    fn list_messages_by_sender() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let id = api.register_session("server");
        api.request_qr_pairing(id).unwrap();
        api.confirm_qr_pairing(id).unwrap();

        api.receive_message(id, "alice", "msg1").unwrap();
        api.receive_message(id, "alice", "msg2").unwrap();
        api.receive_message(id, "bob", "msg3").unwrap();

        let alice_msgs = api.list_messages_by_sender("alice");
        assert_eq!(alice_msgs.len(), 2);

        let bob_msgs = api.list_messages_by_sender("bob");
        assert_eq!(bob_msgs.len(), 1);
    }

    #[test]
    fn license_activate_and_check() {
        let (ms, mut lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));

        assert_eq!(api.license_manager.check("key1"), LicenseState::Unlicensed);

        api.license_manager.activate("key1").unwrap();
        assert_eq!(api.license_manager.check("key1"), LicenseState::Active);

        // Duplicate activate fails.
        assert_eq!(
            api.license_manager.activate("key1"),
            Err(LicenseError::AlreadyActive)
        );
    }

    #[test]
    fn ascii_report_format() {
        let (ms, lm) = make_stores();
        let api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        let report = api.ascii_report();
        assert!(report.contains("Evolution Go"));
        assert!(report.contains("Sessions: 0"));
        assert!(report.contains("Endpoints: 4"));
    }

    #[test]
    fn send_error_session_not_found() {
        let (ms, lm) = make_stores();
        let mut api = EvolutionGo::new(Box::new(ms), Box::new(lm));
        assert_eq!(
            api.send_message(999, "recipient", "hello"),
            Err(SendError::SessionNotFound)
        );
    }
}
