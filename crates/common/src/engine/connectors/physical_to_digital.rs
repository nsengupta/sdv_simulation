use super::projection::{Projector, ProjectionError};
use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::domain_types::PhysicalCarVocabulary;
use crate::fsm::{CornerLightsIncompleteCause, CornerLightsSwitchDirection, FsmEvent};
use crate::signals::VssSignal;

#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicalToDigitalProjector;

impl Projector<PhysicalCarVocabulary, DigitalTwinCarVocabulary> for PhysicalToDigitalProjector {
    fn project(&self, input: PhysicalCarVocabulary) -> Result<DigitalTwinCarVocabulary, ProjectionError> {
        let fsm = match input {
            PhysicalCarVocabulary::TelemetryUpdate(vss) => match vss {
                VssSignal::VehicleSpeed(kmh) => FsmEvent::UpdateSpeed(kmh.clamp(0.0, 255.0) as u8),
                VssSignal::EngineRpm(rpm) => FsmEvent::UpdateRpm(rpm),
                VssSignal::AmbientLux(lux) => FsmEvent::UpdateAmbientLux(lux),
            },
            PhysicalCarVocabulary::TimerTick => FsmEvent::TimerTick,
            PhysicalCarVocabulary::SystemReset => FsmEvent::PowerOff,
            PhysicalCarVocabulary::CornerLightsCommandConfirmed { on_command } => {
                if on_command {
                    FsmEvent::CornerLightsOnConfirmed
                } else {
                    FsmEvent::CornerLightsOffConfirmed
                }
            }
            PhysicalCarVocabulary::CornerLightsCommandRejected { on_command } => {
                FsmEvent::CornerLightsActuationIncomplete {
                    direction: if on_command {
                        CornerLightsSwitchDirection::On
                    } else {
                        CornerLightsSwitchDirection::Off
                    },
                    cause: CornerLightsIncompleteCause::NegativeAck,
                }
            }
        };
        Ok(DigitalTwinCarVocabulary::Fsm(fsm))
    }
}
