use serde::Serialize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoProfile {
    P720F30,
    P1080F60,
    P1440F60,
}
#[derive(Debug, Default)]
pub struct MediaCapabilities {
    pub h264_available: bool,
    pub certified_hardware_profiles: Vec<VideoProfile>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct EncoderChoice {
    pub requested: VideoProfile,
    pub selected: VideoProfile,
    pub fallback: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CodecError {
    H264Unavailable,
}
pub fn select_codec(
    cap: &MediaCapabilities,
    requested: VideoProfile,
) -> Result<EncoderChoice, CodecError> {
    if !cap.h264_available {
        return Err(CodecError::H264Unavailable);
    }
    let selected = if requested == VideoProfile::P720F30
        || cap.certified_hardware_profiles.contains(&requested)
    {
        requested
    } else {
        VideoProfile::P720F30
    };
    Ok(EncoderChoice {
        requested,
        selected,
        fallback: selected != requested,
    })
}
