use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key};
use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::Renderer;
use iced::{mouse, Color, Point, Rectangle};
use iced_graphics::geometry::path::Builder;
use iced_graphics::geometry::LineJoin;
use iced_graphics::geometry::Style::Solid;

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
            ShapeColor::Custom(color) => color
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
    fn get_first(&self) -> Point {
        self.points.first().unwrap_or(&Point::ORIGIN).clone()
    }

    fn get_last(&self) -> Point {
        self.points.last().unwrap_or(&self.get_first()).clone()
    }

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
            !shape_points.iter().any(|point| Self::is_near(*point, eraser_point, eraser_size))
        });
    }
}

pub fn get_draw_points(point1: Point, point2: Point) -> (Point, Point) {
    let (mut start, mut end) = (point1, point2);
    if start.x > end.x { std::mem::swap(&mut start.x, &mut end.x); }
    if start.y > end.y { std::mem::swap(&mut start.y, &mut end.y); }

    (start, end)
}

#[allow(dead_code)]
pub struct Annotation<'a, Message, Theme>
{
    on_press: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_release: Option<Box<dyn Fn(Shape) -> Message + 'a>>,
    on_release_point: Option<Box<dyn Fn(Point, Point) -> Message + 'a>>,
    on_esc: Option<Message>,
    on_confirm: Option<Message>,
    theme: Option<Theme>,
    cache: canvas::Cache,
    shape: Shape,
}

#[allow(dead_code)]
impl<'a, Message, Theme> Annotation<'a, Message, Theme>
{
    pub fn new(shape: Shape) -> Self {
        Self {
            on_press: None,
            on_drag: None,
            on_release: None,
            on_release_point: None,
            on_esc: None,
            on_confirm: None,
            theme: None,
            cache: Default::default(),
            shape,
        }
    }

    pub fn on_drag<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(Point) -> Message,
    {
        self.on_drag = Some(Box::new(callback));
        self
    }

    pub fn on_press<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(Point) -> Message,
    {
        self.on_press = Some(Box::new(callback));
        self
    }

    pub fn on_release<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(Shape) -> Message,
    {
        self.on_release = Some(Box::new(callback));
        self
    }

    pub fn on_release_point<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(Point, Point) -> Message,
    {
        self.on_release_point = Some(Box::new(callback));
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
}


impl<'a, Message: Clone, Theme> canvas::Program<Message, Theme> for Annotation<'a, Message, Theme>
{
    type State = AnnotationState;

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
                state.points.push(cursor_position);

                let mut message = None;
                if let Some(callback) = &self.on_press {
                    message = Some(callback(cursor_position));
                }
                (canvas::event::Status::Captured, message)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.updating {
                    let mut message = None;

                    if self.shape.s_type == ShapeType::Eraser {
                        state.erase_at(cursor_position, self.shape.stroke.f32() * 5.0);
                    } else {
                        state.points.push(cursor_position);
                        if let Some(callback) = &self.on_drag {
                            message = Some(callback(cursor_position));
                        }
                    }

                    (canvas::event::Status::Captured, message)
                } else {
                    (canvas::event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.updating = false;
                state.shapes.push((self.shape, state.points.clone()));


                let mut message = None;
                if let Some(callback) = &self.on_release_point {
                    message = Some(callback(state.get_first(), state.get_last()));
                }
                if let Some(callback) = &self.on_release {
                    message = Some(callback(self.shape));
                }

                state.points.clear();
                self.cache.clear();

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
        cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let shapes_frame = self.cache.draw(renderer, bounds.size(), |frame| {
            for (shape, points) in &state.shapes {
                draw_shape(frame, shape, points);
            }
        });

        let mut frame = Frame::new(renderer, bounds.size());

        draw_shape(&mut frame, &self.shape, &state.points);

        if self.shape.s_type == ShapeType::Eraser {
            if let Some(cursor_pos) = cursor.position_in(bounds) {
                let path = Path::circle(cursor_pos, 20.0);
                let color = Color::from_rgba(1.0, 1.0, 1.0, 0.7);
                let fill = iced::widget::canvas::Fill::from(color);
                frame.fill(&path, fill);
            }
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


fn draw_shape(frame: &mut Frame, shape: &Shape, points: &Vec<Point>) {
    if points.len() >= 2 {
        let color = shape.color.into_iced_color(shape.is_solid);
        match &shape.s_type {
            ShapeType::Rectangle => {
                let (top_left, bottom_right) = get_draw_points(points.first().unwrap().clone(), points.last().unwrap().clone());
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
                let path = Path::line(points.first().unwrap().clone(), points.last().unwrap().clone());
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
                    frame.stroke(&path, Stroke {
                        style: Solid(color),
                        width: shape.stroke.f32(),
                        line_cap: Default::default(),
                        line_join: LineJoin::Round,
                        line_dash: Default::default(),
                    });
                }
            }
            ShapeType::Circle => {
                // Get the center and radius based on the first and last points
                let center = points.first().unwrap().clone();
                let radius = points.first().unwrap().distance(points.last().unwrap().clone());
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
