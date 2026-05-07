//! Vehicle-wide calibration constants shared across FSM/domain logic.
//!
//! Keep only globally meaningful tuning values here. Module-local implementation
//! constants should remain in their owning modules.

use std::time::Duration;

pub const RPM_IDLE: u16 = 800;
pub const RPM_REDLINE_THRESHOLD: u16 = 7000;
pub const RPM_GREENLINE_THRESHOLD: u16 = 6000;

pub const RPM_STRESS_DURATION_THRESHOLD_SECS: u64 = 5;

pub const LUX_ON_THRESHOLD: u16 = 30;
pub const LUX_OFF_THRESHOLD: u16 = 45;

/// Maximum time to wait for an ON command ACK before timeout recovery.
pub const CORNER_LIGHTS_ON_ACK_WAIT: Duration = Duration::from_secs(2);
/// Maximum time to wait for an OFF command ACK before timeout recovery.
pub const CORNER_LIGHTS_OFF_ACK_WAIT: Duration = Duration::from_secs(2);
