//! Gateway wiring: controller install, background loops, and CAN read loop. Keeps `main` thin.

use anyhow::Result;
use common::{
    PhysicalCarVocabulary, VehicleController, VehicleControllerRuntimeOptions, VehicleEvent,
    VssSignal,
};
use socketcan::{CanSocket, Socket};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::actuation_scaffold;
use crate::actuation_scaffold::DEFAULT_ACK_NACK_RESPONSE_PROB;

/// If set to a float in `0.0..=1.0`, the corner-light plant randomly sends **no** ACK after the
/// command frame (see `actuation_scaffold`).
pub const ENV_PLANT_DROP_RESPONSE_PROB: &str = "CORNER_LIGHT_PLANT_DROP_RESPONSE_PROB";
/// If set to a float in `0.0..=1.0`, this controls ACK-vs-NACK split **when the plant responds**.
///
/// Semantics: value is `P(ACK)`; default is [`DEFAULT_ACK_NACK_RESPONSE_PROB`] (currently `0.7`).
pub const ENV_PLANT_ACK_NACK_RESPONSE_PROB: &str = "CORNER_LIGHT_PLANT_ACK_NACK_RESPONSE_PROB";
use crate::corner_light_actuation_can::{
    decode_corner_light_payload_from_can_frame, physical_from_corner_light_payload,
    wire_correlation_meta,
};
use crate::ingress;

/// Default SocketCAN interface (matches emulator).
pub const DEFAULT_CAN_INTERFACE: &str = "vcan0";

const TIMER_TICK_MS: u64 = 100;
/// Simulated actuator delay before the plant writes the ACK frame onto the same CAN interface.
const ACTUATION_ACK_DELAY_MS: u64 = 150;

pub struct GatewayLaunchConfig<'a> {
    pub car_identity: &'a str,
    pub print_timer_tick: bool,
    pub can_interface: &'a str,
}

/// Messages forwarded from the dedicated CAN reader thread into async gateway flow.
enum CanIngressEnvelope {
    Physical(PhysicalCarVocabulary),
    ActuationResponse {
        physical: PhysicalCarVocabulary,
        session: u16,
        sequence: u32,
    },
}

pub async fn run(launch: GatewayLaunchConfig<'_>) -> Result<()> {
    let (cmd_tx, cmd_rx) = actuation_scaffold::actuator_command_channel();

    let runtime_options = VehicleControllerRuntimeOptions {
        log_timer_tick: launch.print_timer_tick,
        actuation_command_tx: Some(cmd_tx),
        ..VehicleControllerRuntimeOptions::default()
    };

    let (controller, _join) = VehicleController::install_and_start_with_options(
        launch.car_identity.to_string(),
        runtime_options,
    )
    .await
    .map_err(|e| anyhow::anyhow!("spawn actor: {e}"))?;

    let plant_dont_respond_prob = std::env::var(ENV_PLANT_DROP_RESPONSE_PROB)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|p| (0.0..=1.0).contains(p))
        .unwrap_or(0.0);
    let plant_ack_nack_response_prob = std::env::var(ENV_PLANT_ACK_NACK_RESPONSE_PROB)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|p| (0.0..=1.0).contains(p))
        .unwrap_or(DEFAULT_ACK_NACK_RESPONSE_PROB);
    if plant_dont_respond_prob > 0.0 {
        println!(
            "[gateway] {}={plant_dont_respond_prob} — corner-light plant may sit tight (no ACK) after each command; twin uses TimerTick ACK timeout when pending",
            ENV_PLANT_DROP_RESPONSE_PROB
        );
    }
    println!(
        "[gateway] {}={plant_ack_nack_response_prob} (P(ACK) when plant responds; default via actuation scaffold)",
        ENV_PLANT_ACK_NACK_RESPONSE_PROB
    );

    actuation_scaffold::spawn_corner_light_can_plant(
        cmd_rx,
        launch.can_interface.to_string(),
        Duration::from_millis(ACTUATION_ACK_DELAY_MS),
        plant_dont_respond_prob,
        plant_ack_nack_response_prob,
    );

    controller
        .send_power_on()
        .await
        .map_err(|e| anyhow::anyhow!("PowerOn: {e:?}"))?;

    spawn_timer_tick_loop(controller.clone());

    println!(
        "⚡ Gateway on {} — CAN → VehicleEvent → PhysicalCarVocabulary → DigitalTwinCarVocabulary → VirtualCarActor",
        launch.can_interface
    );
    if launch.print_timer_tick {
        println!("[gateway] TimerTick heartbeat logging enabled (--print-timer-tick)");
    }

    let (can_tx, can_rx) = mpsc::unbounded_channel();
    let _can_reader = spawn_can_reader_thread(launch.can_interface.to_string(), can_tx)?;
    run_can_ingress_dispatch_loop(controller, can_rx).await
}

fn spawn_timer_tick_loop(controller: VehicleController) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(TIMER_TICK_MS)).await;
            let physical = ingress::vehicle_event_to_physical_vocabulary(VehicleEvent::TimerTick);
            let _ = controller.submit_physical_car_event(physical).await;
        }
    });
}

/// Dedicated OS thread for blocking `read_frame()` loop.
///
/// Why this is not an async task:
/// - `read_frame()` is a blocking syscall.
/// - this listener runs forever in steady-state.
/// - a dedicated thread avoids occupying Tokio worker/blocking-pool capacity indefinitely.
fn spawn_can_reader_thread(
    can_interface: String,
    tx: mpsc::UnboundedSender<CanIngressEnvelope>,
) -> Result<JoinHandle<()>> {
    let socket = CanSocket::open(&can_interface)?;
    let thread_name = format!("gateway-can-reader-{can_interface}");
    let handle = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            loop {
                let frame = match socket.read_frame() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("[gateway-can-reader]: read_frame failed: {e:?}");
                        continue;
                    }
                };
                if let Some(sig) = VssSignal::from_can_frame(&frame) {
                    let ev = VehicleEvent::TelemetryUpdate(sig);
                    let physical = ingress::vehicle_event_to_physical_vocabulary(ev);
                    if tx.send(CanIngressEnvelope::Physical(physical)).is_err() {
                        break;
                    }
                    continue;
                }
                if let Some(payload) = decode_corner_light_payload_from_can_frame(&frame) {
                    let Some(physical) = physical_from_corner_light_payload(payload) else {
                        continue;
                    };
                    if let Some((session, sequence)) = wire_correlation_meta(&frame) {
                        if tx
                            .send(CanIngressEnvelope::ActuationResponse {
                                physical,
                                session,
                                sequence,
                            })
                            .is_err()
                        {
                            break;
                        }
                    } else if tx.send(CanIngressEnvelope::Physical(physical)).is_err() {
                        break;
                    }
                }
            }
        })?;
    Ok(handle)
}

async fn run_can_ingress_dispatch_loop(
    controller: VehicleController,
    mut rx: mpsc::UnboundedReceiver<CanIngressEnvelope>,
) -> Result<()> {
    while let Some(msg) = rx.recv().await {
        match msg {
            CanIngressEnvelope::Physical(physical) => {
                controller
                    .submit_physical_car_event(physical)
                    .await
                    .map_err(|e| anyhow::anyhow!("submit physical car event: {e:?}"))?;
            }
            CanIngressEnvelope::ActuationResponse {
                physical,
                session,
                sequence,
            } => {
                println!(
                    "[actuation-can-ingress wire session={session} seq={sequence}]: {:?} (CAN path)",
                    physical
                );
                controller
                    .submit_physical_car_event(physical)
                    .await
                    .map_err(|e| anyhow::anyhow!("submit physical car event: {e:?}"))?;
            }
        }
    }
    Err(anyhow::anyhow!(
        "CAN ingress channel closed: reader thread exited"
    ))
}
