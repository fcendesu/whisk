//! Generated audio runs independently of the CPU video conversion/viewer loop.
use crate::{benchmark::AudioContinuity, loopback::MediaError};
use futures_util::StreamExt;
use livekit::{
    prelude::RemoteAudioTrack,
    webrtc::{
        audio_frame::AudioFrame, audio_source::native::NativeAudioSource,
        audio_stream::native::NativeAudioStream,
    },
};
use std::time::Duration;
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
    time::{Instant, MissedTickBehavior, interval, timeout},
};

pub(crate) struct ToneReport {
    pub frames: [u64; 2],
    pub max_gap_ms: u64,
    pub frequency_hz: f64,
    pub verified: bool,
    pub source_calls: u64,
    pub source_max_interval_ms: u64,
    pub source_max_wait_ms: u64,
    pub received_frames: [u64; 2],
    pub receive_max_interval_ms: u64,
    pub observation_ms: u64,
}
pub(crate) struct AudioTask {
    phase: watch::Sender<bool>,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<ToneReport, MediaError>>,
}
impl AudioTask {
    pub fn start(source: NativeAudioSource, track: RemoteAudioTrack, started: Instant) -> Self {
        let (phase, phase_rx) = watch::channel(false);
        let (stop, stop_rx) = oneshot::channel();
        let task = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| MediaError::Decode)?;
            runtime.block_on(monitor(source, track, started, phase_rx, stop_rx))
        });
        Self {
            phase,
            stop: Some(stop),
            task,
        }
    }
    pub fn focus(&self, second: bool) {
        self.phase.send_replace(second);
    }
    pub async fn stopped_unexpectedly(&mut self) -> MediaError {
        match (&mut self.task).await {
            Ok(Err(error)) => error,
            _ => MediaError::Decode,
        }
    }
    pub async fn finish(mut self) -> Result<ToneReport, MediaError> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        timeout(Duration::from_secs(2), &mut self.task)
            .await
            .map_err(|_| MediaError::Decode)?
            .map_err(|_| MediaError::Decode)?
    }
}
impl Drop for AudioTask {
    fn drop(&mut self) {
        // spawn_blocking cannot be aborted once started. Closing this explicit
        // signal ends receive monitoring and stops the source between submissions.
        // Async source completion is bounded; a synchronous SDK call cannot be preempted.
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
async fn monitor(
    source: NativeAudioSource,
    track: RemoteAudioTrack,
    started: Instant,
    phase: watch::Receiver<bool>,
    stop: oneshot::Receiver<()>,
) -> Result<ToneReport, MediaError> {
    let mut stream = NativeAudioStream::new(track.rtc_track(), 48_000, 1);
    let result = monitor_io(
        move |frame| {
            let source = source.clone();
            async move {
                source
                    .capture_frame(&frame)
                    .await
                    .map_err(|_| MediaError::Publish)
            }
        },
        &mut stream,
        started,
        phase,
        stop,
    )
    .await;
    stream.close();
    result
}

#[derive(Default)]
struct FeedReport {
    calls: u64,
    max_interval_ms: u64,
    max_wait_ms: u64,
}

struct FeedTask {
    stop: watch::Sender<bool>,
    task: JoinHandle<Result<FeedReport, MediaError>>,
}
impl Drop for FeedTask {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

async fn feed_audio<C, F>(
    mut capture: C,
    started: Instant,
    mut stop: watch::Receiver<bool>,
) -> Result<FeedReport, MediaError>
where
    C: FnMut(AudioFrame<'static>) -> F,
    F: std::future::Future<Output = Result<(), MediaError>>,
{
    let mut ticks = interval(Duration::from_millis(20));
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut generated = 0u64;
    let mut report = FeedReport::default();
    let mut last_ms = 0;
    loop {
        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() { break; }
            }
            _ = ticks.tick() => {
                if *stop.borrow() { break; }
                let samples: Vec<i16> = (0..960).map(|offset| (8000.0*(std::f64::consts::TAU*440.0*(generated+offset) as f64/48_000.0).sin()) as i16).collect();
                generated += 960;
                let frame = AudioFrame {data:samples.into(), sample_rate:48_000, num_channels:1, samples_per_channel:960};
                let call_start = Instant::now();
                let now = started.elapsed().as_millis() as u64;
                report.max_interval_ms = report.max_interval_ms.max(now.saturating_sub(last_ms));
                last_ms = now;
                // Allow the in-flight SDK completion to finish; cancelling it can
                // strand its native callback. The async wait remains bounded.
                timeout(Duration::from_millis(500),capture(frame)).await.map_err(|_|MediaError::Publish)?.map_err(|_|MediaError::Publish)?;
                report.calls += 1;
                report.max_wait_ms = report.max_wait_ms.max(call_start.elapsed().as_millis() as u64);
            }
        }
    }
    Ok(report)
}

async fn monitor_io<C, F, S>(
    capture: C,
    stream: &mut S,
    started: Instant,
    phase: watch::Receiver<bool>,
    mut stop: oneshot::Receiver<()>,
) -> Result<ToneReport, MediaError>
where
    C: FnMut(AudioFrame<'static>) -> F + Send + 'static,
    F: std::future::Future<Output = Result<(), MediaError>> + Send,
    S: futures_util::Stream<Item = AudioFrame<'static>> + Unpin,
{
    let (feed_stop, feed_stopped) = watch::channel(false);
    let task = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| MediaError::Publish)?;
        runtime.block_on(feed_audio(capture, started, feed_stopped))
    });
    let mut feed = FeedTask {
        stop: feed_stop,
        task,
    };
    let mut completed = None;
    let mut measured = 0u64;
    let mut crossings = 0u64;
    let mut previous = 0i16;
    let mut continuity = AudioContinuity::default();
    let mut activity = AudioContinuity::default();
    let outcome = loop {
        tokio::select! {
            _ = &mut stop => break Ok(()),
            result = &mut feed.task => {
                completed = Some(result);
                break Err(MediaError::Publish);
            }
            frame = stream.next() => {
                let Some(frame) = frame else { break Err(MediaError::Decode) };
                if frame.sample_rate != 48_000 || frame.num_channels != 1 { break Err(MediaError::Decode); }
                let second = *phase.borrow();
                let elapsed = started.elapsed().as_millis() as u64;
                activity.observe(second, elapsed);
                if frame.data.iter().any(|sample| sample.unsigned_abs()>500) {
                    continuity.observe(second, elapsed);
                    for sample in frame.data.iter().copied() {
                        if (sample>=0)!=(previous>=0) {crossings+=1;}
                        previous=sample;
                    }
                    measured += frame.data.len() as u64;
                }
            }
        }
    };
    // Observation ends when receive monitoring stops, before source cleanup.
    let observation_ms = started.elapsed().as_millis() as u64;
    feed.stop.send_replace(true);
    let source = match completed {
        Some(result) => result,
        None => timeout(Duration::from_secs(2), &mut feed.task)
            .await
            .map_err(|_| MediaError::Publish)?,
    };
    outcome?;
    let source = source.map_err(|_| MediaError::Publish)??;
    let continuity = continuity.finish(observation_ms);
    let activity = activity.finish(observation_ms);
    let frequency_hz = if measured > 0 {
        crossings as f64 * 48_000.0 / (2.0 * measured as f64)
    } else {
        0.0
    };
    Ok(ToneReport {
        source_calls: source.calls,
        source_max_interval_ms: source.max_interval_ms,
        source_max_wait_ms: source.max_wait_ms,
        received_frames: activity.audible_frames,
        receive_max_interval_ms: activity.max_gap_ms,
        observation_ms,
        frames: continuity.audible_frames,
        max_gap_ms: continuity.max_gap_ms,
        frequency_hz,
        verified: continuity.continuous && (420.0..460.0).contains(&frequency_hz),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    async fn stalled_source_does_not_stall_receive(blocking: bool) {
        let (phase, phase_rx) = watch::channel(false);
        let (stop, stop_rx) = oneshot::channel();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            phase.send_replace(true);
            tokio::time::sleep(Duration::from_millis(350)).await;
            let _ = stop.send(());
        });
        let mut received = Box::pin(futures_util::stream::unfold(
            (interval(Duration::from_millis(10)), 0u64),
            |(mut ticks, sample)| async move {
                ticks.tick().await;
                let samples: Vec<i16> = (sample..sample + 480)
                    .map(|n| if n % 109 < 55 { 8000 } else { -8000 })
                    .collect();
                Some((
                    AudioFrame {
                        data: samples.into(),
                        sample_rate: 48_000,
                        num_channels: 1,
                        samples_per_channel: 480,
                    },
                    (ticks, sample + 480),
                ))
            },
        ));
        let calls = Arc::new(AtomicUsize::new(0));
        let call_count = calls.clone();
        let report = monitor_io(
            move |_frame| {
                call_count.fetch_add(1, Ordering::SeqCst);
                async move {
                    if blocking {
                        std::thread::sleep(Duration::from_millis(400));
                    } else {
                        tokio::time::sleep(Duration::from_millis(400)).await;
                    }
                    Ok(())
                }
            },
            &mut received,
            Instant::now(),
            phase_rx,
            stop_rx,
        )
        .await
        .unwrap();
        assert!(
            calls.load(Ordering::SeqCst) > 0,
            "source stall was never exercised"
        );
        assert!(
            report.max_gap_ms < 250,
            "source stall prevented receive polling: {}ms",
            report.max_gap_ms
        );
        assert!(
            report.frames.iter().all(|n| *n >= 10),
            "receive work lost a focus phase: {:?}",
            report.frames
        );
        assert!(report.verified);
        assert!(
            report.source_max_wait_ms >= 390,
            "source backpressure was not measured"
        );
        assert!(
            report.observation_ms < 650,
            "source cleanup leaked into receive timing"
        );
    }

    #[tokio::test]
    async fn source_backpressure_does_not_block_audio_receive() {
        stalled_source_does_not_stall_receive(false).await;
    }
    #[tokio::test]
    async fn synchronous_source_call_does_not_block_audio_receive() {
        stalled_source_does_not_stall_receive(true).await;
    }
    #[tokio::test]
    async fn receive_end_stops_and_joins_the_source_worker() {
        let started = Arc::new(tokio::sync::Notify::new());
        let input_started = started.clone();
        let mut input = Box::pin(futures_util::stream::unfold((), move |_| {
            let notify = input_started.clone();
            async move {
                notify.notified().await;
                None::<(AudioFrame<'static>, ())>
            }
        }));
        let calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = calls.clone();
        let (_phase, phase_rx) = watch::channel(false);
        let (_stop, stop_rx) = oneshot::channel();
        let result = monitor_io(
            move |_| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                started.notify_one();
                async {
                    tokio::time::sleep(Duration::from_millis(80)).await;
                    Ok(())
                }
            },
            &mut input,
            Instant::now(),
            phase_rx,
            stop_rx,
        )
        .await;
        assert!(matches!(result, Err(MediaError::Decode)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "source continued after receive failure"
        );
    }

    #[tokio::test]
    async fn source_failure_is_propagated_instead_of_certifying_receive_only_audio() {
        let mut input = futures_util::stream::pending::<AudioFrame<'static>>();
        let (_phase, phase_rx) = watch::channel(false);
        let (_stop, stop_rx) = oneshot::channel();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            monitor_io(
                |_| async { Err(MediaError::Publish) },
                &mut input,
                Instant::now(),
                phase_rx,
                stop_rx,
            ),
        )
        .await
        .expect("source failure must stop receive monitoring");
        assert!(matches!(result, Err(MediaError::Publish)));
    }
}
