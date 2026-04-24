# SDV simulation (draft)

Rust workspace that simulates a small **vehicle data path** inspired by **VSS (Vehicle Signal Specification)** ideas: telemetry is modeled as named signals, encoded on a **SocketCAN** bus, and consumed by a **gateway** that runs a simple **finite-state machine** over asynchronous events.

This is a hands-on learning / demo project, not production software.

## Requirements

- **Linux** with SocketCAN (typical for `vcan` or real CAN hardware).
- **Rust** toolchain compatible with the workspace (edition 2024).

## How To Run (Linux quick start)

`vcan0` is the default and currently preferred interface because both binaries are wired to it in code.

Run these setup commands first (requires `sudo`):

```bash
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0
```

Then start the apps in two terminals (no `sudo` needed):

```bash
# Terminal A — producer
cargo run -p emulator
```

```bash
# Terminal B — consumer / gateway
cargo run -p gateway
```

When done, stop both with `Ctrl+C`, then tear down `vcan0` (requires `sudo`):

```bash
sudo ip link del vcan0
```

If you use a different interface name, update the hardcoded interface strings in `crates/emulator/src/main.rs` and `crates/gateway/src/main.rs`.

## What You Should See (outputs)

- **Terminal A (`emulator`)** should print a startup line and then continue running while publishing speed/RPM CAN frames.
- **Terminal B (`gateway`)** should print startup output, state/action logs, and heartbeat receipt logs from the virtual car actor while consuming CAN frames from `vcan0`.
- Both processes are long-running by design; stop with `Ctrl+C` when done.

### Paste Terminal A output here (`bash`)

```bash
# Example: paste `cargo run -p emulator` runtime output here.
# (Milestone-1)
```

### Paste Terminal B output here (`bash`)

```bash
# Example: paste `cargo run -p gateway` runtime output here.
# Include actor heartbeat, transitions, and warning/recovery lines.
# (Milestone-1)
```

### Milestone output snapshots (chronological descending)

#### `Milestone-1` output snapshot (`bash`)

```bash
# Paste a representative combined snapshot for Milestone-1 here.
# Keep this section stable after the Milestone-1 check-in.
```

## Current Architecture (milestones)

- `Milestone-1` (latest): FSM spec + step contract split, warning recovery behavior, and raw transition sink abstraction.

## Architecture And Design

### Gateway behavior (high level)

- **Context** (`VehicleContext`) holds latest RPM/speed and health flags used by FSM guards.
- **Events**: `PowerOn`, `PowerOff`, `UpdateRpm`, `UpdateSpeed`, and periodic `TimerTick`.
- **FSM spec**: `transition(...)` + `output(...)` in `common::fsm::engine` are the canonical transition/action rules.
- **Execution wrapper**: `step(...)` in `common::fsm::step` derives context from event payload, calls `transition/output`, and returns `StepResult`.
- **Time handling**: `transition(...)` takes `now` explicitly (no hidden clock calls), which keeps time-based behavior deterministic in tests.
- **Warning recovery**: `Warning(began_at)` is recovered on `TimerTick` only when cooldown elapsed and RPM is at/below recovery threshold; recovers to `Driving` or `Idle` based on speed.
- **Transition sink**: actor can emit raw transition records through `TransitionRecordSink` (best-effort, warn-and-continue on sink full/closed).

### What it does

1. **Emulator (`emulator`)** — Acts like a minimal “virtual ECU”: a toy vehicle model (`VirtualCar`) updates speed and RPM on a timer, encodes them as `VssSignal` values, and **writes standard CAN frames** to a Linux CAN interface (default `vcan0`).
2. **Gateway (`gateway`)** — Opens the same interface, **reads CAN frames**, decodes known frames into `VssSignal`, maps them into FSM events, and sends them to the actor. A Tokio loop sends periodic `TimerTick` heartbeat events.
3. **Common library (`common`)** — Shared types and behavior: VSS-style signals, CAN encode/decode, FSM (`transition/output`), step contract (`step` + `StepResult`), and the virtual car actor with optional transition sink.

Together, the crates demonstrate: **encode → CAN → decode → domain events → stateful logic**, which mirrors patterns used in software-defined vehicle stacks (without tying the repo to a specific OEM stack).

### Crates

| Crate | Role |
|--------|------|
| `common` | `VssSignal`, FSM spec (`transition/output`), step contract, actor runtime, transition sink abstraction |
| `emulator` | Sends simulated telemetry frames at ~10 Hz |
| `gateway` | CAN ingress + event mapping + Tokio tick loop feeding the actor |

### CAN mapping (concrete protocol in code)

Signals use **11-bit standard IDs** and **2-byte big-endian** payloads:

| Signal (concept) | CAN ID | Payload |
|------------------|--------|---------|
| Vehicle speed | `0x101` | `u16`, scaled: km/h × 100 (decode divides by 100) |
| Engine RPM | `0x102` | `u16`, RPM as integer |

Unknown IDs or non-standard frames are ignored by the ingress path (unless and until the decoder
is extended).

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
