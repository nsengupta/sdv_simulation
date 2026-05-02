//! Gateway wiring: controller install, background loops, and CAN read loop. Keeps `main` thin.

use anyhow::Result;
use common::{VehicleController, VehicleControllerRuntimeOptions, VehicleEvent, VssSignal};
use socketcan::{CanSocket, Socket};
use std::time::Duration;

use crate::actuation_scaffold;
use crate::ingress;

/// Default SocketCAN interface (matches emulator).
pub const DEFAULT_CAN_INTERFACE: &str = "vcan0";

const TIMER_TICK_MS: u64 = 100;
/// Simulated actuator delay before ACK is delivered on the feedback channel.
const ACTUATION_ACK_DELAY_MS: u64 = 150;

pub struct GatewayLaunchConfig<'a> {
    pub car_identity: &'a str,
    pub print_timer_tick: bool,
    pub can_interface: &'a str,
}

pub async fn run(launch: GatewayLaunchConfig<'_>) -> Result<()> {
    let (cmd_tx, cmd_rx, sensor_feedback_tx, sensor_feedback_rx) = actuation_scaffold::actuator_io_channels();

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

    actuation_scaffold::spawn_ack_emulator_plant(
        cmd_rx,
        sensor_feedback_tx,
        Duration::from_millis(ACTUATION_ACK_DELAY_MS),
    );
    actuation_scaffold::spawn_actuation_feedback_ingress(sensor_feedback_rx, controller.clone());

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

    run_can_read_loop(controller, launch.can_interface).await
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

async fn run_can_read_loop(controller: VehicleController, can_interface: &str) -> Result<()> {
    let socket = CanSocket::open(can_interface)?;
    loop {
        let frame = socket.read_frame()?;
        if let Some(sig) = VssSignal::from_can_frame(&frame) {
            let ev = VehicleEvent::TelemetryUpdate(sig);
            let physical = ingress::vehicle_event_to_physical_vocabulary(ev);
            controller
                .submit_physical_car_event(physical)
                .await
                .map_err(|e| anyhow::anyhow!("submit physical car event: {e:?}"))?;
        }
    }
}
