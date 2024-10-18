use crate::assets::FONT_FAMILY_BOLD;
use crate::config::app_version;
use crate::gui::common::icons::Icon;
use crate::gui::style::button::ButtonType;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{Button, Container, Row, Text};
use crate::gui::windows::main::MainWindowEvent;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::horizontal_space;
use iced::widget::text::LineHeight;
use iced::{Alignment, Length};

pub fn footer<'a>() -> Container<'a, MainWindowEvent> {
    let version = Row::new()
        .align_y(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill)
        .spacing(5)
        .push(
            Text::new(app_version()).font(FONT_FAMILY_BOLD).size(12.0)
        );

    let footer_row = Row::new()
        .padding([0, 10])
        .align_y(Alignment::Center)
        .push(version)
        .push(horizontal_space().width(Length::Shrink))
        .push(
            Button::new(
                Icon::Browser.to_text()
                    .size(15.0)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .line_height(LineHeight::Relative(1.0)),
            )
                .class(ButtonType::Transparent)
                .on_press(MainWindowEvent::OpenWebPage("https://github.com/sh1zen/RustProject".parse().unwrap()))
        )
        .push(horizontal_space().width(Length::Fill))
        .push(
            Button::new(
                Icon::Info.to_text()
                    .size(15.0)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .line_height(LineHeight::Relative(1.0)),
            )
                .class(ButtonType::Transparent)
                .on_press(MainWindowEvent::OpenInfo)
        );

    Container::new(footer_row)
        .class(ContainerType::Footer)
        .height(30)
        .align_y(Vertical::Center)
        .padding(0)
}