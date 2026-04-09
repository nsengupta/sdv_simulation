use std::time::Instant;
use common::VehicleState;

pub struct VehicleContext {
    // Current telemetry snapshot
    pub speed: f64,
    pub rpm: u16,

    // HSM Logic Helpers
    pub rpm_stress_timer: Option<Instant>,
    pub current_state: VehicleState,
}

impl VehicleContext {
    pub fn new() -> Self {
        Self {
            speed: 0.0,
            rpm: 0,
            rpm_stress_timer: None,
            current_state: VehicleState::Operational,
        }
    }

    /// Reset the stress timer if we drop below threshold
    pub fn clear_stress(&mut self) {
        self.rpm_stress_timer = None;
    }

    /// Start the timer if it's not already running
    pub fn start_stress_timer(&mut self) {
        if self.rpm_stress_timer.is_none() {
            self.rpm_stress_timer = Some(Instant::now());
        }
    }
}