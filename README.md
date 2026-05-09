# forza-telemetry

Decode and record Forza Motorsport / Horizon UDP telemetry into a clean
v2 CSV per session. Future tooling (a Blender side inspector) reads these
captures back to drive Kingpin's `PhysicalFrame` look-dev rig.

The crate exposes a small library plus a `forza-telemetry` binary; the
library boundary is intentionally clean so a future inspector UI can link
it without dragging in tokio.

## What it currently does

- Decodes all four documented Forza Data Out wire formats:
  - `232 bytes` Sled (FM7/FM/FH base telemetry).
  - `311 bytes` Dash (FM7 / FM 2023).
  - `324 bytes` Dash Horizon (FH4 / FH5: 12-byte gap after sled).
  - `331 bytes` FM 2023 extras (Dash + per-tire wear + track ordinal).
- Fixes the historical bugs in the older decoder: explicit little-endian
  reads, `i32` for `car_ordinal` / `car_class` / etc., correctly named
  `tire_slip_ratio_*`, and tire temperatures converted Fahrenheit -> Celsius
  at decode time.
- Writes a per-session CSV with a `# schema_version=2 source=<format>` comment
  header and stable column ordering. Optional fields serialize as empty
  cells. Temporal columns are reserved here; later phases populate them.

The previous capture (`motorsport_8_hakone_1_lap.csv`) remains in place as
a historical artifact and is not compatible with the v2 schema.

## Install

```bash
cargo install --path .
```

## Record a session

Forza ships its UDP telemetry on a port you configure in-game. Default for
this tool is `7777`; point the game at the same port. Captures land under
`captures/<wallclock>_<source>_ord<carOrdinal>_track<trackOrdinal|na>.csv`.

```bash
forza-telemetry record --host 0.0.0.0 --port 7777 --out captures/
```

Hit Ctrl-C to stop. The writer flushes and closes the CSV cleanly.

Override the output directory or port as needed:

```bash
forza-telemetry record --port 23555 --out /tmp/forza-runs
```

## Library

```rust
use forza_telemetry::{decode, Source};

let bytes: &[u8] = /* a Forza Data Out datagram */;
let packet = decode(bytes, /* recv_time_ns */ 0)?;
match packet.source {
    Source::DashFm => println!("FM dash, ordinal={}", packet.car_ordinal),
    _ => {}
}
```

See `src/decoder` for the wire formats, `src/csv_writer` for the v2 schema,
and `src/stream/udp_server.rs` for the listener loop.

## Math

Every derivation the aggregator runs (low-pass filter, body slip angle,
ride-height baseline, per-corner normal load, wheel-radius auto-cal,
steady-state classifier) is written out with LaTeX, units, and sign
conventions in [`docs/math.md`](docs/math.md). If you change a formula
in code, update the doc to match.

## Tests

```bash
cargo test
```

Phase-1 fixtures live under `tests/fixtures/*.bin`. The synthetic builder
in `tests/common/mod.rs` is the source of truth used by the tests; real
captures dropped into the fixtures directory get exercised automatically by
`real_fixtures_decode_when_present`.

## Status

All seven phases of the rebuild are landed:

- [x] Phase 1: decoder + crate restructure.
- [x] Phase 2: v2 CSV writer (raw + cheap normalizations).
- [x] Phase 3: UDP listener + `forza-telemetry record` CLI.
- [x] Phase 4: aggregator basics (lowpassed accels, yaw rate, body slip).
- [x] Phase 5: ride-height baseline learner with cruise gating.
- [x] Phase 6: car DB lookup + per-corner normal-load reconstruction.
- [x] Phase 7: wheel-radius auto-cal, steady-state classifier, sidecar TOML.

A `<wallclock>_<source>_ord<N>_track<M>.session.toml` sidecar is written
next to each capture on graceful shutdown, recording the resolved per-car
calibration plus any auto-calibrated wheel radii learned during the run.

### Per-car overrides

Drop a TOML next to your run and pass it via `--cars`:

```toml
[[car]]
ordinal = 2735
mass = 1620.0
cog_height = 0.50
front_weight_bias = 0.50
```

User entries win field-by-field; missing fields fall back to the bundled
defaults under `src/car_db/data/defaults.toml`.

```bash
forza-telemetry record --cars cars.toml --out captures/
```
