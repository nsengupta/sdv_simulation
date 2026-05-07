//! CAN envelope adapter for corner-light device payloads.

use common::ActuationCommand;
use socketcan::{CanFrame, EmbeddedFrame, StandardId};

use crate::devices::corner_lights::codec::{
    decode_payload, encode_payload, CornerLightActuationPayload, KIND_ACK_OFF, KIND_ACK_ON,
    KIND_CMD_OFF, KIND_CMD_ON, KIND_NACK_OFF, KIND_NACK_ON,
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

pub fn actuation_command_wire_meta(cmd: &ActuationCommand) -> (u16, u32) {
    let cid = match cmd {
        ActuationCommand::SwitchCornerLightsOn { correlation_id }
        | ActuationCommand::SwitchCornerLightsOff { correlation_id } => correlation_id,
    };
    (cid.session_id as u16, cid.sequence_no as u32)
}

/// Command-frame encoder kept for protocol tests/documentation; runtime plant ingress is channel-local.
#[allow(dead_code)]
pub fn encode_command_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_CMD_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_CMD_OFF,
    };
    build_frame(kind, cmd)
}

pub fn encode_ack_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_ACK_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_ACK_OFF,
    };
    build_frame(kind, cmd)
}

pub fn encode_nack_frame(cmd: &ActuationCommand) -> Result<CanFrame, socketcan::Error> {
    let kind = match cmd {
        ActuationCommand::SwitchCornerLightsOn { .. } => KIND_NACK_ON,
        ActuationCommand::SwitchCornerLightsOff { .. } => KIND_NACK_OFF,
    };
    build_frame(kind, cmd)
}

fn is_corner_light_frame(frame: &CanFrame) -> bool {
    let id = match frame.id() {
        socketcan::Id::Standard(s) => s.as_raw(),
        _ => return false,
    };
    id == ID_CORNER_LIGHTS
}

pub fn decode_payload_from_can_frame(frame: &CanFrame) -> Option<CornerLightActuationPayload> {
    if !is_corner_light_frame(frame) {
        return None;
    }
    decode_payload(frame.data())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::corner_lights::codec::payload_to_physical;
    use common::{CorrelationId, PhysicalCarVocabulary};

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
        let payload = decode_payload_from_can_frame(&frame).expect("decode payload");
        let phys = payload_to_physical(payload).expect("maps");
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
        let payload = decode_payload_from_can_frame(&frame).expect("decode payload");
        assert!(payload_to_physical(payload).is_none());
    }

    #[test]
    fn actuation_command_wire_meta_truncates_like_encode() {
        let cmd = ActuationCommand::SwitchCornerLightsOn {
            correlation_id: sample_corr(),
        };
        assert_eq!(actuation_command_wire_meta(&cmd), (0xabcd, 0x11223344));
    }
}
