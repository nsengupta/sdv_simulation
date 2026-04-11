//! Digital twin runtime model: snapshot, invariants, and actor mailbox vocabulary.
//!
//! Depends on [`crate::fsm`] for [`FsmState`], [`FsmEvent`], and [`VehicleContext`] only — the FSM
//! crate module does not reference this layer.

mod car_behaviour_checker;

use crate::fsm::{FsmEvent, FsmState, VehicleContext};
use car_behaviour_checker::{law_kinetic_locking_holds, law_rpm_above_threshold_holds};
use ractor::RpcReplyPort;

/// Runtime snapshot of the vehicle digital twin: identity, FSM state, and sensor context.
#[derive(Debug, Clone)]
pub struct DigitalTwinCar {
    pub identity: String,
    /// The primary logical state of the digital twin
    pub current_state: FsmState,
    /// Sensor / health context associated with this twin
    pub context: VehicleContext,
}

impl DigitalTwinCar {
    /// Checks identity and context invariants on a snapshot (e.g. after `GetStatus`).
    /// The "Master Guardian"
    /// Returns Ok(()) if all safety laws are satisfied, or an Err describing the violation.
    pub fn verify_all_invariants(&self) -> Result<(), String> {
        if self.identity.is_empty() {
            return Err("identity must not be empty".to_owned());
        }
        if !self.context.is_healthy() {
            return Err("vehicle context failed health invariants".to_owned());
        }

        law_kinetic_locking_holds(&self.current_state, &self.context)?;
        law_rpm_above_threshold_holds(&self.current_state, &self.context)?;

        // 3. Add more 'Laws' here as the project grows...

        Ok(())
    }
}

/// Actor mailbox vocabulary for the digital twin: FSM traffic plus request/reply such as [`Self::GetStatus`].
///
/// [`FsmEvent`] stays `Clone` and free of [`RpcReplyPort`]; embed domain events via [`Self::Fsm`].
#[derive(Debug)]
pub enum DigitalTwinCarVocabulary {
    /// Drive the FSM (telemetry patches are applied in the actor before [`crate::fsm::transition`]).
    Fsm(FsmEvent),
    /// Return a snapshot of the twin; does **not** call [`crate::fsm::transition`].
    GetStatus(RpcReplyPort<DigitalTwinCar>),
}

impl From<FsmEvent> for DigitalTwinCarVocabulary {
    fn from(evt: FsmEvent) -> Self {
        Self::Fsm(evt)
    }
}

/// Returned when a [`DigitalTwinCarVocabulary`] is not an [`FsmEvent`] wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotFsmVocabulary;

impl TryFrom<DigitalTwinCarVocabulary> for FsmEvent {
    type Error = NotFsmVocabulary;

    fn try_from(value: DigitalTwinCarVocabulary) -> Result<Self, Self::Error> {
        match value {
            DigitalTwinCarVocabulary::Fsm(e) => Ok(e),
            DigitalTwinCarVocabulary::GetStatus(_) => Err(NotFsmVocabulary),
        }
    }
}

impl DigitalTwinCarVocabulary {
    /// Borrow the inner [`FsmEvent`] when this message is [`Self::Fsm`].
    pub fn as_fsm_event(&self) -> Option<&FsmEvent> {
        match self {
            Self::Fsm(e) => Some(e),
            Self::GetStatus(_) => None,
        }
    }

    /// Take the inner [`FsmEvent`] when this message is [`Self::Fsm`].
    pub fn into_fsm_event(self) -> Option<FsmEvent> {
        match self {
            Self::Fsm(e) => Some(e),
            Self::GetStatus(_) => None,
        }
    }
}
