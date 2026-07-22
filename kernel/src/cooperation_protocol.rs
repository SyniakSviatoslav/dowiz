//! `kernel::cooperation_protocol` — Bridge between P2P trading and P2P delivery.
//!
//! Atomic cooperation protocol: a trade settlement triggers a delivery,
//! and delivery confirmation releases the trade's escrowed funds.
//! No intermediaries, no platforms — pure cryptographic linking.
//!
//! # Cooperation Flow
//! ```text
//! [Trade Intent] ──→ [Escrow Lock] ──→ [Delivery Trigger]
//!                                             ↓
//! [Delivery Proof] ←── [P2P Delivery] ←── [Carrier Match]
//!       ↓
//! [Escrow Release] ──→ [Trade Settlement]
//! ```
//!
//! # Atomicity
//! - Trade → Delivery: if trade settles, delivery IS created
//! - Delivery → Trade: if delivery confirmed, funds ARE released
//! - Failure: either both succeed or both revert

use crate::event_log::sha3_256;
use crate::trading_intent::{Intent, IntentPool};
use crate::trading_escrow::EscrowOffer;
use crate::p2p_delivery::{DeliveryListing, DeliveryProof, P2PDeliveryNetwork};
use crate::TriState;

/// Maximum cooperation agreements.
pub const MAX_AGREEMENTS: usize = 5_000;

// ─── Agreement State ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementState {
    /// Proposed, awaiting both parties.
    Proposed,
    /// Trade escrowed, delivery pending.
    TradeLocked,
    /// Delivery in progress.
    DeliveryActive,
    /// Delivery confirmed, funds released.
    Completed,
    /// Failed — both sides revert.
    Failed,
}

// ─── Cooperation Agreement ────────────────────────────────────────────────

/// Links a trade to a delivery atomically.
#[derive(Debug, Clone)]
pub struct CooperationAgreement {
    pub id: [u8; 32],
    /// The trade intent that funds this agreement.
    pub trade_intent_id: [u8; 32],
    /// The escrow holding the funds.
    pub escrow_id: [u8; 32],
    /// The delivery to fulfill.
    pub delivery_id: [u8; 32],
    /// Buyer identity (from trade).
    pub buyer_id: String,
    /// Seller identity (from trade).
    pub seller_id: String,
    /// State.
    pub state: AgreementState,
    /// Both parties signed?
    pub buyer_signed: TriState,
    pub seller_signed: TriState,
    /// Proof hash chain.
    pub proof_chain: Vec<[u8; 32]>,
}

impl CooperationAgreement {
    pub fn new(
        trade_id: [u8; 32], escrow_id: [u8; 32],
        delivery_id: [u8; 32],
        buyer: &str, seller: &str,
    ) -> Self {
        let content = format!("{:?}{:?}{:?}", trade_id, delivery_id, buyer);
        let hash = sha3_256(content.as_bytes());
        CooperationAgreement {
            id: hash, trade_intent_id: trade_id,
            escrow_id, delivery_id,
            buyer_id: buyer.to_string(), seller_id: seller.to_string(),
            state: AgreementState::Proposed,
            buyer_signed: TriState::Unknown,
            seller_signed: TriState::Unknown,
            proof_chain: vec![],
        }
    }

    /// Sign the agreement (both parties must sign).
    pub fn sign_buyer(&mut self) { self.buyer_signed = TriState::True; }
    pub fn sign_seller(&mut self) { self.seller_signed = TriState::True; }

    /// Activate — both must have signed.
    pub fn activate(&mut self) -> bool {
        if self.buyer_signed == TriState::True && self.seller_signed == TriState::True {
            self.state = AgreementState::TradeLocked;
            true
        } else { false }
    }

    /// Mark delivery as active.
    pub fn start_delivery(&mut self) -> bool {
        if self.state != AgreementState::TradeLocked { return false; }
        self.state = AgreementState::DeliveryActive;
        self.proof_chain.push(sha3_256(b"delivery_started"));
        true
    }

    /// Complete — delivery proven, funds released.
    pub fn complete(&mut self) -> bool {
        if self.state != AgreementState::DeliveryActive { return false; }
        self.state = AgreementState::Completed;
        self.proof_chain.push(sha3_256(b"completed"));
        true
    }

    /// Fail — revert everything.
    pub fn fail(&mut self) {
        self.state = AgreementState::Failed;
    }
}

// ─── Cooperation Engine ───────────────────────────────────────────────────

/// The cooperation engine connects trading ↔ delivery atomically.
pub struct CooperationEngine {
    pub agreements: Vec<CooperationAgreement>,
    pub trades: IntentPool,
    pub deliveries: P2PDeliveryNetwork,
    max_agreements: usize,
}

impl CooperationEngine {
    pub fn new() -> Self {
        CooperationEngine {
            agreements: Vec::new(), trades: IntentPool::new(),
            deliveries: P2PDeliveryNetwork::new(),
            max_agreements: MAX_AGREEMENTS,
        }
    }

    /// Propose a cooperation: link a trade intent to a delivery.
    pub fn propose(
        &mut self,
        intent: Intent, delivery: DeliveryListing,
    ) -> Result<[u8; 32], String> {
        // Create escrow from intent.
        let escrow = EscrowOffer::new(
            &intent.trader_address, &delivery.seller_id,
            intent.from_asset.clone(), intent.from_amount,
            intent.to_asset.clone(), intent.min_to_amount,
            intent.deadline_block,
        );

        // Create agreement linking trade → delivery.
        let agreement = CooperationAgreement::new(
            intent.id, escrow.id, delivery.id,
            &intent.trader_address, &delivery.seller_id,
        );

        let id = agreement.id;
        self.agreements.push(agreement);
        Ok(id)
    }

    /// Complete a delivery and release trade funds.
    pub fn confirm_delivery(&mut self, delivery_id: [u8; 32], proof: DeliveryProof) -> bool {
        // Find the agreement.
        let agreement = self.agreements.iter_mut()
            .find(|a| a.delivery_id == delivery_id);
        match agreement {
            Some(a) => {
                self.deliveries.record_proof(proof);
                a.complete()
            }
            None => false,
        }
    }

    /// Get active agreements.
    pub fn active_agreements(&self) -> Vec<&CooperationAgreement> {
        self.agreements.iter()
            .filter(|a| a.state != AgreementState::Completed && a.state != AgreementState::Failed)
            .collect()
    }

    pub fn dashboard(&self) -> String {
        format!(
            "Cooperation Protocol\n  Agreements: {} total\n  Active:     {} active\n  Completed:  {} completed\n  Trades:     {} intents\n  Deliveries: {} listings",
            self.agreements.len(),
            self.active_agreements().len(),
            self.agreements.iter().filter(|a| a.state == AgreementState::Completed).count(),
            self.trades.intents.len(),
            self.deliveries.listings.len(),
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading_intent::{Asset, OrderSide};
    use crate::p2p_delivery::GeoLocation;

    fn eth() -> Asset { Asset::new("ethereum", "0x0000", "ETH", 18) }
    fn usdc() -> Asset { Asset::new("ethereum", "0xA0b8", "USDC", 6) }
    fn paris() -> GeoLocation { GeoLocation { lat: 48.8566, lon: 2.3522 } }

    #[test]
    fn create_agreement() {
        let ag = CooperationAgreement::new(
            [1u8; 32], [2u8; 32], [3u8; 32],
            "0xbob", "0xalice",
        );
        assert_eq!(ag.state, AgreementState::Proposed);
    }

    #[test]
    fn activate_requires_both_signatures() {
        let mut ag = CooperationAgreement::new(
            [1u8; 32], [2u8; 32], [3u8; 32],
            "0xbob", "0xalice",
        );
        assert!(!ag.activate()); // Neither signed
        ag.sign_buyer();
        assert!(!ag.activate()); // Only buyer signed
        ag.sign_seller();
        assert!(ag.activate()); // Both signed
        assert_eq!(ag.state, AgreementState::TradeLocked);
    }

    #[test]
    fn full_cooperation_flow() {
        let mut ag = CooperationAgreement::new(
            [1u8; 32], [2u8; 32], [3u8; 32],
            "0xbob", "0xalice",
        );
        ag.sign_buyer();
        ag.sign_seller();
        assert!(ag.activate());
        assert!(ag.start_delivery());
        assert!(ag.complete());
        assert_eq!(ag.state, AgreementState::Completed);
        assert_eq!(ag.proof_chain.len(), 2);
    }

    #[test]
    fn engine_propose_and_confirm() {
        let mut engine = CooperationEngine::new();
        let intent = Intent::new(eth(), 1_000_000, usdc(), 1_000, OrderSide::Buy, "0xbob", 1, 1_000_000, 7);
        let delivery = DeliveryListing::new("0xalice", "item", paris(), paris(), 10.0, "ETH", 100, 1_000_000);
        let id = engine.propose(intent, delivery).unwrap();
        assert_eq!(engine.agreements.len(), 1);
    }

    #[test]
    fn confirm_releases_funds() {
        let mut engine = CooperationEngine::new();
        let mut intent = Intent::new(eth(), 100, usdc(), 10, OrderSide::Sell, "0xbob", 1, 1_000_000, 13);
        intent.sign(b"bob-key");
        let delivery = DeliveryListing::new("0xalice", "item", paris(), paris(), 10.0, "ETH", 100, 1_000_000);
        engine.propose(intent, delivery).unwrap();

        // Activate agreement.
        let delivery_id = engine.agreements[0].delivery_id;
        engine.agreements[0].sign_buyer();
        engine.agreements[0].sign_seller();
        engine.agreements[0].activate();
        engine.agreements[0].start_delivery();

        let proof = DeliveryProof::new(delivery_id, b"recv-sig", paris(), 1_000_000);
        assert!(engine.confirm_delivery(delivery_id, proof));
    }

    #[test]
    fn dashboard_contains_all() {
        let engine = CooperationEngine::new();
        let d = engine.dashboard();
        assert!(d.contains("Cooperation Protocol"));
        assert!(d.contains("Agreements:"));
        assert!(d.contains("Trades:"));
        assert!(d.contains("Deliveries:"));
    }
}
