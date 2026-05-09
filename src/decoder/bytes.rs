//! Bounds-checked little-endian byte readers.
//!
//! Forza Data Out is documented as little-endian. The previous implementation
//! used [`f32::from_ne_bytes`], which silently produced garbage on big-endian
//! hosts; here we are explicit.

use super::DecodeError;

#[inline]
fn slice(buf: &[u8], offset: usize, len: usize) -> Result<&[u8], DecodeError> {
    buf.get(offset..offset + len)
        .ok_or(DecodeError::Truncated { offset, needed: len })
}

pub fn read_u8(buf: &[u8], offset: usize) -> Result<u8, DecodeError> {
    Ok(slice(buf, offset, 1)?[0])
}

pub fn read_i8(buf: &[u8], offset: usize) -> Result<i8, DecodeError> {
    Ok(slice(buf, offset, 1)?[0] as i8)
}

pub fn read_u16(buf: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let s = slice(buf, offset, 2)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

pub fn read_u32(buf: &[u8], offset: usize) -> Result<u32, DecodeError> {
    let s = slice(buf, offset, 4)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn read_i32(buf: &[u8], offset: usize) -> Result<i32, DecodeError> {
    let s = slice(buf, offset, 4)?;
    Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn read_f32(buf: &[u8], offset: usize) -> Result<f32, DecodeError> {
    let s = slice(buf, offset, 4)?;
    Ok(f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read four consecutive `f32` lanes (FL, FR, RL, RR) at `start`.
pub fn read_wheel_f32(buf: &[u8], start: usize) -> Result<super::Wheel<f32>, DecodeError> {
    Ok(super::Wheel {
        fl: read_f32(buf, start)?,
        fr: read_f32(buf, start + 4)?,
        rl: read_f32(buf, start + 8)?,
        rr: read_f32(buf, start + 12)?,
    })
}

/// Read four consecutive `i32` lanes treated as bool (FL, FR, RL, RR).
pub fn read_wheel_bool_i32(buf: &[u8], start: usize) -> Result<super::Wheel<bool>, DecodeError> {
    Ok(super::Wheel {
        fl: read_i32(buf, start)? != 0,
        fr: read_i32(buf, start + 4)? != 0,
        rl: read_i32(buf, start + 8)? != 0,
        rr: read_i32(buf, start + 12)? != 0,
    })
}
