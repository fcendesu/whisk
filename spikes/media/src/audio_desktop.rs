//! Explicit, bounded microphone test. Startup never opens an audio device.
use gpui::{prelude::*, *};
use media_spike::microphone::{AudioInput, AudioReport, run_audio_probe};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

struct AudioLab {
    status: String,
    result: Option<Receiver<(Result<AudioReport, String>, bool)>>,
    cancelled: Option<Arc<AtomicBool>>,
}
impl AudioLab {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(50)).await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            status: "Ready. Microphone is off.".into(),
            result: None,
            cancelled: None,
        }
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.result.is_some() {
            return;
        }
        let config = ["LIVEKIT_URL", "LIVEKIT_PUBLISH_TOKEN", "LIVEKIT_TOKEN"].map(std::env::var);
        let [Ok(url), Ok(publisher), Ok(viewer)] = config else {
            self.status =
                "Local test credentials are missing. Launch with scripts/run-audio-lab.sh.".into();
            cx.notify();
            return;
        };
        if url::Url::parse(&url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .as_deref()
            != Some("127.0.0.1")
        {
            self.status =
                "The Audio Lab only sends microphone audio to the local loopback fixture.".into();
            cx.notify();
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let (send, receive) = mpsc::sync_channel(1);
        let path = std::env::current_exe()
            .ok()
            .map(|exe| media_spike::ui_metrics::measurement_path(&exe))
            .map(|path| path.with_file_name("Whisk Audio Lab-audio-metrics.json"));
        std::thread::spawn(move || {
            let result=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"Audio worker unavailable.".to_string()).and_then(|runtime| {
    runtime.block_on(run_audio_probe(&url,&publisher,&viewer,Duration::from_secs(10),AudioInput::Microphone,stop)).map_err(|error|format!("Microphone test stopped: {error}. Check microphone access and relaunch for fresh test credentials."))
   });
            let saved = if let (Ok(report), Some(path)) = (&result, path) {
                serde_json::to_vec_pretty(report)
                    .ok()
                    .is_some_and(|bytes| std::fs::write(path, bytes).is_ok())
            } else {
                false
            };
            let _ = send.send((result, saved));
        });
        self.cancelled = Some(cancelled);
        self.result = Some(receive);
        self.status="Starting the local microphone test. Allow microphone access if macOS asks, then speak. Recording stops automatically.".into();
        cx.notify();
    }
    fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(cancelled) = &self.cancelled {
            cancelled.store(true, Ordering::Release);
            self.status = "Stopping microphone and closing test connections…".into();
            cx.notify();
        }
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        let result = self.result.as_ref().map(|receive| receive.try_recv());
        match result {
            Some(Ok((result, saved))) => {
                self.result = None;
                self.cancelled = None;
                self.status = match result {
                    Ok(report) => format!(
                        "Microphone is off. {} decoded frames; {} frames with input above the detection threshold. {} {}",
                        report.decoded_frames,
                        report.non_silent_frames,
                        if report.microphone_verified {
                            "Microphone publish/decode verified."
                        } else {
                            "No audible microphone signal verified; check the selected system input and try speaking."
                        },
                        if saved {
                            "Counter report saved beside the app."
                        } else {
                            "Counter report could not be saved."
                        }
                    ),
                    Err(error) => error,
                };
                cx.notify();
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.result = None;
                self.cancelled = None;
                self.status = "Audio worker stopped unexpectedly. Relaunch the lab.".into();
                cx.notify();
            }
            _ => {}
        }
    }
}
impl Drop for AudioLab {
    fn drop(&mut self) {
        if let Some(cancelled) = &self.cancelled {
            cancelled.store(true, Ordering::Release);
        }
    }
}
impl Render for AudioLab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0x1d222a)).text_color(rgb(0xe9edf2)).font_family(".AppleSystemUIFont").p_8().flex().flex_col().gap_5()
   .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Whisk — Audio Lab"))
   .child(div().text_sm().text_color(rgb(0xa4cfaa)).child(if self.result.is_some(){"MICROPHONE TEST ACTIVE / STARTING"}else{"MICROPHONE OFF"}))
   .child(div().text_lg().child("Test your microphone through the local media server"))
   .child(div().text_sm().text_color(rgb(0xb1bac7)).child("Press Start to capture for ten seconds. Speak normally during the test. Audio goes to the local SFU and is decoded back into counters. Speaker playback is off; audio recordings are never saved."))
   .child(div().flex().gap_3()
    .child(div().id("start-microphone").rounded_md().p_3().bg(rgb(0xf8b869)).text_color(rgb(0x25201a)).cursor_pointer().child("Start 10-second microphone test").on_click(cx.listener(|this,_,_,cx|this.start(cx))))
    .child(div().id("stop-microphone").rounded_md().p_3().bg(rgb(0x303945)).cursor_pointer().child("Stop").on_click(cx.listener(|this,_,_,cx|this.stop(cx)))))
   .child(div().flex_1().text_sm().child(self.status.clone()))
   .child(div().text_xs().text_color(rgb(0x8e98a7)).child("M0 feasibility probe · Counter reports contain no audio samples, device names or credentials. Physical speaker playback and audio quality are not certified."))
    }
}
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(780.), px(480.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Whisk — Audio Lab".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(AudioLab::new),
        )
        .expect("open Audio Lab");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
