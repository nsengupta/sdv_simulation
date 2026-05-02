use super::projection::{Projector, ProjectionError};
use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::domain_types::PhysicalCarVocabulary;
use crate::fsm::FsmEvent;
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
            PhysicalCarVocabulary::CornerLightsOnConfirmed => FsmEvent::CornerLightsOnConfirmed,
            PhysicalCarVocabulary::CornerLightsOffConfirmed => FsmEvent::CornerLightsOffConfirmed,
        };
        Ok(DigitalTwinCarVocabulary::Fsm(fsm))
    }
}
