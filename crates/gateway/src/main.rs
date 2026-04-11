//! Gateway — CAN ingress speaks [`common::VehicleEvent`]; the actor consumes [`common::DigitalTwinCarVocabulary`].
//!
//! ## `DigitalTwinCarVocabulary`
//!
//! Commented examples in `main` (right after spawn): FSM sends, `ractor::call!` / `call_t!` for
//! `GetStatus`, and `TryInto` for FSM-only adapters. Uncomment that block to try them.

use anyhow::Result;
use common::fsm::FsmEvent;
use common::{DigitalTwinCarVocabulary, VehicleEvent, VirtualCarActor, VssSignal};
use ractor;
use socketcan::{CanSocket, Socket};
use std::time::Duration;

/// Virtual car identity passed to [`VirtualCarActor`] at spawn (see [`common::DigitalTwinCar::identity`]).
const VIRTUAL_CAR_IDENTITY: &str = "NASHIK-VC-001";

/// Maps ingress / domain events to FSM events, then wraps them for the actor mailbox.
fn vehicle_event_to_vocabulary(ev: VehicleEvent) -> DigitalTwinCarVocabulary {
    let fsm = match ev {
        VehicleEvent::TelemetryUpdate(vss) => match vss {
            VssSignal::VehicleSpeed(kmh) => FsmEvent::UpdateSpeed(kmh.clamp(0.0, 255.0) as u8),
            VssSignal::EngineRpm(rpm) => FsmEvent::UpdateRpm(rpm),
        },
        VehicleEvent::TimerTick => FsmEvent::TimerTick,
        VehicleEvent::SystemReset => FsmEvent::PowerOff,
    };
    DigitalTwinCarVocabulary::Fsm(fsm)
}

#[tokio::main]
async fn main() -> Result<()> {
    let (actor, _join) = ractor::spawn::<VirtualCarActor>(VIRTUAL_CAR_IDENTITY.to_string())
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

    actor.send_message(FsmEvent::PowerOn.into())?;

    let tick = actor.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = tick.send_message(vehicle_event_to_vocabulary(VehicleEvent::TimerTick));
        }
    });

    let socket = CanSocket::open("vcan0")?;
    println!("⚡ Gateway on vcan0 — CAN → VehicleEvent → DigitalTwinCarVocabulary → VirtualCarActor");

    loop {
        let frame = socket.read_frame()?;
        if let Some(sig) = VssSignal::from_can_frame(&frame) {
            let ev = VehicleEvent::TelemetryUpdate(sig);
            actor.send_message(vehicle_event_to_vocabulary(ev))?;
        }
    }
}
