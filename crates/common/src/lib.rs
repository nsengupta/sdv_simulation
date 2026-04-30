// Sibling order is *dependee before dependent* (foundation first), not "flow" order.
// `digital_twin` imports `fsm`; `fsm` does not import `digital_twin`.
pub mod domain_types;
pub mod fsm;
pub mod digital_twin;
pub mod signals;
pub mod transition_sink;
pub mod vehicle_constants;
pub mod virtual_car_actor;

#[cfg(test)]
mod test;

pub use digital_twin::{DigitalTwinCar, DigitalTwinCarVocabulary, NotFsmVocabulary};
pub use domain_types::{VehicleEvent, VehicleState};
pub use signals::VssSignal;
pub use transition_sink::{
    RawTransitionRecord, TokioMpscTransitionRecordSink, TransitionRecordSink, TransitionSinkError,
};
pub use virtual_car_actor::VirtualCarActor;
