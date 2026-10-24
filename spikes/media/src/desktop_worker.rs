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
    Loopback(u64),
    Stop,
}
pub enum Event {
    Sources(u64, Result<Vec<CaptureSource>, String>),
    Started(u64, Result<watch::Receiver<FrameEvent>, String>),
    NetworkStarted(u64, watch::Receiver<FrameEvent>),
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
                let mut network: Option<std::thread::JoinHandle<()>> = None;
                loop {
                    if stream
                        .as_ref()
                        .is_some_and(|(id, _)| *id != active.load(Ordering::Acquire))
                        && let Some((_, old)) = stream.take()
                    {
                        let _ = old.stop();
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
                        Command::Loopback(id) => {
                            if let Some(old) = network.take() { let _ = old.join(); }
                            if active.load(Ordering::Acquire) != id { continue; }
                            let config = ["LIVEKIT_URL", "LIVEKIT_PUBLISH_TOKEN", "LIVEKIT_TOKEN"].map(std::env::var);
                            let [Ok(url), Ok(publisher), Ok(viewer)] = config else {
                                if output.send(Event::NetworkFinished(id, Err("Start the local SFU fixture and launch with LIVEKIT_URL, LIVEKIT_PUBLISH_TOKEN and LIVEKIT_TOKEN to run the network test.".into()))).is_err() { break; }
                                continue;
                            };
                            let (frames, receiver) = capture::frame_channel();
                            if output.send(Event::NetworkStarted(id, receiver)).is_err() { break; }
                            let generation = active.clone();
                            network = Some(std::thread::spawn(move || {
                                let completion = frames.clone();
                                let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build();
                                let result = match runtime {
                                    Ok(runtime) => runtime.block_on(media_spike::loopback::run_loopback_preview(&url, &publisher, &viewer, Duration::from_secs(30), media_spike::loopback::PreviewTarget { frames, generation, session:id })).map(|report| format!("Network test complete: {} decoded frames · {} · encoder {} / decoder {}. No captured screen was sent.", report.decoded_frames, report.codec.as_deref().unwrap_or("unknown codec"), report.encoder.as_deref().unwrap_or("unknown"), report.decoder.as_deref().unwrap_or("unknown"))).map_err(|error| format!("Network test stopped: {error}")),
                                    Err(_) => Err("Unable to start the native media runtime.".into()),
                                };
                                let message = match result { Ok(message) | Err(message) => message };
                                completion.send(FrameEvent::Finished(message));
                            }));
                        }
                        Command::Stop => {
                            if let Some((_, old)) = stream.take() {
                                let _ = old.stop();
                            }
                        }
                    }
                }
                if let Some((_, old)) = stream { let _ = old.stop(); }
                if let Some(old) = network { let _ = old.join(); }
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
