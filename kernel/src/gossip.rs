//! gossip.rs — std host shim. The pure pub/sub gossip (`GossipTopic`,
//! `GossipMessage`, `GossipBus`, `GossipNode`) lives in `dowiz_core::gossip`.
//! The clock-stamped entry points (`GossipMessage::new`, `GossipBus::publish`,
//! `GossipNode::publish`) are wrapped here as free functions that stamp
//! `crate::now_ms()`.

pub use dowiz_core::gossip::*;

/// `GossipMessage::new` stamped with the current wall clock.
pub fn message_new(topic: GossipTopic, payload: Vec<u8>, seq: u64) -> GossipMessage {
    GossipMessage::new(topic, payload, seq, crate::now_ms())
}

/// `GossipBus::publish` stamped with the current wall clock.
pub fn publish_now(bus: &mut GossipBus, topic: GossipTopic, payload: &[u8]) {
    bus.publish(topic, payload, crate::now_ms());
}

/// `GossipNode::publish` stamped with the current wall clock.
pub fn node_publish(node: &GossipNode, bus: &mut GossipBus, topic: GossipTopic, payload: &[u8]) {
    node.publish(bus, topic, payload, crate::now_ms());
}
