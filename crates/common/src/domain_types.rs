use crate::signals::VssSignal;
pub use crate::vehicle_constants::{
    RPM_GREENLINE_THRESHOLD,
    RPM_IDLE,
    RPM_REDLINE_THRESHOLD,
    RPM_STRESS_DURATION_THRESHOLD_SECS,
};

use serde::{Deserialize, Serialize};

// These are your "DBC" constants.
// They are "User-Defined" for your specific vehicle platform.
pub const ID_SPEED: u32 = 0x123;
pub const ID_RPM:   u32 = 0x124;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleState {
    Off,
    Idle,
    Driving,
    Warning,
    Critical,
}
impl Default for VehicleState {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone)]
pub enum VehicleEvent {
    /// Data received from the Ingress Bus
    TelemetryUpdate(VssSignal),
    /// A system-generated heartbeat or check
    TimerTick,
    /// Emergency stop or system reset
    SystemReset,
}

/// Canonical physical-side vocabulary consumed by projection adapters.
#[derive(Debug, Clone)]
pub enum PhysicalCarVocabulary {
    /// Data received from the Ingress Bus
    TelemetryUpdate(VssSignal),
    /// A system-generated heartbeat or check
    TimerTick,
    /// Emergency stop or system reset
    SystemReset,
}