//! Transport-agnostic payload codec for corner-light actuation.
//!
//! Payload layout (8 bytes):
//! - `data[0]`: kind
//! - `data[1..3]`: session_id (u16, big-endian)
//! - `data[3..7]`: sequence_no (u32, big-endian)
//! - `data[7]`: reserved (currently 0)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerLightActuationPayload {
    pub kind: u8,
    pub session_id: u16,
    pub sequence_no: u32,
}

pub fn encode_payload(payload: CornerLightActuationPayload) -> [u8; 8] {
    let mut data = [0u8; 8];
    data[0] = payload.kind;
    data[1..3].copy_from_slice(&payload.session_id.to_be_bytes());
    data[3..7].copy_from_slice(&payload.sequence_no.to_be_bytes());
    data
}

pub fn decode_payload(data: &[u8]) -> Option<CornerLightActuationPayload> {
    if data.len() < 8 {
        return None;
    }
    Some(CornerLightActuationPayload {
        kind: data[0],
        session_id: u16::from_be_bytes([data[1], data[2]]),
        sequence_no: u32::from_be_bytes([data[3], data[4], data[5], data[6]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::kinds::KIND_CORNER_LIGHT_ACK_ON;

    #[test]
    fn payload_round_trip() {
        let payload = CornerLightActuationPayload {
            kind: KIND_CORNER_LIGHT_ACK_ON,
            session_id: 0xabcd,
            sequence_no: 0x11223344,
        };
        let data = encode_payload(payload);
        assert_eq!(decode_payload(&data), Some(payload));
    }

    #[test]
    fn decode_rejects_short_payload() {
        assert!(decode_payload(&[KIND_CORNER_LIGHT_ACK_ON, 0x00, 0x01]).is_none());
    }
}
