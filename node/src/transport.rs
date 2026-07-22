use dowiz_kernel::mesh::{HubTransport, MeshError, SignedEntry};

pub struct TcpTransport;

impl TcpTransport {
    pub fn new() -> Self { TcpTransport }
}

impl HubTransport for TcpTransport {
    fn send(&self, entry: &SignedEntry) -> Result<(), MeshError> {
        use std::io::Write;
        let addr = "127.0.0.1:4321";
        let bytes = entry.signed_bytes().to_vec();
        match std::net::TcpStream::connect(addr) {
            Ok(mut s) => {
                s.write_all(&bytes).map_err(|e| MeshError::Transport(format!("write: {e}")))?;
                Ok(())
            }
            Err(e) => Err(MeshError::Transport(format!("connect {addr}: {e}"))),
        }
    }

    fn recv(&self) -> Result<Vec<SignedEntry>, MeshError> {
        Err(MeshError::Transport("recv not implemented for TcpTransport".into()))
    }
}

pub struct StubTransport;

impl StubTransport {
    pub fn new() -> Self { StubTransport }
}

impl HubTransport for StubTransport {
    fn send(&self, _entry: &SignedEntry) -> Result<(), MeshError> {
        Ok(())
    }
    fn recv(&self) -> Result<Vec<SignedEntry>, MeshError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::mesh::MlDsaSigner;
    use dowiz_kernel::mesh::Signer;

    fn test_entry() -> SignedEntry {
        let mut seed = [0u8; 32];
        seed[0] = 42;
        let signer = MlDsaSigner::from_seed(&seed, [0u8; 32]);
        let payload = b"hello world".to_vec();
        let sig = signer.sign(&payload);
        SignedEntry {
            prev_hash: [0u8; 32],
            payload,
            sig,
            pubkey: signer.pubkey_bytes(),
        }
    }

    #[test]
    fn stub_transport_roundtrip() {
        let t = StubTransport::new();
        let entry = test_entry();
        assert!(t.send(&entry).is_ok());
        assert!(t.recv().unwrap().is_empty());
    }

    #[test]
    fn signed_entry_verifies() {
        let entry = test_entry();
        assert!(entry.verify_sig(), "freshly signed entry must verify");
    }
}
