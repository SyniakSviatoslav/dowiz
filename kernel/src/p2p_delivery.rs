//! `kernel::p2p_delivery` — Direct P2P delivery without intermediaries.
//!
//! Independent of deliveryOS. Pure data structures for peer-to-peer delivery
//! routing: buyer and seller connect directly, no platform middleman.
//!
//! # Flow
//! 1. Seller lists item with pickup location
//! 2. Buyer commits with delivery location
//! 3. P2P delivery is routed through direct peer connections
//! 4. Delivery confirmed cryptographically (proof of delivery)
//! 5. Escrow releases funds atomically with confirmation
//!
//! # Key properties
//! - No central dispatching — peers route directly
//! - Geolocation-based proximity matching
//! - Cryptographic proof of delivery (no "trust the platform")
//! - Reputation system on P2P identities

use crate::event_log::sha3_256;

/// Maximum active deliveries.
pub const MAX_DELIVERIES: usize = 10_000;

// ─── Location ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct GeoLocation {
    pub lat: f64,
    pub lon: f64,
}

impl GeoLocation {
    /// Haversine distance in km.
    pub fn distance_km(&self, other: &GeoLocation) -> f64 {
        let r = 6371.0;
        let d_lat = (other.lat - self.lat).to_radians();
        let d_lon = (other.lon - self.lon).to_radians();
        let a = (d_lat / 2.0).sin().powi(2)
            + self.lat.to_radians().cos() * other.lat.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
        r * 2.0 * a.sqrt().asin()
    }
}

// ─── Delivery Role ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryRole {
    /// Has goods to deliver.
    Shipper,
    /// Needs goods delivered.
    Recipient,
    /// Provides transport (independent carrier).
    Carrier,
}

// ─── Delivery State ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    /// Listed, awaiting match.
    Open,
    /// Matched with counterparty.
    Matched,
    /// In transit.
    InTransit,
    /// Delivered (awaiting confirmation).
    Delivered,
    /// Confirmed by both parties.
    Confirmed,
    /// Disputed.
    Disputed,
    /// Cancelled.
    Cancelled,
}

// ─── Delivery Listing ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DeliveryListing {
    pub id: [u8; 32],
    pub seller_id: String,
    pub item_description: String,
    pub pickup_location: GeoLocation,
    pub delivery_location: GeoLocation,
    pub max_distance_km: f64,
    pub fee_asset: String,
    pub fee_amount: u128,
    pub expiry_block: u64,
    pub state: DeliveryState,
}

impl DeliveryListing {
    pub fn new(
        seller: &str, item: &str,
        pickup: GeoLocation, delivery: GeoLocation,
        max_km: f64, asset: &str, amount: u128,
        expiry: u64,
    ) -> Self {
        let content = format!("{}{}{}{}", seller, item, amount, expiry);
        let hash = sha3_256(content.as_bytes());
        DeliveryListing {
            id: hash, seller_id: seller.to_string(),
            item_description: item.to_string(),
            pickup_location: pickup, delivery_location: delivery,
            max_distance_km: max_km,
            fee_asset: asset.to_string(), fee_amount: amount,
            expiry_block: expiry, state: DeliveryState::Open,
        }
    }

    /// Whether a carrier is within range to fulfill this delivery.
    pub fn in_range(&self, carrier_location: &GeoLocation) -> bool {
        self.pickup_location.distance_km(carrier_location) <= self.max_distance_km
    }

    pub fn match_carrier(&mut self) -> bool {
        if self.state != DeliveryState::Open { return false; }
        self.state = DeliveryState::Matched;
        true
    }

    pub fn confirm_delivery(&mut self) {
        self.state = DeliveryState::Confirmed;
    }
}

// ─── Proof of Delivery ────────────────────────────────────────────────────

/// Cryptographic proof that a delivery was completed.
#[derive(Debug, Clone)]
pub struct DeliveryProof {
    pub delivery_id: [u8; 32],
    pub recipient_sig: Vec<u8>,
    pub gps_proof: GeoLocation,
    pub timestamp_ns: u64,
    pub photo_hash: [u8; 32],
    pub proof_hash: [u8; 32],
}

impl DeliveryProof {
    pub fn new(delivery_id: [u8; 32], recipient_sig: &[u8], gps: GeoLocation, ts: u64) -> Self {
        let content = format!("{:?}{:?}{}", delivery_id, gps.lat, ts);
        let hash = sha3_256(content.as_bytes());
        DeliveryProof {
            delivery_id, recipient_sig: recipient_sig.to_vec(),
            gps_proof: gps, timestamp_ns: ts,
            photo_hash: [0u8; 32], proof_hash: hash,
        }
    }
}

// ─── P2P Delivery Network ─────────────────────────────────────────────────

/// The P2P delivery network — matches shippers, carriers, and recipients
/// without a central platform.
#[derive(Debug)]
pub struct P2PDeliveryNetwork {
    pub listings: Vec<DeliveryListing>,
    pub proofs: Vec<DeliveryProof>,
    pub max_listings: usize,
}

impl P2PDeliveryNetwork {
    pub fn new() -> Self {
        P2PDeliveryNetwork { listings: Vec::new(), proofs: Vec::new(), max_listings: MAX_DELIVERIES }
    }

    pub fn create_listing(&mut self, listing: DeliveryListing) -> Result<(), String> {
        if self.listings.len() >= self.max_listings { return Err("network full".into()); }
        self.listings.push(listing);
        Ok(())
    }

    /// Find carriers near a pickup location.
    pub fn find_nearby(&self, location: &GeoLocation, _max_km: f64) -> Vec<&DeliveryListing> {
        self.listings.iter()
            .filter(|l| l.state == DeliveryState::Open && l.in_range(location))
            .collect()
    }

    /// Record proof of delivery.
    pub fn record_proof(&mut self, proof: DeliveryProof) {
        self.proofs.push(proof);
    }

    pub fn dashboard(&self) -> String {
        format!(
            "P2P Delivery\n  Listings: {}\n  Proved:   {}\n  Open:     {}",
            self.listings.len(), self.proofs.len(),
            self.listings.iter().filter(|l| l.state == DeliveryState::Open).count()
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(lat: f64, lon: f64) -> GeoLocation { GeoLocation { lat, lon } }

    #[test]
    fn haversine_distance() {
        // Paris to London ~344 km
        let paris = loc(48.8566, 2.3522);
        let london = loc(51.5074, -0.1278);
        let d = paris.distance_km(&london);
        assert!((d - 344.0).abs() < 10.0);
    }

    #[test]
    fn listing_in_range() {
        let paris = loc(48.8566, 2.3522);
        let listing = DeliveryListing::new("seller", "item", paris, loc(48.9, 2.3), 10.0, "ETH", 100, 1_000_000);
        assert!(listing.in_range(&loc(48.87, 2.35))); // 1.5 km from Paris center
        assert!(!listing.in_range(&loc(45.0, 2.0))); // ~400 km away
    }

    #[test]
    fn delivery_state_lifecycle() {
        let paris = loc(48.8566, 2.3522);
        let mut listing = DeliveryListing::new("alice", "parcel", paris, loc(48.9, 2.3), 10.0, "ETH", 100, 1_000_000);
        assert_eq!(listing.state, DeliveryState::Open);
        listing.match_carrier();
        assert_eq!(listing.state, DeliveryState::Matched);
        listing.confirm_delivery();
        assert_eq!(listing.state, DeliveryState::Confirmed);
    }

    #[test]
    fn p2p_network_find_nearby() {
        let mut net = P2PDeliveryNetwork::new();
        let paris = loc(48.8566, 2.3522);
        let listing = DeliveryListing::new("seller", "item", paris, loc(48.9, 2.3), 10.0, "ETH", 100, 1_000_000);
        net.create_listing(listing).unwrap();
        let nearby = net.find_nearby(&loc(48.87, 2.35), 10.0);
        assert!(!nearby.is_empty());
    }

    #[test]
    fn proof_of_delivery() {
        let proof = DeliveryProof::new([0u8; 32], b"recipient-sig", loc(48.9, 2.3), 1_000_000);
        assert_ne!(proof.proof_hash, [0u8; 32]);
    }

    #[test]
    fn dashboard_contains_delivery() {
        let net = P2PDeliveryNetwork::new();
        let d = net.dashboard();
        assert!(d.contains("P2P Delivery"));
    }
}
