use iced::Renderer;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key};
use iced::mouse::{Cursor, Interaction};
use iced::widget::Action;
use iced::widget::canvas;
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, mouse};
use iced_graphics::geometry::LineJoin;
use iced_graphics::geometry::Style::Solid;
use iced_graphics::geometry::path::Builder;

#[derive(Debug, Default, Clone, Copy)]
pub struct Shape {
    pub s_type: ShapeType,
    pub stroke: ShapeStroke,
    pub color: ShapeColor,
    pub is_filled: bool,
    pub is_solid: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ShapeType {
    #[default]
    Personal,
    Rectangle,
    Line,
    Eraser,
    Circle,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ShapeColor {
    #[default]
    Black,
    White,
    Red,
    Green,
    Blue,
    Custom(Color),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ShapeStroke {
    Thin,
    #[default]
    Medium,
    Broad,
}

impl ShapeColor {
    pub fn into_iced_color(self, solid: bool) -> Color {
        let opacity = if solid { 1.0 } else { 0.3 };
        match self {
            ShapeColor::Red => Color::from_rgba8(255, 10, 10, opacity),
            ShapeColor::Green => Color::from_rgba8(10, 255, 10, opacity),
            ShapeColor::Blue => Color::from_rgba8(10, 10, 255, opacity),
            ShapeColor::Black => Color::from_rgba8(0, 0, 0, opacity),
            ShapeColor::White => Color::from_rgba8(255, 255, 255, opacity),
            ShapeColor::Custom(color) => color,
        }
    }
}

impl ShapeStroke {
    pub fn f32(&self) -> f32 {
        match self {
            Self::Thin => 2.0,
            Self::Medium => 5.0,
            Self::Broad => 8.0,
        }
    }
}

#[derive(Default, Clone)]
pub struct AnnotationState {
    pub updating: bool,
    pub points: Vec<Point>,
    pub shapes: Vec<(Shape, Vec<Point>)>,
}

impl AnnotationState {
    // Additional method to check if two points are close enough to be considered an overlap
    fn is_near(point1: Point, point2: Point, threshold: f32) -> bool {
        let dx = point1.x - point2.x;
        let dy = point1.y - point2.y;
        (dx * dx + dy * dy).sqrt() < threshold
    }

    // Method to erase shapes near a given point
    pub fn erase_at(&mut self, eraser_point: Point, eraser_size: f32) {
        self.shapes.retain(|(_, shape_points)| {
            // Retain shapes that do not have any points near the eraser
            !shape_points
                .iter()
                .any(|point| Self::is_near(*point, eraser_point, eraser_size))
        });
    }
}

pub fn get_draw_points(point1: Point, point2: Point) -> (Point, Point) {
    let (mut start, mut end) = (point1, point2);
    if start.x > end.x {
        std::mem::swap(&mut start.x, &mut end.x);
    }
    if start.y > end.y {
        std::mem::swap(&mut start.y, &mut end.y);
    }

    (start, end)
}

pub struct Annotation<Message> {
    on_esc: Option<Message>,
    cache: canvas::Cache,
    shape: Shape,
}

impl<Message> Annotation<Message> {
    pub fn new(shape: Shape) -> Self {
        Self {
            on_esc: None,
            cache: Default::default(),
            shape,
        }
    }

    pub fn on_esc(mut self, message: Message) -> Self {
        self.on_esc = Some(message);
        self
    }
}

impl<Message: Clone, Theme> canvas::Program<Message, Theme> for Annotation<Message> {
    type State = AnnotationState;

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
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.updating = true;
                state.points.push(cursor_position);
                Some(Action::request_redraw())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    if self.shape.s_type == ShapeType::Eraser {
                        state.erase_at(cursor_position, self.shape.stroke.f32() * 5.0);
                        return Some(Action::request_redraw());
                    }

                    state.points.push(cursor_position);
                    Some(Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.updating = false;
                state.shapes.push((self.shape, state.points.clone()));

                state.points.clear();
                self.cache.clear();

                Some(Action::request_redraw())
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
        cursor: Cursor,
    ) -> Vec<Geometry> {
        let shapes_frame = self.cache.draw(renderer, bounds.size(), |frame| {
            for (shape, points) in &state.shapes {
                draw_shape(frame, shape, points);
            }
        });

        let mut frame = Frame::new(renderer, bounds.size());

        draw_shape(&mut frame, &self.shape, &state.points);

        if self.shape.s_type == ShapeType::Eraser
            && let Some(cursor_pos) = cursor.position_in(bounds)
        {
            let path = Path::circle(cursor_pos, 20.0);
            let color = Color::from_rgba(1.0, 1.0, 1.0, 0.7);
            let fill = iced::widget::canvas::Fill::from(color);
            frame.fill(&path, fill);
        }

        vec![shapes_frame, frame.into_geometry()]
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

fn draw_shape(frame: &mut Frame, shape: &Shape, points: &[Point]) {
    if points.len() >= 2 {
        let color = shape.color.into_iced_color(shape.is_solid);
        match &shape.s_type {
            ShapeType::Rectangle => {
                let (top_left, bottom_right) =
                    get_draw_points(*points.first().unwrap(), *points.last().unwrap());
                let size = (bottom_right - top_left).into();
                let path = Path::rectangle(top_left, size);
                if shape.is_filled {
                    let fill = iced::widget::canvas::Fill::from(color);
                    frame.fill(&path, fill);
                } else {
                    let stroke = Stroke::default()
                        .with_width(shape.stroke.f32())
                        .with_color(color)
                        .with_line_join(LineJoin::Round);
                    frame.stroke(&path, stroke);
                }
            }
            ShapeType::Line => {
                let path = Path::line(*points.first().unwrap(), *points.last().unwrap());
                let stroke = Stroke::default()
                    .with_width(shape.stroke.f32())
                    .with_color(color);
                frame.stroke(&path, stroke);
            }
            ShapeType::Personal => {
                let mut builder = Builder::new();
                if let Some(first_point) = points.first() {
                    builder.move_to(*first_point);

                    for point in points.iter() {
                        builder.line_to(*point);
                    }

                    let path = builder.build();
                    frame.stroke(
                        &path,
                        Stroke {
                            style: Solid(color),
                            width: shape.stroke.f32(),
                            line_cap: Default::default(),
                            line_join: LineJoin::Round,
                            line_dash: Default::default(),
                        },
                    );
                }
            }
            ShapeType::Circle => {
                // Get the center and radius based on the first and last points
                let center = *points.first().unwrap();
                let radius = points.first().unwrap().distance(*points.last().unwrap());
                let path = Path::circle(center, radius);

                if shape.is_filled {
                    let fill = iced::widget::canvas::Fill::from(color);
                    frame.fill(&path, fill);
                } else {
                    let stroke = Stroke::default()
                        .with_width(shape.stroke.f32())
                        .with_color(color);
                    frame.stroke(&path, stroke);
                }
            }
            ShapeType::Eraser => {}
        }
    }
}
