//! Black-box style scenarios: spawn actor, prime with stimuli, call `GetStatus`, assert.

use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::fsm::{FsmEvent, FsmState};
use crate::test::ActorGuard;
use crate::transition_sink::TokioMpscTransitionRecordSink;
use crate::VirtualCarActor;
use ractor::concurrency::Duration;
use ractor::Actor;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Default timeout for [`get_snapshot`] and actor `call` in scenario tests.
const DEFAULT_ACTOR_TIMEOUT: Duration = Duration::from_millis(250);

#[cfg(test)]
mod car_driving_scenarios {
    use super::DEFAULT_ACTOR_TIMEOUT;
    use super::*;
    use crate::DigitalTwinCar;
    use ractor::rpc::CallResult;
    use ractor::{Actor, ActorRef};
    use crate::test::ActorGuard;

    /// Snapshot retrieval with a caller-defined timeout.
    async fn get_snapshot(
        actor: &ActorRef<DigitalTwinCarVocabulary>,
        timeout: Duration,
    ) -> DigitalTwinCar {
        // `call().await` is `Result<CallResult<DigitalTwinCar>, MessagingErr<_>>` (not nested `Result`).
        match actor
            .call(
                |port| DigitalTwinCarVocabulary::GetStatus(port),
                Some(timeout),
            )
            .await
        {
            Ok(CallResult::Success(snapshot)) => snapshot,
            Ok(CallResult::SenderError) => panic!("Actor dropped the reply port without responding."),
            Ok(CallResult::Timeout) => panic!(
                "Scenario Timeout: Actor failed to respond within {:?}.",
                timeout,
            ),
            Err(e) => panic!(
                "Scenario Timeout: Actor failed to respond within {:?}. Error: {}",
                timeout,
                e
            ),
        }
    }

    #[tokio::test]
    async fn scenario_cold_start_get_status_shows_off() {
        let (actor, handle) = Actor::spawn(None, VirtualCarActor::default(), "QUICK".into()).await.unwrap();
        let _guard = ActorGuard {
            addr: actor.clone(),
            handle,
        };

        // Standard check using our global constant
        let car = get_snapshot(&actor, DEFAULT_ACTOR_TIMEOUT).await;
        assert_eq!(car.current_state, FsmState::Off);

        // No 'stop' or 'await' needed here.
        // When '_guard' goes out of scope, the actor is stopped automatically.
    }



    #[tokio::test]
    async fn scenario_power_on_then_drive_rpm_enters_driving() {
        let (actor, handle) = Actor::spawn(None, VirtualCarActor::default(), "WARMUP".into()).await.unwrap();
        let _guard = ActorGuard {
            addr: actor.clone(),
            handle,
        };

        // 1. Priming sequence
        actor.send_message(DigitalTwinCarVocabulary::from(FsmEvent::PowerOn)).unwrap();
        actor.send_message(DigitalTwinCarVocabulary::from(FsmEvent::UpdateRpm(1200))).unwrap();

        // 2. Verification
        let car = get_snapshot(&actor, DEFAULT_ACTOR_TIMEOUT).await;
        assert_eq!(car.current_state, FsmState::Driving);
        car.verify_all_invariants().expect("Safety breach on warmup");
    }

    #[tokio::test]
    async fn scenario_rpm_input_ignored_when_ignition_off() {
        let (actor, handle) = Actor::spawn(None, VirtualCarActor::default(), "INVALID".into()).await.unwrap();
        let _guard = ActorGuard {
            addr: actor.clone(),
            handle,
        };

        // 1. Priming: Send an RPM update while the car is still 'Off'
        actor.send_message(DigitalTwinCarVocabulary::from(FsmEvent::UpdateRpm(3000))).unwrap();

        // 2. Verification: State should remain Off
        let car = get_snapshot(&actor, DEFAULT_ACTOR_TIMEOUT).await;
        assert_eq!(car.current_state, FsmState::Off);
        assert_eq!(car.context.rpm, 3000); // Context might update, but FSM ignores it

        car.verify_all_invariants().expect("Safety breach on invalid input");
    }

    #[tokio::test]
    async fn scenario_redline_rpm_from_driving_enters_warning() {
        let (actor, handle) = Actor::spawn(None, VirtualCarActor::default(), "OVERSPEED".into()).await.unwrap();
        let _guard = ActorGuard {
            addr: actor.clone(),
            handle,
        };

        // 1. Build-up: power on → driving RPM → redline RPM → warning.
        actor.send_message(DigitalTwinCarVocabulary::from(FsmEvent::PowerOn)).unwrap();
        actor
            .send_message(DigitalTwinCarVocabulary::from(FsmEvent::UpdateRpm(2000)))
            .unwrap();
        actor
            .send_message(DigitalTwinCarVocabulary::from(FsmEvent::UpdateRpm(7500)))
            .unwrap();

        // 2. Verification
        let car = get_snapshot(&actor, DEFAULT_ACTOR_TIMEOUT).await;
        assert!(matches!(car.current_state, FsmState::Warning(_)));
    }
}
#[tokio::test]
async fn scenario_get_status_after_power_on_reports_idle() {
    // 1. INITIALIZATION (The Clean Slate)
    let (actor_ref, handle) = Actor::spawn(None, VirtualCarActor::default(), "SCENARIO-TEST-01".into())
        .await
        .expect("Failed to start DigitalTwin Actor");
    let _guard = ActorGuard {
        addr: actor_ref.clone(),
        handle,
    };

    // 2. PRIMING (The Build-up)
    actor_ref
        .send_message(FsmEvent::PowerOn.into())
        .expect("Failed to send PowerOn stimulus");

    // 3. VERIFICATION (The Req -> Reply) — [`DEFAULT_ACTOR_TIMEOUT`] like other scenarios
    let twin_snapshot = actor_ref
        .call(
            |port| DigitalTwinCarVocabulary::GetStatus(port),
            Some(DEFAULT_ACTOR_TIMEOUT),
        )
        .await
        .expect("Failed to enqueue GetStatus")
        .expect("Actor failed to respond or timed out during GetStatus request");

    // 4. ASSERTIONS
    assert_eq!(
        twin_snapshot.current_state,
        FsmState::Idle,
        "Car should be in Idle state after PowerOn"
    );

    twin_snapshot
        .verify_all_invariants()
        .expect("Safety invariant breach detected in test snapshot");
}

#[tokio::test]
async fn scenario_raw_transition_records_are_emitted_in_order() {
    let (tx, mut rx) = mpsc::channel(16);
    let sink = Arc::new(TokioMpscTransitionRecordSink::new(tx));

    let (actor_ref, handle) = Actor::spawn(
        None,
        VirtualCarActor::with_transition_sink(sink),
        "SCENARIO-LOGGING-01".into(),
    )
    .await
    .expect("Failed to start DigitalTwin Actor with sink");
    let _guard = ActorGuard {
        addr: actor_ref.clone(),
        handle,
    };

    actor_ref
        .send_message(FsmEvent::PowerOn.into())
        .expect("Failed to send PowerOn stimulus");
    actor_ref
        .send_message(FsmEvent::UpdateRpm(1500).into())
        .expect("Failed to send UpdateRpm stimulus");

    let first = rx.recv().await.expect("Missing first transition record");
    let second = rx.recv().await.expect("Missing second transition record");

    assert_eq!(first.sequence_no, 1);
    assert_eq!(first.transition.event, FsmEvent::PowerOn);
    assert_eq!(first.transition.old_state, FsmState::Off);
    assert_eq!(first.transition.next_state, FsmState::Idle);
    assert_eq!(first.transition.current_ctx.rpm, 0);

    assert_eq!(second.sequence_no, 2);
    assert_eq!(second.transition.event, FsmEvent::UpdateRpm(1500));
    assert_eq!(second.transition.old_state, FsmState::Idle);
    assert_eq!(second.transition.next_state, FsmState::Driving);
    assert_eq!(second.transition.current_ctx.rpm, 1500);

    let twin_snapshot = actor_ref
        .call(
            |port| DigitalTwinCarVocabulary::GetStatus(port),
            Some(DEFAULT_ACTOR_TIMEOUT),
        )
        .await
        .expect("Failed to enqueue GetStatus")
        .expect("Actor failed to respond or timed out during GetStatus request");

    assert_eq!(
        second.transition.current_ctx, twin_snapshot.context,
        "emitted current_ctx must match persisted actor context after transition"
    );
}
