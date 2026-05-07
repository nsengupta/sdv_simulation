//! Corner-light actuation **on-wire** framing for `vcan0` (Tightening Stage 1).
//!
//! Wire format (11-bit standard ID `ID_CORNER_LIGHTS`, DLC 8):
//! - `data[0]`: kind — `KIND_CMD_ON`, `KIND_CMD_OFF`, `KIND_ACK_ON`, `KIND_ACK_OFF`,
//!   `KIND_NACK_ON`, `KIND_NACK_OFF`
//! - `data[1..3]`: `session_id` as **u16** big-endian (truncated from `CorrelationId::session_id`)
//! - `data[3..7]`: `sequence_no` as **u32** big-endian (truncated from `CorrelationId::sequence_no`)
//! - `data[7]`: reserved `0`
//!
//! `source_id` is **not** carried on the bus in this stage; logs use decoded session/seq only.

use common::{ActuationCommand, PhysicalCarVocabulary};
use socketcan::{CanFrame, EmbeddedFrame, StandardId};

use crate::codec::corner_light_actuation::{
    decode_payload, encode_payload, CornerLightActuationPayload,
};
use crate::codec::kinds::{
    KIND_CORNER_LIGHT_ACK_OFF, KIND_CORNER_LIGHT_ACK_ON, KIND_CORNER_LIGHT_CMD_OFF,
    KIND_CORNER_LIGHT_CMD_ON, KIND_CORNER_LIGHT_NACK_OFF, KIND_CORNER_LIGHT_NACK_ON,
};

pub const ID_CORNER_LIGHTS: u16 = 0x204;

fn standard_id() -> Result<StandardId, socketcan::Error> {
    StandardId::new(ID_CORNER_LIGHTS).ok_or_else(|| {
        socketcan::Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid standard id",
        ))
    })
}

fn build_frame(kind: u8, cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let sid = standard_id()?;
    let (session_id, sequence_no) = actuation_command_wire_meta(cmd);
    let data = encode_payload(CornerLightActuationPayload {
        kind,
        session_id,
        sequence_no,
    });
    CanFrame::new(sid, &data).ok_or_else(|| {
        socketcan::Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CAN frame build",
        ))
    })
}

/// Truncated `(session_id, sequence_no)` as on the wire (for logs / correlation with ingress).
pub fn actuation_command_wire_meta(cmd: &ActuationCommand) -> (u16, u32) {
    let cid = match cmd {
        ActuationCommand::SwitchCornerLightsOn { correlation_id }
        | ActuationCommand::SwitchCornerLightsOff { correlation_id } => correlation_id,
    };
    (cid.session_id as u16, cid.sequence_no as u32)
}

/// Encode a controller **command** (gateway egress → bus).
pub fn encode_command_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_CORNER_LIGHT_CMD_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_CORNER_LIGHT_CMD_OFF,
    };
    build_frame(kind, cmd)
}

/// Encode **ACK** after the plant delay (same correlation packing as command).
pub fn encode_ack_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_CORNER_LIGHT_ACK_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_CORNER_LIGHT_ACK_OFF,
    };
    build_frame(kind, cmd)
}

/// Encode **NACK** after the plant delay when command execution is rejected.
pub fn encode_nack_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_CORNER_LIGHT_NACK_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_CORNER_LIGHT_NACK_OFF,
    };
    build_frame(kind, cmd)
}

/// Session / sequence bytes from an ACK or command frame (for logging).
pub fn wire_correlation_meta(frame: &CanFrame) -> Option<(u16, u32)> {
    if !is_corner_light_frame(frame) {
        return None;
    }
    decode_payload(frame.data()).map(|payload| (payload.session_id, payload.sequence_no))
}

fn is_corner_light_frame(frame: &CanFrame) -> bool {
    let id = match frame.id() {
        socketcan::Id::Standard(s) => s.as_raw(),
        _ => return false,
    };
    id == ID_CORNER_LIGHTS
}

/// Decode a corner-light payload from a CAN frame if the frame belongs to this protocol.
pub fn decode_corner_light_payload_from_can_frame(
    frame: &CanFrame,
) -> Option<CornerLightActuationPayload> {
    if !is_corner_light_frame(frame) {
        return None;
    }
    decode_payload(frame.data())
}

/// Map a decoded corner-light payload to physical vocabulary.
/// Command kinds return `None` so ingress does not double-feed the twin.
pub fn physical_from_corner_light_payload(
    payload: CornerLightActuationPayload,
) -> Option<PhysicalCarVocabulary> {
    match payload.kind {
        KIND_CORNER_LIGHT_ACK_ON => Some(PhysicalCarVocabulary::CornerLightsCommandConfirmed {
            on_command: true,
        }),
        KIND_CORNER_LIGHT_ACK_OFF => Some(PhysicalCarVocabulary::CornerLightsCommandConfirmed {
            on_command: false,
        }),
        KIND_CORNER_LIGHT_NACK_ON => Some(PhysicalCarVocabulary::CornerLightsCommandRejected {
            on_command: true,
        }),
        KIND_CORNER_LIGHT_NACK_OFF => Some(PhysicalCarVocabulary::CornerLightsCommandRejected {
            on_command: false,
        }),
        KIND_CORNER_LIGHT_CMD_ON | KIND_CORNER_LIGHT_CMD_OFF => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::CorrelationId;

    fn sample_corr() -> CorrelationId {
        CorrelationId {
            source_id: "test".into(),
            session_id: 0xabcd,
            sequence_no: 0x11223344,
        }
    }

    #[test]
    fn round_trip_ack_on_to_physical() {
        let cmd = ActuationCommand::SwitchCornerLightsOn {
            correlation_id: sample_corr(),
        };
        let frame = encode_ack_frame(&cmd).expect("ack frame");
        let payload = decode_corner_light_payload_from_can_frame(&frame).expect("decode payload");
        let phys = physical_from_corner_light_payload(payload).expect("maps");
        assert!(matches!(
            phys,
            PhysicalCarVocabulary::CornerLightsCommandConfirmed { on_command: true }
        ));
    }

    #[test]
    fn command_frame_not_ingressed_as_ack() {
        let cmd = ActuationCommand::SwitchCornerLightsOff {
            correlation_id: sample_corr(),
        };
        let frame = encode_command_frame(&cmd).expect("cmd frame");
        let payload = decode_corner_light_payload_from_can_frame(&frame).expect("decode payload");
        assert!(physical_from_corner_light_payload(payload).is_none());
    }

    #[test]
    fn actuation_command_wire_meta_truncates_like_encode() {
        let cmd = ActuationCommand::SwitchCornerLightsOn {
            correlation_id: sample_corr(),
        };
        assert_eq!(actuation_command_wire_meta(&cmd), (0xabcd, 0x11223344));
    }

    #[test]
    fn encode_nack_frame_marks_expected_kind() {
        let cmd = ActuationCommand::SwitchCornerLightsOff {
            correlation_id: sample_corr(),
        };
        let frame = encode_nack_frame(&cmd).expect("nack frame");
        assert_eq!(frame.data()[0], KIND_CORNER_LIGHT_NACK_OFF);
    }

    #[test]
    fn nack_frame_maps_to_command_rejected_physical_vocabulary() {
        let cmd = ActuationCommand::SwitchCornerLightsOn {
            correlation_id: sample_corr(),
        };
        let frame = encode_nack_frame(&cmd).expect("nack frame");
        let payload = decode_corner_light_payload_from_can_frame(&frame).expect("decode payload");
        assert!(matches!(
            physical_from_corner_light_payload(payload),
            Some(PhysicalCarVocabulary::CornerLightsCommandRejected { on_command: true })
        ));
    }
}
