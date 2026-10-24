//! Optical timestamp strip for a same-process video round trip. It measures
//! application encode/network/decode age, not display or GPU presentation latency.
use crate::{capture::VideoFrame, loopback::MediaError};
const MAGIC: u16 = 0xa56b;
fn valid(frame: &VideoFrame) -> bool {
    frame.width >= 512
        && frame.height >= 8
        && (frame.width as usize)
            .checked_mul(frame.height as usize)
            .and_then(|n| n.checked_mul(4))
            .is_some_and(|size| size <= 64 * 1024 * 1024 && size == frame.bgra.len())
}
fn checksum(milliseconds: u32) -> u16 {
    milliseconds as u16 ^ (milliseconds >> 16) as u16 ^ 0x5aa5
}
pub fn stamp(frame: &mut VideoFrame, milliseconds: u32) -> Result<(), MediaError> {
    if !valid(frame) {
        return Err(MediaError::InvalidInput);
    }
    let word =
        ((MAGIC as u64) << 48) | ((milliseconds as u64) << 16) | checksum(milliseconds) as u64;
    let data = std::sync::Arc::make_mut(&mut frame.bgra);
    for bit in 0..64 {
        let value = if word & (1 << (63 - bit)) != 0 {
            240
        } else {
            16
        };
        for y in frame.height as usize - 8..frame.height as usize {
            for x in bit * 8..bit * 8 + 8 {
                let offset = (y * frame.width as usize + x) * 4;
                data[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }
    }
    Ok(())
}
pub fn read_stamp(frame: &VideoFrame) -> Option<u32> {
    if !valid(frame) {
        return None;
    }
    let mut word = 0u64;
    for bit in 0..64 {
        let mut sum = 0u32;
        for y in frame.height as usize - 6..frame.height as usize - 3 {
            for x in bit * 8 + 2..bit * 8 + 5 {
                let offset = (y * frame.width as usize + x) * 4;
                sum += frame.bgra[offset..offset + 3]
                    .iter()
                    .map(|value| *value as u32)
                    .sum::<u32>();
            }
        }
        word = (word << 1) | u64::from(sum > 128 * 27);
    }
    let milliseconds = (word >> 16) as u32;
    ((word >> 48) as u16 == MAGIC && word as u16 == checksum(milliseconds)).then_some(milliseconds)
}

pub fn frame_age_ms(frame: &VideoFrame, now_ms: u32) -> Option<u32> {
    now_ms.checked_sub(read_stamp(frame)?)
}
