use iced::{
    advanced::layout::{Limits, Node},
    advanced::renderer,
    advanced::{Clipboard, Layout, Shell, Widget as AdvancedWidget},
    mouse, Color, Element, Event, Point, Rectangle, Size,
};
use iced::advanced::widget::{tree, Tree};

pub struct ScreenSelector {
    start: Option<Point>,
    end: Option<Point>,
}

impl ScreenSelector {
    pub fn new() -> Self {
        ScreenSelector {
            start: None,
            end: None,
        }
    }

    pub fn rectangle(&self) -> Option<Rectangle> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                Some(Rectangle::new(
                    Point::new(start.x.min(end.x), start.y.min(end.y)),
                    Size::new((end.x - start.x).abs(), (end.y - start.y).abs()),
                ))
            }
            _ => None,
        }
    }
}

impl<Message, Renderer, Theme> AdvancedWidget<Message, Renderer, Theme> for ScreenSelector
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Self>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(())
    }

    fn size(&self) -> Size {
        // Restituiamo una dimensione preferita; ad esempio 100x100
        Size::new(100.0, 100.0)
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        // Ottieni la dimensione massima disponibile e restituisci un nodo con quella dimensione
        let size = limits.max();
        Node::new(size)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            let rect = Rectangle::new(
                Point::new(start.x.min(end.x), start.y.min(end.y)),
                Size::new((end.x - start.x).abs(), (end.y - start.y).abs()),
            );

            renderer.fill_rectangle(
                layout.bounds().position(),
                rect.size(),
                Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            );
        }
    }

    fn on_event(
        &mut self,
        _tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle, // Parametro aggiuntivo
    ) -> iced::event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(layout.bounds()) {
                    self.start = Some(position);
                    self.end = Some(position);
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if self.start.is_some() {
                    self.end = Some(position);
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.start.is_some() {
                    self.end = Some(cursor.position().unwrap_or_default());
                    shell.publish(()); // Pubblica un messaggio per finalizzare la selezione
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            }
            _ => iced::event::Status::Ignored,
        }
    }
}

impl<'a, Message, Renderer, Theme> From<ScreenSelector> for Element<'a, Message, Renderer, Theme>
where
    Renderer: 'a + renderer::Renderer,
{
    fn from(selector: ScreenSelector) -> Self {
        Self::new(selector)
    }
}
