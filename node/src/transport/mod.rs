//! Transport bearers for carrying [`crate::Bundle`] between nodes.
//!
//! S2+S3 production bearer: RFC 9000 QUIC + RFC 9174 (TCPCLv4-style) under TLS 1.3
//! (rustls + aws-lc-rs). See [`quic`] for the implementation.

pub mod quic;
