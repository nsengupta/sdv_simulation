//! Virtual ECU / gateway **actor** ([`ractor::Actor`]).
//!
//! ## Message layering
//! - **[`FsmEvent`](crate::fsm::FsmEvent)** — pure FSM vocabulary: `Clone`, no I/O ports.
//! - **[`DigitalTwinCarVocabulary`](crate::digital_twin::DigitalTwinCarVocabulary)** — full mailbox:
//!   wraps [`FsmEvent`](crate::fsm::FsmEvent) via [`DigitalTwinCarVocabulary::Fsm`] plus
//!   request/reply such as [`DigitalTwinCarVocabulary::GetStatus`] ([`RpcReplyPort`]).

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::digital_twin::{DigitalTwinCar, DigitalTwinCarVocabulary};
use crate::fsm::{self, FsmEvent, FsmState, VehicleContext};

/// The Digital Twin Actor
pub struct VirtualCarActor;

impl Default for VirtualCarActor {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Actor for VirtualCarActor {
    type Msg = DigitalTwinCarVocabulary;
    type State = DigitalTwinCar;
    type Arguments = String;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        identity: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        println!("[{identity}]: Initializing Digital Twin...");

        Ok(DigitalTwinCar {
            identity,
            current_state: FsmState::Off,
            context: VehicleContext::default(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        twin_car: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        use DigitalTwinCarVocabulary::{Fsm, GetStatus};

        match message {
            Fsm(evt) => {
                match &evt {
                    FsmEvent::UpdateRpm(r) => twin_car.context.rpm = *r,
                    FsmEvent::UpdateSpeed(s) => twin_car.context.speed = *s,
                    _ => {}
                }

                let next_state = fsm::transition(&twin_car.current_state, &evt, &twin_car.context);

                if next_state != twin_car.current_state {
                    let actions = fsm::output(&twin_car.current_state, &next_state);
                    twin_car.current_state = next_state;
                    for action in actions {
                        action.execute(&twin_car.current_state).await;
                    }
                    println!(
                        "[{}]: Transitioned to {:?}",
                        twin_car.identity, twin_car.current_state
                    );
                }
                Ok(())
            }
            GetStatus(reply) => Self::reply_get_status(reply, twin_car),
        }
    }
}

impl VirtualCarActor {
    fn reply_get_status(
        reply: RpcReplyPort<DigitalTwinCar>,
        twin_car: &DigitalTwinCar,
    ) -> Result<(), ActorProcessingErr> {
        if reply.is_closed() {
            return Ok(());
        }
        reply
            .send(twin_car.clone())
            .map_err(|e| std::io::Error::other(format!("GetStatus reply: {e:?}")))?;
        Ok(())
    }
}
