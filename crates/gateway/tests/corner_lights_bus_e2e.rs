//! Bus-level integration test for corner-light ACK ingress on `vcan0`.
//!
//! This test exercises real SocketCAN I/O:
//! writer socket -> `vcan0` -> reader socket -> wire decode helpers.

use std::time::{Duration, Instant};

use common::{ActuationCommand, CorrelationId, PhysicalCarVocabulary};
use codec::kinds::KIND_CORNER_LIGHT_ACK_ON;
use socketcan::{CanSocket, EmbeddedFrame, Socket};

#[path = "../src/codec/mod.rs"]
mod codec;

#[path = "../src/corner_light_actuation_can.rs"]
mod corner_light_actuation_can;

const TEST_CAN_INTERFACE: &str = "vcan0";

fn sample_corr() -> CorrelationId {
    CorrelationId {
        source_id: "gateway-bus-e2e".to_string(),
        session_id: 0x1234,
        sequence_no: 0x89abcdef,
    }
}

#[tokio::test]
async fn corner_lights_ack_frame_round_trips_over_vcan_and_decodes() {
    let tx = match CanSocket::open(TEST_CAN_INTERFACE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping test: cannot open {TEST_CAN_INTERFACE}: {e}");
            return;
        }
    };
    let rx = match CanSocket::open(TEST_CAN_INTERFACE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping test: cannot open reader on {TEST_CAN_INTERFACE}: {e}");
            return;
        }
    };

    let cmd = ActuationCommand::SwitchCornerLightsOn {
        correlation_id: sample_corr(),
    };
    let frame = corner_light_actuation_can::encode_ack_frame(&cmd).expect("encode ACK frame");

    tx.write_frame(&frame).expect("write ACK frame to vcan");

    let expected_wire = corner_light_actuation_can::actuation_command_wire_meta(&cmd);
    let got = tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let frame = match rx.read_frame() {
                Ok(frame) => frame,
                Err(_) => continue,
            };
            if frame.data().first().copied() == Some(KIND_CORNER_LIGHT_ACK_ON) {
                return Some(frame);
            }
        }
        None
    })
    .await
    .expect("join read loop task")
    .expect("did not receive expected ACK frame kind on vcan0 before timeout");
    let meta = corner_light_actuation_can::wire_correlation_meta(&got).expect("wire metadata");
    assert_eq!(meta, expected_wire);

    let payload = corner_light_actuation_can::decode_corner_light_payload_from_can_frame(&got)
        .expect("decode corner-light payload from CAN");
    let physical = corner_light_actuation_can::physical_from_corner_light_payload(payload)
        .expect("ACK frame should map to physical vocabulary");
    assert!(matches!(
        physical,
        PhysicalCarVocabulary::CornerLightsCommandConfirmed { on_command: true }
    ));
}
