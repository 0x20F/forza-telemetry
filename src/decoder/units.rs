//! Unit conversions applied at decode time so the rest of the pipeline never
//! sees Forza's mixed unit system.

/// Convert Fahrenheit to Celsius. Forza emits tire temperatures in F.
#[inline]
pub fn f_to_c(f: f32) -> f32 {
    (f - 32.0) * (5.0 / 9.0)
}
