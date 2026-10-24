use media_spike::{
    capture::VideoFrame,
    frame_clock::{read_stamp, stamp},
};
use std::sync::Arc;
fn frame() -> VideoFrame {
    VideoFrame {
        width: 640,
        height: 16,
        bgra: vec![90; 640 * 16 * 4].into(),
    }
}
#[test]
fn a_clock_stamp_survives_frame_storage_and_checks_integrity() {
    let mut image = frame();
    stamp(&mut image, 123_456).unwrap();
    assert_eq!(read_stamp(&image), Some(123_456));
    assert_eq!(
        image.bgra[0], 90,
        "clock must preserve the content outside its strip"
    );
    let data = Arc::make_mut(&mut image.bgra);
    // Invert one checksum bit's entire 8x8 block.
    for y in 8..16 {
        for x in 63 * 8..64 * 8 {
            let index = (y * 640 + x) * 4;
            for channel in 0..3 {
                data[index + channel] ^= 255;
            }
        }
    }
    assert_eq!(read_stamp(&image), None);
}
#[test]
fn unmarked_or_invalid_frames_have_no_latency_sample() {
    assert_eq!(read_stamp(&frame()), None);
    let mut too_small = VideoFrame {
        width: 1,
        height: 1,
        bgra: vec![0; 4].into(),
    };
    assert!(stamp(&mut too_small, 10).is_err());
    assert_eq!(read_stamp(&too_small), None);
}

#[test]
fn a_valid_frame_older_than_ten_seconds_stays_in_the_latency_measurement() {
    use media_spike::frame_clock::frame_age_ms;
    let mut image = frame();
    stamp(&mut image, 10).unwrap();
    assert_eq!(frame_age_ms(&image, 10_011), Some(10_001));
    assert_eq!(frame_age_ms(&image, 9), None);
}
