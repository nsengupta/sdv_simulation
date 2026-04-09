//! Phase 2 generator — the "Virtual ECU".
use anyhow::Result;
use common::{VssSignal, virtual_car::VirtualCar};
use socketcan::{CanSocket, Socket};
use std::{thread, time::Duration};
fn main() -> Result<()> {
    let interface = "vcan0";
    let socket = CanSocket::open(interface)?;
    let mut car = VirtualCar::new();

    println!("🚀 Emulator active on {}. Simulating VSS telemetry...", interface);

    loop {
        // Update physics
        car.update();

        // 1. Send Vehicle.Speed
        let speed_signal = VssSignal::VehicleSpeed(car.speed);
        let speed_frame = speed_signal.to_can_frame()?;
        socket.write_frame(&speed_frame)?;

        // 2. Send Vehicle.Powertrain.CombustionEngine.Speed (RPM)
        let rpm_signal = VssSignal::EngineRpm(car.rpm);
        let rpm_frame = rpm_signal.to_can_frame()?;
        socket.write_frame(&rpm_frame)?;

        // Print local status for debugging
        // print!("\rSending: Speed: {:.2} km/h | RPM: {}    ", car.speed, car.rpm);

        // 10Hz frequency
        thread::sleep(Duration::from_millis(100));
    }
}
