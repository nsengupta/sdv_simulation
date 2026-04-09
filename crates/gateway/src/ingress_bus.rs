// crates/gateway/src/ingress_bus.rs
use common::{VehicleEvent, VssSignal};
use socketcan::{CanSocket, Socket};
use tokio::sync::mpsc;

pub struct IngressBus {
    socket: CanSocket,
    tx: mpsc::Sender<VehicleEvent>,
}

impl IngressBus {
    pub fn new(interface: &str, tx: mpsc::Sender<VehicleEvent>) -> anyhow::Result<Self> {
        let socket = CanSocket::open(interface)?;
        Ok(Self { socket, tx })
    }

    pub async fn start(self) -> anyhow::Result<()> {
        loop {
            // socketcan read is blocking; acceptable for this demo on the multithread runtime.
            let frame = self.socket.read_frame()?;

            if let Some(signal) = VssSignal::from_can_frame(&frame) {
                let event = VehicleEvent::TelemetryUpdate(signal);
                self.tx.send(event).await?;
            }
        }
    }
}