use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use iced::widget::{Column, Container, Row, Space};
use iced::{Alignment, Length};
use iced_video_player::{Video};

pub fn client_page(app: &App, video_src: Option<Video>) -> Container<appMessage, StyleType> {
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
        .height(400)
        .align_items(Alignment::Center);

    let video = Container::new(Space::new(0, 0));

    let content = Column::new()
        .push(video)
        .spacing(8)
        .push(actions)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}