//! Phase 4 tests: drive synthetic packet streams through the aggregator and
//! check the temporal columns it populates.

mod common;

use common::PacketSpec;
use forza_telemetry::aggregator::steady_state::{
    FLAG_BRAKING, FLAG_COASTING, FLAG_CORNERING, FLAG_STEADY,
};
use forza_telemetry::aggregator::AggregatorSession;
use forza_telemetry::car_db::CarCalibration;
use forza_telemetry::{decode, Source};

const FRAME_NS: u64 = 16_666_667; // ~60 Hz

fn build_packet(recv_time_ns: u64, vel: [f32; 3], accel: [f32; 3], yaw_rate: f32) -> forza_telemetry::RawPacket {
    let mut spec = PacketSpec::distinct();
    spec.velocity_local = vel;
    spec.acceleration_local = accel;
    spec.angular_velocity = [0.0, yaw_rate, 0.0];
    let bytes = spec.to_bytes(Source::DashFm);
    decode(&bytes, recv_time_ns).unwrap()
}

fn build_with_susp(
    recv_time_ns: u64,
    accel: [f32; 3],
    yaw_rate: f32,
    susp_meters: [f32; 4],
) -> forza_telemetry::RawPacket {
    let mut spec = PacketSpec::distinct();
    spec.velocity_local = [0.0, 0.0, 30.0];
    spec.acceleration_local = accel;
    spec.angular_velocity = [0.0, yaw_rate, 0.0];
    spec.susp_travel_meters = susp_meters;
    let bytes = spec.to_bytes(Source::DashFm);
    decode(&bytes, recv_time_ns).unwrap()
}

#[test]
fn body_slip_populates_only_when_moving() {
    let mut agg = AggregatorSession::new();

    // First packet: stationary -> body slip None.
    let pkt0 = build_packet(0, [0.0, 0.0, 0.0], [0.0; 3], 0.0);
    let frame0 = agg.ingest(&pkt0);
    assert!(frame0.body_slip_angle_rad.is_none());

    // Second packet: 30 m/s forward -> slip ~ 0.
    let pkt1 = build_packet(FRAME_NS, [0.0, 0.0, 30.0], [0.0; 3], 0.0);
    let frame1 = agg.ingest(&pkt1);
    let slip = frame1.body_slip_angle_rad.expect("slip available");
    assert!(slip.abs() < 1e-5);
}

#[test]
fn lowpass_columns_converge_on_constant_inputs() {
    let mut agg = AggregatorSession::new();

    // Drive a constant longitudinal accel (Z forward), zero lateral, and a
    // constant yaw rate. After a few hundred frames the EMAs should be
    // very close to the inputs.
    let target_long = 4.0;
    let target_yaw = 0.5;
    let mut last = None;
    for i in 0..240 {
        // 240 frames at 60 Hz = 4 s, well past 5*tau.
        let pkt = build_packet(
            (i as u64) * FRAME_NS,
            [0.0, 0.0, 30.0],
            [0.0, 0.0, target_long],
            target_yaw,
        );
        last = Some(agg.ingest(&pkt));
    }

    let frame = last.unwrap();
    let long_lp = frame.acceleration_long_lp.unwrap();
    let lat_lp = frame.acceleration_lat_lp.unwrap();
    let yaw_lp = frame.yaw_rate_lp.unwrap();

    assert!((long_lp - target_long).abs() < 0.01, "long_lp = {long_lp}");
    assert!(lat_lp.abs() < 0.01, "lat_lp = {lat_lp}");
    assert!((yaw_lp - target_yaw).abs() < 0.01, "yaw_lp = {yaw_lp}");
}

#[test]
fn first_frame_initialises_lowpass_to_sample() {
    let mut agg = AggregatorSession::new();
    let pkt = build_packet(0, [0.0, 0.0, 30.0], [0.0, 0.0, 7.0], 0.3);
    let frame = agg.ingest(&pkt);
    // First sample sets the EMA so the value equals the input.
    assert!((frame.acceleration_long_lp.unwrap() - 7.0).abs() < 1e-5);
    assert!((frame.yaw_rate_lp.unwrap() - 0.3).abs() < 1e-5);
}

#[test]
fn lowpass_lags_a_step_input() {
    let mut agg = AggregatorSession::new();

    // 30 frames of zero accel -> EMA settles at 0.
    for i in 0..30 {
        let pkt = build_packet((i as u64) * FRAME_NS, [0.0, 0.0, 30.0], [0.0; 3], 0.0);
        agg.ingest(&pkt);
    }
    // Step to long_accel = 10 m/s^2; the EMA should be well below 10 on the
    // first frame after the step (because tau = 0.2 s, frame dt ~ 0.0167 s).
    let pkt = build_packet(30 * FRAME_NS, [0.0, 0.0, 30.0], [0.0, 0.0, 10.0], 0.0);
    let frame = agg.ingest(&pkt);
    let long_lp = frame.acceleration_long_lp.unwrap();
    assert!(long_lp > 0.0, "should have moved off zero, got {long_lp}");
    assert!(long_lp < 5.0, "should not have jumped to target, got {long_lp}");
}

#[test]
fn pure_lateral_velocity_yields_pi_over_two_slip() {
    let mut agg = AggregatorSession::new();
    let pkt = build_packet(0, [10.0, 0.0, 0.0], [0.0; 3], 0.0);
    let frame = agg.ingest(&pkt);
    let slip = frame.body_slip_angle_rad.unwrap();
    assert!((slip - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
}

#[test]
fn flat_cruise_learns_baseline_and_compression_is_signed() {
    let mut agg = AggregatorSession::new();

    // 30 frames of flat cruise (zero accels, zero yaw) at 0.05 m of
    // suspension travel. Baseline should emerge by frame 29.
    let cruise_susp = [0.05; 4];
    let mut last = None;
    for i in 0..40 {
        let pkt = build_with_susp((i as u64) * FRAME_NS, [0.0; 3], 0.0, cruise_susp);
        last = Some(agg.ingest(&pkt));
    }
    let frame = last.unwrap();
    assert!((frame.ride_height_baseline_fl.unwrap() - 0.05).abs() < 1e-5);
    assert!(frame.suspension_travel_relative_fl.unwrap().abs() < 1e-5);

    // Now drive a "cornering" frame: high lateral g (excluded from learner)
    // with extra compression on the front-right corner (more outside
    // weight transfer). Baseline must not move; relative travel must be
    // positive on the loaded corner.
    let corner_susp = [0.05, 0.07, 0.05, 0.07]; // FR + RR more compressed
    let pkt = build_with_susp(40 * FRAME_NS, [5.0, 0.0, 0.0], 0.4, corner_susp);
    let frame = agg.ingest(&pkt);
    assert!((frame.ride_height_baseline_fl.unwrap() - 0.05).abs() < 1e-5);
    assert!((frame.ride_height_baseline_fr.unwrap() - 0.05).abs() < 1e-5);
    assert!((frame.suspension_travel_relative_fr.unwrap() - 0.02).abs() < 1e-5);
    assert!(frame.suspension_travel_relative_fl.unwrap().abs() < 1e-5);
}

#[test]
fn normal_load_columns_populate_when_calibration_present() {
    let calib = CarCalibration {
        mass: Some(1000.0),
        cog_height: Some(0.5),
        wheelbase: Some(2.5),
        track_f: Some(1.5),
        track_r: Some(1.5),
        front_weight_bias: Some(0.55),
        ..Default::default()
    };
    let mut agg = AggregatorSession::with_calibration(calib);

    let pkt = build_with_susp(0, [0.0; 3], 0.0, [0.05; 4]);
    let frame = agg.ingest(&pkt);
    let total = frame.normal_load_fl.unwrap()
        + frame.normal_load_fr.unwrap()
        + frame.normal_load_rl.unwrap()
        + frame.normal_load_rr.unwrap();
    let expected = 1000.0 * 9.80665;
    assert!((total - expected).abs() < 1e-1, "total = {total}");
    // 55% on front axle.
    let front = frame.normal_load_fl.unwrap() + frame.normal_load_fr.unwrap();
    assert!((front - expected * 0.55).abs() < 1e-1);
}

#[test]
fn normal_load_columns_empty_without_calibration() {
    let mut agg = AggregatorSession::new();
    let pkt = build_with_susp(0, [0.0; 3], 0.0, [0.05; 4]);
    let frame = agg.ingest(&pkt);
    assert!(frame.normal_load_fl.is_none());
    assert!(frame.normal_load_rr.is_none());
}

#[test]
fn coast_window_learns_wheel_radius_for_rwd_front() {
    let mut agg = AggregatorSession::new();
    let r_target = 0.34_f32;
    let speed = 30.0_f32;
    let omega = speed / r_target;

    let mut spec = PacketSpec::distinct();
    spec.velocity_local = [0.0, 0.0, speed];
    spec.acceleration_local = [0.0; 3];
    spec.angular_velocity = [0.0; 3];
    spec.speed = speed;
    spec.accel = 0;
    spec.brake = 0;
    spec.drivetrain_type = 1; // RWD; front pair is undriven
    spec.wheel_rotation_speed = [omega, omega, omega, omega];

    let mut last = None;
    for i in 0..40 {
        let bytes = spec.to_bytes(Source::DashFm);
        let pkt = decode(&bytes, (i as u64) * FRAME_NS).unwrap();
        last = Some(agg.ingest(&pkt));
    }
    let frame = last.unwrap();
    let (rf, rr) = agg.learned_radii();
    assert!(rf.is_some(), "front radius should be learned");
    assert!((rf.unwrap() - r_target).abs() < 1e-3);
    assert!(rr.is_none(), "rear radius should not learn for RWD");
    assert!((frame.wheel_radius_learned_f.unwrap() - r_target).abs() < 1e-3);
    assert!(frame.wheel_radius_learned_r.is_none());
}

#[test]
fn steady_state_transition_resets_time_in_state() {
    let mut agg = AggregatorSession::new();

    // Settled cruise frame: zero accels, no pedal input.
    let mut spec = PacketSpec::distinct();
    spec.velocity_local = [0.0, 0.0, 30.0];
    spec.acceleration_local = [0.0; 3];
    spec.angular_velocity = [0.0; 3];
    spec.accel = 0;
    spec.brake = 0;

    let bytes = spec.to_bytes(Source::DashFm);
    let pkt0 = decode(&bytes, 0).unwrap();
    let f0 = agg.ingest(&pkt0);
    let flags0 = f0.steady_state_flags.unwrap();
    assert!(flags0 & FLAG_COASTING != 0);
    assert!(flags0 & FLAG_STEADY != 0);
    assert_eq!(f0.time_in_state_ms, Some(0));

    // 500 ms later, same flags -> time_in_state grows.
    let pkt1 = decode(&bytes, 500_000_000).unwrap();
    let f1 = agg.ingest(&pkt1);
    assert_eq!(f1.steady_state_flags, Some(flags0));
    let t = f1.time_in_state_ms.unwrap();
    assert!((t as i32 - 500).abs() <= 1);

    // Now hit the brakes and turn the wheel: state changes -> reset.
    spec.acceleration_local = [3.0, 0.0, -4.0];
    spec.angular_velocity = [0.0, 0.5, 0.0];
    spec.accel = 0;
    spec.brake = 200;
    let bytes2 = spec.to_bytes(Source::DashFm);
    let pkt2 = decode(&bytes2, 600_000_000).unwrap();
    let f2 = agg.ingest(&pkt2);
    let flags2 = f2.steady_state_flags.unwrap();
    assert_ne!(flags2, flags0);
    assert!(flags2 & FLAG_BRAKING != 0);
    assert!(flags2 & FLAG_CORNERING != 0);
    assert_eq!(f2.time_in_state_ms, Some(0));
}

#[test]
fn baseline_columns_empty_until_window_filled() {
    let mut agg = AggregatorSession::new();
    let cruise_susp = [0.05; 4];
    // 10 frames is not enough; learner needs 30.
    let mut last = None;
    for i in 0..10 {
        let pkt = build_with_susp((i as u64) * FRAME_NS, [0.0; 3], 0.0, cruise_susp);
        last = Some(agg.ingest(&pkt));
    }
    let frame = last.unwrap();
    assert!(frame.ride_height_baseline_fl.is_none());
    assert!(frame.suspension_travel_relative_fl.is_none());
}
