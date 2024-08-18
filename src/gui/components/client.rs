use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::gui::video::VideoPlayer;
use iced::widget::{Column, Container, Row};
use iced::{Alignment, Length};

pub fn client_page(app: &App) -> Container<appMessage, StyleType> {
    let actions = Row::new()
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
        .align_items(Alignment::Center);

    let video = VideoPlayer::new(&app.video);

    let content = Column::new()
        .spacing(8)
        .push(video)
        .push(actions)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}