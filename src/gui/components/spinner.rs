use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::canvas;
use iced::widget::canvas::{Event, Frame, Geometry};
use iced::{Color, Point, Rectangle, Renderer};
use iced_graphics::geometry::{Path, Stroke, Style};

pub struct Spinner {}

#[allow(dead_code)]
impl Spinner {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct SpinnerRotation {
    rotation: f32,
}

impl<'a, Message, Theme> canvas::Program<Message, Theme> for Spinner
{
    type State = SpinnerRotation;

    fn update(&self, state: &mut Self::State, _event: Event, _bounds: Rectangle, _cursor: Cursor) -> (Status, Option<Message>) {
        state.rotation = state.rotation + 0.01;
        (Status::Ignored, None)
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

        // Define spinner properties
        let spinner_radius = 40.0;
        let circle_radius = 8.0;
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // Calculate rotation angle in radians
        let angle = state.rotation * 2.0 * std::f32::consts::PI;

        // Calculate the position of the rotating circle
        let circle_x = center.x + spinner_radius * angle.cos();
        let circle_y = center.y + spinner_radius * angle.sin();

        // Draw the rotating circle
        let path = Path::circle(Point::new(circle_x, circle_y), circle_radius);
        frame.fill(&path, Color::from_rgb(0.0, 0.6, 1.0));

        // Draw a background circle for aesthetics
        let background = Path::circle(center, spinner_radius);
        frame.stroke(
            &background,
            Stroke {
                style: Style::Solid(Color::from_rgb(1.0, 1.0, 1.0)),
                width: 2.0,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}