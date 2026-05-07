//! Shared wire-kind constants for gateway codecs.
//!
//! Keep all `KIND_*` values in one place to avoid collisions
//! as additional actuator command families are introduced.

pub const KIND_CORNER_LIGHT_CMD_ON: u8 = 0x01;
pub const KIND_CORNER_LIGHT_CMD_OFF: u8 = 0x02;
pub const KIND_CORNER_LIGHT_ACK_ON: u8 = 0x81;
pub const KIND_CORNER_LIGHT_ACK_OFF: u8 = 0x82;
pub const KIND_CORNER_LIGHT_NACK_ON: u8 = 0xC1;
pub const KIND_CORNER_LIGHT_NACK_OFF: u8 = 0xC2;
