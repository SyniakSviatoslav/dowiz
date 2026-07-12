//! RFC 9171 BPv7 codec for [`crate::Bundle`] using the real `bp7` crate
//! (`dtn7/bp7-rs`). This is a faithful 1:1 mapping of the in-memory bundle onto
//! a standards-track BPv7 bundle (primary block + payload block type 1).
//!
//! Field map (see `node/src/transport.md`):
//! | `Bundle`            | BPv7                                            |
//! |---------------------|-------------------------------------------------|
//! | `source`            | primary block Source EID (`dtn://` scheme)     |
//! | `dest`              | primary block Destination EID                  |
//! | `creation_ts`       | primary block Creation Timestamp (DTN time)    |
//! | `lifetime`          | primary block Lifetime                         |
//! | `payload`           | Payload block (type 1) opaque bytes            |
//! | `sender_pk`         | carried inside the PQ envelope (payload bytes) |
//! | `sender_hybrid_pk`  | carried inside the PQ envelope (payload bytes) |
//!
//! `sender_pk` / `sender_hybrid_pk` are upper-layer metadata that, per the
//! transport note, travel *inside* the PQ envelope (the `payload` block). The
//! roundtrip invariant therefore covers `source`, `dest`, `creation_ts`,
//! `lifetime`, and the `payload` bytes; the two key fields are not re-derived
//! from the wire (they are not part of the opaque envelope in this crate's
//! representation) and are left empty on decode.
//!
//! NOTE on EID normalization: `bp7` canonicalizes every `dtn://node/` EID to a
//! trailing-slash form (`dtn://node/`). To keep the roundtrip byte-exact we
//! strip a single trailing `/` on decode, matching the Node's `dtn://node`
//! (no trailing slash) EID convention.

use crate::Bundle;
use super::Transport;

use bp7::bundle::{Bundle as Bp7Bundle, BundleBuilder};
use bp7::crc::CRC_NO;
use bp7::dtntime::{CreationTimestamp, DtnTime};
use bp7::eid::EndpointID;
use bp7::flags::BundleControlFlags;
use bp7::primary::PrimaryBlockBuilder;
use std::time::Duration;

/// Normalize a decoded `dtn://node/` EID back to the Node's no-trailing-slash
/// `dtn://node` form so the roundtrip is byte-exact.
fn norm_eid(s: String) -> String {
    s.strip_suffix('/').unwrap_or(&s).to_string()
}

/// Concrete [`Transport`] backed by the real RFC 9171 `bp7` codec.
pub struct Bp7Transport;

impl Bp7Transport {
    pub fn new() -> Self {
        Bp7Transport
    }
}

impl Default for Bp7Transport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for Bp7Transport {
    fn encode(&self, b: &Bundle) -> Vec<u8> {
        let src = EndpointID::try_from(b.source.as_str())
            .map_err(|e| format!("bad source EID: {e}"))
            .expect("source EID must be a valid dtn/ipn endpoint");
        let dst = EndpointID::try_from(b.dest.as_str())
            .map_err(|e| format!("bad dest EID: {e}"))
            .expect("dest EID must be a valid dtn/ipn endpoint");

        let primary = PrimaryBlockBuilder::default()
            .bundle_control_flags(
                (BundleControlFlags::BUNDLE_MUST_NOT_FRAGMENTED
                    | BundleControlFlags::BUNDLE_STATUS_REQUEST_DELIVERY)
                    .bits(),
            )
            .destination(dst)
            .source(src.clone())
            .report_to(src)
            .creation_timestamp(CreationTimestamp::with_time_and_seq(
                b.creation_ts as DtnTime,
                0,
            ))
            .lifetime(Duration::from_secs(b.lifetime))
            .build()
            .expect("primary block builds");

        let mut bp7_bundle = BundleBuilder::default()
            .primary(primary)
            .payload(b.payload.clone())
            .build()
            .expect("bundle builds with a payload block");

        // No CRC: keep the codec minimal; production deployments should set CRC_16/32.
        bp7_bundle.set_crc(CRC_NO);
        bp7_bundle.to_cbor()
    }

    fn decode(&self, raw: &[u8]) -> Result<Bundle, String> {
        let bp7_bundle = Bp7Bundle::try_from(raw).map_err(|e| format!("bp7 parse error: {e}"))?;

        // RED gate: a BPv7 bundle with no payload block cannot map back.
        let payload = bp7_bundle
            .payload()
            .ok_or_else(|| "missing payload block".to_string())?
            .clone();

        let source = norm_eid(bp7_bundle.primary.source.to_string());
        let dest = norm_eid(bp7_bundle.primary.destination.to_string());
        let creation_ts = bp7_bundle.primary.creation_timestamp.dtntime();
        let lifetime = bp7_bundle.primary.lifetime.as_secs();

        Ok(Bundle {
            source,
            dest,
            sender_pk: Vec::new(),
            sender_hybrid_pk: Vec::new(),
            creation_ts,
            lifetime,
            payload,
        })
    }
}

// ── RED+GREEN tests: S1 codec falsifiable invariants ───────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;

    const SEED_A: [u8; 32] = [1u8; 32];

    fn transport() -> Bp7Transport {
        Bp7Transport::new()
    }

    #[test]
    fn green_bundle_roundtrips_through_bp7() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let bundle = a.make_bundle("dtn://b", b"hello courier", 1000, 3600);

        let raw = transport().encode(&bundle);
        let back = transport().decode(&raw).expect("decode succeeds");

        assert_eq!(back.source, bundle.source, "source EID must match");
        assert_eq!(back.dest, bundle.dest, "dest EID must match");
        assert_eq!(back.creation_ts, bundle.creation_ts, "creation_ts must match");
        assert_eq!(back.lifetime, bundle.lifetime, "lifetime must match");
        assert_eq!(back.payload, bundle.payload, "payload bytes must match");
    }

    #[test]
    fn green_produced_cbor_is_valid_rfc9171_bundle() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let bundle = a.make_bundle("dtn://b", b"hello courier", 1000, 3600);

        let raw = transport().encode(&bundle);
        // Re-parse via bp7's own CBOR entry point (TryFrom<&[u8]> for Bundle).
        let reparsed = Bp7Bundle::try_from(raw.as_slice());
        assert!(reparsed.is_ok(), "output must re-parse as a valid BPv7 bundle");
        // And it must carry a payload block.
        assert!(reparsed.unwrap().payload().is_some());
    }

    #[test]
    fn red_truncated_cbor_fails_decode() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let bundle = a.make_bundle("dtn://b", b"hello courier", 1000, 3600);

        let mut raw = transport().encode(&bundle);
        // Truncate the CBOR to simulate a cut-off transmission.
        assert!(raw.len() > 4);
        raw.truncate(raw.len() - 4);
        assert!(transport().decode(&raw).is_err(), "truncated CBOR must error");

        // Garble the CBOR break marker (final byte, always 0xff) to simulate
        // corruption of the bundle framing — reliably breaks the CBOR array.
        let mut raw2 = transport().encode(&bundle);
        let last = raw2.len() - 1;
        raw2[last] ^= 0xFF;
        assert!(
            transport().decode(&raw2).is_err(),
            "garbled CBOR must error"
        );
    }

    #[test]
    fn red_missing_payload_block_fails_decode() {
        // Build a BPv7 bundle with NO payload block and serialize it.
        let src = EndpointID::try_from("dtn://a").unwrap();
        let dst = EndpointID::try_from("dtn://b").unwrap();
        let primary = PrimaryBlockBuilder::default()
            .destination(dst)
            .source(src.clone())
            .report_to(src)
            .creation_timestamp(CreationTimestamp::with_time_and_seq(1000, 0))
            .lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        // Empty canonical set => no payload block.
        let mut bp7_bundle = Bp7Bundle::new(primary, vec![]);
        bp7_bundle.set_crc(CRC_NO);
        let raw = bp7_bundle.to_cbor();

        assert!(
            transport().decode(&raw).is_err(),
            "bundle without a payload block must fail to map back"
        );
    }
}
