//! Unit tests for the vehicle FSM (`transition` / `output`).

use crate::digital_twin::DigitalTwinCar;
use crate::fsm::{output, transition, FsmAction, FsmEvent, FsmState, VehicleContext};

/// Healthy `VehicleContext` matching a valid digital twin (same values as `VehicleContext::default()`).
fn valid_twin_context() -> VehicleContext {
    VehicleContext {
        rpm: 0,
        speed: 0,
        fuel_level: 85,
        oil_pressure: 30,
        tyre_pressure_ok: true,
    }
}

#[test]
fn test_high_rpm_warning_and_cooldown() {
    let mut ctx = valid_twin_context();
    let mut state = FsmState::Idle;

    // 1. Move to Driving
    ctx.rpm = 1200;
    state = transition(&state, &FsmEvent::UpdateRpm(1200), &ctx);
    assert_eq!(state, FsmState::Driving);

    // 2. Trigger Warning
    state = transition(&state, &FsmEvent::UpdateRpm(6500), &ctx);
    match state {
        FsmState::Warning(_) => (), // Success
        _ => panic!("Should have transitioned to Warning"),
    }

    // 3. Check Cooldown (should NOT transition back yet even if RPM is low)
    let actions = output(&FsmState::Driving, &state);
    assert!(actions.contains(&FsmAction::StartBuzzer));

    state = transition(&state, &FsmEvent::UpdateRpm(3000), &ctx);
    // Still in warning because 5 seconds haven't passed
    match state {
        FsmState::Warning(_) => (),
        _ => panic!("Should still be in Warning during cooldown"),
    }
}

#[test]
fn test_standard_commute_flow() {
    let mut car = DigitalTwinCar {
        identity: "NASHIK-VC-001".to_string(),
        current_state: FsmState::Off,
        context: valid_twin_context(),
    };

    let sequence = vec![
        (FsmEvent::PowerOn, FsmState::Idle),
        (FsmEvent::UpdateRpm(1500), FsmState::Driving),
        (FsmEvent::UpdateSpeed(50), FsmState::Driving),
        (FsmEvent::UpdateSpeed(0), FsmState::Idle),
        (FsmEvent::PowerOff, FsmState::Off),
    ];

    for (event, expected_state) in sequence {
        car.current_state = transition(&car.current_state, &event, &car.context);
        assert_eq!(car.current_state, expected_state);
    }
}

#[test]
fn test_illegal_shutdown_attempt() {
    let mut car = DigitalTwinCar {
        identity: "NASHIK-VC-001".to_string(),
        current_state: FsmState::Driving,
        context: VehicleContext {
            rpm: 3000,
            speed: 80,
            ..VehicleContext::default()
        },
    };

    // Attempting PowerOff while Driving
    car.current_state = transition(&car.current_state, &FsmEvent::PowerOff, &car.context);

    // Invariant: Should still be Driving
    assert_eq!(car.current_state, FsmState::Driving);
}
