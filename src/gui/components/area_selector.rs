use crate::gui::common::datastructure::ScreenRect;
use crate::utils::evaluate_points;
use iced::widget::canvas;
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::Renderer;
use iced_core::keyboard::key::Named;
use iced_core::keyboard::{Event, Key};
use iced_core::mouse::{Cursor, Interaction};
use iced_core::{mouse, Color, Point, Rectangle, Size};
use iced_graphics::geometry;
use iced_graphics::geometry::{LineCap, LineDash, LineJoin, Style};

#[derive(Default, Clone, Copy)]
pub struct AreaSelectorState {
    pub updating: bool,
    pub initial_pos: Option<Point>,
    pub final_pos: Option<Point>,
}

#[allow(dead_code)]
pub struct AreaSelector<'a, Message, Theme>
{
    on_press: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_release: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    on_release_rect: Option<Box<dyn Fn(ScreenRect) -> Message + 'a>>,
    on_esc: Option<Message>,
    on_confirm: Option<Message>,
    theme: Option<Theme>,
}

#[allow(dead_code)]
impl<'a, Message, Theme> AreaSelector<'a, Message, Theme>
{
    pub fn new() -> Self {
        Self {
            on_press: None,
            on_drag: None,
            on_release: None,
            on_release_rect: None,
            on_esc: None,
            on_confirm: None,
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

    pub fn on_esc(mut self, message: Message) -> Self
    {
        self.on_esc = Some(message);
        self
    }

    pub fn on_confirm(mut self, message: Message) -> Self
    {
        self.on_confirm = Some(message);
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

        ScreenRect::default()
    }
}


impl<'a, Message: Clone, Theme> canvas::Program<Message, Theme> for AreaSelector<'a, Message, Theme>
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
            canvas::Event::Keyboard(event) => match event {
                Event::KeyPressed { key, .. } => {
                    if key == Key::Named(Named::Escape) {
                        (canvas::event::Status::Captured, self.on_esc.clone())
                    } else if key == Key::Named(Named::Enter) {
                        (canvas::event::Status::Captured, self.on_confirm.clone())
                    } else {
                        (canvas::event::Status::Ignored, None)
                    }
                }
                _ => (canvas::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.updating = true;
                state.initial_pos = Some(cursor_position);
                state.final_pos = Some(cursor_position);

                let mut message = None;
                if let Some(callback) = &self.on_press {
                    message = Some(callback(cursor_position.x, cursor_position.y));
                }
                (canvas::event::Status::Captured, message)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    state.final_pos = Some(cursor_position);

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
                    message = Some(callback(Self::calc_rect(state.initial_pos, state.final_pos)));
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
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        let overlay = geometry::Fill::from(Color::from_rgba(0.0, 0.0, 0.0, 0.4));

        if let (Some(mut initial_pos), Some(mut final_pos)) = (state.initial_pos, state.final_pos) {
            (initial_pos, final_pos) = evaluate_points(initial_pos, final_pos);

            let selection = Path::rectangle(initial_pos, (final_pos - initial_pos).into());
            let stroke = Stroke {
                style: Style::Solid(Color::from_rgba8(255, 255, 255, 0.2)),
                width: 1.0,
                line_cap: LineCap::default(),
                line_join: LineJoin::default(),
                line_dash: LineDash::default(),
            };
            frame.fill_rectangle(
                Point::new(0.0, 0.0),
                Size {
                    height: initial_pos.y,
                    width: bounds.width,
                },
                overlay,
            );
            frame.fill_rectangle(
                Point::new(0.0, final_pos.y),
                Size {
                    height: bounds.height - final_pos.y,
                    width: bounds.width,
                },
                overlay,
            );
            frame.fill_rectangle(
                Point::new(0.0, initial_pos.y),
                Size {
                    height: final_pos.y - initial_pos.y,
                    width: initial_pos.x,
                },
                overlay,
            );
            frame.fill_rectangle(
                Point::new(final_pos.x, initial_pos.y),
                Size {
                    height: final_pos.y - initial_pos.y,
                    width: bounds.width - final_pos.x,
                },
                overlay,
            );

            frame.stroke(&selection, stroke);

            let (width, height) = (final_pos.x - initial_pos.x, final_pos.y - initial_pos.y);

            let horizontal_segment_len = if width > 80.0 { 20.0 } else { width / 4.0 };
            let vertical_segment_len = if height > 80.0 { 20.0 } else { height / 4.0 };

            let edge_stroke = Stroke {
                style: Style::Solid(Color::WHITE),
                width: 4.0,
                line_cap: LineCap::Square,
                line_join: LineJoin::default(),
                line_dash: LineDash {
                    segments: &[
                        horizontal_segment_len,
                        width - (2.0 * horizontal_segment_len),
                        horizontal_segment_len,
                        0.0,
                        vertical_segment_len,
                        height - (2.0 * vertical_segment_len),
                        vertical_segment_len,
                        0.0,
                        horizontal_segment_len,
                        width - (2.0 * horizontal_segment_len),
                        horizontal_segment_len,
                        0.0,
                        vertical_segment_len,
                        height - (2.0 * vertical_segment_len),
                        vertical_segment_len,
                    ],
                    offset: 0,
                },
            };
            frame.stroke(&selection, edge_stroke);
        } else {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), overlay);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Interaction {
        Interaction::Crosshair
    }
}

