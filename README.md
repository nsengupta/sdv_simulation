# SDV simulation (draft)

Rust workspace that simulates a small **vehicle data path** inspired by **VSS (Vehicle Signal Specification)** ideas: telemetry is modeled as named signals, encoded on a **SocketCAN** bus, and consumed by a **gateway** that runs a simple **finite-state machine** over asynchronous events.

This is a hands-on learning / demo project, not production software.

## What it does

1. **Emulator (`emulator`)** — Acts like a minimal “virtual ECU”: a toy vehicle model (`VirtualCar`) updates speed and RPM on a timer, encodes them as `VssSignal` values, and **writes standard CAN frames** to a Linux CAN interface (default `vcan0`).
2. **Gateway (`gateway`)** — Opens the same interface, **reads CAN frames** in an ingress task, decodes known frames back into `VssSignal`, and forwards them as **`VehicleEvent::TelemetryUpdate`** on an async channel. A Tokio loop combines **incoming telemetry** with **periodic `TimerTick` events** (heartbeat) and feeds everything into an FSM handler.
3. **Common library (`common`)** — Shared types: VSS-style signals, CAN encode/decode, vehicle events/states, calibration constants, and the virtual car simulation.

Together, the crates demonstrate: **encode → CAN → decode → domain events → stateful logic**, which mirrors patterns used in software-defined vehicle stacks (without tying the repo to a specific OEM stack).

## Crates

| Crate | Role |
|--------|------|
| `common` | `VssSignal` (speed, RPM), `VehicleEvent`, `VehicleState`, `VirtualCar`, CAN framing helpers |
| `emulator` | Sends simulated telemetry frames at ~10 Hz |
| `gateway` | `IngressBus` + Tokio runtime + `VehicleContext` + `decide_next_state` transitions |

## CAN mapping (concrete protocol in code)

Signals use **11-bit standard IDs** and **2-byte big-endian** payloads:

| Signal (concept) | CAN ID | Payload |
|------------------|--------|---------|
| Vehicle speed | `0x101` | `u16`, scaled: km/h × 100 (decode divides by 100) |
| Engine RPM | `0x102` | `u16`, RPM as integer |

Unknown IDs or non-standard frames are ignored by the ingress path (unless and until the decoder 
is extended).

## Gateway behavior (high level)

- **Context** (`VehicleContext`) holds latest speed/RPM, optional RPM stress timer start time, and current `VehicleState`.
- **Events**: telemetry updates, `TimerTick` (from the main loop’s sleep), and `SystemReset` (for future use).
- **Transitions** (`decide_next_state`): e.g. from **Operational** to **Warning** if RPM stays in a “stress” regime long enough (see `STRESS_DURATION_THRESHOLD_SECS` and stress timer logic in `fsm`). **Critical** is mostly a placeholder; recovery paths can be extended.

## Requirements

- **Linux** with SocketCAN (typical for `vcan` or real CAN hardware).
- **Rust** toolchain compatible with the workspace (edition 2024).

## Running (outline)

You need a CAN interface (often **`vcan0`** for development). Creation and bring-up of `vcan` is environment-specific; once `vcan0` exists:

```bash
# Terminal A — producer
cargo run -p emulator

# Terminal B — consumer / gateway
cargo run -p gateway
```

Adjust the interface name in source if you use something other than `vcan0`.

## Dependencies (not exhaustive)

- **`socketcan`** — CAN sockets and frames on Linux  
- **`tokio`** (gateway) — async runtime, channels, timers  
- **`anyhow`** — convenient error handling in binaries  
- **`rand`** (common) — lightweight randomness for the virtual car  

## Future work (ideas)

- Configurable CAN interface via CLI or env  
- `spawn_blocking` (or dedicated thread) for blocking socket reads without stalling the async runtime  
- Richer VSS coverage, diagnostics, or recording  

---

*This README is a **draft**; we will extend it as the codebase grows.*
