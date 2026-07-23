//! gossip.rs — Lightweight topic-based pub/sub gossip for inter-module data
//! propagation.
//!
//! Inspired by `crate::mesh::GossipImport`'s fan-out pattern but without the
//! PQ signing overhead. Used for fast propagation of telemetry, predictions,
//! and state changes between engine modules (predictor → resilience →
//! engine_loop → offline).
//!
//! ## Architecture
//! - `GossipTopic` — typed topic identifier (CPU_LOAD, PREDICTION, BACKUP, ...)
//! - `GossipMessage` — a timestamped message on a topic
//! - `GossipNode` — subscribes to topics, fans out to peers
//! - `GossipBus` — shared message bus connecting all nodes
//!
//! ## Usage
//! ```
//! use dowiz_kernel::gossip::{GossipBus, GossipTopic, GossipMessage};
//!
//! let mut bus = GossipBus::new();
//! let id = bus.subscribe(GossipTopic::Telemetry);
//! bus.publish(GossipTopic::Telemetry, "cpu:0.5".as_bytes());
//! let msgs = bus.drain(id);
//! assert!(!msgs.is_empty());
//! ```

use std::collections::{HashMap, VecDeque};

/// A message topic in the gossip bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GossipTopic {
    /// System telemetry (cpu, memory, frame time).
    Telemetry,
    /// Predictor outcomes.
    Prediction,
    /// Resilience state changes.
    Resilience,
    /// Backup snapshots.
    Backup,
    /// Error/failure events.
    Error,
    /// State synchronization.
    StateSync,
    /// Custom/debug topic.
    Custom(u8),
}

impl GossipTopic {
    pub fn name(&self) -> &'static str {
        match self {
            GossipTopic::Telemetry => "telemetry",
            GossipTopic::Prediction => "prediction",
            GossipTopic::Resilience => "resilience",
            GossipTopic::Backup => "backup",
            GossipTopic::Error => "error",
            GossipTopic::StateSync => "state_sync",
            GossipTopic::Custom(_) => "custom",
        }
    }
}

/// A single gossip message: timestamped payload on a topic.
#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub topic: GossipTopic,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
    pub seq: u64,
}

impl GossipMessage {
    pub fn new(topic: GossipTopic, payload: Vec<u8>, seq: u64) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        GossipMessage { topic, payload, timestamp_ms: ts, seq }
    }

    /// Interpret payload as UTF-8 string (for inspection).
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.payload).unwrap_or("<binary>")
    }
}

/// A subscriber ID (index into the bus's subscriber list).
pub type SubscriberId = usize;

/// Shared message bus: topics → subscribers → message queues.
///
/// Thread-safe for single-threaded use (the engine runs single-threaded).
/// For multi-threaded use, wrap in `Mutex` or use atomic sequencer.
#[derive(Debug, Clone)]
pub struct GossipBus {
    subscribers: HashMap<GossipTopic, Vec<SubscriberId>>,
    queues: HashMap<SubscriberId, VecDeque<GossipMessage>>,
    next_id: SubscriberId,
    seq: u64,
}

impl GossipBus {
    pub fn new() -> Self {
        GossipBus {
            subscribers: HashMap::new(),
            queues: HashMap::new(),
            next_id: 0,
            seq: 0,
        }
    }

    /// Subscribe to a topic. Returns a subscriber ID for draining messages.
    pub fn subscribe(&mut self, topic: GossipTopic) -> SubscriberId {
        let id = self.next_id;
        self.next_id += 1;
        self.subscribers.entry(topic).or_default().push(id);
        self.queues.entry(id).or_default();
        id
    }

    /// Subscribe to multiple topics at once.
    pub fn subscribe_all(&mut self, topics: &[GossipTopic]) -> SubscriberId {
        let id = self.next_id;
        self.next_id += 1;
        for &topic in topics {
            self.subscribers.entry(topic).or_default().push(id);
        }
        self.queues.entry(id).or_default();
        id
    }

    /// Unsubscribe a subscriber from a specific topic.
    pub fn unsubscribe(&mut self, id: SubscriberId, topic: GossipTopic) {
        if let Some(subs) = self.subscribers.get_mut(&topic) {
            subs.retain(|&s| s != id);
        }
    }

    /// Publish a message to all subscribers of a topic.
    pub fn publish(&mut self, topic: GossipTopic, payload: &[u8]) {
        self.seq += 1;
        let msg = GossipMessage::new(topic, payload.to_vec(), self.seq);
        if let Some(subs) = self.subscribers.get(&topic) {
            for &sid in subs {
                if let Some(queue) = self.queues.get_mut(&sid) {
                    queue.push_back(msg.clone());
                    // Bound queue size
                    if queue.len() > 1000 {
                        queue.pop_front();
                    }
                }
            }
        }
    }

    /// Drain all pending messages for a subscriber.
    pub fn drain(&mut self, id: SubscriberId) -> Vec<GossipMessage> {
        self.queues.get_mut(&id)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    }

    /// Check if a subscriber has pending messages.
    pub fn has_messages(&self, id: SubscriberId) -> bool {
        self.queues.get(&id).map(|q| !q.is_empty()).unwrap_or(false)
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.queues.len()
    }

    /// Total messages published so far.
    pub fn total_published(&self) -> u64 {
        self.seq
    }

    /// Remove a subscriber entirely.
    pub fn remove_subscriber(&mut self, id: SubscriberId) {
        self.queues.remove(&id);
        for subs in self.subscribers.values_mut() {
            subs.retain(|&s| s != id);
        }
    }
}

impl Default for GossipBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: create telemetry payload from key-value pairs.
pub fn telemetry_payload(key: &str, value: f64) -> Vec<u8> {
    format!("{}:{:.6}", key, value).into_bytes()
}

/// Convenience: parse a telemetry payload back into key + value.
pub fn parse_telemetry(payload: &[u8]) -> Option<(&str, f64)> {
    let s = std::str::from_utf8(payload).ok()?;
    let colon = s.find(':')?;
    let key = &s[..colon];
    let val: f64 = s[colon + 1..].parse().ok()?;
    Some((key, val))
}

/// Gossip node: lightweight wrapper that connects a module to the gossip bus.
///
/// Each module (predictor, resilience, engine_loop, offline) gets a GossipNode
/// that subscribes to relevant topics and publishes its own data.
#[derive(Debug, Clone)]
pub struct GossipNode {
    pub id: SubscriberId,
    pub name: String,
}

impl GossipNode {
    /// Register this node on the bus, subscribing to the given topics.
    pub fn register(bus: &mut GossipBus, name: &str, topics: &[GossipTopic]) -> Self {
        let id = bus.subscribe_all(topics);
        GossipNode { id, name: name.to_string() }
    }

    /// Publish a message through this node.
    pub fn publish(&self, bus: &mut GossipBus, topic: GossipTopic, payload: &[u8]) {
        bus.publish(topic, payload);
    }

    /// Drain all messages for this node.
    pub fn drain(&self, bus: &mut GossipBus) -> Vec<GossipMessage> {
        bus.drain(self.id)
    }

    /// Check for pending messages.
    pub fn has_messages(&self, bus: &GossipBus) -> bool {
        bus.has_messages(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_and_publish() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        bus.publish(GossipTopic::Telemetry, b"cpu:0.5");
        assert!(bus.has_messages(id));
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].as_str(), "cpu:0.5");
    }

    #[test]
    fn multiple_subscribers() {
        let mut bus = GossipBus::new();
        let a = bus.subscribe(GossipTopic::Prediction);
        let b = bus.subscribe(GossipTopic::Prediction);
        bus.publish(GossipTopic::Prediction, b"latency:0.8");
        assert_eq!(bus.drain(a).len(), 1);
        assert_eq!(bus.drain(b).len(), 1);
    }

    #[test]
    fn topic_isolation() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        bus.publish(GossipTopic::Backup, b"snapshot");
        assert!(!bus.has_messages(id));
        bus.publish(GossipTopic::Telemetry, b"cpu:0.3");
        assert!(bus.has_messages(id));
    }

    #[test]
    fn unsubscribe() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Error);
        bus.unsubscribe(id, GossipTopic::Error);
        bus.publish(GossipTopic::Error, b"err");
        assert!(!bus.has_messages(id));
    }

    #[test]
    fn gossip_node_register() {
        let mut bus = GossipBus::new();
        let node = GossipNode::register(&mut bus, "predictor", &[
            GossipTopic::Telemetry,
            GossipTopic::Backup,
        ]);
        node.publish(&mut bus, GossipTopic::Telemetry, b"mem:0.7");
        let msgs = node.drain(&mut bus);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].topic, GossipTopic::Telemetry);
    }

    #[test]
    fn telemetry_payload_roundtrip() {
        let payload = telemetry_payload("cpu_load", 0.75);
        let (key, val) = parse_telemetry(&payload).unwrap();
        assert_eq!(key, "cpu_load");
        assert!((val - 0.75).abs() < 1e-6);
    }

    #[test]
    fn queue_bounded() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Custom(1));
        for _ in 0..1500 {
            bus.publish(GossipTopic::Custom(1), b"data");
        }
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), 1000, "queue bounded to 1000");
    }

    #[test]
    fn multiple_topics_one_subscriber() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe_all(&[GossipTopic::Telemetry, GossipTopic::Error]);
        bus.publish(GossipTopic::Telemetry, b"cpu:0.5");
        bus.publish(GossipTopic::Error, b"err:1");
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), 2);
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn gossip_no_subscribers_no_crash() {
        let mut bus = GossipBus::new();
        bus.publish(GossipTopic::Telemetry, b"data");
        // Must not panic with no subscribers
        assert_eq!(bus.total_published(), 1);
    }

    #[test]
    fn gossip_subscriber_no_matching_topic() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        bus.publish(GossipTopic::Error, b"err");
        assert!(!bus.has_messages(id),
            "subscriber must not receive messages from unsubscribed topics");
    }

    #[test]
    fn gossip_remove_and_resubscribe() {
        let mut bus = GossipBus::new();
        let id1 = bus.subscribe(GossipTopic::Telemetry);
        bus.remove_subscriber(id1);
        assert!(!bus.has_messages(id1), "removed subscriber must have no messages");
        let id2 = bus.subscribe(GossipTopic::Telemetry);
        assert!(id2 != id1, "new subscriber must get new ID");
        bus.publish(GossipTopic::Telemetry, b"after_resub");
        assert!(bus.has_messages(id2), "new subscriber must receive messages");
    }

    #[test]
    fn gossip_empty_payload() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Custom(0));
        bus.publish(GossipTopic::Custom(0), b"");
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].payload.is_empty());
        assert_eq!(msgs[0].as_str(), "");
    }

    #[test]
    fn gossip_invalid_utf8_payload() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Error);
        bus.publish(GossipTopic::Error, &[0xFF, 0xFE, 0x80]);
        let msgs = bus.drain(id);
        assert_eq!(msgs[0].as_str(), "<binary>",
            "invalid UTF-8 must be shown as <binary>");
    }

    #[test]
    fn gossip_flood_100k_messages() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        for i in 0..100_000 {
            bus.publish(GossipTopic::Telemetry, &telemetry_payload("msg", i as f64));
        }
        assert_eq!(bus.total_published(), 100_000);
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), 1000, "queue must be bounded to 1000: {}", msgs.len());
    }

    #[test]
    fn gossip_multiple_subscribers_same_topic() {
        let mut bus = GossipBus::new();
        let subs: Vec<_> = (0..50).map(|_| bus.subscribe(GossipTopic::Prediction)).collect();
        bus.publish(GossipTopic::Prediction, b"test");
        for &s in &subs {
            assert_eq!(bus.drain(s).len(), 1, "subscriber {s} must receive message");
        }
    }

    #[test]
    fn gossip_node_register_drain_cycle() {
        let mut bus = GossipBus::new();
        let node = GossipNode::register(&mut bus, "tester", &[
            GossipTopic::Telemetry,
            GossipTopic::Resilience,
        ]);
        bus.publish(GossipTopic::Telemetry, b"t1");
        bus.publish(GossipTopic::Resilience, b"r1");
        bus.publish(GossipTopic::Telemetry, b"t2");
        let msgs = node.drain(&mut bus);
        assert_eq!(msgs.len(), 3, "node must receive all published messages");
    }

    #[test]
    fn telemetry_payload_parse_chaos() {
        // Normal
        assert!(parse_telemetry(b"cpu:0.5").is_some());
        // Missing colon
        assert!(parse_telemetry(b"cpu0.5").is_none());
        // Empty
        assert!(parse_telemetry(b"").is_none());
        // Non-numeric value
        assert!(parse_telemetry(b"key:abc").is_none());
        // Multiple colons → parse fails on "1:2:3"
        assert!(parse_telemetry(b"key:1:2:3").is_none());
    }

    #[test]
    fn gossip_remove_nonexistent_subscriber() {
        let mut bus = GossipBus::new();
        bus.remove_subscriber(999);
        // Must not panic
    }

    #[test]
    fn gossip_unsubscribe_from_unsubscribed_topic() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Error);
        bus.unsubscribe(id, GossipTopic::Telemetry); // never subscribed to this
        bus.publish(GossipTopic::Error, b"test");
        assert!(bus.has_messages(id), "must still receive subscribed topic messages");
    }

    // ── JAMMING / SPOOFING / INJECTION ──────────────────────────────────

    #[test]
    fn gossip_jamming_binary_payloads() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        // Binary jamming: null bytes, high bytes, escape sequences
        let jamming_payloads: [&[u8]; 5] = [
            &[0x00, 0x00, 0x00, 0x00],
            &[0xFF, 0xFF, 0xFF, 0xFF],
            &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05],
            b"\x1b[2J\x1b[H", // ANSI escape
            &[0xDE, 0xAD, 0xBE, 0xEF],
        ];
        for payload in &jamming_payloads {
            bus.publish(GossipTopic::Telemetry, payload);
        }
        let msgs = bus.drain(id);
        assert_eq!(msgs.len(), jamming_payloads.len(),
            "all jamming payloads must be accepted (gossip is binary-safe)");
        for (i, msg) in msgs.iter().enumerate() {
            assert_eq!(msg.payload.as_slice(), jamming_payloads[i],
                "jamming payload {i} must be preserved byte-exact");
        }
    }

    #[test]
    fn gossip_spoofed_topic_injection() {
        let mut bus = GossipBus::new();
        let id_sensor = bus.subscribe(GossipTopic::Telemetry);
        let id_pred = bus.subscribe(GossipTopic::Prediction);
        let id_err = bus.subscribe(GossipTopic::Error);
        // Publish to topics that should NOT reach subscribers of other topics
        bus.publish(GossipTopic::Telemetry, b"sensor_data");
        bus.publish(GossipTopic::Prediction, b"pred_data");
        bus.publish(GossipTopic::Error, b"err_data");
        // Drain from each subscriber
        let sensor_msgs = bus.drain(id_sensor);
        let pred_msgs = bus.drain(id_pred);
        let err_msgs = bus.drain(id_err);
        assert_eq!(sensor_msgs.len(), 1);
        assert_eq!(sensor_msgs[0].topic, GossipTopic::Telemetry);
        assert_eq!(pred_msgs.len(), 1);
        assert_eq!(pred_msgs[0].topic, GossipTopic::Prediction);
        assert_eq!(err_msgs.len(), 1);
        assert_eq!(err_msgs[0].topic, GossipTopic::Error);
    }

    #[test]
    fn gossip_topic_spoofing_via_wrong_topic() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        // Publish to a different topic that the subscriber didn't subscribe to
        bus.publish(GossipTopic::Prediction, b"should_not_leak");
        assert!(!bus.has_messages(id),
            "telemetry subscriber must not receive Prediction messages");
    }

    #[test]
    fn gossip_injection_too_large_payload_truncated() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        let large: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        bus.publish(GossipTopic::Telemetry, &large);
        let msgs = bus.drain(id);
        // Each message has capacity limit; if too large, may be truncated
        // But must not panic or corrupt the bus
        assert!(!msgs.is_empty(), "bus must accept large payload");
    }

    /// Penetration: rapid subscribe/unsubscribe must not leak resources
    #[test]
    fn gossip_penetration_rapid_sub_unsub() {
        let mut bus = GossipBus::new();
        for _ in 0..1000 {
            let id = bus.subscribe(GossipTopic::Telemetry);
            bus.remove_subscriber(id);
        }
        // Bus must still accept normal operations
        let id = bus.subscribe(GossipTopic::Telemetry);
        bus.publish(GossipTopic::Telemetry, b"post_stress");
        assert!(bus.has_messages(id));
    }

    /// Time‑critical: messages preserve timestamp ordering
    #[test]
    fn gossip_timestamp_monotonic_per_subscriber() {
        let mut bus = GossipBus::new();
        let id = bus.subscribe(GossipTopic::Telemetry);
        for i in 0..50 {
            bus.publish(GossipTopic::Telemetry, &telemetry_payload("idx", i as f64));
        }
        let msgs = bus.drain(id);
        let mut prev_ts = 0u64;
        for msg in &msgs {
            assert!(msg.timestamp_ms >= prev_ts,
                "messages must be in chronological order: {} < {}", msg.timestamp_ms, prev_ts);
            assert!(msg.seq > 0, "sequence number must be > 0");
            prev_ts = msg.timestamp_ms;
        }
    }

    /// Jitter: messages received within a short window must all be deliverable
    #[test]
    fn gossip_jitter_burst_reception() {
        let mut bus = GossipBus::new();
        let ids: Vec<_> = (0..4).map(|_| bus.subscribe(GossipTopic::Telemetry)).collect();
        for _ in 0..100 {
            bus.publish(GossipTopic::Telemetry, b"burst");
        }
        for &id in &ids {
            let msgs = bus.drain(id);
            assert_eq!(msgs.len(), 100, "each subscriber must get 100 burst messages");
        }
    }
}
