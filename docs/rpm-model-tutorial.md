# RPM Model Tutorial (Intuition First)

This note explains the RPM update logic used in `emulator`.
It is written as a quick tutorial you can revisit later.

## 1) What the model is trying to do

Each simulation tick, we update RPM using:

`next = current + proportional_push + small_noise`

Where:

- `proportional_push` moves RPM toward a target value (high or low phase)
- `small_noise` (jitter) adds realism so the signal is not perfectly smooth

Code form:

`r_{t+1} = r_t + k * (T_t - r_t) + jitter`

- `r_t`: current RPM
- `T_t`: target RPM for this phase
- `k`: proportional gain (`0 < k < 1`)

---

## 2) What "error" means

Error is just "distance from target":

`e_t = T_t - r_t`

- if `e_t > 0`: RPM is below target, update pushes upward
- if `e_t < 0`: RPM is above target, update pulls downward

So "error" here means control gap, not a software failure.

---

## 3) Why RPM moves smoothly (not instantly)

Ignoring jitter and clamp for a moment:

`r_{t+1} = r_t + k * (T - r_t)`

This becomes:

`r_{t+1} = (1-k) * r_t + k * T`

So each tick keeps part of old value and mixes in part of target.

Equivalent error form:

`e_{t+1} = (1-k) * e_t`

Meaning: every tick, error shrinks by factor `(1-k)`.

For `k = 0.1`, error retains `90%` each tick.

---

## 4) Closed-form (one phase, fixed target)

After `n` ticks with same target `T`:

`r_n = T + (r_0 - T) * (1-k)^n`

This is the key equation for quick reasoning:

- `(1-k)^n` is the remaining fraction of initial gap
- bigger `k` means faster convergence
- bigger `n` (longer phase) means closer to target

---

## 5) Why 15-second flipping matters

In current config, target flips every `target_flip_period_secs` (currently 15s):

- high target block
- low target block
- high target block
- ...

So the system converges **partially** within each block, then target changes.
If block duration is short relative to convergence speed, RPM may not fully settle.

Practical implication:

- high block must be long/strong enough to cross stress thresholds
- low block must be long/strong enough to recover below thresholds

---

## 6) Will GREEN/RED crossing always happen?

Not automatically. Crossing is guaranteed only if config supports it.

Depends on:

- target levels (`high_target_rpm`, `low_target_rpm`)
- gain (`proportional_gain`)
- phase duration (`target_flip_period_secs`)
- jitter amplitude
- clamps (`idle..redline`)

If high target is too low or high phase too short, RED may never be crossed.

---

## 7) Role of jitter in physical realism

Jitter represents small real-world fluctuations:

- tiny throttle corrections
- varying mechanical/electrical loads
- combustion and drivetrain micro-variations
- sensor noise

It should be small. The trend should come from target tracking, not from jitter.

---

## 8) How to make threshold behavior deterministic in tests

For deterministic contract tests:

1. set `jitter_amplitude = 0.0`
2. choose fixed `k`, targets, and phase duration
3. simulate exact number of ticks
4. assert crossings/non-crossings against GREEN/RED/recovery thresholds

This turns "likely behavior" into "provable behavior".

---

## 9) Quick tuning checklist

- Want faster rise/fall? increase `k` (carefully).
- Want longer time near high/low? increase phase duration.
- Want guaranteed RED crossing? ensure high target and dwell time allow it.
- Want smoother trace? reduce jitter.

---

## 10) One-line mental model

The RPM model is a first-order controller:
"move a fraction of the gap toward the current phase target each tick, plus small noise."

---

## 11) Math symbols used in threshold formulas

When we compute "minimum ticks needed to cross a threshold", two symbols appear often.

### Ceiling: `ceil(...)` or `⌈...⌉`

`⌈x⌉` means: smallest integer greater than or equal to `x`.

Examples:

- `⌈23.1⌉ = 24`
- `⌈24.0⌉ = 24`

Why needed here:

- simulation ticks are whole numbers
- if math says `23.1` ticks, you need `24` actual ticks

### Natural log: `ln(...)`

`ln` is logarithm base `e`. In this context it is used to solve equations where the unknown is in the exponent.

Typical form from the RPM model:

`(1-k)^n <= ratio`

To solve for `n`, apply log on both sides:

`log(a^n) = n * log(a)`

So:

`n >= ln(ratio) / ln(1-k)`

Any log base would work (base-10, base-2, etc.), as long as used consistently on numerator and denominator. `ln` is just the standard choice in control/math notes.

### Worked example (current simulation-style values)

Goal: minimum ticks needed to reach GREEN threshold.

Given:

- `k = 0.1`
- `T = 6500` (high target)
- `H = 6000` (GREEN threshold)
- `r0 = 800` (start RPM)

From one-phase formula:

`r_n = T + (r0 - T) * (1-k)^n`

Require crossing:

`r_n >= H`

Equivalent inequality:

`(1-k)^n <= (T-H)/(T-r0)`

Substitute:

`0.9^n <= (6500-6000)/(6500-800) = 500/5700 = 0.087719...`

Take natural logs:

`n >= ln(0.087719...) / ln(0.9)`

Numerically:

- `ln(0.087719...) ~= -2.4336`
- `ln(0.9) ~= -0.10536`
- ratio `~= 23.10`

Ticks must be integers:

`n_min = ceil(23.10) = 24`

Interpretation:

- need at least **24 ticks** in high phase to guarantee crossing 6000 from 800 (in noise-free fixed-target analysis)
- with current `15`-tick high phase, this crossing is not guaranteed.
