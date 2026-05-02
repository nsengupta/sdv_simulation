//! FSM Step Contract (authoritative vocabulary)
//!
//! This module defines the single state-transition boundary:
//! `step(current_state, current_ctx, event, now) -> StepResult`.
//!
//! Canonical input model:
//! - `event` payload is canonical input.
//! - `current_ctx` is the materialized snapshot before processing this event.
//! - `modified_ctx` is produced by this step; callers must not mutate context outside `step`.
//!
//! Output model:
//! - `next_state`: state after this event.
//! - `modified_ctx`: context after this event.
//! - `actions`: pure domain intents (no hardware/network calls).
//! - `transition_record`: audit snapshot for observability/replay.
//!
//! Boundary rule:
//! - Domain emits [`ActorModeHintFromDomain`]; runtime actor owns `ActorMode` and mailbox behavior.

use super::machineries::{FsmAction, FsmEvent, FsmState, LightingState, VehicleContext};
use crate::engine::op_strategy::transition_map::{output, transition};
use crate::vehicle_constants::{LUX_OFF_THRESHOLD, LUX_ON_THRESHOLD};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorModeHintFromDomain {
    Normal,
    Transitioning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomainAction {
    StartBuzzer,
    StopBuzzer,
    PublishStateSync,
    LogWarning(String),
    RequestCornerLightsOn,
    RequestCornerLightsOff,
    EnterMode(ActorModeHintFromDomain),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionRecord {
    pub at: Instant,
    pub event: FsmEvent,
    pub old_state: FsmState,
    pub next_state: FsmState,
    pub old_ctx: VehicleContext,
    pub current_ctx: VehicleContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    pub next_state: FsmState,
    pub modified_ctx: VehicleContext,
    pub actions: Vec<DomainAction>,
    pub transition_record: TransitionRecord,
}

pub fn step(
    current_state: &FsmState,
    current_ctx: &VehicleContext,
    event: &FsmEvent,
    now: Instant,
) -> StepResult {
    let mut modified_ctx = current_ctx.clone();
    match event {
        FsmEvent::UpdateRpm(rpm) => modified_ctx.rpm = *rpm,
        FsmEvent::UpdateSpeed(speed) => modified_ctx.speed = *speed,
        FsmEvent::UpdateAmbientLux(lux) => modified_ctx.ambient_lux = *lux,
        FsmEvent::CornerLightsOnConfirmed => modified_ctx.lighting_state = LightingState::On,
        FsmEvent::CornerLightsOffConfirmed => modified_ctx.lighting_state = LightingState::Off,
        FsmEvent::PowerOn | FsmEvent::PowerOff | FsmEvent::TimerTick => {}
    }

    let next_state = transition(current_state, event, &modified_ctx, now);
    let mut actions: Vec<DomainAction> = output(current_state, &next_state)
        .into_iter()
        .filter_map(map_fsm_action)
        .collect();

    match (&current_ctx.lighting_state, event) {
        (LightingState::Off, FsmEvent::UpdateAmbientLux(lux)) if *lux <= LUX_ON_THRESHOLD => {
            modified_ctx.lighting_state = LightingState::OnRequested;
            actions.push(DomainAction::RequestCornerLightsOn);
        }
        (LightingState::On, FsmEvent::UpdateAmbientLux(lux)) if *lux >= LUX_OFF_THRESHOLD => {
            modified_ctx.lighting_state = LightingState::OffRequested;
            actions.push(DomainAction::RequestCornerLightsOff);
        }
        _ => {}
    }

    if matches!(next_state, FsmState::Warning(_)) {
        actions.push(DomainAction::EnterMode(ActorModeHintFromDomain::Transitioning));
    } else {
        actions.push(DomainAction::EnterMode(ActorModeHintFromDomain::Normal));
    }

    StepResult {
        next_state: next_state.clone(),
        modified_ctx: modified_ctx.clone(),
        actions,
        transition_record: TransitionRecord {
            at: now,
            event: event.clone(),
            old_state: current_state.clone(),
            next_state,
            old_ctx: current_ctx.clone(),
            current_ctx: modified_ctx,
        },
    }
}

fn map_fsm_action(action: FsmAction) -> Option<DomainAction> {
    match action {
        FsmAction::StartBuzzer => Some(DomainAction::StartBuzzer),
        FsmAction::StopBuzzer => Some(DomainAction::StopBuzzer),
        FsmAction::PublishStateSync => Some(DomainAction::PublishStateSync),
        FsmAction::LogWarning(msg) => Some(DomainAction::LogWarning(msg)),
        FsmAction::None => None,
    }
}
