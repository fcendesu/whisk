use media_spike::capture::{self, CaptureSource, FrameEvent, FrameStream, SourceId};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
};
use std::time::Duration;
use tokio::sync::watch;

pub enum Command {
    Sources(u64),
    Start(u64, SourceId),
    Loopback(u64, Option<SourceId>),
    SaveMetrics(std::path::PathBuf, media_spike::ui_metrics::UiFrameReport),
    Stop,
}
pub enum Event {
    MetricsSaved(bool),
    Sources(u64, Result<Vec<CaptureSource>, String>),
    Started(u64, Result<watch::Receiver<FrameEvent>, String>),
    NetworkStarted(u64, watch::Receiver<FrameEvent>, bool),
    NetworkFinished(u64, Result<String, String>),
}
pub struct Worker {
    pub commands: SyncSender<Command>,
    pub events: Receiver<Event>,
    pub generation: Arc<AtomicU64>,
}
impl Worker {
    pub fn new() -> Self {
        let (commands, input) = mpsc::sync_channel(4);
        let (output, events) = mpsc::sync_channel(4);
        let generation = Arc::new(AtomicU64::new(0));
        let active = generation.clone();
        std::thread::Builder::new()
            .name("whisk-native-capture".into())
            .spawn(move || {
                let mut stream: Option<(u64, FrameStream)> = None;
                let mut network: Option<(u64, std::thread::JoinHandle<()>)> = None;
                loop {
                    if stream
                        .as_ref()
                        .is_some_and(|(id, _)| *id != active.load(Ordering::Acquire))
                        && let Some((_, old)) = stream.take()
                    {
                        let _ = old.stop();
                    }
                    if network.as_ref().is_some_and(|(_, task)| task.is_finished())
                        && let Some((id, old)) = network.take() {
                            let _ = old.join();
                            if stream.as_ref().is_some_and(|(stream_id, _)| *stream_id == id)
                                && let Some((_, old)) = stream.take() { let _ = old.stop(); }
                    }
                    let command = match input.recv_timeout(Duration::from_millis(50)) {
                        Ok(command) => command,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    };
                    match command {
                        Command::Sources(id) => {
                            let result =
                                capture::available_sources().map_err(|error| error.to_string());
                            if output.send(Event::Sources(id, result)).is_err() {
                                break;
                            }
                        }
                        Command::Start(id, source) => {
                            if let Some((_, old)) = stream.take() {
                                let _ = old.stop();
                            }
                            if active.load(Ordering::Acquire) != id {
                                continue;
                            }
                            let result = capture::open_source(source);
                            let receiver = match result {
                                Ok(new) => {
                                    if active.load(Ordering::Acquire) != id {
                                        let _ = new.stop();
                                        continue;
                                    }
                                    let frames = new.frames.clone();
                                    stream = Some((id, new));
                                    Ok(frames)
                                }
                                Err(error) => Err(error.to_string()),
                            };
                            if output.send(Event::Started(id, receiver)).is_err() {
                                break;
                            }
                        }
                        Command::Loopback(id, source) => {
                            if let Some((_, old)) = network.take() { let _ = old.join(); }
                            if active.load(Ordering::Acquire) != id { continue; }
                            let config = ["LIVEKIT_URL", "LIVEKIT_PUBLISH_TOKEN", "LIVEKIT_TOKEN"].map(std::env::var);
                            let [Ok(url), Ok(publisher), Ok(viewer)] = config else {
                                if output.send(Event::NetworkFinished(id, Err("Start the local SFU fixture and launch with LIVEKIT_URL, LIVEKIT_PUBLISH_TOKEN and LIVEKIT_TOKEN to run the network test.".into()))).is_err() { break; }
                                continue;
                            };
                            let input = if let Some(source) = source {
                                if let Some((_, old)) = stream.take() { let _ = old.stop(); }
                                match capture::open_source(source) {
                                    Ok(new) => {
                                        if active.load(Ordering::Acquire) != id { let _ = new.stop(); continue; }
                                        let frames = new.frames.clone();
                                        stream = Some((id, new));
                                        Some(frames)
                                    }
                                    Err(error) => {
                                        if output.send(Event::NetworkFinished(id, Err(error.to_string()))).is_err() { break; }
                                        continue;
                                    }
                                }
                            } else { None };
                            let captured = input.is_some();
                            let (frames, receiver) = capture::frame_channel();
                            if output.send(Event::NetworkStarted(id, receiver, captured)).is_err() { break; }
                            let generation = active.clone();
                            network = Some((id, std::thread::spawn(move || {
                                let completion = frames.clone();
                                let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build();
                                let result = match runtime {
                                    Ok(runtime) => runtime.block_on(async {
                                        let target = media_spike::loopback::PreviewTarget { frames, generation, session:id };
                                        if input.is_none() && let Ok(name)=std::env::var("WHISK_MEDIA_PROFILE") {
                                            let profile=match name.as_str(){
                                                "720p30"=>media_spike::codec::VideoProfile::P720F30,
                                                "1080p60"=>media_spike::codec::VideoProfile::P1080F60,
                                                "1440p60"=>media_spike::codec::VideoProfile::P1440F60,
                                                _=>return Err(media_spike::loopback::MediaError::InvalidInput),
                                            };
                                            let second=std::env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").map_err(|_|media_spike::loopback::MediaError::InvalidInput)?;
                                            let phase=std::env::var("WHISK_MEDIA_PHASE_SECONDS").unwrap_or_else(|_|"15".into()).parse::<u64>().map_err(|_|media_spike::loopback::MediaError::InvalidInput)?;
                                            let report=media_spike::switching::run_profile_preview(&url,&publisher,&second,&viewer,Duration::from_secs(phase),profile,target).await?;
                                            save_report(&report);
                                            return Ok(format!("Experimental {name} complete: {} / {} decoded frames across two focus phases. Generated audio continuity: {}. Not quality certified.",report.received_frames[0],report.received_frames[1],report.audio_verified));
                                        }
                                        let report = match input {
                                            Some(input) => media_spike::loopback::run_capture_preview(&url, &publisher, &viewer, Duration::from_secs(30), input, target).await,
                                            None => media_spike::loopback::run_loopback_preview(&url, &publisher, &viewer, Duration::from_secs(30), target).await,
                                        }?;
                                        save_report(&report);
                                        Ok::<_, media_spike::loopback::MediaError>(format!("{} complete: {} decoded frames · {} · encoder {} / decoder {}. Publishing stopped.", if captured { "Selected-window test" } else { "Generated-video test" }, report.decoded_frames, report.codec.as_deref().unwrap_or("unknown codec"), report.encoder.as_deref().unwrap_or("unknown"), report.decoder.as_deref().unwrap_or("unknown")))
                                    }).map_err(|error| format!("Network test stopped: {error}. Relaunch the lab to refresh its short-lived test credentials.")),
                                    Err(_) => Err("Unable to start the native media runtime.".into()),
                                };
                                let message = match result { Ok(message) | Err(message) => message };
                                completion.send(FrameEvent::Finished(message));
                            })));
                        }
                        Command::SaveMetrics(path, report) => {
                            let saved = serde_json::to_vec_pretty(&report).ok().is_some_and(|json| std::fs::write(path,json).is_ok());
                            if output.send(Event::MetricsSaved(saved)).is_err() { break; }
                        }
                        Command::Stop => {
                            if let Some((_, old)) = stream.take() {
                                let _ = old.stop();
                            }
                        }
                    }
                }
                if let Some((_, old)) = stream { let _ = old.stop(); }
                if let Some((_, old)) = network { let _ = old.join(); }
            })
            .expect("start native capture worker");
        Self {
            commands,
            events,
            generation,
        }
    }
    pub fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

// Counter/backend evidence only: no source labels, pixels, endpoint or tokens.
fn save_report(report: &impl serde::Serialize) {
    if let Ok(path) = std::env::var("WHISK_MEDIA_REPORT_PATH")
        && let Ok(json) = serde_json::to_vec_pretty(report)
    {
        let _ = std::fs::write(path, json);
    }
}
