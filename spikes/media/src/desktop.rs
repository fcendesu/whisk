//! M0 native local-preview shell, not the production messaging client.
mod desktop_worker;
use desktop_worker::{Command, Event, Worker};
use gpui::{img, prelude::*, *};
use media_spike::{
    capture::{CaptureSource, FrameEvent, SourceId, VideoFrame},
    render::{FrameQueue, current_frame, enqueue_latest},
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::watch;

struct MediaLab {
    icon: Arc<RenderImage>,
    worker: Worker,
    sources: Vec<CaptureSource>,
    frames: Option<watch::Receiver<FrameEvent>>,
    queue: FrameQueue,
    preview: Option<Arc<RenderImage>>,
    status: String,
    selected: Option<SourceId>,
}
impl MediaLab {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(33)).await;
                if this.update(cx, |view, cx| view.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let mut icon = image::load_from_memory_with_format(
            include_bytes!("../../../assets/whisk-128.png"),
            image::ImageFormat::Png,
        )
        .expect("provided Whisk icon")
        .to_rgba8();
        for pixel in icon.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        Self {
            icon: Arc::new(RenderImage::new(vec![image::Frame::new(icon)])),
            worker: Worker::new(),
            sources: Vec::new(),
            frames: None,
            queue: FrameQueue::default(),
            preview: None,
            status: "Ready. Discover a source or render the test pattern.".into(),
            selected: None,
        }
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Ok(event) = self.worker.events.try_recv() {
            let generation = self.worker.generation.load(Ordering::Acquire);
            match event {
                Event::Sources(id, result) if id == generation => match result {
                    Ok(sources) => {
                        self.status = format!(
                            "{} sources available. Select one for a local preview.",
                            sources.len()
                        );
                        self.sources = sources;
                    }
                    Err(error) => {
                        self.status = error;
                        self.sources.clear();
                    }
                },
                Event::Started(id, result) if id == generation => match result {
                    Ok(frames) => {
                        self.frames = Some(frames);
                        self.status =
                            "Local preview active · 720p30 requested · Not broadcasting".into();
                    }
                    Err(error) => {
                        self.status = error;
                        self.selected = None;
                    }
                },
                Event::NetworkStarted(id, frames, captured) if id == generation => {
                    self.frames = Some(frames);
                    self.status = if captured { "Publishing selected window → H.264 → configured SFU → native decode → GPUI. Stops after 30 seconds." } else { "Native H.264 → configured SFU → native decode → GPUI. Sending a generated pattern only; this test lasts 30 seconds." }.into();
                }
                Event::NetworkFinished(id, result) if id == generation => {
                    self.frames = None;
                    self.status = match result {
                        Ok(message) | Err(message) => message,
                    };
                }
                _ => {}
            }
            changed = true;
        }
        let event = self
            .frames
            .as_mut()
            .and_then(media_spike::capture::next_frame_event);
        match event {
            Some(FrameEvent::Frame(frame)) => {
                self.set_frame(frame, cx);
                changed = true;
            }
            Some(FrameEvent::Finished(message)) => {
                self.frames = None;
                self.status = message;
                changed = true;
            }
            Some(FrameEvent::Error(error)) => {
                self.stop(cx);
                self.status = error.to_string();
                changed = true;
            }
            _ => {}
        }
        if changed {
            cx.notify();
        }
    }
    fn clear_preview(&mut self, cx: &mut Context<Self>) {
        self.frames = None;
        self.queue = FrameQueue::default();
        if let Some(old) = self.preview.take() {
            // Defer until GPUI restores the current window to App.windows.
            cx.defer(move |cx| cx.drop_image(old, None));
        }
    }
    fn stop(&mut self, cx: &mut Context<Self>) {
        self.worker.next_generation();
        let _ = self.worker.commands.try_send(Command::Stop);
        self.clear_preview(cx);
        self.selected = None;
        self.status = "Preview stopped.".into();
        cx.notify();
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.clear_preview(cx);
        self.selected = None;
        let id = self.worker.next_generation();
        self.status = if self.worker.commands.try_send(Command::Sources(id)).is_ok() {
            "Discovering screens and windows…"
        } else {
            "Capture worker is busy. Retry shortly."
        }
        .into();
        cx.notify();
    }
    fn start(&mut self, source: SourceId, cx: &mut Context<Self>) {
        let id = self.worker.next_generation();
        self.clear_preview(cx);
        self.selected = Some(source);
        self.status = if self
            .worker
            .commands
            .try_send(Command::Start(id, source))
            .is_ok()
        {
            "Opening local preview…"
        } else {
            "Capture worker is busy. Retry shortly."
        }
        .into();
        cx.notify();
    }
    fn network(&mut self, cx: &mut Context<Self>) {
        self.clear_preview(cx);
        self.selected = None;
        let id = self.worker.next_generation();
        self.status = if self
            .worker
            .commands
            .try_send(Command::Loopback(id, None))
            .is_ok()
        {
            "Connecting two native participants to the configured SFU…"
        } else {
            "Media worker is busy. Retry shortly."
        }
        .into();
        cx.notify();
    }
    fn publish_selected(&mut self, cx: &mut Context<Self>) {
        let Some(source @ SourceId::Window(_)) = self.selected else {
            self.status =
                "Select a window for local preview first, then click Publish selected window."
                    .into();
            cx.notify();
            return;
        };
        self.clear_preview(cx);
        let id = self.worker.next_generation();
        self.status = if self
            .worker
            .commands
            .try_send(Command::Loopback(id, Some(source)))
            .is_ok()
        {
            "Opening selected window for a 30-second publish/decode test…"
        } else {
            "Media worker is busy. Retry shortly."
        }
        .into();
        cx.notify();
    }
    fn set_frame(&mut self, frame: Arc<VideoFrame>, cx: &mut Context<Self>) {
        enqueue_latest(&mut self.queue, frame);
        if let Some(frame) = current_frame(&self.queue)
            && let Some(buffer) =
                image::RgbaImage::from_raw(frame.width, frame.height, frame.bgra.to_vec())
        {
            // RenderImage expects BGRA despite image::RgbaImage's type name.
            let new = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
            if let Some(old) = self.preview.replace(new) {
                // Defer until GPUI restores the current window to App.windows.
                cx.defer(move |cx| cx.drop_image(old, None));
            }
        }
    }
    fn pattern(&mut self, cx: &mut Context<Self>) {
        self.stop(cx);
        let colors = [
            [0x69, 0xb8, 0xf8, 255],
            [0x83, 0x82, 0x67, 255],
            [0x9b, 0x69, 0xba, 255],
            [0xb2, 0xbc, 0x86, 255],
        ];
        let mut bgra = Vec::with_capacity(640 * 360 * 4);
        for y in 0..360 {
            for x in 0..640 {
                bgra.extend_from_slice(&colors[(x / 160 + y / 90) % colors.len()]);
            }
        }
        self.set_frame(
            Arc::new(VideoFrame {
                width: 640,
                height: 360,
                bgra: bgra.into(),
            }),
            cx,
        );
        self.status = "Test pattern · CPU upload to GPU · No screen capture or broadcast".into();
        cx.notify();
    }
}
impl Render for MediaLab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().bg(rgb(0x161a20)).text_color(rgb(0xe8e9ec)).font_family(".AppleSystemUIFont")
            .child(div().w(px(76.)).flex_shrink_0().h_full().flex().flex_col().items_center().pt_6().gap_4().bg(rgb(0x101318))
                .child(img(self.icon.clone()).size(px(48.)).rounded_xl())
                .child(div().w(px(36.)).h(px(2.)).bg(rgb(0x2c323d)))
                .child(div().rounded_lg().bg(rgb(0xf8b869)).text_color(rgb(0x25201a)).p_3().child("M0")))
            .child(div().w(px(260.)).flex_shrink_0().h_full().flex().flex_col().bg(rgb(0x1d222a)).p_5().gap_4()
                .child(div().text_xl().font_weight(FontWeight::BOLD).child("Whisk"))
                .child(div().text_sm().text_color(rgb(0x8e98a7)).child("NATIVE MEDIA LAB"))
                .child(div().id("discover").rounded_md().p_3().bg(rgb(0x303945)).cursor_pointer().child("Refresh sources").on_click(cx.listener(|this, _, _, cx| this.refresh(cx))))
                .child(div().id("sources").flex_1().overflow_y_scroll().flex().flex_col().gap_2()
                    .children(self.sources.iter().enumerate().map(|(index, source)| {
                        let id = source.id;
                        div().id(("source", index)).p_3().rounded_md().text_sm().cursor_pointer()
                            .bg(rgb(if self.selected == Some(id) { 0x48513e } else { 0x262d37 }))
                            .child(source.label.clone()).on_click(cx.listener(move |this, _, _, cx| this.start(id, cx)))
                    })))
                .child(div().text_xs().text_color(rgb(0x8e98a7)).child("Selecting a source opens a local preview. Publish selected window sends that window to your configured SFU for 30 seconds.")))
            .child(div().flex_1().min_w_0().h_full().flex().flex_col().p_6().gap_5()
                .child(div().flex().items_center().justify_between()
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Screen preview"))
                    .child(div().rounded_md().px_3().py_1().bg(rgb(0x28372f)).text_sm().text_color(rgb(0xa4cfaa)).child("MEDIA LAB")))
                .child(div().relative().w_full().min_w_0().flex_1().min_h_0().rounded_xl().bg(rgb(0x0e1116)).overflow_hidden().flex().items_center().justify_center()
                    .when_some(self.preview.clone(), |view, image| view.child(img(image).absolute().size_full().object_fit(ObjectFit::Contain)))
                    .when(self.preview.is_none(), |view| view.child(div().flex().flex_col().gap_3().items_center()
                        .child(div().text_xl().child("A place for your next conversation"))
                        .child(div().text_sm().text_color(rgb(0x8e98a7)).child("Start by testing the native video surface.")))))
                .child(div().w_full().min_w_0().text_sm().text_color(rgb(0xb1bac7)).child(self.status.clone()))
                .child(div().flex().flex_wrap().gap_3()
                    .child(div().id("test-pattern").rounded_md().px_4().py_3().bg(rgb(0xf8b869)).text_color(rgb(0x25201a)).cursor_pointer().child("Test video surface").on_click(cx.listener(|this, _, _, cx| this.pattern(cx))))
                    .child(div().id("network").rounded_md().px_4().py_3().bg(rgb(0x303945)).cursor_pointer().child("Test network video").on_click(cx.listener(|this, _, _, cx| this.network(cx))))
                    .child(div().id("publish-selected").rounded_md().px_4().py_3().bg(rgb(0x48513e)).cursor_pointer().child("Publish selected window").on_click(cx.listener(|this, _, _, cx| this.publish_selected(cx))))
                    .child(div().id("stop").rounded_md().px_4().py_3().bg(rgb(0x303945)).cursor_pointer().child("Stop preview").on_click(cx.listener(|this, _, _, cx| this.stop(cx)))))
                .child(div().text_xs().text_color(rgb(0x727e90)).child("M0 feasibility build · ScreenCaptureKit → bounded frame handoff → GPUI · Voice and messaging are not connected yet.")))
    }
}
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1180.), px(780.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(
                        if std::env::var_os("WHISK_MEDIA_NETWORK_MODE").is_some() {
                            "Whisk — Network Lab"
                        } else {
                            "Whisk — Media Lab"
                        }
                        .into(),
                    ),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(MediaLab::new),
        )
        .expect("open Whisk native window");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
