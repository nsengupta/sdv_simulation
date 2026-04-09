pub mod model;
pub mod transitions;

use common::{VehicleEvent, VehicleState, VssSignal};
pub use model::VehicleContext;
pub use transitions::decide_next_state;
pub use common::domain_types::RPM_STRESS_THRESHOLD;

pub fn handle_vehicle_event(ctx: &mut VehicleContext, event: VehicleEvent) {
    // 1. Update Context Data (The "Imperative" part)
    match event {
        VehicleEvent::TelemetryUpdate(signal) => match signal {
            VssSignal::VehicleSpeed(s) => ctx.speed = s,
            VssSignal::EngineRpm(r) => {
                ctx.rpm = r;
                if r > RPM_STRESS_THRESHOLD
                {
                    println!("rpm above threshold={}, starting stress timer", RPM_STRESS_THRESHOLD);
                    ctx.start_stress_timer();
                } else {
                    println!("rpm below threshold={}, clearing stress timer",  RPM_STRESS_THRESHOLD);
                    ctx.clear_stress();
                }
            }
        },
        VehicleEvent::TimerTick => { /* Context updates based on time if needed */ },
        VehicleEvent::SystemReset => {
            ctx.clear_stress();
            ctx.current_state = VehicleState::Operational;
            return;
        }
    }

    // 2. Decide and Transition
    let next_state = decide_next_state(ctx.current_state, ctx);

    // 3. Log transitions (Side Effects)
    if next_state != ctx.current_state {
        ctx.current_state = next_state;
    }

    println!("🔄 [TRANSITION] {:?} -> {:?}", ctx.current_state, next_state);
}