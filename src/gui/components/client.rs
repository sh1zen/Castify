use iced::{Alignment, Length};
use iced::widget::{Container, Row};

use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;

pub fn client_page(app: &App) -> Container<appMessage, StyleType> {
    let content = Row::new()
        .align_items(Alignment::Center).spacing(10)
        .push(
            FilledButton::new("Record")
                .icon(Icon::Download)
                .build()
                .on_press(appMessage::CloseRequested)
        )
        .push(
            FilledButton::new("Exit")
                .icon(Icon::Stop)
                .build()
                .on_press(appMessage::CloseRequested)
        )
        .height(400)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}