use iced::widget::canvas::{Frame, Path};
use iced::widget::{canvas, Canvas};
use iced::{Element, Renderer};
use iced_core::{mouse, Color, Length, Point, Rectangle, Size};


#[derive(Debug, Clone)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Clone, Copy)]
pub struct AreaSelectorState {
    pub updating: bool,
    pub start: Option<Point>,
    pub end: Option<Point>,
}

pub struct AreaSelector<'a, Message, Theme>
{
    on_press: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_release: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_release_rect: Option<Box<dyn Fn(ScreenRect) -> Message + 'a>>,
    theme: Option<Theme>,
}


impl<'a, Message, Theme> AreaSelector<'a, Message, Theme>
{
    pub fn new() -> Self {
        Self {
            on_press: None,
            on_drag: None,
            on_release: None,
            on_release_rect: None,
            theme: None,
        }
    }

    pub fn on_drag<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(f32, f32) -> Message,
    {
        self.on_drag = Some(Box::new(callback));
        self
    }

    pub fn on_press<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(f32, f32) -> Message,
    {
        self.on_press = Some(Box::new(callback));
        self
    }

    pub fn on_release<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(f32, f32) -> Message,
    {
        self.on_release = Some(Box::new(callback));
        self
    }

    pub fn on_release_rect<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(ScreenRect) -> Message,
    {
        self.on_release_rect = Some(Box::new(callback));
        self
    }

    fn calc_rect(start: Option<Point>, end: Option<Point>) -> ScreenRect {
        if let Some(start) = start {
            if let Some(end) = end {
                return ScreenRect {
                    x: start.x.min(end.x),
                    y: start.y.min(end.y),
                    width: (start.x - end.x).abs(),
                    height: (start.y - end.y).abs(),
                };
            }
        }

        ScreenRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn view<'b>(oop: AreaSelector<'b, Message, Theme>) -> Element<'b, Message, Theme>
    where
        Message: 'b,
        Theme: 'b,
    {
        let canvas = Canvas::new(oop)
            .width(Length::Fill)
            .height(Length::Fill);

        canvas.into()
    }
}


impl<'a, Message, Theme> canvas::Program<Message, Theme> for AreaSelector<'a, Message, Theme>
{
    type State = AreaSelectorState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
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
                    message = Some(callback(cursor_position.x, cursor_position.y));
                }
                (canvas::event::Status::Captured, message)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    state.end = Some(cursor_position);

                    let mut message = None;
                    if let Some(callback) = &self.on_drag {
                        message = Some(callback(cursor_position.x, cursor_position.y));
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
                    message = Some(callback(cursor_position.x, cursor_position.y));
                } else if let Some(callback) = &self.on_release_rect {
                    message = Some(callback(Self::calc_rect(state.start, state.end)));
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
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<<Renderer as iced::widget::canvas::Renderer>::Geometry> {
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

