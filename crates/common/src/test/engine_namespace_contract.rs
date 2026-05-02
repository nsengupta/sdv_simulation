//! Compatibility tests for new engine namespace aliases.

use crate::fsm::VehicleContext;
use crate::engine::controller::virtual_car_actor::VirtualCarActor;
use crate::VehicleControllerContext;

#[test]
fn given_virtual_car_actor_when_named_then_is_explicitly_present_under_controller() {
    let actor_type_name = std::any::type_name::<VirtualCarActor>();
    assert!(
        actor_type_name.contains("VirtualCarActor"),
        "controller module should keep VirtualCarActor explicitly visible"
    );
}

#[test]
fn given_vehicle_controller_context_alias_when_defaulted_then_matches_vehicle_context_default() {
    let alias_ctx = VehicleControllerContext::default();
    let legacy_ctx = VehicleContext::default();

    assert_eq!(alias_ctx, legacy_ctx, "context alias must preserve semantics");
}
