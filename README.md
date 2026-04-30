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
# (Milestone-1)
cargo run -p emulator
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
     Running `target/debug/emulator`
🚀 Emulator active on vcan0. Simulating VSS telemetry...
DEBUG: Time=13s | RPM=1366 | Target=6500
DEBUG: Time=13s | RPM=1876 | Target=6500
DEBUG: Time=13s | RPM=2337 | Target=6500
DEBUG: Time=13s | RPM=2753 | Target=6500
DEBUG: Time=13s | RPM=3131 | Target=6500
DEBUG: Time=14s | RPM=3466 | Target=6500
DEBUG: Time=14s | RPM=3772 | Target=6500
DEBUG: Time=14s | RPM=4048 | Target=6500
DEBUG: Time=14s | RPM=4288 | Target=6500
DEBUG: Time=14s | RPM=4508 | Target=6500
DEBUG: Time=14s | RPM=4702 | Target=6500
DEBUG: Time=14s | RPM=4885 | Target=6500
DEBUG: Time=14s | RPM=5051 | Target=6500
DEBUG: Time=14s | RPM=5192 | Target=6500
DEBUG: Time=14s | RPM=5320 | Target=6500
DEBUG: Time=15s | RPM=4906 | Target=1200
```

### Paste Terminal B output here (`bash`)

```bash
# (Milestone-1)
cargo run -p gateway
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/gateway`
[NASHIK-VC-001]: Initializing Digital Twin...
[ACTION]: 📡 Publishing to Cloud: Idle
⚡ Gateway on vcan0 — CAN → VehicleEvent → DigitalTwinCarVocabulary → VirtualCarActor
[NASHIK-VC-001]: Transitioned to Idle
[ACTION]: 📡 Publishing to Cloud: Driving
[NASHIK-VC-001]: Transitioned to Driving
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
...
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
[ACTION]: 🔊 BUZZER ON - High Stress Detected!
[ALERT]: Overspeed detected!
[NASHIK-VC-001]: Transitioned to Warning(Instant { tv_sec: 56863, tv_nsec: 499413717 })
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
...

```

### Milestone output snapshots (chronological descending)

#### `Milestone-1` output snapshot (`bash`)

```bash
# Gateway output
Physical Car name: NASHIK-VC-001, initializing its Digital Twin ...
[ACTION]: 📡 Publishing to Cloud: Idle
[NASHIK-VC-001]: Transitioned to Idle
⚡ Gateway on vcan0 — CAN → VehicleEvent → DigitalTwinCarVocabulary → VirtualCarActor
[ACTION]: 📡 Publishing to Cloud: Driving
[NASHIK-VC-001]: Transitioned to Driving
[NASHIK-VC-001]: received heartbeat TimerTick
[ACTION]: 🔊 BUZZER ON - High Stress Detected!
[ALERT]: Overspeed detected!
[NASHIK-VC-001]: Transitioned to Warning(Instant { tv_sec: 57204, tv_nsec: 621285269 })
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
#..
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
[ACTION]: 🔇 BUZZER OFF - System Normal.
[NASHIK-VC-001]: Transitioned to Driving
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
#..
[ACTION]: 🔊 BUZZER ON - High Stress Detected!
[ALERT]: Overspeed detected!
[NASHIK-VC-001]: Transitioned to Warning(Instant { tv_sec: 57221, tv_nsec: 952978030 })
[NASHIK-VC-001]: received heartbeat TimerTick
[NASHIK-VC-001]: received heartbeat TimerTick
#..

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

## FSM + Lighting Contract (spec for upcoming extension)

This section describes a planned extension: corner-light actuation based on ambient light (`lux`)
while preserving the current top-level FSM (`Off`, `Idle`, `Driving`, `Warning`).

### Goal

Add a **lighting sub-state** in context so that:

- low ambient light requests front corner lights ON,
- the system remains in a **pending** sub-state until an actuator acknowledgment event is received,
- repeated sensor updates do not spam actuator commands.

This keeps the current machine as an **extended FSM** (top-level state + orthogonal context), not a
full hierarchical state machine yet.

### Scope and Non-Goals

- Scope: sensor-driven corner-light control and acknowledgment-driven completion.
- Non-goal: replacing the existing primary FSM state model.
- Non-goal: introducing a full multi-region/hierarchical statechart runtime.

### Proposed Context Extension

- `lighting_state: LightingState`
- `ambient_lux: u16` (or equivalent normalized representation)

`LightingState`:

- `Off`
- `OnRequested`
- `On`
- `OffRequested`

### Proposed Event Vocabulary

- `UpdateAmbientLux(u16)` — ambient sensor update from ingress path.
- `CornerLightsOnConfirmed` — actuator/body-controller ACK for ON.
- `CornerLightsOffConfirmed` — actuator/body-controller ACK for OFF.
- `CornerLightsActuationFailed` (optional) — negative ACK or timeout/error path.

### Proposed Domain Actions

- `RequestCornerLightsOn`
- `RequestCornerLightsOff`
- `LogLightingInfo(String)` (optional)
- `LogLightingFault(String)` (optional, for failure/timeout path)

### Threshold Contract (hysteresis)

Use separate thresholds:

- `LUX_ON_THRESHOLD`
- `LUX_OFF_THRESHOLD` where `LUX_OFF_THRESHOLD > LUX_ON_THRESHOLD`

Reason: avoid rapid ON/OFF toggling near one boundary.

### Transition Contract (lighting sub-state)

Given `lighting_state` and incoming event:

1. `Off` + `UpdateAmbientLux(lux <= LUX_ON_THRESHOLD)`  
   -> `OnRequested` + emit `RequestCornerLightsOn`
2. `OnRequested` + `CornerLightsOnConfirmed`  
   -> `On`
3. `On` + `UpdateAmbientLux(lux >= LUX_OFF_THRESHOLD)`  
   -> `OffRequested` + emit `RequestCornerLightsOff`
4. `OffRequested` + `CornerLightsOffConfirmed`  
   -> `Off`
5. `OnRequested` + repeated low-lux updates  
   -> stay `OnRequested` (no duplicate ON command)
6. `OffRequested` + repeated high-lux updates  
   -> stay `OffRequested` (no duplicate OFF command)
7. Optional robustness: pending + failure/timeout  
   -> retry policy or safe fallback + `LogLightingFault(...)`

### Main FSM Interaction Policy

Lighting remains orthogonal to primary drive state:

- primary FSM (`Off`, `Idle`, `Driving`, `Warning`) continues to be the authoritative operational state;
- lighting logic runs in context as a secondary concern;
- when primary state is `Off`, effective lighting should be forced/kept `Off` (or ON requests blocked).

### Behavioral Guarantees (contract-level invariants)

- ON request emits only from `LightingState::Off`.
- OFF request emits only from `LightingState::On`.
- Pending states resolve only via ACK/failure/timeout events.
- Duplicate sensor updates do not cause duplicate actuator requests.
- Existing warning/buzzer logic remains independent from lighting actuation.

### Architecture Mapping (where this belongs)

- Signal encode/decode: `common::signals` (`VssSignal`)
- Ingress mapping to actor vocabulary: `gateway/src/main.rs`
- FSM vocabulary/context/actions: `common::fsm::machineries`
- Transition and output rules: `common::fsm::engine`
- Step boundary for context mutation + domain actions: `common::fsm::step`
- Side-effect execution and ACK ingestion path: `common::virtual_car_actor`

### Limitations (current and expected)

- Actuator ACK channel is modeled, not a production-grade body-controller integration.
- Timing/timeout policy is deliberately simple for simulation clarity.
- No formal concurrent-region statechart runtime yet; orthogonal behavior is represented through context.
- Determinism depends on explicit event ordering and `now` handling at the step boundary.

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
