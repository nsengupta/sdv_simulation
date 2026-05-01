use crate::domain_types::VehicleState;
use std::time::Instant;
use time::{OffsetDateTime, UtcOffset, macros::format_description};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingState {
    Off,
    OnRequested,
    On,
    OffRequested,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleContext {
    pub rpm: u16,
    pub speed: u8,
    pub fuel_level: u8,
    pub oil_pressure: u8,
    pub tyre_pressure_ok: bool,
    pub ambient_lux: u16,
    pub lighting_state: LightingState,
}

impl Default for VehicleContext {
    fn default() -> Self {
        Self {
            rpm: 0,
            speed: 0,
            fuel_level: 85,
            oil_pressure: 30,
            tyre_pressure_ok: true,
            ambient_lux: 100,
            lighting_state: LightingState::Off,
        }
    }
}

impl VehicleContext {
    pub fn is_healthy(&self) -> bool {
        self.fuel_level > 5 && self.oil_pressure > 10 && self.tyre_pressure_ok
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FsmState {
    Off,
    Idle,
    Driving,
    Warning(Instant),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FsmEvent {
    PowerOn,
    PowerOff,
    // Atomic updates from the bus
    UpdateRpm(u16),
    UpdateSpeed(u8),
    UpdateAmbientLux(u16),
    CornerLightsOnConfirmed,
    CornerLightsOffConfirmed,
    // Internal triggers
    TimerTick,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FsmAction {
    /// Trigger the physical buzzer (e.g., for overspeed/high RPM)
    StartBuzzer,
    /// Stop the physical buzzer
    StopBuzzer,
    /// Log a high-priority system alert
    LogWarning(String),
    /// Notify an external cloud/telemetry API of a state change
    PublishStateSync,
    /// No action required
    None,
}

impl FsmAction {
    /// The execute method now accepts the current state.
    /// This allows actions like 'PublishStateSync' to actually know
    /// WHAT state they are syncing without storing it redundantly.
    pub async fn execute(&self, current_fsm_state: &FsmState) {
        match self {
            Self::StartBuzzer => {
                println!(
                    "[ACTION @ {}]: 🔊 BUZZER ON - High Stress Detected!",
                    action_timestamp()
                );
            }
            Self::StopBuzzer => {
                println!(
                    "[ACTION @ {}]: 🔇 BUZZER OFF - System Normal.",
                    action_timestamp()
                );
            }
            Self::LogWarning(msg) => {
                eprintln!("[ALERT @ {}]: {}", action_timestamp(), msg);
            }
            Self::PublishStateSync => {
                // Here we use the parameter to perform the conversion
                let public_state = VehicleState::from(current_fsm_state);
                println!(
                    "[ACTION @ {}]: 📡 Publishing to Cloud: {:?}",
                    action_timestamp(),
                    public_state
                );
            }
            Self::None => {}
        }
    }
}

fn action_timestamp() -> String {
    let now = OffsetDateTime::now_utc().to_offset(UtcOffset::UTC);
    let hms = now
        .format(format_description!("[hour]:[minute]:[second]"))
        .unwrap_or_else(|_| "00:00:00".to_string());
    format!("{hms} {:09}", now.nanosecond())
}

impl From<&FsmState> for VehicleState {
    fn from(fsm: &FsmState) -> Self {
        match fsm {
            FsmState::Off => VehicleState::Off,
            FsmState::Idle => VehicleState::Idle,
            FsmState::Driving => VehicleState::Driving,
            FsmState::Warning(_) => VehicleState::Warning,
        }
    }
}
