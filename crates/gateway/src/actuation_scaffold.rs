//! Actuation simulation on **`vcan0`**: controller → `ActuationCommand` channel (brain has no CAN
//! knowledge) → plant writes **command** then **ACK/NACK** CAN frames; gateway read loop decodes
//! responses into [`PhysicalCarVocabulary`] like telemetry.
//!
//! **Random “sit tight” (no ACK):** set env `CORNER_LIGHT_PLANT_DROP_RESPONSE_PROB` to a value in
//! `0.0..=1.0` (e.g. `0.15`). After the usual post-command delay the plant may emit **no** ACK
//! frame; the twin still advances on `TimerTick` and recovers via ACK-wait timeout (`[ALERT @ …]`
//! from the controller runtime).
//!
//! **`Send` and RNG:** `tokio::spawn` requires this async block to be **`Send`**. `rand::thread_rng()`
//! returns a **`ThreadRng` that is not `Send`**, so keeping `let mut rng = thread_rng()` alive
//! across any `.await` makes the future non-`Send` and the crate will not compile. **`rand::random::<f64>()`**
//! takes one sample inside a synchronous stretch (after `sleep`, before the next `recv`); the
//! RNG is used only for that call, so no non-`Send` handle crosses an await point.
//!
//! TODO(actuation-child-actor): replace Tokio tasks with supervised child actors; keep the same
//! [`ActuationCommand`] contract on the channel side.

use std::time::Duration;

use common::ActuationCommand;
use socketcan::{CanSocket, Socket};
use tokio::sync::mpsc;

use crate::corner_light_actuation_can::{
    actuation_command_wire_meta, encode_ack_frame, encode_command_frame, encode_nack_frame,
};

const DEFAULT_CHANNEL_CAPACITY: usize = 64;
/// Default ACK probability when the plant chooses to send a response frame.
/// Used by [`should_send_ack_response`].
pub const DEFAULT_ACK_NACK_RESPONSE_PROB: f64 = 0.7;

/// After the post-command delay: **`true`** → send ACK; **`false`** → sit tight (random drop).
/// Uses `rand::random` (not a stored `ThreadRng`) so the plant `async` block stays `Send`; see module docs.
fn should_ack_or_not(dont_respond_probability: f64) -> bool {
    dont_respond_probability <= 0.0 || rand::random::<f64>() >= dont_respond_probability
}

/// For a response frame decision: **`true`** means ACK, **`false`** means NACK.
fn should_send_ack_response(ack_nack_response_probability: f64) -> bool {
    let p = ack_nack_response_probability.clamp(0.0, 1.0);
    rand::random::<f64>() < p
}

pub fn actuator_command_channel() -> (
    mpsc::Sender<ActuationCommand>,
    mpsc::Receiver<ActuationCommand>,
) {
    mpsc::channel(DEFAULT_CHANNEL_CAPACITY)
}

/// Simulated body ECU: consumes commands from the channel, writes **command** then response CAN
/// frames (`ACK` or `NACK`) onto `can_interface` (same bus as telemetry).
///
/// `dont_respond_probability`: per command, after `ack_delay`, probability **not** to send the
/// response frame.
/// `ack_nack_response_probability`: when plant responds, probability that the response is ACK
/// (`0.0` = always NACK, `1.0` = always ACK).
pub fn spawn_corner_light_can_plant(
    mut cmd_rx: mpsc::Receiver<ActuationCommand>,
    can_interface: String,
    ack_delay: Duration,
    dont_respond_probability: f64,
    ack_nack_response_probability: f64,
) {
    tokio::spawn(async move {
        let socket = match CanSocket::open(&can_interface) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[corner-light-actuator]: cannot open CAN {can_interface} for actuation TX: {e}"
                );
                return;
            }
        };
        while let Some(cmd) = cmd_rx.recv().await {
            let cmd_frame = match encode_command_frame(&cmd) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[corner-light-actuator]: encode command CAN frame failed: {e:?}");
                    continue;
                }
            };
            if let Err(e) = socket.write_frame(&cmd_frame) {
                eprintln!("[corner-light-actuator]: command write_frame failed: {e:?}");
                continue;
            }
            tokio::time::sleep(ack_delay).await;
            if !should_ack_or_not(dont_respond_probability) {
                let (session, seq) = actuation_command_wire_meta(&cmd);
                eprintln!(
                    "[corner-light-actuator]: sit tight — no ACK after delay (random drop; wire session={session} seq={seq}); \
                     gateway will not ingress corner-light ACK; twin recovers on TimerTick if pending times out"
                );
                continue;
            }
            let send_ack = should_send_ack_response(ack_nack_response_probability);
            let (session, seq) = actuation_command_wire_meta(&cmd);
            eprintln!(
                "[corner-light-actuator]: responding with {} (wire session={session} seq={seq})",
                if send_ack { "ACK" } else { "NACK" }
            );
            let response_frame = match if send_ack {
                encode_ack_frame(&cmd)
            } else {
                encode_nack_frame(&cmd)
            } {
                Ok(f) => f,
                Err(e) => {
                    eprintln!(
                        "[corner-light-actuator]: encode {} CAN frame failed: {e:?}",
                        if send_ack { "ACK" } else { "NACK" }
                    );
                    continue;
                }
            };
            if let Err(e) = socket.write_frame(&response_frame) {
                eprintln!(
                    "[corner-light-actuator]: {} write_frame failed: {e:?}",
                    if send_ack { "ACK" } else { "NACK" }
                );
            }
        }
    });
}
