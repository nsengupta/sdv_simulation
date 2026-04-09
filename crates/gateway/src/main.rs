// crates/gateway/src/main.rs

use anyhow::Result;
use common::VehicleEvent;
use std::time::Duration;
use tokio::sync::mpsc;

mod fsm;
mod ingress_bus;

use crate::fsm::{handle_vehicle_event, VehicleContext};
use crate::ingress_bus::IngressBus;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configuration
    let interface = "vcan0";
    let (tx, mut rx) = mpsc::channel(100); // Buffer for 100 events

    // 2. Initialize the Ingress Bus (The Sensory System)
    let bus = IngressBus::new(interface, tx)?;

    // Spawn the bus listener into its own task
    tokio::spawn(async move {
        if let Err(e) = bus.start().await {
            eprintln!("🛑 Ingress Bus Critical Error: {}", e);
        }
    });

    // 3. Initialize the FSM Context (The Private Memory)
    let mut context = VehicleContext::new();

    println!("⚡ Gateway Logic Engine Active. Monitoring VSS Stream...");

    // 4. The Processing Loop (The "Heartbeat")
    // We use a select! or a timeout to handle periodic FSM checks (TimerTicks)
    loop {
        tokio::select! {
            // Handle incoming telemetry from the bus
            Some(event) = rx.recv() => {
                handle_vehicle_event(&mut context, event);
            }

            // Periodic internal heartbeat (e.g., every 100ms)
            // This ensures the FSM checks temporal conditions (the 5s stress rule)
            // even if no new CAN frames are arriving.
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                handle_vehicle_event(&mut context, VehicleEvent::TimerTick);
            }
        }

        // Optional: High-level Dashboard Output for the developer
        // print!("\r[Current State: {:?}] Speed: {:.2} | RPM: {}    ",
        //        context.current_state, context.speed, context.rpm);
    }
}
