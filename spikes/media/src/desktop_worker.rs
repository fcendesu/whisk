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
    Stop,
}
pub enum Event {
    Sources(u64, Result<Vec<CaptureSource>, String>),
    Started(u64, Result<watch::Receiver<FrameEvent>, String>),
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
                        Command::Stop => {
                            if let Some((_, old)) = stream.take() {
                                let _ = old.stop();
                            }
                        }
                    }
                }
                if let Some((_, old)) = stream {
                    let _ = old.stop();
                }
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
