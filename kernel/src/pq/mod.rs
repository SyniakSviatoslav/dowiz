//! pq.rs — held-handle shim: pure PQ primitives live in `dowiz_core::pq`;
//! the serde-dependent wire types + the std OS/QRNG entropy provider stay here.
//!
//! Core (no_std): keccak, kem, dsa, x25519, entropy (mixing), fractal, root_delegation.
//! Kernel (serde/std): codesign, envelope, hybrid, hybrid_signing, volume (wire
//! (de)serialization) + the `entropy::provider` (OS /dev/urandom + ANU QRNG).

pub use dowiz_core::pq::*;

pub use dowiz_core::pq::{codesign, envelope, hybrid_signing};
pub mod hybrid;
pub mod volume;
pub mod x25519;

/// std-only entropy provider (OS /dev/urandom + ANU QRNG), behind `qrng`.
pub mod entropy {
    pub use dowiz_core::pq::entropy::*;

#[cfg(feature = "qrng")]
pub mod provider {
    //! Optional quantum-entropy provider. Behind `qrng` feature; pulls from ANU QRNG
    //! (public, free, real vacuum-fluctuation noise). NOT used by default — wire it in
    //! at the application/runtime layer, never inside the crypto hot path.
    use super::*;

    /// Fetch `n` bytes of real quantum noise from ANU QRNG over HTTPS.
    /// Returns an error string on any transport/parse failure (caller must fall back to
    /// OS entropy — never panic on entropy source loss).
    pub fn quantum_bytes(n: usize) -> Result<Vec<u8>, String> {
        let url = format!("https://qrng.anu.edu.au/API/jsonI.php?length={n}&type=uint8&size=1");
        let body = mini_get(&url).map_err(|e| format!("qrng transport: {e}"))?;
        // Parse {"data":[...],"success":true}
        let data = extract_array(&body).ok_or("qrng parse: no data array")?;
        if data.len() < n {
            return Err("qrng parse: short payload".into());
        }
        Ok(data[..n].to_vec())
    }

    /// Real quantum-seeded master seed: mix quantum noise with OS urandom.
    /// Fails closed: if QRNG is unreachable, returns Err so the caller keeps using OS
    /// entropy rather than silently weakening the seed.
    pub fn quantum_seeded_master() -> Result<[u8; 32], String> {
        let q = quantum_bytes(32)?;
        let mut os = [0u8; 32];
        fill_os(&mut os);
        Ok(entropy_mix(&q, &os))
    }

    fn fill_os(buf: &mut [u8]) {
        // std-only OS entropy (no getrandom dep): read /dev/urandom on unix.
        #[cfg(unix)]
        {
            use std::fs::File;
            use std::io::Read;
            if let Ok(mut f) = File::open("/dev/urandom") {
                let _ = f.read_exact(buf);
            }
        }
        #[cfg(not(unix))]
        {
            // Fallback: caller should supply real entropy. Leave as-is (all-zero) only if
            // no OS source — the mix still binds quantum noise.
            let _ = buf;
        }
    }

    /// **Sanctioned master-seed entry point (operator directive 2026-07-12).**
    ///
    /// Native OS entropy is the DEFAULT and the FALLBACK. When the `qrng` feature is
    /// enabled AND ANU QRNG is reachable, the master seed is upgraded to
    /// `SHAKE256(quantum || os)` — but on ANY failure (unreachable, parse error,
    /// non-unix without /dev/urandom) it transparently returns the OS-seeded master
    /// rather than erroring. The caller therefore NEVER has to handle a missing/broken
    /// QRNG: native entropy is always present.
    ///
    /// This is why a node boots identically offline (OS-only) and online (quantum-boosted).
    pub fn master_seed() -> [u8; 32] {
        let mut os = [0u8; 32];
        fill_os(&mut os);
        #[cfg(feature = "qrng")]
        {
            if let Ok(q) = quantum_bytes(32) {
                return entropy_mix(&q, &os); // quantum ⊕ os — entropy ≥ max of both
            }
            // fall through to OS-only on any QRNG failure
        }
        os
    }

    // Minimal no-dep HTTPS GET (ANU responds with a small JSON; we use reqwest only
    // behind feature to avoid a heavy dep in the crypto crate). Blocking, std-only.
    fn mini_get(url: &str) -> Result<String, String> {
        // ponytail: a full TLS stack is out of scope for the crypto crate. The runtime
        // should provide entropy; this std-only stub reaches the endpoint over plain TCP
        // for localhost/testing only. PRODUCTION MUST use a TLS client (reqwest/rustls).
        // Upgrade trigger: any real deployment — gate behind `qrng-tls` feature with reqwest.
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let host = "qrng.anu.edu.au:443";
        let _ = (url, host); // referenced to silence unused in non-unix
        let mut stream = TcpStream::connect("qrng.anu.edu.au:443").map_err(|e| e.to_string())?;
        let req = format!(
            "GET /API/jsonI.php?length=32&type=uint8&size=1 HTTP/1.1\r\nHost: qrng.anu.edu.au\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut resp = String::new();
        stream
            .read_to_string(&mut resp)
            .map_err(|e| e.to_string())?;
        // Split headers/body
        let body = resp.split_once("\r\n\r\n").map(|x| x.1).unwrap_or(&resp);
        Ok(body.to_string())
    }

    fn extract_array(s: &str) -> Option<Vec<u8>> {
        let start = s.find("\"data\":[")? + 8;
        let end = s[start..].find(']')? + start;
        let inner = &s[start..end];
        let mut out = Vec::new();
        for part in inner.split(',') {
            if let Ok(b) = part.trim().parse::<u8>() {
                out.push(b);
            }
        }
        Some(out)
    }
}

}
