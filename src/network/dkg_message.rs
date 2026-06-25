//! DKG message wire format for network transmission.
//!
//! This module defines the wire format for DKG Round 1 messages that are
//! transmitted between validators during distributed key generation.

use crate::crypto::dkg::{DkgError, DkgRound1Message};

/// Wire format version for DKG messages
const DKG_MESSAGE_VERSION: u8 = 1;

/// Serialize a DKG Round 1 message for network transmission.
/// Format:
/// - 1 byte: version
/// - 4 bytes: dealer_index (big-endian)
/// - 96 bytes: shared_public_key (48 bytes a0 + 48 bytes a1)
pub fn serialize_dkg_round1_message(message: &DkgRound1Message) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(101);
    bytes.push(DKG_MESSAGE_VERSION);
    bytes.extend_from_slice(&message.to_bytes());
    bytes
}

/// Deserialize a DKG Round 1 message from network bytes.
pub fn deserialize_dkg_round1_message(bytes: &[u8]) -> Result<DkgRound1Message, DkgError> {
    if bytes.len() < 101 {
        return Err(DkgError::InvalidMessageLength);
    }

    // Check version
    if bytes[0] != DKG_MESSAGE_VERSION {
        return Err(DkgError::InvalidMessageLength);
    }

    // Deserialize the message payload
    DkgRound1Message::from_bytes(&bytes[1..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bls_keys::{G1Point, SharedPublicKey};

    #[test]
    fn test_dkg_message_wire_format_roundtrip() {
        let a0 = G1Point::new(0xABCDEF1234567890, true);
        let a1 = G1Point::new(0x1122334455667788, false);
        let shared_key = SharedPublicKey::new(a0, a1);
        let message = DkgRound1Message::new(999, shared_key);

        let serialized = serialize_dkg_round1_message(&message);
        let deserialized = deserialize_dkg_round1_message(&serialized).unwrap();

        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_dkg_message_rejects_short_input() {
        let short_bytes = vec![1, 2, 3];
        let result = deserialize_dkg_round1_message(&short_bytes);
        assert_eq!(result, Err(DkgError::InvalidMessageLength));
    }

    #[test]
    fn test_dkg_message_version_check() {
        let bytes = vec![0u8; 101]; // Wrong version (0 instead of 1)
        let result = deserialize_dkg_round1_message(&bytes);
        assert_eq!(result, Err(DkgError::InvalidMessageLength));
    }
}
