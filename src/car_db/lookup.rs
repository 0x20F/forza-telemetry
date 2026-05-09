//! `CarCalibration` + ordinal lookup.
//!
//! The bundled `defaults.toml` is parsed lazily; user-supplied overrides
//! parse the same `[[car]]` schema and win field-by-field. Missing
//! ordinals return an all-`None` calibration so callers can simply check
//! `Option::is_some()` per field.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Bundled default car calibrations. Available so tools can introspect
/// what ordinals ship with the crate.
pub const BUNDLED_DEFAULTS: &str = include_str!("data/defaults.toml");

/// Static parameters describing the vehicle. Mirrors the fields Kingpin's
/// `VehicleCalibration` consumes; unknown values are `None` so the
/// aggregator can degrade gracefully when a car has no entry.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CarCalibration {
    pub mass: Option<f32>,
    pub cog_height: Option<f32>,
    pub wheelbase: Option<f32>,
    pub track_f: Option<f32>,
    pub track_r: Option<f32>,
    pub wheel_radius_f: Option<f32>,
    pub wheel_radius_r: Option<f32>,
    pub front_weight_bias: Option<f32>,
    pub spring_rate_f: Option<f32>,
    pub spring_rate_r: Option<f32>,
    pub steering_ratio: Option<f32>,
    pub ackermann_ratio: Option<f32>,
    pub pressure_f: Option<f32>,
    pub pressure_r: Option<f32>,
    pub sidewall_height_f: Option<f32>,
    pub sidewall_height_r: Option<f32>,
}

#[derive(Deserialize)]
struct CarEntry {
    ordinal: i32,
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(flatten)]
    calibration: CarCalibration,
}

#[derive(Deserialize, Default)]
struct CarFile {
    #[serde(default)]
    car: Vec<CarEntry>,
}

/// Look up a calibration for `ordinal`, with optional user overrides.
///
/// Resolution order: bundled defaults -> user TOML -> field-by-field merge
/// where a `Some` in the user TOML beats the bundled value. Any I/O or
/// parse failure on `custom` is treated as "no override" - we never panic
/// from a malformed user file at session start.
pub fn lookup(ordinal: i32, custom: Option<&Path>) -> CarCalibration {
    let mut cal = lookup_in(BUNDLED_DEFAULTS, ordinal);
    if let Some(path) = custom {
        if let Ok(text) = std::fs::read_to_string(path) {
            let user = lookup_in(&text, ordinal);
            cal = merge(cal, user);
        }
    }
    cal
}

fn lookup_in(toml_text: &str, ordinal: i32) -> CarCalibration {
    let parsed: CarFile = toml::from_str(toml_text).unwrap_or_default();
    parsed
        .car
        .into_iter()
        .find(|e| e.ordinal == ordinal)
        .map(|e| e.calibration)
        .unwrap_or_default()
}

fn merge(base: CarCalibration, over: CarCalibration) -> CarCalibration {
    CarCalibration {
        mass: over.mass.or(base.mass),
        cog_height: over.cog_height.or(base.cog_height),
        wheelbase: over.wheelbase.or(base.wheelbase),
        track_f: over.track_f.or(base.track_f),
        track_r: over.track_r.or(base.track_r),
        wheel_radius_f: over.wheel_radius_f.or(base.wheel_radius_f),
        wheel_radius_r: over.wheel_radius_r.or(base.wheel_radius_r),
        front_weight_bias: over.front_weight_bias.or(base.front_weight_bias),
        spring_rate_f: over.spring_rate_f.or(base.spring_rate_f),
        spring_rate_r: over.spring_rate_r.or(base.spring_rate_r),
        steering_ratio: over.steering_ratio.or(base.steering_ratio),
        ackermann_ratio: over.ackermann_ratio.or(base.ackermann_ratio),
        pressure_f: over.pressure_f.or(base.pressure_f),
        pressure_r: over.pressure_r.or(base.pressure_r),
        sidewall_height_f: over.sidewall_height_f.or(base.sidewall_height_f),
        sidewall_height_r: over.sidewall_height_r.or(base.sidewall_height_r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_entry_resolves() {
        let c = lookup(2735, None);
        assert!(c.mass.is_some());
        assert!(c.cog_height.is_some());
        assert!(c.front_weight_bias.is_some());
    }

    #[test]
    fn unknown_ordinal_is_all_none() {
        let c = lookup(-1, None);
        assert!(c.mass.is_none());
        assert!(c.cog_height.is_none());
    }

    #[test]
    fn user_override_wins_field_by_field() {
        let user_toml = r#"
            [[car]]
            ordinal = 2735
            mass = 9999.0
        "#;
        // Pretend defaults already had this mass; user overrides it with
        // a sentinel value while leaving other fields untouched.
        let bundled = lookup_in(BUNDLED_DEFAULTS, 2735);
        let user = lookup_in(user_toml, 2735);
        let merged = merge(bundled.clone(), user);
        assert_eq!(merged.mass, Some(9999.0));
        assert_eq!(merged.cog_height, bundled.cog_height);
    }

    #[test]
    fn missing_user_file_is_silently_ignored() {
        let c = lookup(2735, Some(Path::new("/definitely/does/not/exist.toml")));
        assert!(c.mass.is_some(), "should fall back to bundled default");
    }
}
