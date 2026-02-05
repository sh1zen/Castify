use crate::gui::components::video::pipeline::VideoPrimitive;
use crate::gui::components::video::Video;
use iced::{
    advanced::{self, layout, widget, Widget},
    Element,
};
use iced_wgpu::primitive::Renderer as PrimitiveRenderer;
use std::marker::PhantomData;
use std::sync::atomic::Ordering;
use std::{sync::Arc, time::Duration};

/// Video player widget which displays the current frame of a [`Video`].
pub struct VideoPlayer<'a, Message, Theme, Renderer>
where
    Renderer: PrimitiveRenderer,
{
    video: &'a Video,
    on_end_of_stream: Option<Message>,
    on_new_frame: Option<Message>,
    _phantom: PhantomData<(Theme, Renderer)>,
}

impl<'a, Message, Theme, Renderer> VideoPlayer<'a, Message, Theme, Renderer>
where
    Renderer: PrimitiveRenderer,
{
    /// Creates a new video player widget for a given video.
    pub fn new(video: &'a Video) -> Self {
        VideoPlayer {
            video,
            on_end_of_stream: None,
            on_new_frame: None,
            _phantom: Default::default(),
        }
    }

    /// Message to send when the stream ends (channel closed).
    pub fn on_end_of_stream(self, on_end_of_stream: Message) -> Self {
        VideoPlayer {
            on_end_of_stream: Some(on_end_of_stream),
            ..self
        }
    }

    /// Message to send when a new frame is available for rendering.
    pub fn on_new_frame(self, on_new_frame: Message) -> Self {
        VideoPlayer {
            on_new_frame: Some(on_new_frame),
            ..self
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VideoPlayer<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: PrimitiveRenderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: iced::Length::Fill,
            height: iced::Length::Fill,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounds = limits.resolve(iced::Length::Fill, iced::Length::Fill, limits.max());
        let (width, height) = self.video.size();
        if width <= 0 || height <= 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return layout::Node::new(bounds);
        }

        let video_aspect = width as f32 / height as f32;
        if !video_aspect.is_finite() || video_aspect <= 0.0 {
            return layout::Node::new(bounds);
        }

        // Fixed aspect ratio + never exceed available size.
        let viewport_aspect = bounds.width / bounds.height;
        let size = if viewport_aspect > video_aspect {
            iced::Size::new(bounds.height * video_aspect, bounds.height)
        } else {
            iced::Size::new(bounds.width, bounds.width / video_aspect)
        };

        layout::Node::new(size)
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &advanced::renderer::Style,
        layout: advanced::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let (w, h) = self.video.size();
        let inner = self.video.0.borrow();
        renderer.draw_primitive(
            layout.bounds(),
            VideoPrimitive::new(
                inner.id,
                Arc::clone(&inner.frame),
                (w as _, h as _),
                Arc::clone(&inner.has_new_frame),
            ),
        );
    }

    fn update(
        &mut self,
        _state: &mut widget::Tree,
        event: &iced::Event,
        _layout: advanced::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        let inner = self.video.0.borrow();

        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if inner.is_eos_flag.load(Ordering::SeqCst) {
                if let Some(on_eos) = self.on_end_of_stream.clone() {
                    shell.publish(on_eos);
                }
                return;
            }

            if !inner.paused {
                let redraw_interval = 1.0 / inner.framerate.max(1.0);
                let until_redraw =
                    redraw_interval - (*now - inner.next_redraw).as_secs_f64() % redraw_interval;
                let next = *now + Duration::from_secs_f64(until_redraw);
                shell.request_redraw_at(next);

                if let (true, Some(on_new_frame)) = (
                    inner.has_new_frame.load(Ordering::Acquire),
                    self.on_new_frame.clone(),
                ) {
                    shell.publish(on_new_frame);
                }
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<VideoPlayer<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + PrimitiveRenderer,
{
    fn from(video_player: VideoPlayer<'a, Message, Theme, Renderer>) -> Self {
        Self::new(video_player)
    }
}
