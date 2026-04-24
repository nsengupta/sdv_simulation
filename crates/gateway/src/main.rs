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

#[cfg(test)]
mod tests {
    use super::vehicle_event_to_vocabulary;
    use common::fsm::FsmEvent;
    use common::{DigitalTwinCarVocabulary, VehicleEvent, VssSignal};

    #[test]
    fn smoke_timer_tick_maps_to_fsm_timer_tick() {
        let msg = vehicle_event_to_vocabulary(VehicleEvent::TimerTick);
        match msg {
            DigitalTwinCarVocabulary::Fsm(FsmEvent::TimerTick) => {}
            other => panic!("unexpected mapping: {other:?}"),
        }
    }

    #[test]
    fn smoke_system_reset_maps_to_power_off() {
        let msg = vehicle_event_to_vocabulary(VehicleEvent::SystemReset);
        match msg {
            DigitalTwinCarVocabulary::Fsm(FsmEvent::PowerOff) => {}
            other => panic!("unexpected mapping: {other:?}"),
        }
    }

    #[test]
    fn smoke_vehicle_speed_is_clamped_before_update_speed() {
        let low = vehicle_event_to_vocabulary(VehicleEvent::TelemetryUpdate(
            VssSignal::VehicleSpeed(-4.0),
        ));
        let high = vehicle_event_to_vocabulary(VehicleEvent::TelemetryUpdate(
            VssSignal::VehicleSpeed(500.0),
        ));

        match low {
            DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateSpeed(v)) => assert_eq!(v, 0),
            other => panic!("unexpected low-speed mapping: {other:?}"),
        }
        match high {
            DigitalTwinCarVocabulary::Fsm(FsmEvent::UpdateSpeed(v)) => assert_eq!(v, 255),
            other => panic!("unexpected high-speed mapping: {other:?}"),
        }
    }
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
