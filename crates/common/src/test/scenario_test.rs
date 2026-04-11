//! Black-box style scenarios: spawn actor, prime with stimuli, call `GetStatus`, assert.

use crate::digital_twin::DigitalTwinCarVocabulary;
use crate::fsm::{FsmEvent, FsmState};
use crate::VirtualCarActor;
use ractor::concurrency::Duration;
use ractor::Actor;

#[tokio::test]
async fn test_black_box_status_retrieval() {
    // 1. INITIALIZATION (The Clean Slate)
    // We spawn a fresh actor for this specific scenario.
    let (actor_ref, handle) = Actor::spawn(None, VirtualCarActor, "SCENARIO-TEST-01".into())
        .await
        .expect("Failed to start DigitalTwin Actor");

    // 2. PRIMING (The Build-up)
    // Send a standard stimulus. In a full scenario, you might have several of these.
    actor_ref
        .send_message(FsmEvent::PowerOn.into())
        .expect("Failed to send PowerOn stimulus");

    // 3. VERIFICATION (The Req -> Reply)
    // We use `call` to request the status.
    // This will wait until the Actor has processed `PowerOn` and then `GetStatus`.
    let timeout = Duration::from_millis(200);

    let twin_snapshot = actor_ref
        .call(
            |port| DigitalTwinCarVocabulary::GetStatus(port),
            Some(timeout),
        )
        .await
        .expect("Failed to enqueue GetStatus")
        .expect("Actor failed to respond or timed out during GetStatus request");

    // 4. ASSERTIONS
    // Check that the internal state changed as a result of our priming.
    assert_eq!(
        twin_snapshot.current_state,
        FsmState::Idle,
        "Car should be in Idle state after PowerOn"
    );

    // Verify safety invariants on the snapshot
    twin_snapshot
        .verify_all_invariants()
        .expect("Safety invariant breach detected in test snapshot");

    // 5. TEARDOWN
    // Gracefully stop the actor to ensure a clean slate for the next test.
    actor_ref.stop(None);
    handle.await.unwrap();
}
