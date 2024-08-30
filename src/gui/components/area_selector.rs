use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced::advanced::mouse;
use iced::widget::canvas::{Frame, Geometry, Path, Program};
use iced::widget::{canvas, Canvas, Container};
use iced::mouse::Cursor;
use iced::alignment::{Horizontal, Vertical};
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::messages::AreaSelectionMessage;

#[derive(Debug, Clone, Copy)]
pub struct AreaSelector {
    start: Option<Point>,
    end: Option<Point>,
}

impl AreaSelector {
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    pub fn view(&self) -> Element<AreaSelectionMessage> {
        let canvas = Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(canvas)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }
}

impl Program<AreaSelectionMessage> for AreaSelector {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (canvas::event::Status, Option<AreaSelectionMessage>) {
        println!("AreaSelector update method called with event: {:?}", event);

        let cursor_position = if let Some(position) = cursor.position_in(bounds) {
            position
        } else {
            return (canvas::event::Status::Ignored, None);
        };

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                println!("Mouse button pressed at ({}, {})", cursor_position.x, cursor_position.y);
                (canvas::event::Status::Captured, Some(AreaSelectionMessage::StartSelection { x: cursor_position.x, y: cursor_position.y }))
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(_) = self.start {
                    println!("Cursor moved to ({}, {})", cursor_position.x, cursor_position.y);

                    (canvas::event::Status::Captured, Some(AreaSelectionMessage::UpdateSelection { x: cursor_position.x, y: cursor_position.y }))
                } else {
                    (canvas::event::Status::Ignored, None)
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                println!("Mouse button released");

                (canvas::event::Status::Captured, Some(AreaSelectionMessage::EndSelection))
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &StyleType,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Vec<Geometry> {
        println!("Drawing AreaSelector"); // Aggiungi questo per vedere se la funzione viene chiamata

        let mut frame = Frame::new(renderer, bounds.size());

        let background_path = Path::rectangle(Point::new(0.0, 0.0), bounds.size());
        frame.fill(&background_path, Color::from_rgba(0.0, 0.0, 0.0, 0.5)); // nero semi-trasparente

        if let (Some(start), Some(end)) = (self.start, self.end) {
            let rect = Rectangle::new(start, Size::new(end.x - start.x, end.y - start.y));
            let path = Path::rectangle(Point::new(rect.x, rect.y), rect.size());
            frame.fill(&path, Color::from_rgba(0.3, 0.7, 0.9, 0.5));
        }

        vec![frame.into_geometry()]
    }
}
