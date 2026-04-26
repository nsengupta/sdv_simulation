//! Actor-oriented contract tests (mailbox -> step -> persistence/emit sequencing).

use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::fsm::{FsmEvent, FsmState};
use crate::test::ActorGuard;
use crate::transition_sink::TokioMpscTransitionRecordSink;
use crate::VirtualCarActor;
use ractor::concurrency::Duration;
use ractor::Actor;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Timeout for actor call in contract tests.
const DEFAULT_ACTOR_TIMEOUT: Duration = Duration::from_millis(250);

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
