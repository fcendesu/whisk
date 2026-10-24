//! Two synthetic publishers and one viewer; hardware/audio certification is separate.
use crate::{
    capture::VideoFrame,
    loopback::{MediaError, bgra_to_i420, decoded_bgra},
    room::{ProbeRoom, connect_probe},
};
use futures_util::StreamExt;
use livekit::{
    options::{TrackPublishOptions, VideoCodec, VideoEncoding},
    prelude::*,
    webrtc::{
        video_frame::{VideoFrame as RtcFrame, VideoRotation},
        video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
        video_stream::native::NativeVideoStream,
    },
};
use serde::Serialize;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until, timeout};

#[derive(Debug, Serialize)]
pub struct SwitchingReport {
    pub submitted_frames: [u64; 2],
    pub received_frames: [u64; 2],
    pub wrong_source_frames: u64,
    pub duration_ms: u64,
    pub audio_verified: bool,
    pub quality_certified: bool,
}

pub async fn run_switch_probe(
    url: &str,
    first_token: &str,
    second_token: &str,
    viewer_token: &str,
    phase: Duration,
) -> Result<SwitchingReport, MediaError> {
    if phase.is_zero()
        || phase > Duration::from_secs(300)
        || [first_token, second_token, viewer_token]
            .iter()
            .any(|token| token.trim().is_empty())
    {
        return Err(MediaError::InvalidInput);
    }
    let first = connect_probe(url, first_token)
        .await
        .map_err(|_| MediaError::Connect)?;
    let second = match connect_probe(url, second_token).await {
        Ok(room) => room,
        Err(_) => {
            let _ = first.close().await;
            return Err(MediaError::Connect);
        }
    };
    let mut viewer = match connect_probe(url, viewer_token).await {
        Ok(room) => room,
        Err(_) => {
            let _ = first.close().await;
            let _ = second.close().await;
            return Err(MediaError::Connect);
        }
    };
    let result = probe([&first, &second], &mut viewer, phase).await;
    let first_closed = first.close().await;
    let second_closed = second.close().await;
    let viewer_closed = viewer.close().await;
    let report = result?;
    if first_closed.is_err() || second_closed.is_err() || viewer_closed.is_err() {
        return Err(MediaError::Close);
    }
    Ok(report)
}

async fn probe(
    publishers: [&ProbeRoom; 2],
    viewer: &mut ProbeRoom,
    phase: Duration,
) -> Result<SwitchingReport, MediaError> {
    let sources: [NativeVideoSource; 2] = std::array::from_fn(|_| {
        NativeVideoSource::new(
            VideoResolution {
                width: 1280,
                height: 720,
            },
            true,
        )
    });
    let tracks: [LocalVideoTrack; 2] = std::array::from_fn(|index| {
        LocalVideoTrack::create_video_track(
            &format!("whisk-synthetic-{index}"),
            RtcVideoSource::Native(sources[index].clone()),
        )
    });
    let mut publications = Vec::new();
    for (publisher, track) in publishers.iter().zip(&tracks) {
        let publication = timeout(
            Duration::from_secs(10),
            publisher.sdk_room().local_participant().publish_track(
                LocalTrack::Video(track.clone()),
                TrackPublishOptions {
                    source: TrackSource::Screenshare,
                    video_codec: VideoCodec::H264,
                    simulcast: false,
                    video_encoding: Some(VideoEncoding {
                        max_bitrate: 2_000_000,
                        max_framerate: 30.,
                    }),
                    ..Default::default()
                },
            ),
        )
        .await
        .map_err(|_| MediaError::Publish)?
        .map_err(|_| MediaError::Publish)?;
        publications.push(publication);
    }
    let result=async {
        let mut remote:[Option<RemoteVideoTrack>;2]=[None,None];
        timeout(Duration::from_secs(10),async {
            while remote.iter().any(Option::is_none) {
                match viewer.next_event().await {
                    Some(RoomEvent::TrackSubscribed {track:RemoteTrack::Video(track),publication,..})=>{
                        if let Some(index)=publications.iter().position(|expected|expected.sid()==publication.sid()) { remote[index]=Some(track); }
                    }
                    Some(_)=>{},
                    None=>return Err(MediaError::Subscribe),
                }
            }
            Ok(())
        }).await.map_err(|_|MediaError::Subscribe)??;
        let selected=[remote[0].take().ok_or(MediaError::Subscribe)?,remote[1].take().ok_or(MediaError::Subscribe)?];
        let patterns=[[40,140,230,255],[230,120,40,255]].map(|color|VideoFrame {width:1280,height:720,bgra:color.into_iter().cycle().take(1280*720*4).collect::<Vec<_>>().into()});
        let mut report=SwitchingReport{submitted_frames:[0;2],received_frames:[0;2],wrong_source_frames:0,duration_ms:0,audio_verified:false,quality_certified:false};
        let started=Instant::now();
        for (focus,track) in selected.iter().enumerate() {
            let mut stream=NativeVideoStream::new(track.rtc_track());
            let end=Instant::now()+phase;
            let mut ticks=interval(Duration::from_secs_f64(1./30.));
            ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _=sleep_until(end)=>break,
                    _=ticks.tick()=>{
                        for index in 0..2 {
                            let frame=RtcFrame{rotation:VideoRotation::VideoRotation0,timestamp_us:started.elapsed().as_micros().min(i64::MAX as u128) as i64,frame_metadata:None,buffer:bgra_to_i420(&patterns[index])?};
                            if sources[index].capture_frame(&frame) { report.submitted_frames[index]+=1; }
                        }
                    }
                    event=viewer.next_event()=>{
                        match event {
                            Some(RoomEvent::ConnectionStateChanged(ConnectionState::Disconnected | ConnectionState::Reconnecting)) | None=>return Err(MediaError::Connect),
                            Some(RoomEvent::TrackUnsubscribed{publication,..}) if publication.sid()==publications[focus].sid()=>return Err(MediaError::Decode),
                            _=>{},
                        }
                    }
                    frame=stream.next()=>{
                        let frame=decoded_bgra(&frame.ok_or(MediaError::Decode)?)?;
                        let [b,g,r,_]:[u8;4]=frame.bgra[..4].try_into().map_err(|_|MediaError::Decode)?;
                        if b<15 && g<15 && r<15 { continue; }
                        let correct=if focus==0 {r>180 && b<80 && g>80} else {b>180 && r<80 && g>80};
                        if !correct { report.wrong_source_frames+=1; }
                        report.received_frames[focus]+=1;
                    }
                }
            }
            stream.close();
        }
        report.duration_ms=started.elapsed().as_millis() as u64;
        if report.received_frames.contains(&0) { return Err(MediaError::NoFrames); }
        Ok(report)
    }.await;
    let mut clean = true;
    for (publisher, publication) in publishers.iter().zip(&publications) {
        if !matches!(
            timeout(
                Duration::from_secs(5),
                publisher
                    .sdk_room()
                    .local_participant()
                    .unpublish_track(&publication.sid())
            )
            .await,
            Ok(Ok(_))
        ) {
            clean = false;
        }
    }
    let report = result?;
    if !clean {
        return Err(MediaError::Close);
    }
    Ok(report)
}
