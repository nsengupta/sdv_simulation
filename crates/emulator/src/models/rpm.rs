use rand::RngExt;

use super::config::RpmModelConfig;

#[derive(Debug, Clone)]
pub struct RpmModel {
    cfg: RpmModelConfig,
}

impl RpmModel {
    pub fn new(cfg: RpmModelConfig) -> Self {
        Self { cfg }
    }

    pub fn next_rpm(&self, current_rpm: u16, epoch_secs: u64) -> u16 {
        let target_rpm = self.target_rpm_for_epoch(epoch_secs);

        let gap = target_rpm - current_rpm as f32;
        let mut rng = rand::rng();
        let jitter =
            rng.random_range(-self.cfg.jitter_amplitude..self.cfg.jitter_amplitude);
        let adjustment = (gap * self.cfg.proportional_gain) + jitter;

        (current_rpm as f32 + adjustment).clamp(
            self.cfg.idle_rpm as f32,
            self.cfg.redline_rpm as f32,
        ) as u16
    }

    pub fn target_rpm_for_epoch(&self, epoch_secs: u64) -> f32 {
        if (epoch_secs / self.cfg.target_flip_period_secs) % 2 == 0 {
            self.cfg.high_target_rpm
        } else {
            self.cfg.low_target_rpm
        }
    }
}
