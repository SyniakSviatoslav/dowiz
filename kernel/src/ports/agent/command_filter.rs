use crate::event_log::sha3_256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactCommand {
    pub command_id: CommandId,
    pub payload: Vec<u8>,
    pub expected_mac: Option<[u8; 32]>,
}

impl ExactCommand {
    pub fn new(command_id: CommandId, payload: Vec<u8>) -> Self {
        ExactCommand {
            command_id,
            payload,
            expected_mac: None,
        }
    }

    pub fn with_mac(mut self, mac: [u8; 32]) -> Self {
        self.expected_mac = Some(mac);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u8);

impl CommandId {
    pub fn new(b: u8) -> Self {
        CommandId(b)
    }

    pub fn discriminant(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandCatalog {
    golden: Vec<ExactCommand>,
}

impl CommandCatalog {
    pub fn new() -> Self {
        CommandCatalog::default()
    }

    pub fn register(&mut self, cmd: ExactCommand) {
        self.golden.push(cmd);
    }

    pub fn golden(&self) -> &[ExactCommand] {
        &self.golden
    }

    pub fn mac(key: &[u8; 32], command_id: CommandId, payload: &[u8], nonce: u64) -> [u8; 32] {
        let mut preimage = Vec::with_capacity(32 + 1 + payload.len() + 8);
        preimage.extend_from_slice(key);
        preimage.push(command_id.discriminant());
        preimage.extend_from_slice(payload);
        preimage.extend_from_slice(&nonce.to_le_bytes());
        sha3_256(&preimage)
    }

    pub fn verify(
        &self,
        bytes: &[u8],
        mac_key: Option<&[u8; 32]>,
    ) -> Result<(), CommandVerifyError> {
        let min_len = 1 + 2 + 8 + 32;
        if bytes.len() < min_len {
            return Err(CommandVerifyError::Truncated);
        }
        let command_id = CommandId(bytes[0]);
        let payload_len = u16::from_le_bytes([bytes[1], bytes[2]]) as usize;
        let total_expected = 1 + 2 + payload_len + 8 + 32;
        if bytes.len() != total_expected {
            return Err(CommandVerifyError::BadLength);
        }
        let payload = bytes[3..3 + payload_len].to_vec();
        let nonce = u64::from_le_bytes(
            bytes[3 + payload_len..3 + payload_len + 8]
                .try_into()
                .unwrap(),
        );
        let supplied_mac: [u8; 32] = bytes[3 + payload_len + 8..3 + payload_len + 8 + 32]
            .try_into()
            .unwrap();

        let matched = self
            .golden
            .iter()
            .find(|g| g.command_id == command_id && g.payload == payload);
        let golden = matched.ok_or(CommandVerifyError::UnknownCommand)?;

        match golden.expected_mac {
            Some(expected) => {
                let key = mac_key.ok_or(CommandVerifyError::MacNotBound)?;
                let computed = Self::mac(key, command_id, &payload, nonce);
                if computed != supplied_mac || computed != expected {
                    return Err(CommandVerifyError::MacMismatch);
                }
            }
            None => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandVerifyError {
    UnknownCommand,
    MacMismatch,
    Truncated,
    BadLength,
    MacNotBound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_command_roundtrip_with_mac() {
        let mut catalog = CommandCatalog::new();
        let cmd = ExactCommand::new(CommandId::new(0x01), vec![0xAB, 0xCD]);
        let key: [u8; 32] = *b"0123456789abcdef0123456789abcdef";
        let nonce: u64 = 0x1122334455667788;
        let mac = CommandCatalog::mac(&key, cmd.command_id, &cmd.payload, nonce);
        let bound = cmd.with_mac(mac);
        catalog.register(bound);

        let mut bytes = Vec::new();
        bytes.push(CommandId::new(0x01).discriminant());
        bytes.extend_from_slice(&(vec![0xAB, 0xCD].len() as u16).to_le_bytes());
        bytes.extend_from_slice(&[0xAB, 0xCD]);
        bytes.extend_from_slice(&nonce.to_le_bytes());
        bytes.extend_from_slice(&mac);

        assert!(catalog.verify(&bytes, Some(&key)).is_ok());

        let mut bad_len = bytes.clone();
        bad_len.push(0x00);
        assert!(catalog.verify(&bad_len, Some(&key)).is_err());

        let mut bad_payload = bytes.clone();
        bad_payload[3] ^= 0x01;
        assert!(catalog.verify(&bad_payload, Some(&key)).is_err());

        let mut bad_mac = bytes.clone();
        bad_mac[bytes.len() - 1] ^= 0x01;
        assert!(catalog.verify(&bad_mac, Some(&key)).is_err());

        assert!(catalog.verify(&bytes[..10], Some(&key)).is_err());

        let mut alt_bytes = bytes.clone();
        alt_bytes[2] ^= 0x01;
        assert!(catalog.verify(&alt_bytes, Some(&key)).is_err());
    }

    #[test]
    fn mac_same_input_same_output() {
        let key = [0x11u8; 32];
        let a = CommandCatalog::mac(
            &key,
            CommandId::new(0x01),
            &[0xAB, 0xCD],
            0x0102030405060708,
        );
        let b = CommandCatalog::mac(
            &key,
            CommandId::new(0x01),
            &[0xAB, 0xCD],
            0x0102030405060708,
        );
        assert_eq!(a, b, "SHA3-256 MAC is deterministic");
    }
}
