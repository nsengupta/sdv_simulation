use std::time::{SystemTime, UNIX_EPOCH};
use common::domain_types::RPM_IDLE;

use crate::models::{
    AmbientRoadLightModel, PhysicalWorldModelConfig, RpmModel, SpeedModel,
};

pub struct PhysicalCar {
    speed_kph: f64,
    rpm: u16,
    ambient_lux: u16,
    speed_model: SpeedModel,
    rpm_model: RpmModel,
    ambient_road_light_model: AmbientRoadLightModel,
}

impl PhysicalCar {
    pub fn new() -> Self {
        Self::new_with_config(PhysicalWorldModelConfig::daytime_tunnel_profile())
    }

    pub fn new_with_config(cfg: PhysicalWorldModelConfig) -> Self {
        let speed_model = SpeedModel::new(cfg.speed);
        let rpm_model = RpmModel::new(cfg.rpm);
        let ambient_road_light_model = AmbientRoadLightModel::new(cfg.ambient_road_light);

        Self {
            speed_kph: 0.0,
            rpm: RPM_IDLE,
            ambient_lux: 850,
            speed_model,
            rpm_model,
            ambient_road_light_model,
        }
    }

    pub fn speed_kph(&self) -> f64 {
        self.speed_kph
    }

    pub fn rpm(&self) -> u16 {
        self.rpm
    }

    pub fn ambient_lux(&self) -> u16 {
        self.ambient_lux
    }

    /// Physics-Lite orchestrator.
    ///
    /// Collects fresh physical-world values from dedicated models without
    /// embedding signal-generation formulas directly in this function.
    pub fn update(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.speed_kph = self.speed_model.next_speed_kph(self.speed_kph);
        self.rpm = self.rpm_model.next_rpm(self.rpm, now);
        self.ambient_lux = self.ambient_road_light_model.next_ambient_lux(now);

        let target_rpm = self.rpm_model.target_rpm_for_epoch(now);
        println!(
            "DEBUG: Time={}s | SpeedKph={:.2} | RPM={} (Target={}) | AmbientLux={}",
            now % 60,
            self.speed_kph,
            self.rpm,
            target_rpm,
            self.ambient_lux
        );
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicalCar;
    use common::domain_types::{RPM_IDLE, RPM_REDLINE_THRESHOLD};

    #[test]
    fn smoke_new_car_starts_at_idle_and_standstill() {
        let car = PhysicalCar::new();
        assert_eq!(car.speed_kph(), 0.0);
        assert_eq!(car.rpm(), RPM_IDLE);
        assert!((0..=1200).contains(&car.ambient_lux()));
    }

    #[test]
    fn smoke_update_keeps_values_within_expected_bounds() {
        let mut car = PhysicalCar::new();
        for _ in 0..32 {
            car.update();
            assert!((0.0..=160.0).contains(&car.speed_kph()));
            assert!((RPM_IDLE..=RPM_REDLINE_THRESHOLD).contains(&car.rpm()));
            assert!((0..=1200).contains(&car.ambient_lux()));
        }
    }
}