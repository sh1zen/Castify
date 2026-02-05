use crate::gui::common::datastructure::ScreenRect;
use crate::utils::evaluate_points;
use iced::Renderer;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key};
use iced::mouse::{Cursor, Interaction};
use iced::widget::Action;
use iced::widget::canvas;
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Size, mouse};
use iced_graphics::geometry;
use iced_graphics::geometry::{LineCap, LineDash, LineJoin, Style};

#[derive(Default, Clone, Copy)]
pub struct AreaSelectorState {
    pub updating: bool,
    pub initial_pos: Option<Point>,
    pub final_pos: Option<Point>,
}

pub struct AreaSelector<'a, Message> {
    on_release_rect: Option<Box<dyn Fn(ScreenRect) -> Message + 'a>>,
    on_esc: Option<Message>,
    on_confirm: Option<Message>,
}

impl<'a, Message> AreaSelector<'a, Message> {
    pub fn new() -> Self {
        Self {
            on_release_rect: None,
            on_esc: None,
            on_confirm: None,
        }
    }

    pub fn on_esc(mut self, message: Message) -> Self {
        self.on_esc = Some(message);
        self
    }

    pub fn on_confirm(mut self, message: Message) -> Self {
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
        if let Some(start) = start
            && let Some(end) = end
        {
            return ScreenRect {
                x: start.x.min(end.x),
                y: start.y.min(end.y),
                width: (start.x - end.x).abs(),
                height: (start.y - end.y).abs(),
            };
        }

        ScreenRect::default()
    }
}

impl<'a, Message: Clone, Theme> canvas::Program<Message, Theme> for AreaSelector<'a, Message> {
    type State = AreaSelectorState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;

        match event {
            iced::Event::Keyboard(Event::KeyPressed { key, .. }) => {
                if *key == Key::Named(Named::Escape) {
                    self.on_esc
                        .clone()
                        .map(|m| Action::publish(m).and_capture())
                } else if *key == Key::Named(Named::Enter) {
                    self.on_confirm
                        .clone()
                        .map(|m| Action::publish(m).and_capture())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.updating = true;
                state.initial_pos = Some(cursor_position);
                state.final_pos = Some(cursor_position);
                Some(Action::request_redraw())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    state.final_pos = Some(cursor_position);
                    Some(Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.updating = false;

                let message = self
                    .on_release_rect
                    .as_ref()
                    .map(|callback| callback(Self::calc_rect(state.initial_pos, state.final_pos)));

                match message {
                    Some(msg) => Some(Action::publish(msg).and_capture()),
                    None => Some(Action::request_redraw()),
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
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
