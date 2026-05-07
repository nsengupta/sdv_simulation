//! Bus-level integration test for corner-light ACK ingress on `vcan0`.
//!
//! This test exercises real SocketCAN I/O:
//! writer socket -> `vcan0` -> reader socket -> wire decode helpers.

use std::time::{Duration, Instant};

use common::{ActuationCommand, CorrelationId, PhysicalCarVocabulary};
use codec::kinds::{
    KIND_CORNER_LIGHT_ACK_ON, KIND_CORNER_LIGHT_CMD_ON, KIND_CORNER_LIGHT_NACK_ON,
};
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

fn open_bus_pair() -> Option<(CanSocket, CanSocket)> {
    let tx = match CanSocket::open(TEST_CAN_INTERFACE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping test: cannot open {TEST_CAN_INTERFACE}: {e}");
            return None;
        }
    };
    let rx = match CanSocket::open(TEST_CAN_INTERFACE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping test: cannot open reader on {TEST_CAN_INTERFACE}: {e}");
            return None;
        }
    };
    Some((tx, rx))
}

async fn recv_first_frame_with_kind(rx: CanSocket, kind: u8, timeout: Duration) -> Option<socketcan::CanFrame> {
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let frame = match rx.read_frame() {
                Ok(frame) => frame,
                Err(_) => continue,
            };
            if frame.data().first().copied() == Some(kind) {
                return Some(frame);
            }
        }
        None
    })
    .await
    .expect("join read loop task")
}

async fn recv_first_ack_or_nack(rx: CanSocket, timeout: Duration) -> Option<socketcan::CanFrame> {
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let frame = match rx.read_frame() {
                Ok(frame) => frame,
                Err(_) => continue,
            };
            let Some(kind) = frame.data().first().copied() else {
                continue;
            };
            if matches!(kind, KIND_CORNER_LIGHT_ACK_ON | KIND_CORNER_LIGHT_NACK_ON) {
                return Some(frame);
            }
        }
        None
    })
    .await
    .expect("join read loop task")
}

#[tokio::test]
async fn corner_lights_ack_frame_round_trips_over_vcan_and_decodes() {
    let Some((tx, rx)) = open_bus_pair() else {
        return;
    };

    let cmd = ActuationCommand::SwitchCornerLightsOn {
        correlation_id: sample_corr(),
    };
    let frame = corner_light_actuation_can::encode_ack_frame(&cmd).expect("encode ACK frame");

    tx.write_frame(&frame).expect("write ACK frame to vcan");

    let expected_wire = corner_light_actuation_can::actuation_command_wire_meta(&cmd);
    let got = recv_first_frame_with_kind(rx, KIND_CORNER_LIGHT_ACK_ON, Duration::from_secs(2))
        .await
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

#[tokio::test]
async fn corner_lights_nack_frame_round_trips_over_vcan_and_decodes() {
    let Some((tx, rx)) = open_bus_pair() else {
        return;
    };
    let cmd = ActuationCommand::SwitchCornerLightsOn {
        correlation_id: sample_corr(),
    };
    let frame = corner_light_actuation_can::encode_nack_frame(&cmd).expect("encode NACK frame");
    tx.write_frame(&frame).expect("write NACK frame to vcan");

    let got = recv_first_frame_with_kind(rx, KIND_CORNER_LIGHT_NACK_ON, Duration::from_secs(2))
        .await
        .expect("did not receive expected NACK frame kind on vcan0 before timeout");
    let payload = corner_light_actuation_can::decode_corner_light_payload_from_can_frame(&got)
        .expect("decode corner-light payload from CAN");
    let physical = corner_light_actuation_can::physical_from_corner_light_payload(payload)
        .expect("NACK frame should map to physical vocabulary");
    assert!(matches!(
        physical,
        PhysicalCarVocabulary::CornerLightsCommandRejected { on_command: true }
    ));
}

#[tokio::test]
async fn corner_lights_command_frame_is_not_ingressed_as_physical_event() {
    let Some((tx, rx)) = open_bus_pair() else {
        return;
    };
    let cmd = ActuationCommand::SwitchCornerLightsOn {
        correlation_id: sample_corr(),
    };
    let frame = corner_light_actuation_can::encode_command_frame(&cmd).expect("encode CMD frame");
    tx.write_frame(&frame).expect("write command frame to vcan");

    let got = recv_first_frame_with_kind(rx, KIND_CORNER_LIGHT_CMD_ON, Duration::from_secs(2))
        .await
        .expect("did not receive expected CMD frame kind on vcan0 before timeout");
    let payload = corner_light_actuation_can::decode_corner_light_payload_from_can_frame(&got)
        .expect("decode corner-light payload from CAN");
    assert!(
        corner_light_actuation_can::physical_from_corner_light_payload(payload).is_none(),
        "command frames must not be ingressed as physical ACK/NACK events"
    );
}

#[tokio::test]
async fn corner_lights_no_response_window_has_no_ack_or_nack_frames() {
    let Some((tx, rx)) = open_bus_pair() else {
        return;
    };
    let cmd = ActuationCommand::SwitchCornerLightsOn {
        correlation_id: sample_corr(),
    };
    let frame = corner_light_actuation_can::encode_command_frame(&cmd).expect("encode CMD frame");
    tx.write_frame(&frame).expect("write command frame to vcan");

    let maybe_response = recv_first_ack_or_nack(rx, Duration::from_millis(300)).await;
    assert!(
        maybe_response.is_none(),
        "unexpected ACK/NACK observed in no-response window without plant response"
    );
}
