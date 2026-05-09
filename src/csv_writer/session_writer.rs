//! `SessionWriter` opens a per-session v2 CSV and serializes EnrichedFrames.
//!
//! - Filename: `<wallclock>_<source>_ord<N>_track<M>.csv` under `out_dir`.
//! - Line 1: `# schema_version=2 source=<format> session_started=<rfc3339>`.
//! - Line 2: header derived from `EnrichedFrame` field names.
//! - Each subsequent line: one frame.
//!
//! The sidecar `.session.toml` is written by Phase 7's `close()` once
//! aggregator state is meaningful enough to record.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use csv::Writer;
use serde::Serialize;

use crate::car_db::CarCalibration;
use crate::decoder::{RawPacket, Source};

use super::enriched::EnrichedFrame;
use super::SCHEMA_VERSION;

/// Static-ish session metadata captured at the start of a recording.
///
/// `track_id` is `None` for sources that don't carry it (everything but
/// FM 2023 extras).
#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub source: Source,
    pub car_ordinal: i32,
    pub track_id: Option<i32>,
    pub session_started: DateTime<Utc>,
}

impl SessionMeta {
    /// Pull as much metadata from the first decoded packet as possible.
    pub fn from_first_packet(packet: &RawPacket) -> Self {
        Self {
            source: packet.source,
            car_ordinal: packet.car_ordinal,
            track_id: packet.track_id,
            session_started: Utc::now(),
        }
    }

    fn filename(&self) -> String {
        // Wall-clock formatted compactly, but still human-decodable.
        let ts = self.session_started.format("%Y%m%dT%H%M%SZ");
        let track = match self.track_id {
            Some(id) => format!("track{}", id),
            None => "trackna".to_string(),
        };
        format!(
            "{}_{}_ord{}_{}.csv",
            ts,
            self.source.as_str(),
            self.car_ordinal,
            track
        )
    }
}

/// Owning writer for a single capture session.
pub struct SessionWriter {
    writer: Writer<BufWriter<File>>,
    path: PathBuf,
    meta: SessionMeta,
    frames_written: u64,
}

impl SessionWriter {
    /// Open `out_dir/<wallclock>_<source>_ord<N>_track<M>.csv` and write the
    /// schema-v2 comment header.
    pub fn open(out_dir: impl AsRef<Path>, meta: SessionMeta) -> io::Result<Self> {
        let dir = out_dir.as_ref();
        fs::create_dir_all(dir)?;
        let path = dir.join(meta.filename());
        let file = File::create(&path)?;
        let mut buf = BufWriter::new(file);
        writeln!(
            buf,
            "# schema_version={} source={} session_started={}",
            SCHEMA_VERSION,
            meta.source.as_str(),
            meta.session_started.to_rfc3339_opts(SecondsFormat::Secs, true),
        )?;
        let writer = csv::WriterBuilder::new().has_headers(true).from_writer(buf);
        Ok(Self {
            writer,
            path,
            meta,
            frames_written: 0,
        })
    }

    /// Convert the raw packet into an `EnrichedFrame` and serialize it.
    pub fn write_packet(&mut self, packet: &RawPacket) -> csv::Result<()> {
        let frame = EnrichedFrame::from_raw(packet);
        self.write_frame(&frame)
    }

    /// Serialize an already-built enriched frame. Useful for aggregator phases
    /// that compute temporal columns alongside raw fields.
    pub fn write_frame(&mut self, frame: &EnrichedFrame) -> csv::Result<()> {
        self.writer.serialize(frame)?;
        self.frames_written += 1;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn meta(&self) -> &SessionMeta {
        &self.meta
    }

    pub fn frames_written(&self) -> u64 {
        self.frames_written
    }

    /// Flush and drop the writer. Use [`SessionWriter::close_with_summary`]
    /// to also emit the `.session.toml` sidecar with resolved calibration
    /// and learned auto-cal values.
    pub fn close(mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Flush the CSV and write a `<csv_basename>.session.toml` sidecar with
    /// the resolved car calibration and any learned auto-cal values.
    pub fn close_with_summary(mut self, summary: &SessionSummary) -> io::Result<()> {
        self.writer.flush()?;
        let sidecar = self.path.with_extension("session.toml");
        let body = SidecarToml {
            schema_version: SCHEMA_VERSION,
            csv: self.path.file_name().map(|s| s.to_string_lossy().into_owned()),
            source: self.meta.source.as_str().to_string(),
            car_ordinal: self.meta.car_ordinal,
            track_id: self.meta.track_id,
            session_started: self
                .meta
                .session_started
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            frames_written: self.frames_written,
            calibration: summary.calibration.clone(),
            learned: summary.learned.clone(),
        };
        let text = toml::to_string_pretty(&body).map_err(io::Error::other)?;
        fs::write(&sidecar, text)?;
        Ok(())
    }
}

/// Resolved per-session state captured at shutdown. The CSV writer
/// serializes this into the `.session.toml` sidecar so consumers don't
/// have to re-resolve calibration data later.
#[derive(Debug, Default, Clone)]
pub struct SessionSummary {
    pub calibration: CarCalibration,
    pub learned: LearnedValues,
}

/// Auto-calibrated values learned from the recording session.
#[derive(Debug, Default, Clone, Serialize)]
pub struct LearnedValues {
    pub wheel_radius_f: Option<f32>,
    pub wheel_radius_r: Option<f32>,
}

#[derive(Debug, Serialize)]
struct SidecarToml {
    schema_version: u32,
    csv: Option<String>,
    source: String,
    car_ordinal: i32,
    track_id: Option<i32>,
    session_started: String,
    frames_written: u64,
    calibration: CarCalibration,
    learned: LearnedValues,
}
