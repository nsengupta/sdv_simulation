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
use crate::engine::controller::actuation_manager::{
    ActuationManager, DefaultActuationManager,
};
use crate::engine::controller::vehicle_controller::VehicleControllerRuntimeOptions;
use crate::fsm::{self, ActorModeHintFromDomain, DomainAction, FsmEvent, FsmState, VehicleContext};
use crate::transition_sink::{RawTransitionRecord, TransitionRecordSink, TransitionSinkError};

/// The Digital Twin Actor
pub struct VirtualCarActor {
    transition_sink: Option<Arc<dyn TransitionRecordSink>>,
    actuation_manager: Arc<dyn ActuationManager>,
}

#[derive(Debug, Clone)]
pub struct VirtualCarActorArgs {
    pub identity: String,
    pub runtime_options: VehicleControllerRuntimeOptions,
}

impl From<String> for VirtualCarActorArgs {
    fn from(identity: String) -> Self {
        Self {
            identity,
            runtime_options: VehicleControllerRuntimeOptions::default(),
        }
    }
}

impl From<&str> for VirtualCarActorArgs {
    fn from(identity: &str) -> Self {
        Self::from(identity.to_string())
    }
}

pub struct VirtualCarRuntimeState {
    twin_car: DigitalTwinCar,
    next_sequence_no: u64,
    runtime_options: VehicleControllerRuntimeOptions,
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
            actuation_manager: Arc::new(DefaultActuationManager),
        }
    }
}

impl VirtualCarActor {
    #[allow(dead_code)]
    pub fn with_transition_sink(transition_sink: Arc<dyn TransitionRecordSink>) -> Self {
        Self {
            transition_sink: Some(transition_sink),
            actuation_manager: Arc::new(DefaultActuationManager),
        }
    }
}

#[async_trait]
impl Actor for VirtualCarActor {
    type Msg = DigitalTwinCarVocabulary;
    type State = VirtualCarRuntimeState;
    type Arguments = VirtualCarActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let identity = args.identity;
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
            runtime_options: args.runtime_options,
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
                if matches!(evt, FsmEvent::TimerTick) && runtime_state.runtime_options.log_timer_tick {
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
                        DomainAction::EnterMode(hint) => {
                            mode = match hint {
                                ActorModeHintFromDomain::Normal => ActorMode::Normal,
                                ActorModeHintFromDomain::Transitioning => ActorMode::Transitioning,
                            };
                        }
                        other_action => {
                            if let Err(err) = self
                                .actuation_manager
                                .execute(&other_action, &runtime_state.twin_car)
                                .await
                            {
                                eprintln!(
                                    "[{}]: actuation failure for {:?}: {:?}",
                                    runtime_state.twin_car.identity, other_action, err
                                );
                            }
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
