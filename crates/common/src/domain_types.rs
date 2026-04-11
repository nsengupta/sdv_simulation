use crate::signals::VssSignal;

// --- Vehicle Calibration Constants ---
pub const RPM_IDLE: u16 = 800;
pub const RPM_REDLINE: u16 = 7000;
pub const RPM_STRESS_THRESHOLD: u16 = 6000;

pub const STRESS_DURATION_THRESHOLD_SECS: u64 = 5;

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