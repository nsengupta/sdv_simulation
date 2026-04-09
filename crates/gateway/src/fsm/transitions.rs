use common::VehicleState;
use crate::fsm::model::VehicleContext;
use std::time::Duration;
use common::domain_types::STRESS_DURATION_THRESHOLD_SECS;

/// The "Pure" Decision Engine.
/// It looks at the current state and the accumulated context to determine
/// what the new state should be.
pub fn decide_next_state(current_state: VehicleState, ctx: &VehicleContext) -> VehicleState {
    match current_state {
        VehicleState::Operational => {
            if is_under_extreme_stress(ctx) {
                VehicleState::Warning
            } else {
                VehicleState::Operational
            }
        }
        VehicleState::Warning => {
            if is_recovered(ctx) {
                VehicleState::Operational
            } else if is_critical_failure(ctx) {
                VehicleState::Critical
            } else {
                VehicleState::Warning
            }
        }
        VehicleState::Critical => {
            // Usually requires a SystemReset event to exit
            VehicleState::Critical
        }
    }
}

// --- Domain-Specific Predicates ---

fn is_under_extreme_stress(ctx: &VehicleContext) -> bool {
    if let Some(start) = ctx.rpm_stress_timer {
        return start.elapsed() >= Duration::from_secs(STRESS_DURATION_THRESHOLD_SECS);
    }
    false
}

fn is_recovered(ctx: &VehicleContext) -> bool {
    // Logic: Stress timer has been cleared (RPM dropped)
    ctx.rpm_stress_timer.is_none()
}

fn is_critical_failure(_ctx: &VehicleContext) -> bool {
    // Placeholder for future logic (e.g. Engine Temperature > 120°C)
    false
}