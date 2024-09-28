use iced::widget as w;

use crate::gui::style::styles::csx::StyleType as Theme;

pub type IcedRenderer = iced::Renderer;

pub type Element<'a, Message, Theme, Renderer> = iced::Element<'a, Message, Theme, Renderer>;


pub type Container<'a, Message> = w::Container<'a, Message, Theme, IcedRenderer>;
pub type Row<'a, Message> = w::Row<'a, Message, Theme, IcedRenderer>;
pub type Column<'a, Message> = w::Column<'a, Message, Theme, IcedRenderer>;

pub type Text<'a> = iced::advanced::widget::Text<'a, Theme, IcedRenderer>;
pub type TextInput<'a, Message> = w::TextInput<'a, Message, Theme, IcedRenderer>;
pub type Button<'a, Message> = w::Button<'a, Message, Theme, IcedRenderer>;
pub type Stack<'a, Message> = w::Stack<'a, Message, Theme, IcedRenderer>;
pub type PickList<'a, T, L, V, Message> = w::PickList<'a, T, L, V, Message, Theme, IcedRenderer>;
pub type Scrollable<'a, Message> = w::Scrollable<'a, Message, Theme, IcedRenderer>;


use crate::gui::style::styles::csx::StyleType;
pub use w::Space;


pub trait IcedParentExt<'a, Message> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a, Message, StyleType, IcedRenderer>>;
}

impl<'a, Message> IcedParentExt<'a, Message> for Column<'a, Message> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a, Message, Theme, IcedRenderer>>,
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
        E: Into<Element<'a, Message, Theme, IcedRenderer>>,
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
