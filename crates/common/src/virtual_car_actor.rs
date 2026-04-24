//! Virtual ECU / gateway **actor** ([`ractor::Actor`]).
//!
//! ## Message layering
//! - **[`FsmEvent`](crate::fsm::FsmEvent)** — pure FSM vocabulary: `Clone`, no I/O ports.
//! - **[`DigitalTwinCarVocabulary`](crate::digital_twin::DigitalTwinCarVocabulary)** — full mailbox:
//!   wraps [`FsmEvent`](crate::fsm::FsmEvent) via [`DigitalTwinCarVocabulary::Fsm`] plus
//!   request/reply such as [`DigitalTwinCarVocabulary::GetStatus`] ([`RpcReplyPort`]).

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::sync::Arc;

use crate::digital_twin::{DigitalTwinCar, DigitalTwinCarVocabulary};
use crate::fsm::{self, ActorModeHintFromDomain, DomainAction, FsmEvent, FsmState, VehicleContext};
use crate::transition_sink::{RawTransitionRecord, TransitionRecordSink, TransitionSinkError};

/// The Digital Twin Actor
pub struct VirtualCarActor {
    transition_sink: Option<Arc<dyn TransitionRecordSink>>,
}

pub struct VirtualCarRuntimeState {
    twin_car: DigitalTwinCar,
    next_sequence_no: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActorMode {
    Normal,
    Transitioning,
}

impl Default for VirtualCarActor {
    fn default() -> Self {
        Self {
            transition_sink: None,
        }
    }
}

impl VirtualCarActor {
    pub fn with_transition_sink(transition_sink: Arc<dyn TransitionRecordSink>) -> Self {
        Self {
            transition_sink: Some(transition_sink),
        }
    }
}

#[async_trait]
impl Actor for VirtualCarActor {
    type Msg = DigitalTwinCarVocabulary;
    type State = VirtualCarRuntimeState;
    type Arguments = String;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        identity: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        println!(
            "Physical Car name: {identity}, initializing its Digital Twin ..."
        );

        Ok(VirtualCarRuntimeState {
            twin_car: DigitalTwinCar {
                identity,
                current_state: FsmState::Off,
                context: VehicleContext::default(),
            },
            next_sequence_no: 1,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        runtime_state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        use DigitalTwinCarVocabulary::{Fsm, GetStatus};

        match message {
            Fsm(evt) => {
                if matches!(evt, FsmEvent::TimerTick) {
                    // TODO: rate-limit once structured logging is introduced.
                    println!(
                        "[{}]: received heartbeat TimerTick",
                        runtime_state.twin_car.identity
                    );
                }
                let result =
                    fsm::step(&runtime_state.twin_car.current_state, &runtime_state.twin_car.context, &evt, std::time::Instant::now());
                let old_state = runtime_state.twin_car.current_state.clone();
                let mut mode = ActorMode::Normal;

                // Persist actor state first (non-negotiable ordering before transition log emit).
                runtime_state.twin_car.current_state = result.next_state.clone();
                runtime_state.twin_car.context = result.modified_ctx;

                self.try_emit_transition_record(runtime_state, result.transition_record);

                for action in result.actions {
                    match action {
                        DomainAction::StartBuzzer => {
                            fsm::FsmAction::StartBuzzer.execute(&runtime_state.twin_car.current_state).await;
                        }
                        DomainAction::StopBuzzer => {
                            fsm::FsmAction::StopBuzzer.execute(&runtime_state.twin_car.current_state).await;
                        }
                        DomainAction::PublishStateSync => {
                            fsm::FsmAction::PublishStateSync.execute(&runtime_state.twin_car.current_state).await;
                        }
                        DomainAction::LogWarning(msg) => {
                            fsm::FsmAction::LogWarning(msg).execute(&runtime_state.twin_car.current_state).await;
                        }
                        DomainAction::EnterMode(hint) => {
                            mode = match hint {
                                ActorModeHintFromDomain::Normal => ActorMode::Normal,
                                ActorModeHintFromDomain::Transitioning => ActorMode::Transitioning,
                            };
                        }
                    }
                }

                if runtime_state.twin_car.current_state != old_state {
                    println!(
                        "[{}]: Transitioned to {:?}",
                        runtime_state.twin_car.identity, runtime_state.twin_car.current_state
                    );
                }
                let _ = mode;
                Ok(())
            }
            GetStatus(reply) => Self::reply_get_status(reply, &runtime_state.twin_car),
        }
    }
}

impl VirtualCarActor {
    fn try_emit_transition_record(
        &self,
        runtime_state: &mut VirtualCarRuntimeState,
        transition_record: fsm::TransitionRecord,
    ) {
        let Some(sink) = &self.transition_sink else {
            return;
        };

        let sequence_no = runtime_state.next_sequence_no;
        runtime_state.next_sequence_no = runtime_state.next_sequence_no.saturating_add(1);

        let raw = RawTransitionRecord {
            car_identity: runtime_state.twin_car.identity.clone(),
            sequence_no,
            transition: transition_record,
        };

        if let Err(err) = sink.try_emit(raw) {
            match err {
                TransitionSinkError::Full => {
                    eprintln!(
                        "[{}]: dropping transition record: sink full",
                        runtime_state.twin_car.identity
                    );
                }
                TransitionSinkError::Closed => {
                    eprintln!(
                        "[{}]: dropping transition record: sink closed",
                        runtime_state.twin_car.identity
                    );
                }
            }
        }
    }

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
