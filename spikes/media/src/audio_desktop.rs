// Shared native audio UI. The compiled binary selects microphone or speaker mode;
// an environment variable cannot turn the Speaker Lab into a microphone test.
use gpui::{prelude::*, *};
use media_spike::microphone::{AudioInput, AudioReport, run_audio_probe};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};
fn speaker_mode() -> bool {
    env!("CARGO_BIN_NAME") == "whisk-speaker-spike"
}
fn title() -> &'static str {
    if speaker_mode() {
        "Whisk — Speaker Lab"
    } else {
        "Whisk — Audio Lab"
    }
}
fn report_path() -> Option<PathBuf> {
    std::env::current_exe().ok().map(|exe| {
        media_spike::ui_metrics::measurement_path(&exe).with_file_name(if speaker_mode() {
            "Whisk Speaker Lab-audio-metrics.json"
        } else {
            "Whisk Audio Lab-audio-metrics.json"
        })
    })
}
struct AudioLab {
    status: String,
    result: Option<Receiver<(Result<AudioReport, String>, bool)>>,
    cancelled: Option<Arc<AtomicBool>>,
    last_report: Option<AudioReport>,
    save_ack: Option<Receiver<bool>>,
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
            status: if speaker_mode() {
                "Ready. Speakers idle; microphone off."
            } else {
                "Ready. Microphone is off."
            }
            .into(),
            result: None,
            cancelled: None,
            last_report: None,
            save_ack: None,
        }
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.result.is_some() || self.save_ack.is_some() {
            return;
        }
        let config = ["LIVEKIT_URL", "LIVEKIT_PUBLISH_TOKEN", "LIVEKIT_TOKEN"].map(std::env::var);
        let [Ok(url), Ok(publisher), Ok(viewer)] = config else {
            self.status =
                "Local test credentials are missing. Relaunch using the appropriate lab script."
                    .into();
            cx.notify();
            return;
        };
        if url::Url::parse(&url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .as_deref()
            != Some("127.0.0.1")
        {
            self.status = "This lab only uses the local loopback media fixture.".into();
            cx.notify();
            return;
        }
        self.last_report = None;
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let (send, receive) = mpsc::sync_channel(1);
        let path = report_path();
        std::thread::spawn(move || {
            let result=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"Audio worker unavailable.".to_string()).and_then(|runtime|runtime.block_on(run_audio_probe(&url,&publisher,&viewer,Duration::from_secs(if speaker_mode(){3}else{10}),if speaker_mode(){AudioInput::SpeakerTest}else{AudioInput::Microphone},stop)).map_err(|error|format!("Audio test stopped: {error}. Check the system device and relaunch for fresh fixture credentials.")));
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
        self.status=if speaker_mode(){"Starting a quiet 440 Hz tone through the local SFU. Playback lasts three seconds. Microphone remains off."}else{"Starting the local microphone test. Allow microphone access if macOS asks, then speak. Recording stops automatically."}.into();
        cx.notify();
    }
    fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(cancelled) = &self.cancelled {
            cancelled.store(true, Ordering::Release);
            self.status = "Stopping audio and closing test connections…".into();
            cx.notify();
        }
    }
    fn confirm(&mut self, heard: bool, cx: &mut Context<Self>) {
        if self.result.is_some() || self.save_ack.is_some() {
            return;
        }
        let Some(report) = self.last_report.as_mut() else {
            return;
        };
        if report.confirm_playback(heard).is_err() {
            return;
        }
        let bytes = serde_json::to_vec_pretty(report).ok();
        let path = report_path();
        let (send, receive) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let saved = match (bytes, path) {
                (Some(bytes), Some(path)) => std::fs::write(path, bytes).is_ok(),
                _ => false,
            };
            let _ = send.send(saved);
        });
        self.save_ack = Some(receive);
        self.status = "Saving your playback feedback…".into();
        cx.notify();
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        if let Some(ack) = self.save_ack.as_ref().map(|receive| receive.try_recv()) {
            match ack {
                Ok(saved) => {
                    self.save_ack = None;
                    let heard = self
                        .last_report
                        .as_ref()
                        .and_then(|report| report.user_heard_tone)
                        == Some(true);
                    self.status = format!(
                        "{} {}",
                        if heard {
                            "You confirmed hearing the tone. Speaker playback verified; microphone remained off."
                        } else {
                            "You reported no sound. Playback is not verified; check the selected output device and volume."
                        },
                        if saved {
                            "Feedback report saved beside the app."
                        } else {
                            "Feedback report could not be saved."
                        }
                    );
                    cx.notify();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.save_ack = None;
                    self.status = "Feedback could not be saved.".into();
                    cx.notify();
                }
                _ => {}
            }
        }
        match self.result.as_ref().map(|receive| receive.try_recv()) {
            Some(Ok((result, saved))) => {
                self.result = None;
                self.cancelled = None;
                self.status = match result {
                    Ok(report) => {
                        let status = if speaker_mode() {
                            format!(
                                "Tone stopped; microphone remained off. {} decoded frames. Did you hear the tone? {}",
                                report.decoded_frames,
                                if saved {
                                    "Counter report saved; playback awaits your feedback."
                                } else {
                                    "Counter report could not be saved."
                                }
                            )
                        } else {
                            format!(
                                "Microphone is off. {} decoded frames; {} frames above the input threshold. {} {}",
                                report.decoded_frames,
                                report.non_silent_frames,
                                if report.microphone_verified {
                                    "Microphone publish/decode verified."
                                } else {
                                    "No audible microphone input verified."
                                },
                                if saved {
                                    "Counter report saved beside the app."
                                } else {
                                    "Counter report could not be saved."
                                }
                            )
                        };
                        self.last_report = Some(report);
                        status
                    }
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
        let feedback = speaker_mode()
            && self.result.is_none()
            && self.last_report.as_ref().is_some_and(|report| {
                report.playback_started
                    && report.non_silent_frames > 0
                    && report.user_heard_tone.is_none()
            });
        div().size_full().bg(rgb(0x1d222a)).text_color(rgb(0xe9edf2)).font_family(".AppleSystemUIFont").p_8().flex().flex_col().gap_5()
   .child(div().text_2xl().font_weight(FontWeight::BOLD).child(title()))
   .child(div().text_sm().text_color(rgb(0xa4cfaa)).child(if speaker_mode(){"MICROPHONE OFF · SPEAKER TEST"}else if self.result.is_some(){"MICROPHONE TEST ACTIVE / STARTING"}else{"MICROPHONE OFF"}))
   .child(div().text_lg().child(if speaker_mode(){"Check speaker playback through the local media server"}else{"Test your microphone through the local media server"}))
   .child(div().text_sm().text_color(rgb(0xb1bac7)).child(if speaker_mode(){"Set a comfortable system volume, then play a quiet three-second test tone. Generated audio goes through the local SFU to your selected system output. This test does not capture microphone audio."}else{"Press Start to capture for ten seconds. Speak normally during the test. Audio goes to the local SFU and is decoded back into counters. Speaker playback is off; audio recordings are never saved."}))
   .child(div().flex().gap_3()
    .child(div().id("start-audio").rounded_md().p_3().bg(rgb(0xf8b869)).text_color(rgb(0x25201a)).cursor_pointer().child(if speaker_mode(){"Play quiet 3-second tone"}else{"Start 10-second microphone test"}).on_click(cx.listener(|this,_,_,cx|this.start(cx))))
    .child(div().id("stop-audio").rounded_md().p_3().bg(rgb(0x303945)).cursor_pointer().child("Stop").on_click(cx.listener(|this,_,_,cx|this.stop(cx)))))
   .when(feedback,|view|view.child(div().flex().gap_3()
    .child(div().id("heard-tone").rounded_md().p_3().bg(rgb(0x48513e)).cursor_pointer().child("I heard the tone").on_click(cx.listener(|this,_,_,cx|this.confirm(true,cx))))
    .child(div().id("no-sound").rounded_md().p_3().bg(rgb(0x303945)).cursor_pointer().child("No sound").on_click(cx.listener(|this,_,_,cx|this.confirm(false,cx))))))
   .child(div().flex_1().text_sm().child(self.status.clone()))
   .child(div().text_xs().text_color(rgb(0x8e98a7)).child("M0 feasibility probe · Counter reports contain no audio samples, device names or credentials. Playback verification requires your explicit feedback; audio quality is not certified."))
    }
}
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(780.), px(560.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(title().into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(AudioLab::new),
        )
        .expect("open audio lab");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
