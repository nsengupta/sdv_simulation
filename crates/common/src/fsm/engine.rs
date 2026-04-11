use super::machineries::{FsmAction, FsmEvent, FsmState, VehicleContext};
use std::time::Instant;

pub fn transition(state: &FsmState, event: &FsmEvent, ctx: &VehicleContext) -> FsmState {
    use FsmEvent::*;
    use FsmState::*;

    // [ CURRENT ]      | [ EVENT ]           | [ CONTEXT ]           | [ NEXT ]
    // -----------------|---------------------|-----------------------|------------
    match (state, event, ctx) {
        // --- POWER ON ---
        (Off, PowerOn, c) if c.is_healthy() => Idle,

        // --- POWER OFF: only valid from Idle (stationary / safe shutdown).
        // From other states, PowerOff is wrong input and is ignored (state unchanged).
        (Idle, PowerOff, _) => Off,

        // --- MOVEMENT LOGIC ---
        (Idle, UpdateRpm(rpm), _) if *rpm > 1000 => Driving,
        (Driving, UpdateSpeed(s), _) if *s == 0 => Idle,

        // --- WARNING LOGIC ---
        (Driving, UpdateRpm(rpm), _) if *rpm > 6000 => Warning(Instant::now()),

        // --- DEFAULT BEHAVIOR ---
        (current, event, _) => {
            if matches!(event, PowerOff) {
                eprintln!("[REJECTED]: PowerOff is invalid while in state {:?}", current);
            }
            current.clone()
        }
    }
}

pub fn output(old_state: &FsmState, new_state: &FsmState) -> Vec<FsmAction> {
    use FsmState::*;
    use FsmAction::*;

    match (old_state, new_state) {
        // Entering Warning state from Driving
        (Driving, Warning(_)) => vec![StartBuzzer, LogWarning("Overspeed detected!".to_string())],

        // Recovering from Warning back to Driving
        (Warning(_), Driving) => vec![StopBuzzer],

        // General state sync on any transition
        (old, new) if old != new => vec![PublishStateSync],

        _ => vec![],
    }
}
