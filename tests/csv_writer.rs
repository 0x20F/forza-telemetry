//! Phase 2 tests: open a session, write a few packets, parse the resulting
//! CSV back out, and check that raw fields plus cheap normalizations
//! round-trip with the right shape.

mod common;

use std::fs;

use chrono::TimeZone;
use chrono::Utc;
use common::PacketSpec;
use forza_telemetry::car_db::CarCalibration;
use forza_telemetry::csv_writer::{
    LearnedValues, SessionMeta, SessionSummary, SessionWriter, SCHEMA_VERSION,
};
use forza_telemetry::{decode, Source};
use tempfile::tempdir;

fn fixed_meta(source: Source, ordinal: i32, track: Option<i32>) -> SessionMeta {
    SessionMeta {
        source,
        car_ordinal: ordinal,
        track_id: track,
        session_started: Utc.with_ymd_and_hms(2026, 5, 9, 9, 15, 6).unwrap(),
    }
}

#[test]
fn comment_header_then_csv_header_then_rows() {
    let dir = tempdir().unwrap();
    let mut spec = PacketSpec::distinct();
    spec.steer = 64;
    spec.accel = 255;
    spec.brake = 0;

    let bytes = spec.to_bytes(Source::DashFm);
    let pkt = decode(&bytes, 1234).unwrap();

    let meta = fixed_meta(Source::DashFm, 2735, None);
    let mut writer = SessionWriter::open(dir.path(), meta.clone()).unwrap();
    writer.write_packet(&pkt).unwrap();
    let path = writer.path().to_path_buf();
    writer.close().unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let mut lines = contents.lines();

    let comment = lines.next().expect("comment line");
    assert!(comment.starts_with(&format!("# schema_version={}", SCHEMA_VERSION)));
    assert!(comment.contains("source=dash_fm"));
    assert!(comment.contains("session_started=2026-05-09T09:15:06Z"));

    let header = lines.next().expect("csv header");
    let fields: Vec<&str> = header.split(',').collect();
    // Spot-check a handful of expected columns from each section.
    for required in [
        "recv_time_ns",
        "source",
        "is_race_on",
        "timestamp_ms",
        "acceleration_local_x",
        "tire_temp_c_fl",
        "tire_slip_ratio_fl",
        "steer",
        "steer_normalized",
        "accel_normalized",
        "body_slip_angle_rad",
        "normal_load_fl",
        "wheel_radius_learned_f",
        "steady_state_flags",
    ] {
        assert!(
            fields.contains(&required),
            "missing column {required} in header"
        );
    }

    let row = lines.next().expect("first row");
    let values: Vec<&str> = row.split(',').collect();
    assert_eq!(values.len(), fields.len(), "row width mismatch");

    let by_name: std::collections::HashMap<_, _> =
        fields.iter().copied().zip(values.iter().copied()).collect();
    assert_eq!(by_name["recv_time_ns"], "1234");
    assert_eq!(by_name["source"], "dash_fm");
    assert_eq!(by_name["is_race_on"], "true");
    assert_eq!(by_name["car_ordinal"], "2735");
    assert_eq!(by_name["steer"], "64");
    // 64 / 127 ~= 0.5039
    let steer_norm: f32 = by_name["steer_normalized"].parse().unwrap();
    assert!((steer_norm - 64.0 / 127.0).abs() < 1e-4);
    let accel_norm: f32 = by_name["accel_normalized"].parse().unwrap();
    assert!((accel_norm - 1.0).abs() < 1e-6);
    let brake_norm: f32 = by_name["brake_normalized"].parse().unwrap();
    assert!(brake_norm.abs() < 1e-6);
    assert_eq!(by_name["body_slip_angle_rad"], ""); // reserved, empty
    assert_eq!(by_name["normal_load_fl"], "");
}

#[test]
fn sled_packet_writes_dash_optionals_as_empty() {
    let dir = tempdir().unwrap();
    let spec = PacketSpec::distinct();
    let bytes = spec.to_bytes(Source::Sled);
    let pkt = decode(&bytes, 0).unwrap();

    let meta = fixed_meta(Source::Sled, pkt.car_ordinal, None);
    let mut writer = SessionWriter::open(dir.path(), meta).unwrap();
    writer.write_packet(&pkt).unwrap();
    let path = writer.path().to_path_buf();
    writer.close().unwrap();

    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_path(&path)
        .unwrap();
    let headers = rdr.headers().unwrap().clone();
    let record = rdr.records().next().unwrap().unwrap();

    let by_name: std::collections::HashMap<_, _> = headers
        .iter()
        .zip(record.iter())
        .map(|(h, v)| (h.to_string(), v.to_string()))
        .collect();

    // Sled has no dash data, so every dash-only field is empty.
    for empty_col in [
        "position_x",
        "speed",
        "steer",
        "steer_normalized",
        "accel",
        "tire_wear_fl",
        "track_id",
    ] {
        assert_eq!(
            by_name[empty_col], "",
            "expected {empty_col} empty on sled row, got {:?}",
            by_name[empty_col]
        );
    }

    // Sled-side fields still populate.
    assert_eq!(by_name["car_ordinal"], "2735");
    assert!(by_name["acceleration_local_x"].parse::<f32>().is_ok());
}

#[test]
fn fm2023_packet_carries_tire_wear_and_track_id() {
    let dir = tempdir().unwrap();
    let mut spec = PacketSpec::distinct();
    spec.tire_wear = [0.10, 0.20, 0.30, 0.40];
    spec.track_id = 9_999;
    let bytes = spec.to_bytes(Source::Fm2023Extras);
    let pkt = decode(&bytes, 0).unwrap();

    let meta = fixed_meta(Source::Fm2023Extras, pkt.car_ordinal, pkt.track_id);
    let mut writer = SessionWriter::open(dir.path(), meta.clone()).unwrap();
    writer.write_packet(&pkt).unwrap();
    let path = writer.path().to_path_buf();
    writer.close().unwrap();

    // Filename should reflect the track and ordinal.
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    assert!(name.contains("ord2735"));
    assert!(name.contains("track9999"));
    assert!(name.contains("fm2023_extras"));

    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_path(&path)
        .unwrap();
    let headers = rdr.headers().unwrap().clone();
    let record = rdr.records().next().unwrap().unwrap();
    let lookup = |col: &str| -> String {
        let i = headers.iter().position(|h| h == col).expect(col);
        record.get(i).unwrap().to_string()
    };

    assert_eq!(lookup("track_id"), "9999");
    let wear: f32 = lookup("tire_wear_fl").parse().unwrap();
    assert!((wear - 0.10).abs() < 1e-5);
    let wear_rr: f32 = lookup("tire_wear_rr").parse().unwrap();
    assert!((wear_rr - 0.40).abs() < 1e-5);
}

#[test]
fn close_with_summary_emits_session_toml_sidecar() {
    let dir = tempdir().unwrap();
    let spec = PacketSpec::distinct();
    let bytes = spec.to_bytes(Source::DashFm);
    let pkt = decode(&bytes, 0).unwrap();

    let meta = fixed_meta(Source::DashFm, pkt.car_ordinal, None);
    let mut writer = SessionWriter::open(dir.path(), meta).unwrap();
    writer.write_packet(&pkt).unwrap();
    let csv_path = writer.path().to_path_buf();

    let summary = SessionSummary {
        calibration: CarCalibration {
            mass: Some(1500.0),
            wheelbase: Some(2.7),
            front_weight_bias: Some(0.55),
            ..Default::default()
        },
        learned: LearnedValues {
            wheel_radius_f: Some(0.345),
            wheel_radius_r: Some(0.342),
        },
    };
    writer.close_with_summary(&summary).unwrap();

    let sidecar = csv_path.with_extension("session.toml");
    assert!(sidecar.exists(), "sidecar at {sidecar:?} should exist");
    let body = std::fs::read_to_string(&sidecar).unwrap();
    assert!(body.contains("schema_version = 2"));
    assert!(body.contains("source = \"dash_fm\""));
    assert!(body.contains("car_ordinal = 2735"));
    assert!(body.contains("frames_written = 1"));
    assert!(body.contains("[calibration]"));
    assert!(body.contains("mass = 1500"));
    assert!(body.contains("[learned]"));
    assert!(body.contains("wheel_radius_f = 0.34"));
}

#[test]
fn writes_multiple_rows_with_one_header_block() {
    let dir = tempdir().unwrap();
    let spec = PacketSpec::distinct();
    let bytes = spec.to_bytes(Source::DashFm);
    let pkt = decode(&bytes, 0).unwrap();

    let meta = fixed_meta(Source::DashFm, pkt.car_ordinal, None);
    let mut writer = SessionWriter::open(dir.path(), meta).unwrap();
    for _ in 0..5 {
        writer.write_packet(&pkt).unwrap();
    }
    assert_eq!(writer.frames_written(), 5);
    let path = writer.path().to_path_buf();
    writer.close().unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    // 1 comment + 1 header + 5 rows
    assert_eq!(lines.len(), 7);
    assert!(lines[0].starts_with("# schema_version="));
    assert!(lines[1].contains("recv_time_ns"));
    let header_count = lines.iter().filter(|l| l.contains("recv_time_ns")).count();
    assert_eq!(header_count, 1, "header should appear exactly once");
}
