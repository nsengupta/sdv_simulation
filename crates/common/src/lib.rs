pub mod domain_types;
pub mod signals;
pub mod virtual_car;

// Re-export for convenience
pub use domain_types::{VehicleEvent, VehicleState};
pub use signals::VssSignal;
pub use virtual_car::VirtualCar;