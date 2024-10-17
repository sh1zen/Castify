use crate::config::{app_name, version};
use crate::gui::common::icons::Icon;
use crate::gui::components::button::IconButton;
use crate::gui::widget::{Column, Container, Element};
use crate::gui::windows::main::MainWindowEvent;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::vertical_space;
use iced::Length;

pub fn info_page<'a>() -> Element<'a, MainWindowEvent> {
    let content = Column::new()
        .push(vertical_space().height(Length::Fill))
        .push(IconButton::new().icon(Icon::Version).label(version().to_string()).build().width(240).height(40))
        .push(IconButton::new().icon(Icon::Copyright).label(app_name().to_string()).build().width(240).height(40))
        .push(vertical_space().height(10))
        .push(
            IconButton::new().icon(Icon::User).label("Andrea Frolli".to_string()).build().width(240).height(40).on_press(
                MainWindowEvent::OpenWebPage("https://github.com/sh1zen".parse().unwrap())
            )
        )
        .push(
            IconButton::new().icon(Icon::User).label("Mario De Paola".to_string()).build().width(240).height(40)
        )
        .push(
            IconButton::new().icon(Icon::User).label("Pietro Bella".to_string()).build().width(240).height(40)
        )
        .spacing(8)
        .push(vertical_space().height(Length::Fill));

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Top)
        .into()
}