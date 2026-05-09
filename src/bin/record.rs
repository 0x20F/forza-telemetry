//! `forza-telemetry record` CLI.
//!
//! Binds a UDP socket on the requested host/port, decodes each packet, and
//! writes a per-session CSV file under `--out`. Filename and metadata come
//! from the first decoded packet, so we do not commit to a path until we
//! actually see telemetry. Exits cleanly on Ctrl-C.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use forza_telemetry::aggregator::AggregatorSession;
use forza_telemetry::car_db;
use forza_telemetry::csv_writer::{
    EnrichedFrame, LearnedValues, SessionMeta, SessionSummary, SessionWriter,
};
use forza_telemetry::{decode, stream};

#[derive(Parser, Debug)]
#[command(name = "forza-telemetry", version, about = "Forza Data Out telemetry recorder")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Listen for Forza Data Out UDP packets and write a v2 CSV per session.
    Record {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// UDP port (Forza's documented default is 7777).
        #[arg(long, default_value_t = 7777)]
        port: u16,
        /// Directory captures land in. Created if missing.
        #[arg(long, default_value = "captures")]
        out: PathBuf,
        /// Optional TOML file with per-car calibration overrides.
        #[arg(long)]
        cars: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Record {
            host,
            port,
            out,
            cars,
        } => match run_record(&host, port, &out, cars.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("forza-telemetry record: {err}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_record(
    host: &str,
    port: u16,
    out_dir: &PathBuf,
    cars: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(record_loop(host, port, out_dir, cars))
}

async fn record_loop(
    host: &str,
    port: u16,
    out_dir: &PathBuf,
    cars: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("forza-telemetry: listening on {host}:{port}, writing to {out_dir:?}");

    let mut writer: Option<SessionWriter> = None;
    let mut aggregator: Option<AggregatorSession> = None;
    let mut progress = CalibrationProgress::new();
    let mut drivetrain_type: i32 = -1;
    let mut errors = 0u64;

    let shutdown = async {
        // Ignoring errors: ctrl-c handler installation failures should not
        // crash the listener; the user can still kill the process.
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("forza-telemetry: shutting down...");
    };

    stream::serve(
        (host, port),
        |bytes, recv_ns| {
            let pkt = match decode(bytes, recv_ns) {
                Ok(p) => p,
                Err(err) => {
                    errors += 1;
                    if errors <= 5 || errors % 1000 == 0 {
                        eprintln!("decode error #{errors}: {err}");
                    }
                    return;
                }
            };

            if writer.is_none() {
                let meta = SessionMeta::from_first_packet(&pkt);
                match SessionWriter::open(out_dir, meta) {
                    Ok(w) => {
                        eprintln!("forza-telemetry: writing {:?}", w.path());
                        writer = Some(w);
                    }
                    Err(err) => {
                        eprintln!("forza-telemetry: failed to open csv: {err}");
                        return;
                    }
                }
                let calib = car_db::lookup(pkt.car_ordinal, cars);
                if calib.mass.is_some() {
                    eprintln!(
                        "forza-telemetry: car_ordinal={} -> calibration found",
                        pkt.car_ordinal
                    );
                } else {
                    eprintln!(
                        "forza-telemetry: car_ordinal={} -> no calibration; \
                         normal_load columns will be empty",
                        pkt.car_ordinal
                    );
                }
                aggregator = Some(AggregatorSession::with_calibration(calib));
                drivetrain_type = pkt.drivetrain_type;
            }

            if let (Some(w), Some(agg)) = (writer.as_mut(), aggregator.as_mut()) {
                let frame = agg.ingest(&pkt);
                progress.observe(&frame, drivetrain_type);
                if let Err(err) = w.write_frame(&frame) {
                    eprintln!("forza-telemetry: write error: {err}");
                }
            }
        },
        shutdown,
    )
    .await?;

    if let Some(w) = writer {
        let path = w.path().to_path_buf();
        let frames = w.frames_written();
        if let Some(agg) = aggregator.as_ref() {
            let (radius_f, radius_r) = agg.learned_radii();
            let summary = SessionSummary {
                calibration: agg.calibration().clone(),
                learned: LearnedValues {
                    wheel_radius_f: radius_f,
                    wheel_radius_r: radius_r,
                },
            };
            w.close_with_summary(&summary)?;
        } else {
            w.close()?;
        }
        eprintln!(
            "forza-telemetry: closed {path:?} ({frames} frames, {errors} decode errors)"
        );
    } else {
        eprintln!("forza-telemetry: no packets received; nothing written");
    }

    Ok(())
}

/// Tracks which auto-calibrated values have been learned so we only log
/// each transition once. The first frame after a corner's baseline (or an
/// axle's wheel radius) flips from `None` to `Some` produces a one-line
/// summary; once everything the drivetrain can teach us is settled we
/// also emit a final "auto-cal complete" line.
struct CalibrationProgress {
    baseline_fl: bool,
    baseline_fr: bool,
    baseline_rl: bool,
    baseline_rr: bool,
    radius_f: bool,
    radius_r: bool,
    complete: bool,
}

impl CalibrationProgress {
    fn new() -> Self {
        Self {
            baseline_fl: false,
            baseline_fr: false,
            baseline_rl: false,
            baseline_rr: false,
            radius_f: false,
            radius_r: false,
            complete: false,
        }
    }

    fn observe(&mut self, frame: &EnrichedFrame, drivetrain: i32) {
        log_corner(&mut self.baseline_fl, "FL", frame.ride_height_baseline_fl);
        log_corner(&mut self.baseline_fr, "FR", frame.ride_height_baseline_fr);
        log_corner(&mut self.baseline_rl, "RL", frame.ride_height_baseline_rl);
        log_corner(&mut self.baseline_rr, "RR", frame.ride_height_baseline_rr);
        log_axle(&mut self.radius_f, "front", frame.wheel_radius_learned_f);
        log_axle(&mut self.radius_r, "rear", frame.wheel_radius_learned_r);

        if !self.complete && self.is_complete(drivetrain) {
            eprintln!("forza-telemetry: auto-cal complete");
            self.complete = true;
        }
    }

    fn is_complete(&self, drivetrain: i32) -> bool {
        let baselines_done =
            self.baseline_fl && self.baseline_fr && self.baseline_rl && self.baseline_rr;
        if !baselines_done {
            return false;
        }
        match drivetrain {
            // Forza convention: 0=FWD (rear undriven), 1=RWD (front undriven),
            // 2=AWD (both axles learn during coast). Anything else: be
            // conservative and require both axles.
            0 => self.radius_r,
            1 => self.radius_f,
            _ => self.radius_f && self.radius_r,
        }
    }
}

fn log_corner(flag: &mut bool, label: &str, value: Option<f32>) {
    if !*flag {
        if let Some(v) = value {
            eprintln!("forza-telemetry: ride-height baseline {label} learned: {v:.4} m");
            *flag = true;
        }
    }
}

fn log_axle(flag: &mut bool, label: &str, value: Option<f32>) {
    if !*flag {
        if let Some(v) = value {
            eprintln!("forza-telemetry: wheel radius ({label}) learned: {v:.4} m");
            *flag = true;
        }
    }
}
