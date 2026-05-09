//! Decoder integration tests.
//!
//! Synthetic byte buffers are built through `tests/common/mod.rs`, decoded,
//! and asserted to round-trip. Real captures from a running game can be
//! dropped into `tests/fixtures/*.bin`; the trailing block exercises any
//! that are present without forcing capture as a prerequisite.

mod common;

use std::fs;
use std::path::PathBuf;

use common::PacketSpec;
use forza_telemetry::{decode, DecodeError, Source};

const F32_EPS: f32 = 1e-5;
const TEMP_EPS: f32 = 1e-3;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn unsupported_length_returns_error() {
    let buf = vec![0u8; 100];
    match decode(&buf, 0) {
        Err(DecodeError::UnsupportedLength(100)) => {}
        other => panic!("expected UnsupportedLength(100), got {:?}", other),
    }
}

#[test]
fn empty_buffer_is_unsupported_length() {
    let buf: Vec<u8> = vec![];
    assert!(matches!(decode(&buf, 0), Err(DecodeError::UnsupportedLength(0))));
}

#[test]
fn sled_232_round_trips() {
    let spec = PacketSpec::distinct();
    let bytes = spec.to_bytes(Source::Sled);
    assert_eq!(bytes.len(), 232);

    let pkt = decode(&bytes, 42).expect("decode");

    assert_eq!(pkt.source, Source::Sled);
    assert_eq!(pkt.recv_time_ns, 42);
    assert!(pkt.is_race_on);
    assert_eq!(pkt.timestamp_ms, spec.timestamp_ms);
    assert!((pkt.engine_max_rpm - spec.engine_max_rpm).abs() < F32_EPS);
    assert_eq!(pkt.acceleration_local, spec.acceleration_local);
    assert_eq!(pkt.velocity_local, spec.velocity_local);
    assert_eq!(pkt.angular_velocity, spec.angular_velocity);

    assert_eq!(pkt.normalized_suspension_travel.fl, spec.norm_susp_travel[0]);
    assert_eq!(pkt.normalized_suspension_travel.fr, spec.norm_susp_travel[1]);
    assert_eq!(pkt.normalized_suspension_travel.rl, spec.norm_susp_travel[2]);
    assert_eq!(pkt.normalized_suspension_travel.rr, spec.norm_susp_travel[3]);

    assert_eq!(pkt.tire_slip_ratio.fl, spec.tire_slip_ratio[0]);
    assert_eq!(pkt.tire_slip_ratio.fr, spec.tire_slip_ratio[1]);
    assert_eq!(pkt.tire_slip_ratio.rl, spec.tire_slip_ratio[2]);
    assert_eq!(pkt.tire_slip_ratio.rr, spec.tire_slip_ratio[3]);

    assert!(!pkt.wheel_on_rumble.fl);
    assert!(pkt.wheel_on_rumble.fr);
    assert!(!pkt.wheel_on_rumble.rl);
    assert!(pkt.wheel_on_rumble.rr);

    assert_eq!(pkt.car_ordinal, 2735);
    assert_eq!(pkt.car_class, 5);
    assert_eq!(pkt.performance_index, 800);
    assert_eq!(pkt.drivetrain_type, 2);
    assert_eq!(pkt.num_cylinders, 8);

    // Dash-only fields stay None on a sled packet.
    assert!(pkt.position.is_none());
    assert!(pkt.speed.is_none());
    assert!(pkt.steer.is_none());
    assert!(pkt.tire_wear.is_none());
    assert!(pkt.track_id.is_none());
}

#[test]
fn dash_fm_311_round_trips_and_converts_temps() {
    let mut spec = PacketSpec::distinct();
    spec.tire_temp_f = [32.0, 212.0, 100.0, 50.0]; // 0, 100, 37.78, 10 C
    let bytes = spec.to_bytes(Source::DashFm);
    assert_eq!(bytes.len(), 311);

    let pkt = decode(&bytes, 0).expect("decode");

    assert_eq!(pkt.source, Source::DashFm);
    assert_eq!(pkt.position, Some(spec.position));
    assert_eq!(pkt.speed, Some(spec.speed));
    assert_eq!(pkt.power_w, Some(spec.power_w));
    assert_eq!(pkt.torque_nm, Some(spec.torque_nm));

    let temps = pkt.tire_temp_c;
    assert!((temps.fl - 0.0).abs() < TEMP_EPS, "fl={}", temps.fl);
    assert!((temps.fr - 100.0).abs() < TEMP_EPS, "fr={}", temps.fr);
    assert!((temps.rl - 37.7777_8).abs() < 1e-2, "rl={}", temps.rl);
    assert!((temps.rr - 10.0).abs() < TEMP_EPS, "rr={}", temps.rr);

    assert_eq!(pkt.lap_number, Some(spec.lap_number));
    assert_eq!(pkt.race_position, Some(spec.race_position));
    assert_eq!(pkt.accel, Some(spec.accel));
    assert_eq!(pkt.steer, Some(spec.steer));
    assert_eq!(pkt.normalized_driving_line, Some(spec.normalized_driving_line));

    assert!(pkt.tire_wear.is_none());
    assert!(pkt.track_id.is_none());
}

#[test]
fn steering_left_and_right_are_signed_correctly() {
    let mut left_spec = PacketSpec::distinct();
    left_spec.steer = -100;
    let mut right_spec = PacketSpec::distinct();
    right_spec.steer = 100;

    let left = decode(&left_spec.to_bytes(Source::DashFm), 0).expect("decode left");
    let right = decode(&right_spec.to_bytes(Source::DashFm), 0).expect("decode right");

    assert_eq!(left.steer, Some(-100));
    assert_eq!(right.steer, Some(100));
    // Sanity: the sign actually crosses zero, no u8/i8 confusion.
    assert!(left.steer.unwrap() < 0);
    assert!(right.steer.unwrap() > 0);
}

#[test]
fn dash_horizon_324_round_trips_with_12_byte_gap() {
    let spec = PacketSpec::distinct();
    let bytes = spec.to_bytes(Source::DashHorizon);
    assert_eq!(bytes.len(), 324);

    let pkt = decode(&bytes, 0).expect("decode");

    assert_eq!(pkt.source, Source::DashHorizon);
    assert_eq!(pkt.position, Some(spec.position));
    assert_eq!(pkt.speed, Some(spec.speed));
    assert_eq!(pkt.steer, Some(spec.steer));
    // Sled fields still come from the front of the buffer.
    assert_eq!(pkt.car_ordinal, spec.car_ordinal);
}

#[test]
fn fm2023_extras_331_carries_tire_wear_and_track_id() {
    let mut spec = PacketSpec::distinct();
    spec.tire_wear = [0.10, 0.20, 0.30, 0.40];
    spec.track_id = 1234;
    let bytes = spec.to_bytes(Source::Fm2023Extras);
    assert_eq!(bytes.len(), 331);

    let pkt = decode(&bytes, 0).expect("decode");

    assert_eq!(pkt.source, Source::Fm2023Extras);
    let wear = pkt.tire_wear.expect("tire_wear present");
    assert!((wear.fl - 0.10).abs() < F32_EPS);
    assert!((wear.fr - 0.20).abs() < F32_EPS);
    assert!((wear.rl - 0.30).abs() < F32_EPS);
    assert!((wear.rr - 0.40).abs() < F32_EPS);
    assert_eq!(pkt.track_id, Some(1234));

    // Dash fields still wired up.
    assert_eq!(pkt.position, Some(spec.position));
}

#[test]
fn truncated_dash_packet_returns_truncated_after_length_dispatch() {
    // 232 looks like a sled, so the sled portion still decodes; this guards
    // against accidentally pulling dash bytes when the size is sled-shaped.
    let spec = PacketSpec::distinct();
    let mut bytes = spec.to_bytes(Source::DashFm);
    bytes.truncate(232);
    let pkt = decode(&bytes, 0).expect("decode");
    assert_eq!(pkt.source, Source::Sled);
    assert!(pkt.position.is_none());
}

#[test]
fn real_fixtures_decode_when_present() {
    // If captured fixtures exist on disk we exercise them; otherwise this
    // test is a no-op so the suite never breaks waiting on real captures.
    let dir = fixtures_dir();
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(&dir).expect("read fixtures dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("bin") {
            continue;
        }
        let bytes = fs::read(&path).expect("read fixture");
        let pkt = decode(&bytes, 0).unwrap_or_else(|e| {
            panic!("decode {:?} failed: {:?}", path.file_name(), e)
        });
        // Just enough to confirm we read something coherent.
        assert!(pkt.engine_max_rpm.is_finite());
    }
}
