// Sibling order is *dependee before dependent* (foundation first), not "flow" order.
// `digital_twin` imports `fsm`; `fsm` does not import `digital_twin`.
pub mod domain_types;
pub mod engine;
pub mod fsm;
pub mod digital_twin;
pub mod signals;
pub mod transition_sink;
pub mod vehicle_constants;
mod virtual_car_actor;

#[cfg(test)]
mod test;

pub use digital_twin::{DigitalTwinCar, DigitalTwinCarVocabulary, NotFsmVocabulary};
pub use domain_types::{PhysicalCarVocabulary, VehicleEvent, VehicleState};
pub use engine::connectors::{PhysicalToDigitalProjector, Projector, ProjectionError};
pub use engine::context::VehicleControllerContext;
pub use engine::controller::{
    ActuationError, ActuationManager, DefaultActuationManager, VehicleController,
    VehicleControllerError, VehicleControllerRuntimeOptions,
};
pub use signals::VssSignal;
pub use transition_sink::{
    RawTransitionRecord, TokioMpscTransitionRecordSink, TransitionRecordSink, TransitionSinkError,
};
