use crate::signals::VssSignal;

// --- Vehicle Calibration Constants ---
pub const RPM_IDLE: u16 = 800;
pub const RPM_REDLINE: u16 = 7000;
pub const RPM_STRESS_THRESHOLD: u16 = 6000;

pub const STRESS_DURATION_THRESHOLD_SECS: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    Operational, // Normal state
    Warning,     // Engine Stress or Overspeed
    Critical,    // Hardware failure or extreme stress
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