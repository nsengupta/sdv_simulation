use rand::RngExt;

use super::config::SpeedModelConfig;

#[derive(Debug, Clone)]
pub struct SpeedModel {
    cfg: SpeedModelConfig,
}

impl SpeedModel {
    pub fn new(cfg: SpeedModelConfig) -> Self {
        Self { cfg }
    }

    pub fn next_speed_kph(&self, current_speed_kph: f64) -> f64 {
        let mut rng = rand::rng();
        let speed_nudge = rng.random_range(self.cfg.random_nudge_min..self.cfg.random_nudge_max);
        (current_speed_kph + speed_nudge).clamp(self.cfg.min_kph, self.cfg.max_kph)
    }
}
