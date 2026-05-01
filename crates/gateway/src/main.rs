//! Gateway — CAN ingress speaks [`common::VehicleEvent`]; the actor consumes [`common::DigitalTwinCarVocabulary`].
//!
//! ## `DigitalTwinCarVocabulary`
//!
//! Commented examples in `main` (right after spawn): FSM sends, `ractor::call!` / `call_t!` for
//! `GetStatus`, and `TryInto` for FSM-only adapters. Uncomment that block to try them.

use anyhow::Result;
use common::fsm::FsmEvent;
use common::{VehicleController, VehicleControllerRuntimeOptions, VehicleEvent, VssSignal};
use socketcan::{CanSocket, Socket};
use std::{env, time::Duration};

mod ingress;

/// Virtual car identity passed to controller spawn (see [`common::DigitalTwinCar::identity`]).
const VIRTUAL_CAR_IDENTITY: &str = "NASHIK-VC-001";

#[tokio::main]
async fn main() -> Result<()> {
    let print_timer_tick = env::args().any(|arg| arg == "--print-timer-tick");

    let runtime_options = VehicleControllerRuntimeOptions {
        log_timer_tick: print_timer_tick,
    };
    let (controller, _join) = VehicleController::install_and_start_with_options(
        VIRTUAL_CAR_IDENTITY.to_string(),
        runtime_options,
    )
    .await
    .map_err(|e| anyhow::anyhow!("spawn actor: {e}"))?;

    // -------------------------------------------------------------------------
    // DigitalTwinCarVocabulary — uncomment to run (needs `use` lines below).
    // -------------------------------------------------------------------------
    // use common::DigitalTwinCar;
    // use common::NotFsmVocabulary;
    // use std::convert::TryInto;
    //
    // // (1) FSM — same as `FsmEvent::PowerOn.into()` / `send_message(...)`:
    // actor.send_message(DigitalTwinCarVocabulary::Fsm(FsmEvent::PowerOff))?;
    //
    // // (2) GetStatus — Req/Resp snapshot (`DigitalTwinCar`), wait forever:
    // let twin: DigitalTwinCar =
    //     ractor::call!(actor, DigitalTwinCarVocabulary::GetStatus).map_err(|e| anyhow::anyhow!("{e}"))?;
    // println!("[example] twin state: {:?}", twin.current_state);
    //
    // // (3) GetStatus — same with timeout (milliseconds, third macro arg):
    // let twin = ractor::call_t!(actor, DigitalTwinCarVocabulary::GetStatus, 500)
    //     .map_err(|e| anyhow::anyhow!("{e}"))?;
    //
    // // (4) Domain → vocabulary (same as CAN path):
    // actor.send_message(vehicle_event_to_vocabulary(VehicleEvent::TimerTick))?;
    //
    // // (5) FSM-only adapter: extract `FsmEvent` or `NotFsmVocabulary` for `GetStatus`:
    // let msg = DigitalTwinCarVocabulary::Fsm(FsmEvent::TimerTick);
    // let evt: FsmEvent = msg.try_into().map_err(|_: NotFsmVocabulary| anyhow::anyhow!("not FSM"))?;
    // let _ = evt;
    // -------------------------------------------------------------------------

    controller.actor_ref().send_message(FsmEvent::PowerOn.into())?;

    let tick_controller = controller.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let physical = ingress::vehicle_event_to_physical_vocabulary(VehicleEvent::TimerTick);
            let _ = tick_controller.submit_physical_car_event(physical).await;
        }
    });

    let socket = CanSocket::open("vcan0")?;
    println!("⚡ Gateway on vcan0 — CAN → VehicleEvent → PhysicalCarVocabulary → DigitalTwinCarVocabulary → VirtualCarActor");
    if print_timer_tick {
        println!("[gateway] TimerTick heartbeat logging enabled (--print-timer-tick)");
    }

    loop {
        let frame = socket.read_frame()?;
        if let Some(sig) = VssSignal::from_can_frame(&frame) {
            let ev = VehicleEvent::TelemetryUpdate(sig);
            let physical = ingress::vehicle_event_to_physical_vocabulary(ev);
            controller
                .submit_physical_car_event(physical)
                .await
                .map_err(|e| anyhow::anyhow!("submit physical car event: {e:?}"))?;
        }
    }
}
