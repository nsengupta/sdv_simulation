use std::time::{SystemTime, UNIX_EPOCH};
use rand::RngExt;
use common::domain_types::{RPM_IDLE, RPM_REDLINE};

pub struct PhysicalCar {
    pub speed: f64,
    pub rpm: u16,
}

impl PhysicalCar {
    pub fn new() -> Self {
        Self { speed: 0.0, rpm: 800 } // Start idling at 800 RPM
    }

    /// The "Physics-Lite" Update
    pub fn update(&mut self) {
        let mut rng = rand::rng();

        // 1. Random Walk for Speed
        // Bias it slightly upward (0.6) so the car eventually moves
        let speed_nudge = rng.random_range(-0.5..0.6);
        self.speed = (self.speed + speed_nudge).clamp(0.0, 160.0);

        // 2. Mock Logic for RPM
        // In a real car, RPM is linked to Speed/Gear.
        // For now, we'll just simulate a slight "throttle hum."
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 1. Determine the Target and the "Intent"
        let target_rpm = if (now / 15) % 2 == 0 {
            6500.0 // Target is well into the Stress Zone
        } else {
            1200.0 // Target is back in the safe zone
        };

        // 2. Proportional Movement Logic
        // Calculate the 'error' (gap)
        let gap = target_rpm - self.rpm as f32;

        // Move 10% of the gap + a small random jitter to simulate engine vibration
        let adjustment = (gap * 0.1) + (rand::random::<f32>() * 10.0 - 5.0);

        // 3. Apply and Clamp
        self.rpm = (self.rpm as f32 + adjustment)
            .clamp(RPM_IDLE as f32, RPM_REDLINE as f32) as u16;

        println!("DEBUG: Time={}s | RPM={} | Target={}", now % 60, self.rpm, target_rpm);

        // println!("DEBUG: Time={}s | Target RPM={} | Self RPM={} | nudge_range (start)={} | nudge_range (end)={} | \
        //nudge ={}",
        //            now % 60, target_rpm, self.rpm, nudge_range_cloned.start, nudge_range_cloned.end, nudge);
    }
}