//! Phase 2 generator — the "Virtual ECU".

pub mod car_physics;
pub mod models;

use anyhow::Result;
use car_physics::PhysicalCar;
use common::VssSignal;
use socketcan::{CanSocket, Socket};
use std::{thread, time::Duration};

fn main() -> Result<()> {
    let interface = "vcan0";
    let socket = CanSocket::open(interface)?;
    let mut car = PhysicalCar::new();

    println!("🚀 Emulator active on {interface}. Simulating VSS telemetry...");

    loop {
        car.update();

        let speed_signal = VssSignal::VehicleSpeed(car.speed_kph());
        socket.write_frame(&speed_signal.to_can_frame()?)?;

        let rpm_signal = VssSignal::EngineRpm(car.rpm());
        socket.write_frame(&rpm_signal.to_can_frame()?)?;

        let ambient_lux_signal = VssSignal::AmbientLux(car.ambient_lux());
        socket.write_frame(&ambient_lux_signal.to_can_frame()?)?;

        thread::sleep(Duration::from_millis(100));
    }
}
