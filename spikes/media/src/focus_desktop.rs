//! User-started combined microphone/video focus lab. Never starts capture on launch.
use gpui::{prelude::*, *};
use media_spike::{
    capture::frame_channel,
    loopback::PreviewTarget,
    switching::{SwitchingReport, run_microphone_focus_preview},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};
struct FocusLab {
    status: String,
    generation: Arc<AtomicU64>,
    result: Option<Receiver<(Result<SwitchingReport, String>, bool)>>,
}
impl FocusLab {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(50)).await;
                if this.update(cx, |this, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            status: "Ready. Microphone off.".into(),
            generation: Arc::new(AtomicU64::new(0)),
            result: None,
        }
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.result.is_some() {
            return;
        }
        let [Ok(url), Ok(first), Ok(second), Ok(viewer)] = [
            "LIVEKIT_URL",
            "LIVEKIT_PUBLISH_TOKEN",
            "LIVEKIT_SECOND_PUBLISH_TOKEN",
            "LIVEKIT_TOKEN",
        ]
        .map(std::env::var) else {
            self.status = "Relaunch using run-focus-lab.sh for fresh local credentials.".into();
            cx.notify();
            return;
        };
        let session = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let generation = self.generation.clone();
        let (frames, _) = frame_channel();
        let target = PreviewTarget {
            frames,
            generation,
            session,
        };
        let (send, receive) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"Media worker unavailable.".to_string()).and_then(|r|r.block_on(run_microphone_focus_preview(&url,&first,&second,&viewer,target)).map_err(|e|format!("Focus test stopped: {e}. Microphone off. Check permission and relaunch for fresh credentials.")));
            let saved = if let Ok(report) = &result {
                std::env::current_exe()
                    .ok()
                    .map(|exe| {
                        media_spike::ui_metrics::measurement_path(&exe)
                            .with_file_name("Whisk Focus Lab-focus-metrics.json")
                    })
                    .and_then(|path| {
                        serde_json::to_vec_pretty(report)
                            .ok()
                            .map(|bytes| (path, bytes))
                    })
                    .is_some_and(|(path, bytes)| std::fs::write(path, bytes).is_ok())
            } else {
                false
            };
            let _ = send.send((result, saved));
        });
        self.result = Some(receive);
        self.status="Joining the local fixture, then starting the microphone. Allow access if asked. Speak throughout both five-second focus phases. Capture stops within 15 seconds after device setup begins; speaker playback stays off.".into();
        cx.notify();
    }
    fn stop(&mut self, cx: &mut Context<Self>) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        if self.result.is_some() {
            self.status = "Stopping microphone and closing test rooms…".into();
            cx.notify();
        }
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        match self.result.as_ref().map(|rx| rx.try_recv()) {
            Some(Ok((result, saved))) => {
                self.result = None;
                self.status = match result {
                    Ok(r) => format!(
                        "Microphone off. Video frames by focus: {:?}. Decoded audio: {:?}; non-silent: {:?}. Largest PCM receive gap: {} ms (must be <250 ms). Continuity: {}. Physical input proof: {}. {} Quality is not certified.",
                        r.received_frames,
                        r.audio_received_frames,
                        r.audible_audio_frames,
                        r.max_audio_gap_ms,
                        r.audio_verified,
                        r.microphone_verified,
                        if saved {
                            "Counters saved beside app."
                        } else {
                            "Report save failed."
                        }
                    ),
                    Err(e) => e,
                };
                cx.notify();
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.result = None;
                self.status =
                    "Media worker ended unexpectedly. Relaunch the lab before retrying.".into();
                cx.notify();
            }
            _ => {}
        }
    }
}
impl Drop for FocusLab {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}
impl Render for FocusLab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0x1d222a)).text_color(rgb(0xe9edf2)).font_family(".AppleSystemUIFont").p_8().flex().flex_col().gap_5()
        .child(div().text_2xl().child("Whisk — Focus Lab"))
        .child(div().child(if self.result.is_some(){"MICROPHONE TEST STARTING / ACTIVE"}else{"MICROPHONE OFF"}))
        .child(div().text_sm().child("Test microphone continuity while the viewer switches between two generated 720p30 video publishers. Speak throughout both phases. Audio is decoded into counters only; no recordings are saved. Speaker playback is off."))
        .child(div().flex().gap_3()
            .child(div().id("start-focus").p_3().rounded_md().bg(rgb(0xf8b869)).text_color(rgb(0x25201a)).cursor_pointer().child("Start microphone + focus test").on_click(cx.listener(|this,_,_,cx|this.start(cx))))
            .child(div().id("stop-focus").p_3().rounded_md().bg(rgb(0x303945)).cursor_pointer().child("Stop").on_click(cx.listener(|this,_,_,cx|this.stop(cx)))))
        .child(div().text_sm().child(self.status.clone()))
    }
}
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(820.), px(540.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Whisk — Focus Lab".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(FocusLab::new),
        )
        .expect("open focus lab");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
