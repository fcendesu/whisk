//! Explicit, bounded settings for selected-window experiments.
use crate::{codec::VideoProfile, loopback::MediaError};
#[derive(Clone, Copy)]
pub struct CaptureRun {
    pub profile: VideoProfile,
    pub seconds: u64,
}
impl CaptureRun {
    pub fn new(profile: &str, seconds: u64) -> Result<Self, MediaError> {
        if !(1..=600).contains(&seconds) {
            return Err(MediaError::InvalidInput);
        }
        let profile = match profile {
            "720p30" => VideoProfile::P720F30,
            "1080p60" => VideoProfile::P1080F60,
            "1440p60" => VideoProfile::P1440F60,
            _ => return Err(MediaError::InvalidInput),
        };
        Ok(Self { profile, seconds })
    }
    pub fn settings(&self) -> (u32, u32, u32, u64) {
        match self.profile {
            VideoProfile::P720F30 => (1280, 720, 30, 2_000_000),
            VideoProfile::P1080F60 => (1920, 1080, 60, 6_000_000),
            VideoProfile::P1440F60 => (2560, 1440, 60, 10_000_000),
        }
    }
}
pub const TEST_WINDOW_TITLE: &str = "Whisk capture test window";
pub fn owned_test_window(title: Option<&str>, owner: i32, current: i32) -> bool {
    title == Some(TEST_WINDOW_TITLE) && owner == current
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_matching_title_from_another_process_is_not_an_owned_source() {
        assert!(owned_test_window(Some(TEST_WINDOW_TITLE), 42, 42));
        assert!(!owned_test_window(Some(TEST_WINDOW_TITLE), 7, 42));
        assert!(!owned_test_window(Some("Private app"), 42, 42));
        assert!(!owned_test_window(None, 42, 42));
    }
    #[test]
    fn explicit_profile_settings_and_ten_minute_budget() {
        for (name, expected) in [
            ("720p30", (1280, 720, 30, 2_000_000)),
            ("1080p60", (1920, 1080, 60, 6_000_000)),
            ("1440p60", (2560, 1440, 60, 10_000_000)),
        ] {
            let run = CaptureRun::new(name, 600).unwrap();
            assert_eq!(run.settings(), expected);
            assert_eq!(run.seconds, 600);
        }
    }
    #[test]
    fn unknown_profiles_and_unbounded_durations_are_rejected() {
        for (p, s) in [("auto", 600), ("720p30", 0), ("720p30", 601)] {
            assert!(CaptureRun::new(p, s).is_err());
        }
    }
}
