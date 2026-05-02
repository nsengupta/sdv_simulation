//! Shared actuation command/feedback contracts.
//!
//! These types are intentionally runtime-agnostic so the same message model can
//! be used with:
//! - in-process gateway worker threads/tasks,
//! - child actors,
//! - remote transports (CAN/Zenoh/uProtocol) later.

/// Correlation identity for command <-> feedback flows.
///
/// Use scoped identity (`source_id`, `session_id`, `sequence_no`) instead of a
/// single global counter to avoid uniqueness issues across restarts/processes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorrelationId {
    pub source_id: String,
    pub session_id: u64,
    pub sequence_no: u64,
}

/// Outbound actuation intent emitted by controller runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuationCommand {
    SwitchCornerLightsOn { correlation_id: CorrelationId },
    SwitchCornerLightsOff { correlation_id: CorrelationId },
}

/// Inbound actuation feedback consumed by controller runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuationFeedback {
    CornerLightsOnConfirmed { correlation_id: CorrelationId },
    CornerLightsOffConfirmed { correlation_id: CorrelationId },
    CornerLightsActuationFailed {
        correlation_id: CorrelationId,
        reason: String,
    },
}
