use media_spike::codec::{CodecError, MediaCapabilities, VideoProfile::*, select_codec};
#[test]
fn unsupported_1440p60_encoder_falls_back() {
    let cap = MediaCapabilities {
        h264_available: true,
        certified_hardware_profiles: vec![],
    };
    for requested in [P1080F60, P1440F60] {
        let choice = select_codec(&cap, requested).unwrap();
        assert_eq!(choice.selected, P720F30);
        assert!(choice.fallback);
    }
}
#[test]
fn missing_baseline_codec_is_reported_instead_of_false_success() {
    assert_eq!(
        select_codec(&MediaCapabilities::default(), P720F30),
        Err(CodecError::H264Unavailable)
    );
}
#[test]
fn certification_for_one_profile_does_not_unlock_another() {
    let cap = MediaCapabilities {
        h264_available: true,
        certified_hardware_profiles: vec![P1080F60],
    };
    assert_eq!(select_codec(&cap, P1080F60).unwrap().selected, P1080F60);
    assert_eq!(select_codec(&cap, P1440F60).unwrap().selected, P720F30);
}
