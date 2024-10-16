use crate::gui::style::theme::csx::StyleType as Theme;
use iced::widget as w;

pub type IcedRenderer = iced::Renderer;

pub type Element<'a, Message> = iced::Element<'a, Message, Theme, IcedRenderer>;

pub type Container<'a, Message> = w::Container<'a, Message, Theme, IcedRenderer>;
pub type Row<'a, Message> = w::Row<'a, Message, Theme, IcedRenderer>;
pub type Column<'a, Message> = w::Column<'a, Message, Theme, IcedRenderer>;

pub type Text<'a> = iced::advanced::widget::Text<'a, Theme, IcedRenderer>;
pub type TextInput<'a, Message> = w::TextInput<'a, Message, Theme, IcedRenderer>;
pub type Button<'a, Message> = w::Button<'a, Message, Theme, IcedRenderer>;
pub type Stack<'a, Message> = w::Stack<'a, Message, Theme, IcedRenderer>;

pub type PickList<'a, T, L, V, Message> = w::PickList<'a, T, L, V, Message, Theme, IcedRenderer>;
pub type Scrollable<'a, Message> = w::Scrollable<'a, Message, Theme, IcedRenderer>;
pub type Slider<'a, T, Message> = w::Slider<'a, T, Message, Theme>;
pub type Canvas<P, Message> = w::Canvas<P, Message, Theme, IcedRenderer>;


pub use w::horizontal_space;
pub use w::vertical_space;
pub use w::Space;


pub trait IcedParentExt<'a, Message> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a, Message>>;
}

impl<'a, Message> IcedParentExt<'a, Message> for Column<'a, Message> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a, Message>>,
    {
        if condition {
            self.push(element().into())
        } else {
            self
        }
    }
}

impl<'a, Message> IcedParentExt<'a, Message> for Row<'a, Message> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a, Message>>,
    {
        if condition {
            self.push(element().into())
        } else {
            self
        }
    }
}

pub trait IcedButtonExt<'a, Message> {
    fn on_press_if(self, condition: bool, msg: impl FnOnce() -> Message) -> Self;
}

impl<'a, Message> IcedButtonExt<'a, Message> for Button<'a, Message> {
    fn on_press_if(self, condition: bool, msg: impl FnOnce() -> Message) -> Self {
        if condition {
            self.on_press(msg())
        } else {
            self
        }
    }
}
