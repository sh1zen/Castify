use crate::gui::resource::APP_NAME;
use crate::gui::resource::APP_VERSION;
use crate::gui::resource::FONT_SIZE_FOOTER;
use crate::gui::theme::container::ContainerType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message;
use iced::advanced::widget::Text;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::LineHeight;
use iced::widget::{button, Button, Container, Row};
use iced::{Alignment, Length};
use local_ip_address::local_ip;
use crate::gui::theme::styles::buttons::ButtonType;

pub fn footer() -> Container<'static, Message, StyleType> {
    let made_by = Text::new("Made by:  A. Frolli  P. Bella  M. De Paola")
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Right)
        .size(FONT_SIZE_FOOTER);

    let version = Row::new()
        .align_items(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill)
        .spacing(5)
        .push(
            Text::new(format!("{APP_NAME} {APP_VERSION}"))
                .size(FONT_SIZE_FOOTER)
        )
        .push( match local_ip() {
            Ok(ip) => Text::new(ip.to_string()).size(FONT_SIZE_FOOTER + 1.0),
            Err(_) => Text::new(""),
        });

    let footer_row = Row::new()
        .padding([0, 10])
        .align_items(Alignment::Center)
        .push(version)
        .push(get_button_github())
        .push(made_by);

    Container::new(footer_row)
        .height(30)
        .align_y(Vertical::Center)
        .style(ContainerType::Standard)
        .padding(0)
}

fn get_button_github() -> Button<'static, Message, StyleType> {
    button(
        Icon::GitHub.to_text()
            .size(15.0)
            .horizontal_alignment(Horizontal::Center)
            .vertical_alignment(Vertical::Center)
            .line_height(LineHeight::Relative(1.0)),
    )
        .style(ButtonType::Transparent)
        .on_press(Message::OpenWebPage("https://github.com/sh1zen/RustProject".parse().unwrap()))
}