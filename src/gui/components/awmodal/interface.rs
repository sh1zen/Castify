use crate::config::Config;
use crate::gui::widget::{Column, Element};
use castbox::AnyRef;
use iced::Length;
use std::any::Any;

pub trait GuiInterface {
    type Message;

    fn title(&self) -> String;

    fn update(&mut self, _value: AnyRef, _config: &Config) {}

    fn view<'a, 'b>(&'a self, _config: &Config) -> Element<'b, Self::Message>
    where
        'b: 'a,
        Self::Message: Clone + 'b,
    {
        Column::new().spacing(12).into()
    }

    fn width(&self) -> Length {
        Length::Fixed(500.0)
    }

    fn height(&self) -> Length {
        Length::Fixed(300.0)
    }

    fn on_close(&self) -> Option<Self::Message> {
        None
    }
}

pub trait GuiComponent {
    type Message;

    fn as_gui<'a>(&'a self) -> &'a dyn GuiInterface<Message=Self::Message>;

    fn as_mut_gui(&mut self) -> &mut dyn GuiInterface<Message=Self::Message>;

    fn as_mut_any(&mut self) -> Box<&mut dyn Any>;
}