//! In-process actuation simulation: command channel (controller → plant) and feedback channel
//! (plant → ingress). Mirrors “second bus / non-vCAN feedback” until real CAN ACK frames exist.
//!
//! TODO(actuation-child-actor): replace Tokio tasks with supervised child actors; keep the same
//! [`ActuationCommand`] / [`ActuationFeedback`] contracts.

use std::time::Duration;

use common::{
    ActuationCommand, ActuationFeedback, PhysicalCarVocabulary, VehicleController,
};
use tokio::sync::mpsc;

const DEFAULT_CHANNEL_CAPACITY: usize = 64;

pub fn actuator_io_channels() -> (
    mpsc::Sender<ActuationCommand>,
    mpsc::Receiver<ActuationCommand>,
    mpsc::Sender<ActuationFeedback>,
    mpsc::Receiver<ActuationFeedback>,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
    let (fb_tx, fb_rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
    (cmd_tx, cmd_rx, fb_tx, fb_rx)
}

/// Task A — simulates actuator / body ECU: consumes commands, waits, emits feedback.
pub fn spawn_ack_emulator_plant(
    mut cmd_rx: mpsc::Receiver<ActuationCommand>,
    fb_tx: mpsc::Sender<ActuationFeedback>,
    ack_delay: Duration,
) {
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                ActuationCommand::SwitchCornerLightsOn { correlation_id } => {
                    tokio::time::sleep(ack_delay).await;
                    if fb_tx
                        .send(ActuationFeedback::CornerLightsOnConfirmed { correlation_id })
                        .await
                        .is_err()
                    {
                        eprintln!("[ack-emulator]: feedback receiver dropped (on ack)");
                        break;
                    }
                }
                ActuationCommand::SwitchCornerLightsOff { correlation_id } => {
                    tokio::time::sleep(ack_delay).await;
                    if fb_tx
                        .send(ActuationFeedback::CornerLightsOffConfirmed { correlation_id })
                        .await
                        .is_err()
                    {
                        eprintln!("[ack-emulator]: feedback receiver dropped (off ack)");
                        break;
                    }
                }
            }
        }
    });
}

/// Task B — simulates non-`vcan0` ingress: maps feedback into [`PhysicalCarVocabulary`] and submits
/// through the same controller path as CAN telemetry.
pub fn spawn_actuation_feedback_ingress(
    mut fb_rx: mpsc::Receiver<ActuationFeedback>,
    controller: VehicleController,
) {
    tokio::spawn(async move {
        while let Some(fb) = fb_rx.recv().await {
            let physical = match fb {
                ActuationFeedback::CornerLightsOnConfirmed { correlation_id } => {
                    println!(
                        "[actuation-ingress @ corr {:?}]: corner lights ON acknowledged (non-CAN path)",
                        correlation_id
                    );
                    PhysicalCarVocabulary::CornerLightsOnConfirmed
                }
                ActuationFeedback::CornerLightsOffConfirmed { correlation_id } => {
                    println!(
                        "[actuation-ingress @ corr {:?}]: corner lights OFF acknowledged (non-CAN path)",
                        correlation_id
                    );
                    PhysicalCarVocabulary::CornerLightsOffConfirmed
                }
                ActuationFeedback::CornerLightsActuationFailed { correlation_id, reason } => {
                    eprintln!(
                        "[actuation-ingress]: actuation failed corr={correlation_id:?} reason={reason}"
                    );
                    continue;
                }
            };
            if let Err(e) = controller.submit_physical_car_event(physical).await {
                eprintln!("[actuation-ingress]: submit_physical_car_event failed: {e:?}");
            }
        }
    });
}
