//! Behavioral contract tests for lighting sub-state behavior.

use crate::fsm::{step, DomainAction, FsmEvent, FsmState, LightingState, VehicleContext};
use std::time::Instant;

fn valid_twin_context() -> VehicleContext {
    VehicleContext {
        rpm: 0,
        speed: 0,
        fuel_level: 85,
        oil_pressure: 30,
        tyre_pressure_ok: true,
        ambient_lux: 100,
        lighting_state: LightingState::Off,
    }
}

#[test]
fn given_lights_off_when_lux_below_on_threshold_then_requests_corner_lights_on() {
    let current_state = FsmState::Idle;
    let current_ctx = valid_twin_context();

    let result = step(
        &current_state,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(20),
        Instant::now(),
    );

    assert!(result
        .actions
        .contains(&DomainAction::RequestCornerLightsOn));
}

#[test]
fn given_on_requested_when_ack_on_then_no_duplicate_on_request_emitted() {
    let current_state = FsmState::Idle;
    let current_ctx = VehicleContext {
        lighting_state: LightingState::OnRequested,
        ambient_lux: 20,
        ..valid_twin_context()
    };

    let result = step(
        &current_state,
        &current_ctx,
        &FsmEvent::CornerLightsOnConfirmed,
        Instant::now(),
    );

    assert!(!result
        .actions
        .contains(&DomainAction::RequestCornerLightsOn));
}

#[test]
fn given_lights_on_when_lux_above_off_threshold_then_requests_corner_lights_off() {
    let current_state = FsmState::Driving;
    let current_ctx = VehicleContext {
        lighting_state: LightingState::On,
        ambient_lux: 50,
        ..valid_twin_context()
    };

    let result = step(
        &current_state,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(50),
        Instant::now(),
    );

    assert!(result
        .actions
        .contains(&DomainAction::RequestCornerLightsOff));
}

#[test]
fn given_lights_off_when_lux_at_on_threshold_then_requests_corner_lights_on() {
    let result = step(
        &FsmState::Idle,
        &valid_twin_context(),
        &FsmEvent::UpdateAmbientLux(30),
        Instant::now(),
    );

    assert!(result
        .actions
        .contains(&DomainAction::RequestCornerLightsOn));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::OnRequested);
}

#[test]
fn given_lights_off_when_lux_in_deadband_then_does_not_request_corner_lights_on() {
    let result = step(
        &FsmState::Idle,
        &valid_twin_context(),
        &FsmEvent::UpdateAmbientLux(40),
        Instant::now(),
    );

    assert!(!result
        .actions
        .contains(&DomainAction::RequestCornerLightsOn));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::Off);
}

#[test]
fn given_lights_on_when_lux_at_off_threshold_then_requests_corner_lights_off() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::On,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Driving,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(45),
        Instant::now(),
    );

    assert!(result
        .actions
        .contains(&DomainAction::RequestCornerLightsOff));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::OffRequested);
}

#[test]
fn given_lights_on_when_lux_in_deadband_then_does_not_request_corner_lights_off() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::On,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Driving,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(40),
        Instant::now(),
    );

    assert!(!result
        .actions
        .contains(&DomainAction::RequestCornerLightsOff));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::On);
}

#[test]
fn given_lights_on_requested_when_low_lux_arrives_then_does_not_emit_duplicate_on_request() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::OnRequested,
        ambient_lux: 20,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Idle,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(20),
        Instant::now(),
    );

    assert!(!result
        .actions
        .contains(&DomainAction::RequestCornerLightsOn));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::OnRequested);
}

#[test]
fn given_lights_off_requested_when_high_lux_arrives_then_does_not_emit_duplicate_off_request() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::OffRequested,
        ambient_lux: 50,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Driving,
        &current_ctx,
        &FsmEvent::UpdateAmbientLux(50),
        Instant::now(),
    );

    assert!(!result
        .actions
        .contains(&DomainAction::RequestCornerLightsOff));
    assert_eq!(result.modified_ctx.lighting_state, LightingState::OffRequested);
}

#[test]
fn given_on_requested_when_ack_on_then_transitions_to_on() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::OnRequested,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Driving,
        &current_ctx,
        &FsmEvent::CornerLightsOnConfirmed,
        Instant::now(),
    );
    assert_eq!(result.modified_ctx.lighting_state, LightingState::On);
}

#[test]
fn given_off_requested_when_ack_off_then_transitions_to_off() {
    let current_ctx = VehicleContext {
        lighting_state: LightingState::OffRequested,
        ..valid_twin_context()
    };
    let result = step(
        &FsmState::Driving,
        &current_ctx,
        &FsmEvent::CornerLightsOffConfirmed,
        Instant::now(),
    );
    assert_eq!(result.modified_ctx.lighting_state, LightingState::Off);
}
