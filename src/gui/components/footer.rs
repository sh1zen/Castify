use crate::assets::APP_NAME;
use crate::assets::APP_VERSION;
use crate::gui::common::icons::Icon;
use crate::gui::style::button::ButtonType;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{Button, Container, Row, Text};
use crate::gui::windows::main::MainWindowEvent;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::LineHeight;
use iced::{Alignment, Length};

pub fn footer<'a>() -> Container<'a, MainWindowEvent> {
    let made_by = Text::new("Made by:  A. Frolli  P. Bella  M. De Paola")
        .width(Length::Fill)
        .align_x(Horizontal::Right)
        .size(14.0);

    let version = Row::new()
        .align_y(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill)
        .spacing(5)
        .push(
            Text::new(format!("{APP_NAME} {APP_VERSION}")).size(14.0)
        );

    let footer_row = Row::new()
        .padding([0, 10])
        .align_y(Alignment::Center)
        .push(version)
        .push(get_button_github())
        .push(made_by);

    Container::new(footer_row)
        .class(ContainerType::Footer)
        .height(30)
        .align_y(Vertical::Center)
        .padding(0)
}

fn get_button_github<'a>() -> Button<'a, MainWindowEvent> {
    Button::new(
        Icon::GitHub.to_text()
            .size(15.0)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .line_height(LineHeight::Relative(1.0)),
    )
        .class(ButtonType::Transparent)
        .on_press(MainWindowEvent::OpenWebPage("https://github.com/sh1zen/RustProject".parse().unwrap()))
}