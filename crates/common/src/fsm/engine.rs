use super::machineries::{FsmAction, FsmEvent, FsmState, VehicleContext};
use crate::vehicle_constants::{RPM_GREENLINE_THRESHOLD, RPM_STRESS_DURATION_THRESHOLD_SECS};
use std::time::{Duration, Instant};

const RPM_RECOVERY_THRESHOLD: u16 = 5000;

/// Transition spec (runtime source of truth).
///
/// Human table:
/// - Off + PowerOn(healthy ctx) -> Idle
/// - Idle + PowerOff -> Off
/// - Idle + UpdateRpm(rpm > 1000) -> Driving
/// - Driving + UpdateSpeed(0) -> Idle
/// - Driving + UpdateRpm(rpm > stress threshold) -> Warning(now)
/// - Warning + TimerTick + cooldown elapsed + rpm <= recovery threshold -> Driving/Idle
/// - Everything else -> stay in current state
pub fn transition(
    current_state: &FsmState,
    event: &FsmEvent,
    current_ctx: &VehicleContext,
    now: Instant,
) -> FsmState {
    use FsmEvent::*;
    use FsmState::*;

    match current_state {
        Off => match event {
            PowerOn if current_ctx.is_healthy() => Idle,
            PowerOff => {
                eprintln!("[REJECTED]: PowerOff is invalid while in state {:?}", current_state);
                Off
            }
            _ => Off,
        },
        Idle => match event {
            PowerOff => Off,
            UpdateRpm(rpm) if *rpm > 1000 => Driving,
            _ => Idle,
        },
        Driving => match event {
            UpdateSpeed(speed) if *speed == 0 => Idle,
            UpdateRpm(rpm) if *rpm > RPM_GREENLINE_THRESHOLD => Warning(now),
            PowerOff => {
                eprintln!("[REJECTED]: PowerOff is invalid while in state {:?}", current_state);
                Driving
            }
            _ => Driving,
        },
        Warning(began_at) => match event {
            TimerTick if warning_recovery_ready(*began_at, now, current_ctx.rpm) => {
                if current_ctx.speed == 0 {
                    Idle
                } else {
                    Driving
                }
            }
            PowerOff => {
                eprintln!("[REJECTED]: PowerOff is invalid while in state {:?}", current_state);
                Warning(*began_at)
            }
            _ => Warning(*began_at),
        },
    }
}

fn warning_recovery_ready(began_at: Instant, now: Instant, rpm: u16) -> bool {
    let warning_age = now
        .checked_duration_since(began_at)
        .unwrap_or(Duration::ZERO);
    warning_age >= Duration::from_secs(RPM_STRESS_DURATION_THRESHOLD_SECS) && rpm <= RPM_RECOVERY_THRESHOLD
}

pub fn output(old_state: &FsmState, new_state: &FsmState) -> Vec<FsmAction> {
    use FsmState::*;
    use FsmAction::*;

    match (old_state, new_state) {
        // Entering Warning state from Driving
        (Driving, Warning(_)) => vec![StartBuzzer, LogWarning("Overspeed detected!".to_string())],

        // Recovering from Warning
        (Warning(_), Driving) | (Warning(_), Idle) => vec![StopBuzzer],

        // General state sync on any transition
        (old, new) if old != new => vec![PublishStateSync],

        _ => vec![],
    }
}
