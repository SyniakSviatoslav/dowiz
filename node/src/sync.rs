use dowiz_kernel::event_log::{EventLog, MemEventStore, MeshEvent};
use dowiz_kernel::mesh::{HubTransport, MeshError, SignedEntry};

pub struct MeshSync<T: HubTransport> {
    transport: T,
    log: EventLog<MemEventStore>,
}

impl<T: HubTransport> MeshSync<T> {
    pub fn new(transport: T) -> Self {
        MeshSync {
            transport,
            log: EventLog::new(MemEventStore::new()),
        }
    }

    pub fn send_entry(&mut self, entry: SignedEntry) -> Result<(), MeshError> {
        self.transport.send(&entry)?;
        let event = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [0x53, 0x59, 0x4E, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            actor_seq: 0,
            payload: entry.payload.clone(),
        };
        let _ = self.log.append(event);
        Ok(())
    }

    pub fn recv_entries(&self) -> Result<Vec<SignedEntry>, MeshError> {
        self.transport.recv()
    }

    pub fn log(&self) -> &EventLog<MemEventStore> { &self.log }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StubTransport;
    use dowiz_kernel::mesh::MlDsaSigner;
    use dowiz_kernel::mesh::Signer;

    fn test_entry() -> SignedEntry {
        let mut seed = [0u8; 32];
        seed[0] = 42;
        let signer = MlDsaSigner::from_seed(&seed, [0u8; 32]);
        let payload = b"hello".to_vec();
        let sig = signer.sign(&payload);
        SignedEntry {
            prev_hash: [0u8; 32],
            payload,
            sig,
            pubkey: signer.pubkey_bytes(),
        }
    }

    #[test]
    fn sync_stub_works() {
        let mut sync = MeshSync::new(StubTransport::new());
        let entry = test_entry();
        assert!(sync.send_entry(entry).is_ok());
        assert!(sync.recv_entries().unwrap().is_empty());
    }
}
