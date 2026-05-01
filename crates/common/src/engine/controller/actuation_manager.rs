use async_trait::async_trait;
use time::{OffsetDateTime, UtcOffset, macros::format_description};

use crate::digital_twin::DigitalTwinCar;
use crate::domain_types::VehicleState;
use crate::fsm::DomainAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuationError {
    UnsupportedAction(&'static str),
}

#[async_trait]
pub trait ActuationManager: Send + Sync {
    async fn execute(
        &self,
        action: &DomainAction,
        twin: &DigitalTwinCar,
    ) -> Result<(), ActuationError>;
}

#[derive(Debug, Default)]
pub struct DefaultActuationManager;

#[async_trait]
impl ActuationManager for DefaultActuationManager {
    async fn execute(
        &self,
        action: &DomainAction,
        twin: &DigitalTwinCar,
    ) -> Result<(), ActuationError> {
        match action {
            DomainAction::StartBuzzer => {
                // TODO(actuation-child-actor): offload connector I/O to a child actor
                // and keep parent actor loop non-blocking under slow transports.
                println!(
                    "[ACTION @ {}]: 🔊 BUZZER ON - High Stress Detected!",
                    action_timestamp()
                );
            }
            DomainAction::StopBuzzer => {
                // TODO(actuation-child-actor): offload connector I/O to a child actor
                // and keep parent actor loop non-blocking under slow transports.
                println!(
                    "[ACTION @ {}]: 🔇 BUZZER OFF - System Normal.",
                    action_timestamp()
                );
            }
            DomainAction::PublishStateSync => {
                // TODO(actuation-egress): publish through an injected egress connector
                // (CAN/Zenoh/uProtocol) instead of default stdout logging.
                let public_state = VehicleState::from(&twin.current_state);
                println!(
                    "[ACTION @ {}]: 📡 Publishing to Cloud: {:?}",
                    action_timestamp(),
                    public_state
                );
            }
            DomainAction::LogWarning(msg) => {
                // TODO(actuation-observability): route structured warnings to an
                // injected logging/event sink.
                eprintln!("[ALERT @ {}]: {}", action_timestamp(), msg);
            }
            DomainAction::RequestCornerLightsOn => {
                // TODO(actuation-child-actor): move actuator command execution to a
                // dedicated actuation child actor for robust ordering/backpressure.
                println!(
                    "[ACTION @ {}]: 💡 Requesting front corner lights ON.",
                    action_timestamp()
                );
            }
            DomainAction::RequestCornerLightsOff => {
                // TODO(actuation-child-actor): move actuator command execution to a
                // dedicated actuation child actor for robust ordering/backpressure.
                println!(
                    "[ACTION @ {}]: 💡 Requesting front corner lights OFF.",
                    action_timestamp()
                );
            }
            DomainAction::EnterMode(_) => {}
        }

        Ok(())
    }
}

fn action_timestamp() -> String {
    let now = OffsetDateTime::now_utc().to_offset(UtcOffset::UTC);
    let hms = now
        .format(format_description!("[hour]:[minute]:[second]"))
        .unwrap_or_else(|_| "00:00:00".to_string());
    format!("{hms} {:09}", now.nanosecond())
}
