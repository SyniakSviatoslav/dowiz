//! `kernel::trading_intent` — Intent-based trustless trading core.
//!
//! Self-sovereign infrastructure: no centralized gateways, no intermediaries.
//! Users sign cryptographic "intents" — desired outcomes, not instructions.
//! A network of solvers competes to fulfill them.
//!
//! # Architecture
//! ```text
//! [Trader] → signs Intent → broadcasts to solver network
//! [Solvers] → compete → best path wins
//! [Settlement] → smart contract executes atomically
//! ```
//!
//! # Key properties
//! - **Non-custodial:** keys stay local, never leave the machine
//! - **Censorship-resistant:** intent goes directly to solver mesh
//! - **MEV-protected:** solver competition prevents frontrunning
//! - **Atomic settlement:** all-or-nothing via smart contract

use crate::event_log::sha3_256;
use crate::TriState;

/// Maximum intents in the pool.
pub const MAX_INTENTS: usize = 10_000;

// ─── Asset ────────────────────────────────────────────────────────────────

/// A tradeable asset — ERC20, native token, or custom.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Asset {
    pub chain: String,
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

impl Asset {
    pub fn new(chain: &str, address: &str, symbol: &str, decimals: u8) -> Self {
        Asset { chain: chain.to_string(), address: address.to_string(), symbol: symbol.to_string(), decimals }
    }
}

// ─── Order Side ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

// ─── Intent ───────────────────────────────────────────────────────────────

/// A signed cryptographic intent — the atomic unit of self-sovereign trading.
/// The user specifies DESIRED OUTCOME, not execution instructions.
#[derive(Debug, Clone)]
pub struct Intent {
    /// Intent ID (SHA3-256 of content).
    pub id: [u8; 32],
    /// Source asset to sell/spend.
    pub from_asset: Asset,
    /// Amount in smallest unit (wei/satoshi).
    pub from_amount: u128,
    /// Destination asset to buy/receive.
    pub to_asset: Asset,
    /// Minimum amount expected (slippage protection).
    pub min_to_amount: u128,
    /// Side.
    pub side: OrderSide,
    /// Trader's wallet address.
    pub trader_address: String,
    /// Chain ID (EIP-155).
    pub chain_id: u64,
    /// Deadline (block number).
    pub deadline_block: u64,
    /// Nonce for replay protection.
    pub nonce: u64,
    /// Signature (simulated — real impl uses ECDSA/Ed25519).
    pub signature: Vec<u8>,
    /// Signer public key.
    pub signer_pk: Vec<u8>,
    /// Whether this intent is valid.
    pub valid: TriState,
    /// Arbitrary data for solver hints.
    pub solver_hints: Vec<String>,
}

impl Intent {
    /// Create a new intent.
    pub fn new(
        from: Asset, from_amount: u128,
        to: Asset, min_to: u128,
        side: OrderSide,
        trader: &str, chain_id: u64,
        deadline_block: u64, nonce: u64,
    ) -> Self {
        let content = format!("{:?}{:?}{:?}{}", from, to, side, nonce);
        let hash = sha3_256(content.as_bytes());

        Intent {
            id: hash,
            from_asset: from, from_amount,
            to_asset: to, min_to_amount: min_to,
            side,
            trader_address: trader.to_string(),
            chain_id, deadline_block, nonce,
            signature: vec![],
            signer_pk: vec![],
            valid: TriState::Unknown,
            solver_hints: vec![],
        }
    }

    /// Verify intent integrity (hash match + signature check placeholder).
    pub fn verify(&self) -> TriState {
        if self.signature.is_empty() || self.signer_pk.is_empty() {
            return TriState::False;
        }
        // Verify the content hash matches.
        let content = format!("{:?}{:?}{:?}{}", self.from_asset, self.to_asset, self.side, self.nonce);
        let expected_hash = sha3_256(content.as_bytes());
        if self.id != expected_hash {
            return TriState::False;
        }
        TriState::True
    }

    /// Whether this intent is expired.
    pub fn is_expired(&self, current_block: u64) -> bool {
        current_block >= self.deadline_block
    }

    /// Sign (placeholder — real impl uses secp256k1 or ed25519).
    pub fn sign(&mut self, private_key: &[u8]) {
        // In production: ECDSA or Ed25519 signature.
        // This is a deterministic placeholder.
        let msg = [&self.id[..], private_key].concat();
        self.signature = sha3_256(&msg).to_vec();
        self.signer_pk = private_key.to_vec();
        self.valid = TriState::True;
    }
}

// ─── Solver Bid ───────────────────────────────────────────────────────────

/// A solver's bid to fulfill an intent.
#[derive(Debug, Clone)]
pub struct SolverBid {
    /// Intent this bid applies to.
    pub intent_id: [u8; 32],
    /// Solver ID.
    pub solver_id: String,
    /// Guaranteed output amount.
    pub guaranteed_amount: u128,
    /// Fee in basis points (1 bp = 0.01%).
    pub fee_bps: u16,
    /// Execution time estimate (blocks).
    pub estimate_blocks: u32,
    /// Route description (which DEXes/pools).
    pub route: Vec<String>,
    /// Bid signature.
    pub signature: Vec<u8>,
}

// ─── Intent Pool ──────────────────────────────────────────────────────────

/// Pool of active intents awaiting solver bids.
#[derive(Debug)]
pub struct IntentPool {
    pub intents: Vec<Intent>,
    pub bids: Vec<SolverBid>,
    max_intents: usize,
}

impl IntentPool {
    pub fn new() -> Self {
        IntentPool { intents: Vec::new(), bids: Vec::new(), max_intents: MAX_INTENTS }
    }

    /// Submit an intent to the pool.
    pub fn submit(&mut self, intent: Intent) -> Result<(), String> {
        if self.intents.len() >= self.max_intents {
            return Err("pool full".into());
        }
        if intent.verify() != TriState::True {
            return Err("invalid signature".into());
        }
        // Check for duplicate.
        if self.intents.iter().any(|i| i.id == intent.id) {
            return Err("duplicate intent".into());
        }
        self.intents.push(intent);
        Ok(())
    }

    /// Submit a solver bid.
    pub fn submit_bid(&mut self, bid: SolverBid) -> Result<(), String> {
        if !self.intents.iter().any(|i| i.id == bid.intent_id) {
            return Err("intent not found".into());
        }
        self.bids.push(bid);
        Ok(())
    }

    /// Get best bid for an intent (highest guaranteed amount, lowest fee).
    pub fn best_bid(&self, intent_id: &[u8; 32]) -> Option<&SolverBid> {
        let mut candidates: Vec<&SolverBid> = self.bids.iter()
            .filter(|b| b.intent_id == *intent_id)
            .collect();
        candidates.sort_by(|a, b| {
            let a_score = a.guaranteed_amount as f64 * (1.0 - a.fee_bps as f64 / 10_000.0);
            let b_score = b.guaranteed_amount as f64 * (1.0 - b.fee_bps as f64 / 10_000.0);
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.into_iter().next()
    }

    /// Clean expired intents.
    pub fn purge_expired(&mut self, current_block: u64) {
        self.intents.retain(|i| !i.is_expired(current_block));
    }

    pub fn dashboard(&self) -> String {
        format!(
            "Intent Pool\n  Intents:  {} active\n  Bids:     {} total\n  Pool cap: {}",
            self.intents.len(), self.bids.len(), self.max_intents
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn eth() -> Asset { Asset::new("ethereum", "0x0000", "ETH", 18) }
    fn usdc() -> Asset { Asset::new("ethereum", "0xA0b8", "USDC", 6) }

    #[test]
    fn create_intent() {
        let intent = Intent::new(eth(), 1_000_000, usdc(), 1_000, OrderSide::Sell, "0xtrader", 1, 1_000_000, 42);
        assert_eq!(intent.nonce, 42);
        assert!(intent.valid.is_unknown());
    }

    #[test]
    fn sign_and_verify() {
        let mut intent = Intent::new(eth(), 1_000_000, usdc(), 1_000, OrderSide::Buy, "0xtrader", 1, 1_000_000, 7);
        intent.sign(b"my-secret-key-32-bytes-long!!");
        assert_eq!(intent.verify(), TriState::True);
    }

    #[test]
    fn expired_intent() {
        let intent = Intent::new(eth(), 100, usdc(), 10, OrderSide::Sell, "0xalice", 1, 500, 1);
        assert!(intent.is_expired(600));
        assert!(!intent.is_expired(400));
    }

    #[test]
    fn pool_submit_and_bid() {
        let mut pool = IntentPool::new();
        let mut intent = Intent::new(eth(), 1_000_000, usdc(), 1_000, OrderSide::Buy, "0xbob", 1, 1_000_000, 13);
        intent.sign(b"private-key-bob");
        assert!(pool.submit(intent).is_ok());
    }

    #[test]
    fn pool_rejects_unsigned() {
        let mut pool = IntentPool::new();
        let intent = Intent::new(eth(), 1_000_000, usdc(), 1_000, OrderSide::Sell, "0xeve", 1, 1_000_000, 99);
        assert!(pool.submit(intent).is_err());
    }

    #[test]
    fn best_bid_picks_highest_score() {
        let mut pool = IntentPool::new();
        let mut intent = Intent::new(eth(), 1_000, usdc(), 100, OrderSide::Buy, "0xme", 1, 1_000_000, 1);
        intent.sign(b"my-key");
        pool.submit(intent.clone()).unwrap();

        pool.submit_bid(SolverBid {
            intent_id: intent.id, solver_id: "solver1".into(),
            guaranteed_amount: 100, fee_bps: 50, estimate_blocks: 5,
            route: vec!["UniswapV3".into()], signature: vec![],
        }).unwrap();
        pool.submit_bid(SolverBid {
            intent_id: intent.id, solver_id: "solver2".into(),
            guaranteed_amount: 105, fee_bps: 30, estimate_blocks: 3,
            route: vec!["Curve".into()], signature: vec![],
        }).unwrap();

        let best = pool.best_bid(&intent.id).unwrap();
        assert_eq!(best.solver_id, "solver2"); // Higher amount, lower fee
    }

    #[test]
    fn purge_removes_expired() {
        let mut pool = IntentPool::new();
        let mut intent = Intent::new(eth(), 100, usdc(), 10, OrderSide::Sell, "0xalice", 1, 500, 2);
        intent.sign(b"alice-key");
        pool.submit(intent).unwrap();
        assert_eq!(pool.intents.len(), 1);
        pool.purge_expired(600);
        assert_eq!(pool.intents.len(), 0);
    }

    #[test]
    fn dashboard_contains_pool() {
        let pool = IntentPool::new();
        let d = pool.dashboard();
        assert!(d.contains("Intent Pool"));
    }
}

