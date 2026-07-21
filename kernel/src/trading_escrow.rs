//! `kernel::trading_escrow` — Trustless escrow + state channel settlement.
//!
//! Pure data structures for smart-contract-based escrow and bidirectional
//! payment channels. No intermediaries: funds are locked by math, not by
//! a third party.
//!
//! # Escrow lifecycle
//! 1. Offer: party A locks asset X in escrow contract
//! 2. Accept: party B verifies terms and locks asset Y
//! 3. Settle: both parties receive their counterpart's asset (atomic swap)
//! 4. Cancel: either party can claim back after timeout
//!
//! # State Channel lifecycle
//! 1. Open: both parties deposit into channel
//! 2. Update: off-chain signed balance updates (infinite speed)
//! 3. Close: final state submitted to chain

use crate::event_log::sha3_256;
use crate::TriState;

// ─── Escrow State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowState {
    /// Escrow created, waiting for counterparty.
    Pending,
    /// Both parties have deposited.
    Active,
    /// Settlement executed successfully.
    Settled,
    /// Escrow cancelled / refunded.
    Cancelled,
    /// Dispute raised.
    Disputed,
}

// ─── Escrow Offer ─────────────────────────────────────────────────────────

/// A trustless escrow offer for peer-to-peer trading.
#[derive(Debug, Clone)]
pub struct EscrowOffer {
    pub id: [u8; 32],
    /// Party A (maker) address.
    pub party_a: String,
    /// Party B (taker) address.
    pub party_b: String,
    /// Asset party A offers.
    pub offer_asset: super::trading_intent::Asset,
    /// Amount party A offers.
    pub offer_amount: u128,
    /// Asset party A wants in return.
    pub ask_asset: super::trading_intent::Asset,
    /// Amount party A wants.
    pub ask_amount: u128,
    /// Block deadline for acceptance.
    pub deadline_block: u64,
    /// Current state.
    pub state: EscrowState,
    /// SHA3-256 of terms.
    pub terms_hash: [u8; 32],
    /// Party A signature.
    pub sig_a: Vec<u8>,
    /// Party B signature.
    pub sig_b: Vec<u8>,
}

impl EscrowOffer {
    pub fn new(
        a: &str, b: &str,
        offer_asset: super::trading_intent::Asset, offer_amount: u128,
        ask_asset: super::trading_intent::Asset, ask_amount: u128,
        deadline_block: u64,
    ) -> Self {
        let terms = format!("{}{}{}{}{}{}{}", a, b, offer_amount, ask_amount, deadline_block, offer_asset.symbol, ask_asset.symbol);
        let hash = sha3_256(terms.as_bytes());
        EscrowOffer {
            id: hash, party_a: a.to_string(), party_b: b.to_string(),
            offer_asset, offer_amount, ask_asset, ask_amount,
            deadline_block, state: EscrowState::Pending,
            terms_hash: hash, sig_a: vec![], sig_b: vec![],
        }
    }

    pub fn accept(&mut self, sig: &[u8]) {
        self.sig_b = sig.to_vec();
        self.state = EscrowState::Active;
    }

    pub fn settle(&mut self) {
        self.state = EscrowState::Settled;
    }

    pub fn cancel(&mut self) {
        self.state = EscrowState::Cancelled;
    }
}

// ─── State Channel ────────────────────────────────────────────────────────

/// Off-chain balance update in a state channel.
#[derive(Debug, Clone)]
pub struct ChannelUpdate {
    /// Update sequence number.
    pub seq: u64,
    /// Balance for party A.
    pub balance_a: u128,
    /// Balance for party B.
    pub balance_b: u128,
    /// Nonce for replay protection.
    pub nonce: u64,
    /// Party A signature.
    pub sig_a: Vec<u8>,
    /// Party B signature.
    pub sig_b: Vec<u8>,
    /// SHA3-256 of state.
    pub state_hash: [u8; 32],
}

/// Bidirectional payment state channel.
#[derive(Debug, Clone)]
pub struct StateChannel {
    pub id: [u8; 32],
    pub party_a: String,
    pub party_b: String,
    pub asset: super::trading_intent::Asset,
    /// Latest balance update.
    pub latest: ChannelUpdate,
    /// Whether channel is open.
    pub open: TriState,
}

impl StateChannel {
    pub fn new(a: &str, b: &str, asset: super::trading_intent::Asset) -> Self {
        let id = sha3_256(format!("{}{}{}", a, b, asset.symbol).as_bytes());
        let initial = ChannelUpdate {
            seq: 0, balance_a: 0, balance_b: 0, nonce: 0,
            sig_a: vec![], sig_b: vec![], state_hash: [0u8; 32],
        };
        StateChannel { id, party_a: a.to_string(), party_b: b.to_string(), asset, latest: initial, open: TriState::True }
    }

    /// Update channel balances off-chain.
    pub fn update(&mut self, balance_a: u128, balance_b: u128, sig_a: &[u8], sig_b: &[u8]) -> bool {
        if self.open != TriState::True { return false; }
        // First update establishes the total, subsequent updates conserve it.
        if self.latest.seq > 0 && balance_a + balance_b != self.latest.balance_a + self.latest.balance_b {
            return false; // Conservation violation.
        }
        let content = format!("{}{}{}{}", balance_a, balance_b, self.latest.seq + 1, self.latest.nonce + 1);
        let hash = sha3_256(content.as_bytes());
        self.latest = ChannelUpdate {
            seq: self.latest.seq + 1,
            balance_a, balance_b, nonce: self.latest.nonce + 1,
            sig_a: sig_a.to_vec(), sig_b: sig_b.to_vec(), state_hash: hash,
        };
        true
    }

    pub fn close(&mut self) {
        self.open = TriState::False;
    }

    pub fn dashboard(&self) -> String {
        format!(
            "State Channel\n  Asset: {}\n  Bal A: {} | Bal B: {}\n  Seq:   {}\n  Open:  {}",
            self.asset.symbol, self.latest.balance_a, self.latest.balance_b,
            self.latest.seq, self.open
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading_intent::Asset;

    fn eth() -> Asset { Asset::new("ethereum", "0x0000", "ETH", 18) }
    fn btc() -> Asset { Asset::new("bitcoin", "0x0001", "BTC", 8) }

    #[test]
    fn escrow_create() {
        let escrow = EscrowOffer::new("0xalice", "0xbob", eth(), 10_000_000_000, btc(), 100_000_000, 1_000_000);
        assert_eq!(escrow.state, EscrowState::Pending);
    }

    #[test]
    fn escrow_accept_and_settle() {
        let mut escrow = EscrowOffer::new("0xa", "0xb", eth(), 1_000, btc(), 10, 500_000);
        escrow.accept(b"bob-sig");
        assert_eq!(escrow.state, EscrowState::Active);
        escrow.settle();
        assert_eq!(escrow.state, EscrowState::Settled);
    }

    #[test]
    fn state_channel_update_conservation() {
        let mut ch = StateChannel::new("0xalice", "0xbob", eth());
        assert!(ch.update(100, 0, b"a-sig", b"b-sig"));
        assert!(ch.update(70, 30, b"a-sig", b"b-sig"));
        assert_eq!(ch.latest.balance_a, 70);
        assert_eq!(ch.latest.balance_b, 30);
    }

    #[test]
    fn state_channel_rejects_violation() {
        let mut ch = StateChannel::new("0xa", "0xb", eth());
        ch.update(100, 0, b"a-sig", b"b-sig");
        assert!(!ch.update(200, 0, b"a-sig", b"b-sig")); // 200+0 != 100+0
    }

    #[test]
    fn state_channel_close() {
        let mut ch = StateChannel::new("0xa", "0xb", eth());
        ch.close();
        assert_eq!(ch.open, TriState::False);
        assert!(!ch.update(10, 0, b"a-sig", b"b-sig")); // Closed channel rejects
    }

    #[test]
    fn dashboard_contains_channel() {
        let ch = StateChannel::new("0xa", "0xb", eth());
        let d = ch.dashboard();
        assert!(d.contains("State Channel"));
    }

    #[test]
    fn escrow_cancel() {
        let mut escrow = EscrowOffer::new("0xa", "0xb", eth(), 1, btc(), 1, 100);
        escrow.cancel();
        assert_eq!(escrow.state, EscrowState::Cancelled);
    }
}
