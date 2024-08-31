use iced::advanced::mouse;
use iced::mouse::Cursor;
use iced::widget::canvas::{Frame, Path, Program};
use iced::{Color, Element, Point, Rectangle, Size};
use iced::widget::{canvas, Canvas, Container};
use iced_core::{Length, Widget};
use crate::gui::components::raw::screen_area::style::StyleSheet;

#[derive(Default)]
pub struct AreaSelectorState {
    pub updating: bool,
    pub start: Option<Point>,
    pub end: Option<Point>,
}

pub struct AreaSelector<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: StyleSheet,
    Renderer: iced_core::Renderer + iced_graphics::geometry::Renderer,
{
    on_drag: Option<Box<dyn Fn((f32, f32)) -> Message + 'a>>,
    on_release: Option<Box<dyn Fn((f32, f32)) -> Message + 'a>>,
    on_press: Option<Box<dyn Fn((f32, f32)) -> Message + 'a>>,
    style: Theme::Style,
    content: Option<iced_core::Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer> AreaSelector<'a, Message, Theme, Renderer>
where
    Theme: StyleSheet + for<'b> Fn(&'b iced::Theme),
    Renderer: iced_core::Renderer + iced_graphics::geometry::Renderer,
{
    pub fn new() -> Self {
        Self {
            on_drag: None,
            on_release: None,
            on_press: None,
            style: Theme::Style::default(),
            content: None,
        }
    }

    pub fn on_drag<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn((f32, f32)) -> Message,
    {
        self.on_drag = Some(Box::new(callback));
        self
    }

    pub fn on_press<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn((f32, f32)) -> Message,
    {
        self.on_press = Some(Box::new(callback));
        self
    }

    pub fn on_release<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn((f32, f32)) -> Message,
    {
        self.on_release = Some(Box::new(callback));
        self
    }

    pub fn style(mut self, style: impl Into<Theme::Style>) -> Self {
        self.style = style.into();
        self
    }

    pub fn view(&self) -> Element<'a, Message, Theme, Renderer> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a, Message, Theme, Renderer> Program<Message, Theme, Renderer> for AreaSelector<'a, Message, Theme, Renderer>
where
    Theme: StyleSheet,
    Renderer: iced_core::Renderer + iced_graphics::geometry::Renderer,
{
    type State = AreaSelectorState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let cursor_position = if let Some(position) = cursor.position_in(bounds) {
            position
        } else {
            return (canvas::event::Status::Ignored, None);
        };

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.updating = true;
                state.start = Some(cursor_position);
                state.end = Some(cursor_position);

                let mut message = None;
                if let Some(callback) = &self.on_press {
                    message = Some(callback((cursor_position.x, cursor_position.y)));
                }
                (canvas::event::Status::Captured, message)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    state.end = Some(cursor_position);

                    let mut message = None;
                    if let Some(callback) = &self.on_drag {
                        message = Some(callback((cursor_position.x, cursor_position.y)));
                    }
                    (canvas::event::Status::Captured, message)
                } else {
                    (canvas::event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.updating = false;

                let mut message = None;
                if let Some(callback) = &self.on_release {
                    message = Some(callback((cursor_position.x, cursor_position.y)));
                }
                (canvas::event::Status::Captured, message)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Vec<<Renderer as iced_graphics::geometry::Renderer>::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if let (Some(start), Some(end)) = (state.start, state.end) {
            let rect = Path::rectangle(
                Point::new(
                    start.x.min(end.x),
                    start.y.min(end.y),
                ),
                Size::new(
                    (end.x - start.x).abs(),
                    (end.y - start.y).abs(),
                ),
            );
            frame.fill(&rect, Color::from_rgba(0.5, 0.0, 0.5, 0.3));
        }

        vec![frame.into_geometry()]
    }
}

impl<'a, Message, Theme, Renderer> Default for AreaSelector<'a, Message, Theme, Renderer>
where
    Theme: StyleSheet,
    Renderer: iced_core::Renderer + iced_graphics::geometry::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message, Theme, Renderer> From<AreaSelector<'a, Message, Theme, Renderer>>
for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: StyleSheet + 'a,
    Renderer: iced_core::Renderer + iced_graphics::geometry::Renderer + 'a,
{
    fn from(area: AreaSelector<'a, Message, Theme, Renderer>) -> Self {
        Self::new(area)
    }
}
