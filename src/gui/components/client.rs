use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::container::ContainerType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::gui::video::VideoPlayer;
use crate::workers;
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row};
use iced::{Alignment, Length};

pub fn client_page(app: &App) -> Container<appMessage, StyleType> {
    let actions = Row::new()
        .align_items(Alignment::Center).spacing(10)
        .push(
            if workers::save_stream::get_instance().lock().unwrap().is_saving {
                FilledButton::new("Stop")
                    .icon(Icon::Save)
                    .build()
                    .on_press(appMessage::SaveCaptureStop)
            } else {
                FilledButton::new("Save")
                    .icon(Icon::Save)
                    .build()
                    .on_press(appMessage::SaveCapture)
            }
        )
        .push(
            FilledButton::new("Exit")
                .icon(Icon::Stop)
                .build()
                .on_press(appMessage::CloseRequested)
        )
        .align_items(Alignment::Center);

    let video = Container::new(VideoPlayer::new(&app.video))
        .height(Length::Fill)
        .width(Length::Fill).align_x(Horizontal::Center)
        .style(ContainerType::Video);

    let content = Column::new()
        .spacing(20)
        .push(video)
        .push(actions)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
        .padding([0, 0, 10, 0])
}