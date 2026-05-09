# Math reference

This crate decodes Forza UDP telemetry, then enriches each frame with a few
small pieces of physics so downstream tools (Kingpin's `PhysicalFrame` rig,
analysis notebooks, Blender inspectors) get more than what Forza emits on
the wire. This document is the one place every formula is written out
rigorously, with units, sign conventions, and pointers to the code.

If you change a formula in code, change it here too. If something below
disagrees with the source, **the source wins** &mdash; please open a fix.

## Contents

1. [Conventions](#conventions)
2. [Time step](#time-step)
3. [Unit conversions](#unit-conversions)
4. [Cheap per-frame normalizations](#cheap-per-frame-normalizations)
5. [First-order low-pass (EMA)](#first-order-low-pass-ema)
6. [Body slip angle](#body-slip-angle)
7. [Ride-height baseline](#ride-height-baseline)
8. [Per-corner normal load](#per-corner-normal-load)
9. [Wheel-radius auto-calibration](#wheel-radius-auto-calibration)
10. [Steady-state classifier](#steady-state-classifier)

---

## Conventions

Forza's car-local frame, used throughout this crate:

- $\hat{x}$ points to the driver's right.
- $\hat{y}$ points up.
- $\hat{z}$ points forward (the car's heading).
- Angular velocity components: $\omega_x$ = pitch, $\omega_y$ = yaw,
  $\omega_z$ = roll.

All accelerations are in $\mathrm{m/s^2}$, all velocities in $\mathrm{m/s}$,
all angular rates in $\mathrm{rad/s}$, all angles in radians, all masses in
kilograms, all forces in Newtons, all distances in meters, all times in
seconds (unless suffixed `_ms` or `_ns`).

Per-corner quantities are stored in a `Wheel<T>` ordered
front-left, front-right, rear-left, rear-right
($\mathrm{FL}$, $\mathrm{FR}$, $\mathrm{RL}$, $\mathrm{RR}$).

Earth gravity:

\[
g = 9.80665 \,\mathrm{m/s^2}
\]

Source: `src/aggregator/normal_load.rs::GRAVITY_M_S2`.

---

## Time step

Forza ships its own per-packet `timestamp_ms`, but it wraps during long
sessions, so the listener stamps every datagram with a monotonic
nanosecond clock $t_n$ (`recv_time_ns`). The aggregator computes a
per-frame $\Delta t$ from successive receive times:

\[
\Delta t_n =
\begin{cases}
\dfrac{t_n - t_{n-1}}{10^{9}} & \text{if } t_n > t_{n-1} \\
0 & \text{otherwise (first frame, or non-monotonic)}
\end{cases}
\]

The "$\Delta t = 0$" branch is what makes the first-order filter below
treat the very first sample as initialisation rather than a step input.

Source: `src/aggregator/session.rs::AggregatorSession::ingest`.

---

## Unit conversions

### Fahrenheit &rarr; Celsius

Forza emits tire surface temperatures in degrees Fahrenheit. We convert
once at decode time so the rest of the pipeline only ever sees Celsius:

\[
T_{\mathrm{C}} = (T_{\mathrm{F}} - 32) \cdot \tfrac{5}{9}
\]

Variables:

| Symbol | Units | Meaning |
| --- | --- | --- |
| $T_{\mathrm{F}}$ | &deg;F | Wire-format tire temperature |
| $T_{\mathrm{C}}$ | &deg;C | Stored / serialized tire temperature |

Source: `src/decoder/units.rs::f_to_c`.

---

## Cheap per-frame normalizations

Forza encodes pedal positions as unsigned bytes and steering as a signed
byte. We normalize them to the conventional analog ranges for downstream
math (e.g. the steady-state classifier, wheel-radius coast gating):

| CSV column | Wire type | Range out | Formula |
| --- | --- | --- | --- |
| `steer_normalized` | `i8` | $[-1, +1]$ | $\tilde{s} = s / 127$ |
| `accel_normalized` | `u8` | $[0, 1]$ | $\tilde{a} = a / 255$ |
| `brake_normalized` | `u8` | $[0, 1]$ | $\tilde{b} = b / 255$ |
| `clutch_normalized` | `u8` | $[0, 1]$ | $\tilde{c} = c / 255$ |
| `handbrake_normalized` | `u8` | $[0, 1]$ | $\tilde{h} = h / 255$ |

The asymmetric divisor on `steer` ($127$ instead of $128$) is deliberate:
it preserves the symmetric range $[-1, +1]$ at the cost of a single
unreachable code at $-128$, which is much friendlier than producing a
slightly-asymmetric range $[-1, +0.992\ldots]$.

Source: `src/csv_writer/enriched.rs::EnrichedFrame::from_raw`.

---

## First-order low-pass (EMA)

We smooth three "visual" channels (longitudinal accel, lateral accel,
yaw rate) before serializing them. The smoothing is the standard
discrete-time, time-aware first-order IIR:

\[
y_n =
\begin{cases}
x_n & n = 0 \\
\alpha_n\, x_n + (1 - \alpha_n)\, y_{n-1} & n > 0
\end{cases}
\qquad
\alpha_n = \dfrac{\Delta t_n}{\tau + \Delta t_n}
\]

Variables:

| Symbol | Units | Meaning |
| --- | --- | --- |
| $x_n$ | varies | Raw input sample at frame $n$ |
| $y_n$ | same as $x_n$ | Filtered output |
| $\Delta t_n$ | s | Frame interval (see [Time step](#time-step)) |
| $\tau$ | s | Lowpass time constant |
| $\alpha_n$ | &mdash; | Per-frame mixing coefficient, $\alpha_n \in [0, 1)$ |

Why this exact form:

- It's *time-aware*. Forza's UDP rate is nominally 60&nbsp;Hz but jitters,
  and replays from CSV may use coarser cadences. A fixed-$\alpha$ EMA
  would change cutoff with sample rate; this one keeps the $-3\,\mathrm{dB}$
  point at $f_c = 1/(2\pi\tau)$ regardless of $\Delta t$.
- The $n = 0$ branch initialises $y_0 = x_0$ rather than $y_0 = 0$, so
  the first frame doesn't appear as a step from zero up to the actual
  signal value.
- $\Delta t \le 0$ (re-emitted or out-of-order packet) yields
  $\alpha = 0$, i.e. a no-op that returns $y_{n-1}$. Robust under replay.

We use $\tau = 0.2\,\mathrm{s}$ for all three visual lowpasses (about
$0.8\,\mathrm{Hz}$ cutoff): tight enough to follow visible transients,
loose enough to drop per-frame engine / suspension noise.

Sources:

- `src/aggregator/smoothing.rs::Ema`
- `src/aggregator/smoothing.rs::lowpass_alpha`
- `src/aggregator/session.rs::VISUAL_LOWPASS_TAU_S`

---

## Body slip angle

The body slip angle $\beta$ is the angle between the car's heading and
its velocity vector, in the horizontal plane. Forza's local frame puts
heading at $+\hat{z}$, so:

\[
\beta = \mathrm{atan2}(v_x, v_z)
\]

with the speed gate

\[
\beta = \varnothing \iff v_x^2 + v_z^2 < (1\,\mathrm{m/s})^2
\]

Variables:

| Symbol | Units | Meaning |
| --- | --- | --- |
| $v_x$ | m/s | Car-local lateral velocity (positive right) |
| $v_z$ | m/s | Car-local longitudinal velocity (positive forward) |
| $\beta$ | rad | Body slip angle |

Sign convention: $\beta > 0$ means the velocity vector is rotated to the
driver's right relative to the heading &mdash; equivalent to the rear of
the car sliding to the *left*. That's the classic right-hand-turn
oversteer attitude.

Why the gate: at low horizontal speeds, both $v_x$ and $v_z$ approach
zero and `atan2` becomes numerically meaningless. We return `None`
below $1\,\mathrm{m/s}$, which CSV consumers see as an empty cell.

Source: `src/aggregator/slip_angle.rs::body_slip_angle`.

---

## Ride-height baseline

Forza's `SuspensionTravelMeters` is an *absolute* spring displacement
measured from a private rig reference. Kingpin needs the *signed*
travel relative to the static ride height (positive = currently more
compressed than at rest).

We learn the baseline online by collecting per-corner samples while the
chassis is in cruise (low lateral, longitudinal and yaw activity), and
emitting their median once we have enough:

\[
\mathrm{cruise} \iff
\lvert a_{\mathrm{lat}} \rvert \le 1.0 \,\land\,
\lvert a_{\mathrm{long}} \rvert \le 0.8 \,\land\,
\lvert \omega_y \rvert \le 0.05
\]

(thresholds in $\mathrm{m/s^2}$ and $\mathrm{rad/s}$ respectively).

For each corner $i \in \{\mathrm{FL}, \mathrm{FR}, \mathrm{RL}, \mathrm{RR}\}$
maintain a deque $S_i$ of the most recent $K = 30$ cruise samples. Once
$|S_i| \ge K$:

\[
b_i = \mathrm{median}(S_i)
\]

and emit signed relative travel as

\[
r_i(t) = s_i(t) - b_i
\]

Variables:

| Symbol | Units | Meaning |
| --- | --- | --- |
| $s_i(t)$ | m | Current absolute travel at corner $i$ |
| $b_i$ | m | Learned baseline (resting travel) |
| $r_i(t)$ | m | Signed relative travel; $r_i > 0$ &rArr; compressed |
| $K$ | &mdash; | `CRUISE_SAMPLES_REQUIRED` (30) |

Why median, not mean: medians reject the occasional bump or bad sample
that slips past the cruise gate without inflating the variance. Why
$K = 30$: at 60 Hz that's $\sim$0.5 s of cruise, enough to be stable
without delaying baseline emission for too long.

Why feed the *raw* (un-low-passed) chassis signals to the gate: a lagged
$a_{\mathrm{lat}}$ would let the trailing edge of a corner sneak in as
"cruise". The gate has to fire on the actual instantaneous state.

Sources:

- `src/aggregator/suspension.rs::RideHeightLearner`
- `src/aggregator/suspension.rs::is_cruise`

---

## Per-corner normal load

Forza does not publish per-tire normal force. We reconstruct it from a
textbook quasi-static two-track model: static weight distribution plus
longitudinal and lateral load transfer.

### Setup

Static masses on each axle from the front weight bias $\phi$:

\[
m_{\mathrm{front}} = \phi\, m, \qquad
m_{\mathrm{rear}}  = (1 - \phi)\, m
\]

### Load transfers

Longitudinal transfer (front &harr; rear) from the chassis-frame
longitudinal acceleration $a_{\mathrm{long}}$:

\[
\Delta_{\mathrm{long}} = \dfrac{m\, a_{\mathrm{long}}\, h}{L}
\]

Lateral transfers (left &harr; right) per axle, from the chassis-frame
lateral acceleration $a_{\mathrm{lat}}$:

\[
\Delta_{\mathrm{lat,F}} = \dfrac{m_{\mathrm{front}}\, a_{\mathrm{lat}}\, h}{t_F}
\qquad
\Delta_{\mathrm{lat,R}} = \dfrac{m_{\mathrm{rear}}\,  a_{\mathrm{lat}}\, h}{t_R}
\]

### Per-axle and per-corner loads

Each axle's vertical load:

\[
F_{\mathrm{front}} = m_{\mathrm{front}}\, g - \Delta_{\mathrm{long}}
\qquad
F_{\mathrm{rear}}  = m_{\mathrm{rear}}\,  g + \Delta_{\mathrm{long}}
\]

Distribute each axle's load across its two corners, applying the
per-axle lateral transfer:

\[
\begin{aligned}
F_{\mathrm{FL}} &= \tfrac{1}{2}\, F_{\mathrm{front}} + \tfrac{1}{2}\, \Delta_{\mathrm{lat,F}} \\
F_{\mathrm{FR}} &= \tfrac{1}{2}\, F_{\mathrm{front}} - \tfrac{1}{2}\, \Delta_{\mathrm{lat,F}} \\
F_{\mathrm{RL}} &= \tfrac{1}{2}\, F_{\mathrm{rear}}  + \tfrac{1}{2}\, \Delta_{\mathrm{lat,R}} \\
F_{\mathrm{RR}} &= \tfrac{1}{2}\, F_{\mathrm{rear}}  - \tfrac{1}{2}\, \Delta_{\mathrm{lat,R}}
\end{aligned}
\]

By construction:

\[
F_{\mathrm{FL}} + F_{\mathrm{FR}} + F_{\mathrm{RL}} + F_{\mathrm{RR}} = m\, g
\]

### Variables

| Symbol | Units | Source |
| --- | --- | --- |
| $m$ | kg | `CarCalibration::mass` |
| $h$ | m | `CarCalibration::cog_height` (centre-of-gravity height) |
| $L$ | m | `CarCalibration::wheelbase` |
| $t_F, t_R$ | m | `CarCalibration::track_f`, `track_r` |
| $\phi$ | &mdash; | `CarCalibration::front_weight_bias`, $\in [0, 1]$ |
| $a_{\mathrm{long}}$ | m/s$^2$ | `acceleration_local[2]` (raw, *not* low-passed) |
| $a_{\mathrm{lat}}$ | m/s$^2$ | `acceleration_local[0]` (raw) |
| $g$ | m/s$^2$ | $9.80665$ |

### Sign conventions

- $a_{\mathrm{long}} > 0$ &rArr; forward (throttle-on) acceleration
  &rArr; load shifts rearward &rArr; $\Delta_{\mathrm{long}} > 0$
  &rArr; front loses, rear gains.
- $a_{\mathrm{lat}} > 0$ &rArr; centripetal acceleration to the right
  in body frame &rArr; the chassis tilts left, transferring load to the
  *left* (outside of a right-hand turn) &rArr; $\Delta_{\mathrm{lat}} > 0$
  on the left corners.

### Why "all-or-nothing"

`estimate(...)` returns `None` if any of the six calibration fields is
missing. A partial reconstruction (e.g. zero lateral transfer because
$t_F$ is unknown but a non-zero longitudinal one) would silently bias
the outputs. Empty cells in CSV are honest; biased numbers are not.

### Why raw, not low-passed, chassis acceleration

We feed the unsmoothed `acceleration_local` into the load model so the
returned forces describe the *same instantaneous chassis state* as the
rest of the row. Mixing a low-passed `a_lat` with raw slip angles would
produce a row that didn't internally agree with itself.

Source: `src/aggregator/normal_load.rs::estimate`.

---

## Wheel-radius auto-calibration

Effective rolling radius depends on tire spec, pressure, and load &mdash;
none of which Forza exports. While the *driven* axle slips under power,
the *undriven* axle obeys $v = \omega r$ to within drag. So during pure
coast we get a clean per-frame estimate

\[
r_n = \dfrac{|v_n|}{\bar{\omega}_n}, \qquad
\bar{\omega}_n = \tfrac{1}{2}\bigl(\omega_{n,\mathrm{L}} + \omega_{n,\mathrm{R}}\bigr)
\]

and we publish the median once enough samples have accumulated:

\[
\hat{r} = \mathrm{median}\bigl(\{ r_n \mid n \in \text{coast samples} \}\bigr)
\]

### Coast gates

A frame contributes to the radius estimate iff *all* of:

\[
|v| \ge 5 \,\mathrm{m/s} \quad\land\quad
\tilde{a} < 0.05 \quad\land\quad
\tilde{b} < 0.05 \quad\land\quad
|\bar{\omega}| \ge 1 \,\mathrm{rad/s}
\]

where $\tilde{a}, \tilde{b}$ are the normalized accel and brake pedals.

### Drivetrain dispatch

Forza's `DrivetrainType` selects which axle is undriven, and therefore
which $\hat{r}$ we can learn cleanly:

| Code | Drivetrain | Undriven axle | We learn |
| --- | --- | --- | --- |
| 0 | FWD | rear | $\hat{r}_R$ |
| 1 | RWD | front | $\hat{r}_F$ |
| 2 | AWD | none under power, but in *pure coast* none is loaded | both |
| other | unknown | (treated as AWD) | both |

We collect $K = 30$ coast samples per axle in a deque before emitting the
median. The median refines in-place as more coast samples come in.

Source: `src/aggregator/wheel_radius.rs::RadiusLearner`.

---

## Steady-state classifier

A coarse bitmask describing what the car is doing right now, plus
`time_in_state_ms`, the wall-clock duration since the bitmask last
changed.

The flags:

| Bit | Flag | Predicate |
| --- | --- | --- |
| 0 | `COASTING` | $\tilde{a} < 0.05 \land \tilde{b} < 0.05$ |
| 1 | `ACCELERATING` | $\tilde{a} > 0.05$ |
| 2 | `BRAKING` | $\tilde{b} > 0.05$ |
| 3 | `CORNERING` | $\lvert a_{\mathrm{lat}} \rvert > 1.0 \,\lor\, \lvert \omega_y \rvert > 0.05$ |
| 4 | `STEADY` | $\lvert a_{\mathrm{lat}} \rvert < 0.5 \land \lvert a_{\mathrm{long}} \rvert < 0.5 \land \lvert \omega_y \rvert < 0.03$ |

(Thresholds in $\mathrm{m/s^2}$ and $\mathrm{rad/s}$ respectively.)

`time_in_state_ms` is computed by tracking the receive time when the
bitmask last changed:

\[
T_n = \left\lfloor \dfrac{t_n - t_{\mathrm{state\_started}}}{10^{6}} \right\rfloor
\]

where $t_{\mathrm{state\_started}}$ is reset to $t_n$ whenever the
current bitmask differs from the previous one.

The flags are intentionally *independent*: e.g. a trailing-brake corner
entry sets `BRAKING | CORNERING` simultaneously. `STEADY` is the only
flag that overlaps with itself only ("none of the transients fire").

Source: `src/aggregator/steady_state.rs::Classifier`.

---

## Cross references

| Concept | Source | Tests |
| --- | --- | --- |
| EMA | `src/aggregator/smoothing.rs` | `smoothing::tests` |
| Body slip | `src/aggregator/slip_angle.rs` | `slip_angle::tests` |
| Ride height | `src/aggregator/suspension.rs` | `suspension::tests` |
| Normal load | `src/aggregator/normal_load.rs` | `normal_load::tests` |
| Wheel radius | `src/aggregator/wheel_radius.rs` | `wheel_radius::tests` |
| Steady state | `src/aggregator/steady_state.rs` | `steady_state::tests` |
| F &rarr; C | `src/decoder/units.rs` | (covered indirectly by decoder tests) |
| Normalizations | `src/csv_writer/enriched.rs` | (covered by CSV golden tests) |
