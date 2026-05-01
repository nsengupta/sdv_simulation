//! Contract tests for projection boundaries between physical and digital vocabularies.

use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::engine::connectors::{PhysicalToDigitalProjector, Projector};
use crate::fsm::FsmEvent;
use crate::{PhysicalCarVocabulary, VssSignal};

#[test]
fn given_timer_tick_when_projected_then_maps_to_fsm_timer_tick() {
    let projector = PhysicalToDigitalProjector;
    let out = projector
        .project(PhysicalCarVocabulary::TimerTick)
        .expect("projection must succeed");
    match out {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::TimerTick) => {}
        other => panic!("unexpected timer tick mapping: {other:?}"),
    }
}

#[test]
fn given_system_reset_when_projected_then_maps_to_fsm_power_off() {
    let projector = PhysicalToDigitalProjector;
    let out = projector
        .project(PhysicalCarVocabulary::SystemReset)
        .expect("projection must succeed");
    match out {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::PowerOff) => {}
        other => panic!("unexpected reset mapping: {other:?}"),
    }
}

#[test]
fn given_speed_signal_when_projected_then_clamps_to_u8_bounds() {
    let projector = PhysicalToDigitalProjector;
    let low = projector
        .project(PhysicalCarVocabulary::TelemetryUpdate(VssSignal::VehicleSpeed(-1.0)))
        .expect("low speed projection must succeed");
    let high = projector
        .project(PhysicalCarVocabulary::TelemetryUpdate(VssSignal::VehicleSpeed(500.0)))
        .expect("high speed projection must succeed");

    match low {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateSpeed(v)) => assert_eq!(v, 0),
        other => panic!("unexpected low speed mapping: {other:?}"),
    }
    match high {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateSpeed(v)) => assert_eq!(v, 255),
        other => panic!("unexpected high speed mapping: {other:?}"),
    }
}

#[test]
fn given_rpm_signal_when_projected_then_maps_exact_rpm() {
    let projector = PhysicalToDigitalProjector;
    let out = projector
        .project(PhysicalCarVocabulary::TelemetryUpdate(VssSignal::EngineRpm(4321)))
        .expect("rpm projection must succeed");
    match out {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateRpm(v)) => assert_eq!(v, 4321),
        other => panic!("unexpected rpm mapping: {other:?}"),
    }
}

#[test]
fn given_ambient_lux_signal_when_projected_then_maps_to_fsm_ambient_lux() {
    let projector = PhysicalToDigitalProjector;
    let out = projector
        .project(PhysicalCarVocabulary::TelemetryUpdate(VssSignal::AmbientLux(28)))
        .expect("ambient lux projection must succeed");
    match out {
        DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateAmbientLux(v)) => assert_eq!(v, 28),
        other => panic!("unexpected ambient lux mapping: {other:?}"),
    }
}
