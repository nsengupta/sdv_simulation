use common::{ActuationCommand, PhysicalCarVocabulary};

use crate::devices::corner_lights::codec::{
    payload_to_physical, CornerLightActuationPayload, KIND_ACK_OFF, KIND_ACK_ON, KIND_CMD_OFF,
    KIND_CMD_ON, KIND_NACK_OFF, KIND_NACK_ON,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingCornerLightCommand {
    session: u16,
    sequence: u32,
    on_command: bool,
}

#[derive(Debug, Clone)]
pub enum CornerLightPolicyDecision {
    Accept {
        physical: PhysicalCarVocabulary,
        session: u16,
        sequence: u32,
    },
    Ignore(&'static str),
}

#[derive(Debug, Default)]
pub struct CornerLightPolicy {
    pending: Option<PendingCornerLightCommand>,
}

impl CornerLightPolicy {
    pub fn on_command_sent(&mut self, cmd: &ActuationCommand) {
        let pending = match cmd {
            ActuationCommand::SwitchCornerLightsOn { correlation_id } => PendingCornerLightCommand {
                session: correlation_id.session_id as u16,
                sequence: correlation_id.sequence_no as u32,
                on_command: true,
            },
            ActuationCommand::SwitchCornerLightsOff { correlation_id } => PendingCornerLightCommand {
                session: correlation_id.session_id as u16,
                sequence: correlation_id.sequence_no as u32,
                on_command: false,
            },
        };
        self.pending = Some(pending);
    }

    pub fn on_response(&mut self, payload: CornerLightActuationPayload) -> CornerLightPolicyDecision {
        let response_type = match payload.kind {
            KIND_ACK_ON => Some((true, true)),
            KIND_ACK_OFF => Some((false, true)),
            KIND_NACK_ON => Some((true, false)),
            KIND_NACK_OFF => Some((false, false)),
            KIND_CMD_ON | KIND_CMD_OFF => return CornerLightPolicyDecision::Ignore("command-frame"),
            _ => return CornerLightPolicyDecision::Ignore("unknown-kind"),
        };

        let Some((response_on_command, is_ack)) = response_type else {
            return CornerLightPolicyDecision::Ignore("unknown-kind");
        };
        let Some(pending) = self.pending else {
            return CornerLightPolicyDecision::Ignore("no-pending-command");
        };
        if pending.session != payload.session_id || pending.sequence != payload.sequence_no {
            return CornerLightPolicyDecision::Ignore("correlation-mismatch");
        }
        if pending.on_command != response_on_command {
            return CornerLightPolicyDecision::Ignore("direction-mismatch");
        }

        let Some(physical) = payload_to_physical(payload) else {
            return CornerLightPolicyDecision::Ignore("non-ingress-kind");
        };
        if is_ack != matches!(physical, PhysicalCarVocabulary::CornerLightsCommandConfirmed { .. }) {
            return CornerLightPolicyDecision::Ignore("ack-kind-mapping-mismatch");
        }

        self.pending = None;
        CornerLightPolicyDecision::Accept {
            physical,
            session: payload.session_id,
            sequence: payload.sequence_no,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::CorrelationId;

    fn corr(session: u64, sequence: u64) -> CorrelationId {
        CorrelationId {
            source_id: "test".to_string(),
            session_id: session,
            sequence_no: sequence,
        }
    }

    #[test]
    fn accepts_matching_ack_for_pending_command() {
        let mut policy = CornerLightPolicy::default();
        policy.on_command_sent(&ActuationCommand::SwitchCornerLightsOn {
            correlation_id: corr(0x1234, 0xabcdef01),
        });
        let decision = policy.on_response(CornerLightActuationPayload {
            kind: KIND_ACK_ON,
            session_id: 0x1234,
            sequence_no: 0xabcdef01,
        });
        assert!(matches!(
            decision,
            CornerLightPolicyDecision::Accept {
                physical: PhysicalCarVocabulary::CornerLightsCommandConfirmed { on_command: true },
                session: 0x1234,
                sequence: 0xabcdef01
            }
        ));
    }

    #[test]
    fn rejects_correlation_mismatch() {
        let mut policy = CornerLightPolicy::default();
        policy.on_command_sent(&ActuationCommand::SwitchCornerLightsOn {
            correlation_id: corr(0x1234, 0xabcdef01),
        });
        let decision = policy.on_response(CornerLightActuationPayload {
            kind: KIND_ACK_ON,
            session_id: 0x9999,
            sequence_no: 0xabcdef01,
        });
        assert!(matches!(
            decision,
            CornerLightPolicyDecision::Ignore("correlation-mismatch")
        ));
    }

    #[test]
    fn ignores_duplicate_after_acceptance() {
        let mut policy = CornerLightPolicy::default();
        policy.on_command_sent(&ActuationCommand::SwitchCornerLightsOn {
            correlation_id: corr(0x1234, 0xabcdef01),
        });
        let _ = policy.on_response(CornerLightActuationPayload {
            kind: KIND_ACK_ON,
            session_id: 0x1234,
            sequence_no: 0xabcdef01,
        });
        let duplicate = policy.on_response(CornerLightActuationPayload {
            kind: KIND_ACK_ON,
            session_id: 0x1234,
            sequence_no: 0xabcdef01,
        });
        assert!(matches!(
            duplicate,
            CornerLightPolicyDecision::Ignore("no-pending-command")
        ));
    }
}
